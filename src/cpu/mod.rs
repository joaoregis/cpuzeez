//! Processors, APICs, Processors Information and Thread dumps sections, plus
//! the data model the CPU and Caches tabs read (FEATURES.md 3.2-3.6, 4, 5).

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
mod arm;
pub mod clocks;
pub mod db;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86;

use crate::Ctx;
use crate::report::{Node, Section, f, g, gv, section};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct Cache {
    pub level: u32,
    pub kind: char,
    pub size: u64,
    pub ways: u32,
    pub line: u32,
    pub count: usize,
    pub sharing: u32,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Cluster {
    pub kind: String,
    pub cores: usize,
    pub threads: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Logical {
    pub cpu: usize,
    pub apic: u32,
    pub package: u32,
    pub core: u32,
    pub smt: u32,
    pub node: Option<u32>,
    pub ccd: Option<u32>,
    pub ccx: Option<u32>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Cpu {
    pub vendor: String,
    pub brand: String,
    pub name: String,
    pub info: db::Info,
    pub sig: (u32, u32, u32),
    pub cpuid: Option<String>,
    pub ext_cpuid: Option<String>,
    pub instructions: Vec<String>,
    pub microcode: Option<String>,
    pub caches: Vec<Cache>,
    pub sockets: usize,
    pub cores: usize,
    pub threads: usize,
    pub ccds: Option<usize>,
    pub clusters: Vec<Cluster>,
    pub max_leaf: Option<(u32, u32)>,
    pub hypervisor: Option<String>,
    pub base_mhz: Option<f64>,
    pub max_mhz: Option<f64>,
    pub bus_mhz: Option<f64>,
    pub logical: Vec<Logical>,
    /// Per-thread raw dumps (x86: CPUID leaves; plus MSRs when readable).
    pub dumps: Vec<Node>,
}

pub fn get(ctx: &Ctx) -> &Cpu {
    ctx.cpu.get_or_init(|| build(ctx))
}

fn build(ctx: &Ctx) -> Cpu {
    let mut c = Cpu::default();
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    build_x86(ctx, &mut c);
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    arm::build(&mut c);
    if c.microcode.is_none() {
        c.microcode = microcode();
    }
    if c.name.is_empty() {
        c.name = c.info.name.clone().unwrap_or_else(|| db::clean_brand(&c.brand));
    }
    // SMBIOS type 4 fills what CPUID doesn't say: socket, bus clock, rated max.
    if c.info.package.is_none() {
        c.info.package = crate::smbios::socket(ctx);
    }
    if let Some((ext, max, _)) = crate::smbios::processor_clocks(ctx) {
        c.bus_mhz = c.bus_mhz.or((ext > 0).then_some(ext as f64));
        c.max_mhz = c.max_mhz.or((max > 0).then_some(max as f64));
    }
    c
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn build_x86(ctx: &Ctx, c: &mut Cpu) {
    use x86::*;
    c.vendor = vendor();
    c.brand = brand().unwrap_or_default();
    let s = signature();
    c.sig = (s.family, s.model, s.stepping);
    c.cpuid = Some(format!("{:X}.{:X}.{:X}", s.base_family, s.base_model, s.stepping));
    c.ext_cpuid = Some(format!("{:X}.{:X}", s.family, s.model));
    c.instructions = instructions(&c.vendor);
    c.max_leaf = Some((max_leaf(), max_ext_leaf()));
    c.hypervisor = hypervisor();
    c.info = db::lookup(&c.vendor, s.family, s.model, s.stepping, &c.brand);
    let amd = c.vendor == "AuthenticAMD" || c.vendor == "HygonGenuine";

    // Pin to every logical CPU in turn, like CPU-Z, so APIC IDs, hybrid core
    // types and per-core caches come from the thread they describe.
    let n = crate::os::logical_cpus();
    let v = c.vendor.clone();
    let mut per: Vec<(usize, Topo, Vec<CacheDesc>, Vec<(u32, Regs)>)> = (0..n)
        .filter_map(|i| {
            let v = v.clone();
            crate::os::on_cpu(i, move || (topology(&v), caches(&v), dump_leaves())).map(|(t, ca, d)| (i, t, ca, d))
        })
        .collect();
    if per.is_empty() {
        // macOS (Intel): no pinning; describe the current thread only.
        per.push((0, topology(&v), caches(&v), dump_leaves()));
    }

    let mut cores = BTreeSet::new();
    let mut pkgs = BTreeSet::new();
    let mut dies = BTreeSet::new();
    let mut inst: BTreeMap<(u32, char, u64, u32, u32, u32), BTreeSet<u32>> = BTreeMap::new();
    let mut kinds: BTreeMap<String, (BTreeSet<(u32, u32)>, usize)> = BTreeMap::new();
    for (i, t, ca, _) in &per {
        let pkg = t.package();
        pkgs.insert(pkg);
        cores.insert((pkg, t.core()));
        let ccd = t.die_bits.map(|b| t.apic >> b);
        if let Some(d) = ccd {
            dies.insert((pkg, d));
        }
        let has_l3 = ca.iter().any(|x| x.level == 3);
        let kind = t.core_type.map(|ty| match ty {
            0x40 => "P-Cores",
            0x20 if has_l3 => "E-Cores",
            0x20 => "LP-Cores",
            _ => "Type unknown",
        });
        if let Some(k) = kind {
            let e = kinds.entry(k.to_string()).or_default();
            e.0.insert((pkg, t.core()));
            e.1 += 1;
        }
        c.logical.push(Logical {
            cpu: *i,
            apic: t.apic,
            package: pkg,
            core: t.core(),
            smt: t.smt(),
            node: t.node,
            ccd,
            ccx: t.ccx_bits.map(|b| t.apic >> b),
            kind: kind.map(str::to_string),
        });
        for d in ca {
            // One instance per distinct group of sharing threads.
            let shift = 32 - (d.sharing.max(1) - 1).leading_zeros();
            inst.entry((d.level, d.kind, d.size, d.ways, d.line, d.sharing)).or_default().insert(t.apic >> shift);
        }
    }
    c.sockets = pkgs.len().max(1);
    c.cores = cores.len();
    c.threads = per.len().max(n);
    if per.len() == 1 && n > 1 {
        // Unpinned fallback: trust the OS counts.
        c.cores = (n >> per[0].1.smt_bits).max(1);
    }
    if amd && !dies.is_empty() {
        c.ccds = Some(dies.len());
    }
    for (k, (cs, th)) in kinds {
        c.clusters.push(Cluster { kind: k, cores: cs.len(), threads: th });
    }
    // Order: L1D, L1I, L2, L3.
    let mut caches: Vec<Cache> = inst
        .into_iter()
        .map(|((level, kind, size, ways, line, sharing), ids)| Cache {
            level,
            kind,
            size,
            ways,
            line,
            count: ids.len(),
            sharing,
            note: None,
        })
        .collect();
    caches.sort_by_key(|x| {
        (
            x.level,
            match x.kind {
                'D' => 0,
                'I' => 1,
                _ => 2,
            },
            std::cmp::Reverse(x.size),
        )
    });
    c.caches = caches;

    if let Some((base, max, bus)) = intel_freqs() {
        c.base_mhz = Some(base as f64);
        c.max_mhz = Some(max as f64);
        c.bus_mhz = (bus > 0).then_some(bus as f64);
    }

    for (i, t, ca, leaves) in &per {
        c.dumps.push(thread_dump(ctx, *i, t, ca, leaves, amd));
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn thread_dump(
    ctx: &Ctx,
    cpu: usize,
    t: &x86::Topo,
    ca: &[x86::CacheDesc],
    leaves: &[(u32, x86::Regs)],
    amd: bool,
) -> Node {
    let mut n = vec![
        f("APIC ID", t.apic),
        f("Topology", format!("Processor ID {}, Core ID {}, Thread ID {}", t.package(), t.core(), t.smt())),
    ];
    if amd {
        n.push(f(
            "AMD Topology",
            format!(
                "Node {}, CCD {}, CCX {}, core {}, thread {}",
                t.node.unwrap_or(0),
                t.die_bits.map_or(0, |b| t.apic >> b),
                t.ccx_bits.map_or(0, |b| t.apic >> b),
                t.core(),
                t.smt()
            ),
        ));
    }
    if let Some(k) = t.core_type {
        n.push(f("Core Type", format!("0x{k:02X}")));
    }
    for d in ca {
        n.push(f(
            "Cache descriptor",
            format!("Level {}, {}, {}, {} thread(s)", d.level, d.kind, size_text(d.size), d.sharing),
        ));
    }
    let rows = leaves
        .iter()
        .map(|(l, r)| {
            f(format!("0x{l:08X}"), format!("0x{:08X}\t0x{:08X}\t0x{:08X}\t0x{:08X}", r[0], r[1], r[2], r[3]))
        })
        .collect();
    n.push(g("CPUID", rows));
    let msrs: Vec<Node> = interesting_msrs(amd)
        .iter()
        .filter_map(|&m| {
            ctx.hw.rdmsr(cpu, m).map(|v| f(format!("MSR 0x{m:08X}"), format!("0x{:08X}\t0x{:08X}", v >> 32, v as u32)))
        })
        .collect();
    if !msrs.is_empty() {
        n.push(g("MSR", msrs));
    }
    gv(format!("CPU Thread ({}, 0x{:X})", t.package(), cpu), "", n)
}

/// MSRs worth dumping per vendor (architectural ones from the SDM/PPR).
fn interesting_msrs(amd: bool) -> &'static [u32] {
    if amd {
        &[
            0x1B,
            0x8B,
            0xC001_0114,
            0xC001_0058,
            0xC001_0063,
            0xC001_0064,
            0xC001_0065,
            0xC001_0066,
            0xC001_0067,
            0xC001_0068,
            0xC001_0069,
            0xC001_006A,
            0xC001_006B,
            0xC001_0292,
            0xC001_0293,
            0xC001_0299,
            0xC001_029A,
            0xC001_029B,
            0xC001_02B0,
            0xC001_02B1,
        ]
    } else {
        &[
            0x1B, 0x17, 0x8B, 0xCE, 0x194, 0x198, 0x199, 0x1A0, 0x1A2, 0x1AD, 0x606, 0x610, 0x611, 0x619, 0x639, 0x64D,
            0x770, 0x771, 0x774,
        ]
    }
}

pub fn size_text(b: u64) -> String {
    if b >= 1 << 20 && b.is_multiple_of(1 << 20) {
        format!("{} MB", b >> 20)
    } else if b >= 1024 {
        format!("{} KB", b >> 10)
    } else {
        format!("{b} B")
    }
}

/// Linux sysfs cache description (used for ARM, and as a cross-check).
#[allow(dead_code)]
pub fn sysfs_caches(n: usize) -> Vec<Cache> {
    let mut map: BTreeMap<(u32, char, u64, u32, u32), BTreeSet<String>> = BTreeMap::new();
    let mut sharing: BTreeMap<(u32, char, u64, u32, u32), u32> = BTreeMap::new();
    for i in 0..n {
        for idx in 0..8 {
            let base = format!("/sys/devices/system/cpu/cpu{i}/cache/index{idx}");
            let Some(level) = crate::os::read_u64(format!("{base}/level")) else { break };
            let kind = match crate::os::read_trim(format!("{base}/type")).as_deref() {
                Some("Data") => 'D',
                Some("Instruction") => 'I',
                _ => 'U',
            };
            let size = crate::os::read_trim(format!("{base}/size")).and_then(|s| parse_size(&s)).unwrap_or(0);
            let ways = crate::os::read_u64(format!("{base}/ways_of_associativity")).unwrap_or(0) as u32;
            let line = crate::os::read_u64(format!("{base}/coherency_line_size")).unwrap_or(64) as u32;
            let shared = crate::os::read_trim(format!("{base}/shared_cpu_list")).unwrap_or_default();
            let key = (level as u32, kind, size, ways, line);
            *sharing.entry(key).or_default() = cpu_list_len(&shared) as u32;
            map.entry(key).or_default().insert(shared);
        }
    }
    let mut v: Vec<Cache> = map
        .into_iter()
        .map(|(k, s)| Cache {
            level: k.0,
            kind: k.1,
            size: k.2,
            ways: k.3,
            line: k.4,
            count: s.len(),
            sharing: sharing[&k],
            note: None,
        })
        .collect();
    v.sort_by_key(|x| {
        (
            x.level,
            match x.kind {
                'D' => 0,
                'I' => 1,
                _ => 2,
            },
        )
    });
    v
}

#[allow(dead_code)]
fn parse_size(s: &str) -> Option<u64> {
    let (num, mul) = match s.chars().last()? {
        'K' => (&s[..s.len() - 1], 1024),
        'M' => (&s[..s.len() - 1], 1 << 20),
        _ => (s, 1),
    };
    num.parse::<u64>().ok().map(|n| n * mul)
}

#[allow(dead_code)]
fn cpu_list_len(s: &str) -> usize {
    s.split(',')
        .filter(|p| !p.is_empty())
        .map(|p| match p.split_once('-') {
            Some((a, b)) => b.parse::<usize>().unwrap_or(0).saturating_sub(a.parse().unwrap_or(0)) + 1,
            None => 1,
        })
        .sum()
}

#[allow(dead_code)]
fn db_apple(brand: &str) -> Option<db::Info> {
    let info = db::lookup("Apple", 0, 0, 0, brand);
    info.codename.is_some().then_some(info)
}

fn microcode() -> Option<String> {
    #[cfg(windows)]
    {
        let v = crate::os::win::reg_u64(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0", "Update Revision")?;
        // Intel stores the revision in the high dword, AMD in the low one.
        let rev = if v >> 32 != 0 { v >> 32 } else { v & 0xFFFF_FFFF };
        return (rev != 0 && rev != 0xFFFF_FFFF).then(|| format!("0x{rev:X}"));
    }
    #[cfg(target_os = "linux")]
    {
        let s = std::fs::read_to_string("/proc/cpuinfo").ok()?;
        let v = s.lines().find(|l| l.starts_with("microcode"))?.split(':').nth(1)?.trim();
        let n = u64::from_str_radix(v.trim_start_matches("0x"), 16).ok()?;
        // Hypervisors (WSL2/Hyper-V) report all-ones instead of the real revision.
        return (n != 0xFFFF_FFFF).then(|| format!("0x{n:X}"));
    }
    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "x86_64")]
        return arm_free_sysctl("machdep.cpu.microcode_version");
    }
    #[allow(unreachable_code)]
    None
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn arm_free_sysctl(name: &str) -> Option<String> {
    let out = crate::os::cmd("sysctl", &["-n", name])?;
    let n: u64 = out.trim().parse().ok()?;
    Some(format!("0x{n:X}"))
}

// ---- AMD / Intel model-specific extras (privileged) ----

/// (FID, DID, VID, MHz, volts) for each enabled AMD P-state.
fn amd_pstates(ctx: &Ctx, family: u32, model: u32) -> Vec<(u64, u32, u32, u32, f64, f64)> {
    let mut v = Vec::new();
    if family < 0x17 {
        return v;
    }
    // Zen 4 (19h models 10-1F, 60-7F, A0-AF) and later use SVI3.
    let svi3 = family > 0x19 || (family == 0x19 && matches!(model, 0x10..=0x1F | 0x60..=0x7F | 0xA0..=0xAF));
    for i in 0..8 {
        let Some(r) = ctx.hw.rdmsr(0, 0xC001_0064 + i) else { break };
        if r >> 63 == 0 {
            continue;
        }
        let fid = (r & 0xFF) as u32;
        let did = ((r >> 8) & 0x3F) as u32;
        let vid = (((r >> 14) & 0xFF) | ((r >> 24) & 0x100)) as u32;
        let mhz = if did != 0 { fid as f64 * 200.0 / did as f64 } else { 0.0 };
        // SVI3 VIDs count up from 0 V, SVI2 VIDs count down from 1.55 V; both in 6.25 mV steps.
        let volts = if svi3 { vid as f64 * 0.00625 } else { 1.55 - vid as f64 * 0.00625 };
        v.push((r, fid | (did << 8), did, vid, mhz, volts));
    }
    v
}

/// Preferred (highest-performance) cores from ACPI CPPC.
fn preferred_cores(ctx: &Ctx, cpu: &Cpu) -> Option<Vec<u32>> {
    let mut perf: BTreeMap<u32, u64> = BTreeMap::new();
    for l in cpu.logical.iter().filter(|l| l.smt == 0) {
        let hp = {
            #[cfg(target_os = "linux")]
            {
                crate::os::read_u64(format!("/sys/devices/system/cpu/cpu{}/acpi_cppc/highest_perf", l.cpu))
            }
            #[cfg(not(target_os = "linux"))]
            {
                None::<u64>
            }
        }
        .or_else(|| ctx.hw.rdmsr(l.cpu, 0xC001_02B0).map(|r| (r >> 24) & 0xFF));
        perf.insert(l.core, hp?);
    }
    let best = *perf.values().max()?;
    if perf.values().all(|&p| p == best) {
        return None;
    }
    // Core index (#n) is the ordinal of the physical core, not its APIC-derived ID.
    Some(perf.iter().enumerate().filter(|(_, (_, p))| **p == best).map(|(i, _)| i as u32).collect())
}

fn mhz(v: f64) -> String {
    format!("{v:.1} MHz")
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let c = get(ctx);
    let groups = crate::os::cpu_groups();
    let mut proc = vec![f("CPU Groups", groups.len())];
    let mut start = 0u32;
    for (gidx, cnt) in &groups {
        let mask = if *cnt >= 64 { u64::MAX } else { (1u64 << cnt) - 1 };
        proc.push(f(format!("CPU Group {gidx}"), format!("{cnt} threads, mask=0x{mask:X}")));
        start += cnt;
    }
    let _ = start;
    proc.push(f("Number of sockets", c.sockets));
    proc.push(f("Number of threads", c.threads));

    vec![
        section("Processors", proc),
        section("APICs", apic_tree(c)),
        crate::timers::section(ctx),
        section("Processors Information", processors_info(ctx, c)),
        section("Thread dumps", c.dumps.clone()),
    ]
}

fn apic_tree(c: &Cpu) -> Vec<Node> {
    let mut out = Vec::new();
    let pkgs: BTreeSet<u32> = c.logical.iter().map(|l| l.package).collect();
    for (si, p) in pkgs.iter().enumerate() {
        let in_pkg: Vec<&Logical> = c.logical.iter().filter(|l| l.package == *p).collect();
        let mut core_ord = 0;
        let mut thread_ord = in_pkg.first().map_or(0, |l| l.cpu);
        let mut nodes_out = Vec::new();
        let nodes: BTreeSet<u32> = in_pkg.iter().map(|l| l.node.unwrap_or(0)).collect();
        for nd in nodes {
            let in_node: Vec<&&Logical> = in_pkg.iter().filter(|l| l.node.unwrap_or(0) == nd).collect();
            let ccds: BTreeSet<Option<u32>> = in_node.iter().map(|l| l.ccd).collect();
            let mut ccd_out = Vec::new();
            for (di, d) in ccds.iter().enumerate() {
                let in_ccd: Vec<&&&Logical> = in_node.iter().filter(|l| l.ccd == *d).collect();
                let ccxs: BTreeSet<Option<u32>> = in_ccd.iter().map(|l| l.ccx).collect();
                let mut ccx_out = Vec::new();
                for (xi, x) in ccxs.iter().enumerate() {
                    let mut core_out = Vec::new();
                    let cores: BTreeSet<u32> = in_ccd.iter().filter(|l| l.ccx == *x).map(|l| l.core).collect();
                    for co in cores {
                        let th: Vec<Node> = in_ccd
                            .iter()
                            .filter(|l| l.ccx == *x && l.core == co)
                            .map(|l| {
                                let t = f(format!("-- Thread {thread_ord}"), l.apic);
                                thread_ord += 1;
                                t
                            })
                            .collect();
                        let kind = in_ccd.iter().find(|l| l.core == co).and_then(|l| l.kind.clone());
                        let label = match kind {
                            Some(k) if !c.clusters.is_empty() && c.vendor != "AuthenticAMD" => {
                                format!("-- Core {core_ord} (ID {co}) {k}")
                            }
                            _ => format!("-- Core {core_ord} (ID {co})"),
                        };
                        core_out.push(g(label, th));
                        core_ord += 1;
                    }
                    if x.is_some() {
                        ccx_out.push(g(format!("-- CCX {xi}"), core_out));
                    } else {
                        ccx_out.extend(core_out);
                    }
                }
                if d.is_some() {
                    ccd_out.push(g(format!("-- CCD {di}"), ccx_out));
                } else {
                    ccd_out.extend(ccx_out);
                }
            }
            nodes_out.push(g(format!("-- Node {nd}"), ccd_out));
        }
        out.push(g(format!("Socket {si}"), nodes_out));
    }
    out
}

fn cache_line(c: &Cache) -> String {
    let ways = if c.ways == 0 { "fully assoc.".to_string() } else { format!("{}-way", c.ways) };
    let size = if c.size >= 4 << 20 && c.size.is_multiple_of(1 << 20) {
        format!("{} MB", c.size >> 20)
    } else {
        format!("{} KB", c.size >> 10)
    };
    let note = c.note.as_ref().map_or(String::new(), |n| format!(", {n}"));
    format!("{} x {size} ({ways}, {}-byte line{note})", c.count, c.line)
}

fn processors_info(ctx: &Ctx, c: &Cpu) -> Vec<Node> {
    let mut n = Vec::new();
    let cores_per = c.cores / c.sockets.max(1);
    let threads_per = c.threads / c.sockets.max(1);
    let mut s = vec![
        f("Number of cores", format!("{cores_per} (max {cores_per})")),
        f("Number of threads", format!("{threads_per} (max {threads_per})")),
    ];
    for cl in &c.clusters {
        s.push(f("Core Set", format!("{}, {} cores, {} threads", cl.kind, cl.cores, cl.threads)));
    }
    if let Some(d) = c.ccds {
        s.push(f("Number of CCDs", d));
    }
    s.push(f("Manufacturer", &c.vendor));
    s.push(f("Name", &c.name));
    if let Some(v) = &c.info.codename {
        s.push(f("Codename", v));
    }
    s.push(f("Specification", &c.brand));
    if let Some(v) = &c.info.package {
        s.push(f("Package", v));
    }
    if let Some(v) = &c.cpuid {
        s.push(f("CPUID", v));
    }
    if let Some(v) = &c.ext_cpuid {
        s.push(f("Extended CPUID", v));
    }
    if let Some(v) = &c.info.core_stepping {
        s.push(f("Core Stepping", v));
    }
    if let Some(v) = &c.info.technology {
        s.push(f("Technology", v));
    }
    if let Some(v) = c.info.tdp_w {
        s.push(f("TDP Limit", format!("{v:.1} Watts")));
    }
    if let Some(v) = c.info.tjmax_c {
        s.push(f("Tjmax", format!("{v:.1} °C")));
    }
    if let Some((m, src)) = clocks::core_mhz(ctx, 0) {
        s.push(f("Core Speed", format!("{} ({})", mhz(m), src.label())));
        let bus = c.bus_mhz.unwrap_or(100.0);
        s.push(f("Multiplier x Bus Speed", format!("{:.1} x {bus:.1} MHz", m / bus)));
    }
    // CPU-Z's "base frequency" is the core reference clock (BCLK).
    s.push(f("Base frequency (cores)", mhz(c.bus_mhz.unwrap_or(100.0))));
    if let Some(b) = c.base_mhz {
        s.push(f("Rated frequency", mhz(b)));
    }
    s.push(f("Instructions sets", c.instructions.join(", ")));
    if let Some(v) = &c.microcode {
        s.push(f("Microcode Revision", v));
    }
    let names = ["L1 Data cache", "L1 Instruction cache"];
    for ca in &c.caches {
        let label = match (ca.level, ca.kind) {
            (1, 'D') => names[0].to_string(),
            (1, 'I') => names[1].to_string(),
            (l, _) => format!("L{l} cache"),
        };
        s.push(f(label, cache_line(ca)));
    }
    if let Some(p) = preferred_cores(ctx, c) {
        let list: Vec<String> = p.iter().map(|i| format!("#{i}")).collect();
        s.push(f("Preferred cores", format!("{} ({})", p.len(), list.join(", "))));
    }
    if let Some((std, ext)) = c.max_leaf {
        s.push(f("Max CPUID level", format!("{std:08X}h")));
        s.push(f("Max CPUID ext. level", format!("{ext:08X}h")));
    }
    if let Some(h) = &c.hypervisor {
        s.push(f("Hypervisor", h));
    }
    if c.vendor == "AuthenticAMD" || c.vendor == "HygonGenuine" {
        let ps = amd_pstates(ctx, c.sig.0, c.sig.1);
        if !ps.is_empty() {
            s.push(f("FID/VID Control", "yes"));
            s.push(f("# of P-States", ps.len()));
            for (_, fiddid, _, vid, m, v) in &ps {
                s.push(f("P-State", format!("FID 0x{fiddid:X} - VID 0x{vid:X} ({:.2}x - {v:.3} V)", m / 100.0)));
            }
        }
    }
    s.extend(crate::sensors::cpu_nodes(ctx));
    n.push(gv("Socket 1", "ID = 0", s));
    n
}

/// `-core=N`: clock of one core, then exit.
pub fn print_core_clock(ctx: &Ctx, id: usize) {
    let c = get(ctx);
    let cpu = c.logical.iter().filter(|l| l.smt == 0).nth(id).map_or(id, |l| l.cpu);
    match clocks::core_mhz(ctx, cpu) {
        Some((m, src)) => println!("Core #{id}: {} ({})", mhz(m), src.label()),
        None => eprintln!("core #{id}: no clock source"),
    }
}

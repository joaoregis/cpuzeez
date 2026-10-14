//! CPU package/core temperatures, voltage, power and clocks.
//! Intel: IA32_THERM_STATUS (0x19C), IA32_PACKAGE_THERM_STATUS (0x1B1),
//! MSR_TEMPERATURE_TARGET (0x1A2), IA32_PERF_STATUS (0x198), RAPL
//! (0x606 units, 0x610 limits, 0x611/0x639/0x641/0x619/0x64D energy, 0x614 TDP), SDM vol. 4.
//! AMD Zen: SMN THM_TCON_CUR_TMP (0x59800) and per-CCD temperature registers,
//! RAPL MSRs 0xC0010299/0xC001029A/0xC001029B, HW P-state status 0xC0010293 (PPR),
//! and the SMU PM table (see amd_pm.rs) for per-core/L3/SoC data, rails and limits.

use super::amd_pm::{self, FabricClocks, PmData};
use super::{Kind, Monitor};
use crate::Ctx;
use crate::cpu::Cpu;
use crate::report::{Node, f};
use std::sync::Mutex;
use std::time::{Duration, Instant};

fn is_amd(c: &Cpu) -> bool {
    c.vendor == "AuthenticAMD" || c.vendor == "HygonGenuine"
}

/// First logical CPU of every physical core, in core order.
fn core_threads(c: &Cpu) -> Vec<usize> {
    let v: Vec<usize> = c.logical.iter().filter(|l| l.smt == 0).map(|l| l.cpu).collect();
    if v.is_empty() { (0..c.cores.max(1)).collect() } else { v }
}

pub fn monitor(ctx: &Ctx) -> Option<Monitor> {
    let c = crate::cpu::get(ctx);
    let mut m = Monitor::new(if c.name.is_empty() { "CPU" } else { c.name.as_str() });
    let cores = core_threads(c);
    let mut pm = None;
    if is_amd(c) {
        pm = amd(ctx, c, &cores, &mut m);
    } else if c.vendor == "GenuineIntel" {
        intel(ctx, &cores, &mut m);
    }
    #[cfg(target_os = "linux")]
    if !m.readings.iter().any(|r| r.kind == Kind::Power) {
        powercap(&mut m);
    }
    for (i, mhz) in core_clocks(ctx, &cores).into_iter().enumerate() {
        if let Some(v) = mhz {
            m.push(Kind::Clock, format!("Core #{i}"), v, None);
        }
    }
    for (i, l) in pm.iter().flat_map(|d| &d.l3).enumerate() {
        if let Some(v) = l.mhz {
            m.push(Kind::Clock, format!("L3 Cache #{i}"), v, None);
        }
    }
    (!m.readings.is_empty()).then_some(m)
}

/// Enabled cores per L3 domain (CCX, or CCD on Zen 3+), in order; empty when
/// the topology doesn't say.
fn l3_groups(c: &Cpu) -> Vec<usize> {
    let mut m = std::collections::BTreeMap::new();
    for l in c.logical.iter().filter(|l| l.smt == 0) {
        if l.ccd.is_none() && l.ccx.is_none() {
            return Vec::new();
        }
        *m.entry((l.package, l.ccd, l.ccx)).or_insert(0usize) += 1;
    }
    m.into_values().collect()
}

/// Raw SMU PM table, re-read at most every 250 ms: one report (or GUI tick)
/// sees one snapshot, and the SMU isn't asked to refresh it per row.
fn pm_raw(ctx: &Ctx) -> Option<(u32, Vec<f32>)> {
    static LAST: Mutex<Option<(Instant, Option<(u32, Vec<f32>)>)>> = Mutex::new(None);
    if !is_amd(crate::cpu::get(ctx)) {
        return None;
    }
    let mut g = LAST.lock().ok()?;
    if !matches!(&*g, Some((t, _)) if t.elapsed() < Duration::from_millis(250)) {
        *g = Some((Instant::now(), ctx.hw.amd_smu_pm_table()));
    }
    g.as_ref()?.1.clone()
}

/// Peak effective clock (MHz) of every core seen this session.
static PEAK_EFF: Mutex<Vec<f64>> = Mutex::new(Vec::new());

/// Decoded PM table; None for unknown table versions.
fn pm(ctx: &Ctx) -> Option<PmData> {
    let (ver, t) = pm_raw(ctx)?;
    let d = amd_pm::decode(ver, &t, &l3_groups(crate::cpu::get(ctx)))?;
    if let Ok(mut p) = PEAK_EFF.lock() {
        let n = p.len().max(d.cores.len());
        p.resize(n, 0.0);
        for (p, c) in p.iter_mut().zip(&d.cores) {
            *p = p.max(c.eff_mhz.unwrap_or(0.0));
        }
    }
    Some(d)
}

/// FCLK/UCLK/MCLK from the SMU PM table.
pub fn fabric_clocks(ctx: &Ctx) -> Option<FabricClocks> {
    pm(ctx).map(|d| d.clocks).filter(|c| c.fclk.or(c.uclk).or(c.mclk).is_some())
}

/// "Core N max ratio  (effective) R": peak effective clock over BCLK, in
/// CPU-Z's quarter steps. Goes after the sensor rows in Processors Information.
pub fn ratio_fields(ctx: &Ctx) -> Vec<Node> {
    let c = crate::cpu::get(ctx);
    if pm(ctx).is_none_or(|d| d.cores.iter().all(|c| c.eff_mhz.is_none())) {
        return Vec::new();
    }
    let bclk = crate::cpu::bus_mhz(c).filter(|b| *b > 0.0).unwrap_or(100.0);
    // Like CPU-Z: what each core reaches when it alone is loaded (APERF/MPERF
    // over a short busy loop). Without MSR access, the session's PM peaks.
    let loaded: Vec<f64> = c
        .logical
        .iter()
        .filter(|l| l.smt == 0)
        .map_while(|l| {
            // A delayed MSR read can skew one window; retry readings above Fmax.
            let fmax = pm(ctx).and_then(|d| d.max_mhz).or(c.max_mhz).unwrap_or(f64::MAX);
            (0..3)
                .filter_map(|_| crate::cpu::clocks::loaded_mhz(ctx, l.cpu, std::time::Duration::from_millis(150)))
                .find(|m| *m <= fmax * 1.02 && *m > 100.0)
        })
        .collect();
    let peaks = if loaded.len() == c.cores { loaded } else { PEAK_EFF.lock().map(|p| p.clone()).unwrap_or_default() };
    peaks
        .iter()
        .enumerate()
        .map(|(i, mhz)| {
            f(format!("Core {i} max ratio"), format!("(effective) {:.2}", (mhz / bclk * 4.0).round() / 4.0))
        })
        .collect()
}

/// CPU-Z's "AMD Ryzen Power Management" monitor: SMU PM table rails, PPT
/// limit and temperatures (all numbered 0, as CPU-Z does), then the other
/// decoded values, and the raw table under "Hardware registers".
pub fn pm_monitor(ctx: &Ctx) -> Option<Monitor> {
    let (ver, t) = pm_raw(ctx)?;
    let mut m = Monitor::new("AMD Ryzen Power Management");
    if let Some(d) = pm(ctx) {
        for (label, v) in [("CPU VDDCR_VDD", d.vddcr_vdd), ("CPU VDDCR_SOC", d.vddcr_soc), ("CPU VDD_MISC", d.vdd_misc)]
        {
            if let Some(v) = v {
                m.push(Kind::Voltage, label, v, None);
            }
        }
        if let Some(p) = d.ppt {
            m.push(Kind::Power, "PPT Limit", p.limit, Some(p.limit as u32));
        }
        let temps = d.cores.iter().enumerate().map(|(i, c)| (format!("Core #{i}"), Some(c.temp)));
        let l3 = d.l3.iter().enumerate().map(|(i, l)| (format!("L3 Cache #{i}"), l.temp));
        for (label, v) in temps.chain(l3).chain([("SoC".to_string(), d.soc_temp)]) {
            if let Some(v) = v {
                m.push_ch(Kind::Temperature, 0, label, v, v as u32);
            }
        }
        // Not in CPU-Z's monitor; here so telemetry sees every decoded value.
        let val = |l: Option<amd_pm::Limit>| l.and_then(|l| l.value);
        let extra = [
            (Kind::Power, "PPT", val(d.ppt)),
            (Kind::Power, "Socket (SMU)", d.socket_power),
            (Kind::Power, "Core rail (SMU)", d.cores_power),
            (Kind::Current, "TDC Limit", d.tdc.map(|l| l.limit)),
            (Kind::Current, "TDC", val(d.tdc)),
            (Kind::Current, "EDC Limit", d.edc.map(|l| l.limit)),
            (Kind::Current, "EDC", val(d.edc)),
            (Kind::Temperature, "THM Limit", d.thm.map(|l| l.limit)),
            (Kind::Temperature, "THM", val(d.thm)),
            (Kind::Voltage, "VID", d.vid),
            (Kind::Clock, "FCLK", d.clocks.fclk),
            (Kind::Clock, "UCLK", d.clocks.uclk),
            (Kind::Clock, "MCLK", d.clocks.mclk),
        ];
        for (k, label, v) in extra {
            if let Some(v) = v {
                m.push(k, label, v, None);
            }
        }
        for (i, c) in d.cores.iter().enumerate() {
            m.push(Kind::Power, format!("Core #{i}"), c.power, None);
            m.push(Kind::Voltage, format!("Core #{i}"), c.volt, None);
            if let Some(v) = c.eff_mhz {
                m.push(Kind::Clock, format!("Core #{i} (effective)"), v, None);
            }
        }
        for (i, l) in d.l3.iter().enumerate() {
            if let Some(v) = l.mhz {
                m.push(Kind::Clock, format!("L3 Cache #{i}"), v, None);
            }
        }
    }
    m.registers.push(f("PMT version", format!("0x{ver:X}")));
    for (i, row) in t.chunks(10).enumerate() {
        let vals: Vec<String> = row.iter().map(|v| format!("{v:7.2}")).collect();
        m.registers.push(f(format!("{:03}", i * 10), vals.join(" ")));
    }
    Some(m)
}

/// Extra "Processors Information" fields the CPU database could not supply.
pub fn fields(ctx: &Ctx) -> Vec<Node> {
    let c = crate::cpu::get(ctx);
    let mut v = Vec::new();
    let hw = &ctx.hw;
    if is_amd(c) {
        if let Some(s) = hw.amd_smu_version() {
            v.push(f("SMU Version", format!("{}.{}.{}", (s >> 16) & 0xFF, (s >> 8) & 0xFF, s & 0xFF)));
        }
        if let Some((ver, _)) = pm_raw(ctx) {
            v.push(f("PMTV", format!("{ver:08X}")));
        }
        let Some(d) = pm(ctx) else { return v };
        if let Some(x) = d.max_mhz {
            v.push(f("Max Frequency", format!("{x:.0} MHz")));
        }
        // PBO scalar: not shown — the candidate PM cell changes at run time,
        // so it isn't the configured scalar.
        // CPU-Z adds ", fused : N"; the fused limits need an SMU mailbox call we don't make.
        let lims = [
            ("Package Power Tracking (PPT)", d.ppt, "W"),
            ("Thermal Design Current (TDC)", d.tdc, "A"),
            ("Electrical Design Current (EDC)", d.edc, "A"),
        ];
        for (k, l, u) in lims {
            if let Some(l) = l {
                v.push(f(k, format!("Current : {:.1} {u}", l.limit)));
            }
        }
        return v;
    }
    if c.vendor != "GenuineIntel" {
        return v;
    }
    if c.info.tdp_w.is_none() {
        let unit = hw.rdmsr(0, 0x606).map(|u| 1.0 / (1u64 << (u & 0xF)) as f64);
        if let (Some(u), Some(p)) = (unit, hw.rdmsr(0, 0x614)) {
            let w = (p & 0x7FFF) as f64 * u;
            if w > 0.0 {
                v.push(f("TDP Limit", format!("{w:.1} Watts")));
            }
        }
    }
    if c.info.tjmax_c.is_none()
        && let Some(t) = intel_tjmax(ctx)
    {
        v.push(f("Tjmax", format!("{t:.1} degC")));
    }
    v
}

fn intel_tjmax(ctx: &Ctx) -> Option<f64> {
    let t = (ctx.hw.rdmsr(0, 0x1A2)? >> 16) & 0xFF;
    Some(if t == 0 { 100.0 } else { t as f64 })
}

fn intel(ctx: &Ctx, cores: &[usize], m: &mut Monitor) {
    let hw = &ctx.hw;
    if let Some(tj) = intel_tjmax(ctx) {
        if let Some(p) = hw.rdmsr(0, 0x1B1) {
            let d = ((p >> 16) & 0x7F) as u32;
            m.push(Kind::Temperature, "Package", tj - d as f64, Some(d));
        }
        for (i, &cpu) in cores.iter().enumerate() {
            if let Some(s) = hw.rdmsr(cpu, 0x19C).filter(|s| s >> 31 & 1 == 1) {
                let d = ((s >> 16) & 0x7F) as u32;
                m.push(Kind::Temperature, format!("Core #{i}"), tj - d as f64, Some(d));
            }
        }
    }
    if let Some(v) = hw.rdmsr(0, 0x198).map(|s| ((s >> 32) & 0xFFFF) as u32).filter(|&v| v != 0) {
        m.push(Kind::Voltage, "VID", v as f64 / 8192.0, None);
    }
    let Some(units) = hw.rdmsr(0, 0x606) else { return };
    let ej = 1.0 / (1u64 << ((units >> 8) & 0x1F)) as f64;
    let doms = [(0x611u32, "Package"), (0x639, "IA Cores"), (0x641, "GT"), (0x619, "DRAM"), (0x64D, "Platform")];
    let read = |i: usize| hw.rdmsr(0, doms[i].0).map(|e| e & 0xFFFF_FFFF).filter(|&e| e != 0);
    let ctrs: Vec<_> =
        doms.iter().enumerate().map(|(i, (r, _))| (format!("intel:{r:X}"), ej, 1u64 << 32, read(i))).collect();
    for (w, (_, label)) in super::watts(&ctrs, read).into_iter().zip(doms) {
        if let Some(w) = w {
            m.push(Kind::Power, label, w, None);
        }
    }
    if let Some(l) = hw.rdmsr(0, 0x610) {
        let pu = 1.0 / (1u64 << (units & 0xF)) as f64;
        for (label, raw) in [("PL1", l & 0x7FFF), ("PL2", (l >> 32) & 0x7FFF)] {
            if raw != 0 {
                m.push(Kind::Power, label, raw as f64 * pu, Some(raw as u32));
            }
        }
    }
}

/// Fused Tctl offset of the first Zen parts (public k10temp facts).
fn tctl_offset(brand: &str) -> f64 {
    const T: [(&str, f64); 6] = [
        ("1600X", 20.0),
        ("1700X", 20.0),
        ("1800X", 20.0),
        ("2700X", 10.0),
        ("Threadripper 19", 27.0),
        ("Threadripper 29", 27.0),
    ];
    T.iter().find(|(k, _)| brand.contains(k)).map_or(0.0, |t| t.1)
}

/// SMN base of the per-CCD temperature registers.
fn ccd_base(fam: u32, model: u32) -> Option<u32> {
    match (fam, model) {
        (0x17, 0x31 | 0x71) | (0x19, 0x00..=0x0F | 0x20..=0x2F) => Some(0x59954),
        (0x19, 0x10..=0x1F | 0x60..=0x6F | 0xA0..=0xAF) | (0x1A, 0x00..=0x2F | 0x40..=0x4F) => Some(0x59B08),
        _ => None,
    }
}

/// Pushes CPU-Z's AMD rows in its order; returns the PM table for the clock rows.
fn amd(ctx: &Ctx, c: &Cpu, cores: &[usize], m: &mut Monitor) -> Option<PmData> {
    let hw = &ctx.hw;
    let (fam, model) = (c.sig.0, c.sig.1);
    if fam < 0x17 {
        return None;
    }
    if let Some(v) = hw.amd_smn_read(0x59800) {
        let raw = v >> 21;
        let mut t = raw as f64 * 0.125;
        if v & (1 << 19) != 0 {
            t -= 49.0;
        }
        m.push(Kind::Temperature, "Package", t - tctl_offset(&c.brand), Some(raw));
    }
    if let Some(base) = ccd_base(fam, model) {
        let n = c.ccds.unwrap_or(8) as u32;
        let mut k = 0;
        for i in 0..n.min(12) {
            let Some(v) = hw.amd_smn_read(base + 4 * i).filter(|v| v & 0x800 != 0) else { continue };
            m.push(Kind::Temperature, format!("CCD #{k}"), (v & 0x7FF) as f64 * 0.125 - 49.0, Some(v & 0x7FF));
            k += 1;
        }
    }
    let pm = pm(ctx);
    if let Some(d) = &pm {
        if let Some(max) = d.cores.iter().map(|c| c.temp).reduce(f64::max) {
            m.push(Kind::Temperature, "Cores (Max)", max, None);
        }
        for (i, c) in d.cores.iter().enumerate() {
            m.push(Kind::Temperature, format!("Core #{i}"), c.temp, None);
        }
        for (i, l) in d.l3.iter().enumerate() {
            if let Some(t) = l.temp {
                m.push(Kind::Temperature, format!("L3 Cache #{i}"), t, None);
            }
        }
        if let Some(t) = d.soc_temp {
            m.push(Kind::Temperature, "SoC", t, None);
        }
    }
    if let Some(s) = hw.rdmsr(0, 0xC001_0293) {
        let vid = ((s >> 14) & 0xFF) as u32;
        // Zen 4+ (SVI3) VIDs count up from 0 V, SVI2 down from 1.55 V; 6.25 mV steps.
        let svi3 = fam > 0x19 || (fam == 0x19 && matches!(model, 0x10..=0x1F | 0x60..=0x7F | 0xA0..=0xAF));
        let v = if svi3 { vid as f64 * 0.00625 } else { 1.55 - vid as f64 * 0.00625 };
        if v > 0.0 {
            m.push(Kind::Voltage, "VID", v, Some(vid));
        }
    }
    if let Some(d) = &pm {
        if !m.readings.iter().any(|r| r.label == "VID")
            && let Some(v) = d.vid
        {
            m.push(Kind::Voltage, "VID", v, None);
        }
        for (label, v) in [("CPU VDDCR_VDD", d.vddcr_vdd), ("CPU VDDCR_SOC", d.vddcr_soc), ("CPU VDD_MISC", d.vdd_misc)]
        {
            if let Some(v) = v {
                m.push(Kind::Voltage, label, v, None);
            }
        }
    }
    if let Some(units) = hw.rdmsr(0, 0xC001_0299) {
        amd_rapl(ctx, units, cores, m);
    }
    if let Some(d) = &pm {
        // No RAPL (e.g. Linux without the msr module): the SMU's own figures.
        if !m.readings.iter().any(|r| r.kind == Kind::Power) {
            if let Some(p) = d.socket_power.filter(|p| *p > 0.0) {
                m.push(Kind::Power, "Package", p, None);
            }
            if let Some(p) = d.cores_power.filter(|p| *p > 0.0) {
                m.push(Kind::Power, "Cores", p, None);
            }
            for (i, c) in d.cores.iter().enumerate() {
                m.push(Kind::Power, format!("Core #{i}"), c.power, None);
            }
        }
        if let Some(p) = d.ppt {
            m.push(Kind::Power, "PPT Limit", p.limit, Some(p.limit as u32));
        }
    }
    pm
}

/// AMD SMU PPT (W), TDC (A), EDC (A) as (limit, value) for telemetry/throttle detection.
pub fn amd_limits(ctx: &Ctx) -> Option<[(f64, f64); 3]> {
    if !is_amd(crate::cpu::get(ctx)) {
        return None;
    }
    let d = pm(ctx)?;
    let lv = |l: Option<crate::sensors::amd_pm::Limit>| l.map(|l| (l.limit, l.value.unwrap_or(0.0)));
    Some([lv(d.ppt)?, lv(d.tdc)?, lv(d.edc)?])
}

fn amd_rapl(ctx: &Ctx, units: u64, cores: &[usize], m: &mut Monitor) {
    let hw = &ctx.hw;
    let ej = 1.0 / (1u64 << ((units >> 8) & 0x1F)) as f64;
    // Counter 0 = package, then one per core.
    let read = |i: usize| {
        match i {
            0 => hw.rdmsr(0, 0xC001_029B),
            _ => hw.rdmsr(cores[i - 1], 0xC001_029A),
        }
        .map(|e| e & 0xFFFF_FFFF)
    };
    let ctrs: Vec<_> = (0..=cores.len()).map(|i| (format!("amd:{i}"), ej, 1u64 << 32, read(i))).collect();
    let w = super::watts(&ctrs, read);
    if let Some(p) = w[0] {
        m.push(Kind::Power, "Package", p, None);
    }
    let per: Vec<Option<f64>> = w[1..].to_vec();
    if per.iter().any(Option::is_some) {
        m.push(Kind::Power, "Cores", per.iter().flatten().sum(), None);
        for (i, p) in per.iter().enumerate() {
            if let Some(p) = p {
                m.push(Kind::Power, format!("Core #{i}"), *p, None);
            }
        }
    }
}

/// Linux powercap RAPL zones (also present on AMD); energy_uj is root-only
/// on current kernels, so this is often empty without privileges.
#[cfg(target_os = "linux")]
fn powercap(m: &mut Monitor) {
    let Ok(rd) = std::fs::read_dir("/sys/class/powercap") else { return };
    let mut zones: Vec<_> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("intel-rapl:")))
        .collect();
    zones.sort();
    let read = |i: usize| crate::os::read_u64(zones[i].join("energy_uj"));
    let ctrs: Vec<_> = zones
        .iter()
        .enumerate()
        .map(|(i, z)| {
            let wrap = crate::os::read_u64(z.join("max_energy_range_uj")).unwrap_or(u64::MAX);
            (format!("powercap:{}", z.display()), 1e-6, wrap, read(i))
        })
        .collect();
    for (z, w) in zones.iter().zip(super::watts(&ctrs, read)) {
        let Some(w) = w else { continue };
        let name = crate::os::read_trim(z.join("name")).unwrap_or_default();
        let label = match name.as_str() {
            n if n.starts_with("package") => "Package",
            "core" => "Cores",
            "uncore" => "GT",
            "dram" => "DRAM",
            "psys" => "Platform",
            n => n,
        };
        m.push(Kind::Power, label, w, None);
    }
}

/// Current MHz of each physical core: APERF/MPERF when MSRs are readable,
/// else the OS's own figure.
fn core_clocks(ctx: &Ctx, cores: &[usize]) -> Vec<Option<f64>> {
    if ctx.hw.rdmsr(0, 0xE8).is_some() {
        return cores.iter().map(|&c| crate::cpu::clocks::core_mhz(ctx, c).map(|x| x.0)).collect();
    }
    #[cfg(target_os = "linux")]
    return cores
        .iter()
        .map(|c| {
            crate::os::read_u64(format!("/sys/devices/system/cpu/cpu{c}/cpufreq/scaling_cur_freq"))
                .filter(|&k| k > 0)
                .map(|k| k as f64 / 1000.0)
        })
        .collect();
    #[cfg(windows)]
    {
        let per = super::windows::cpu_mhz();
        return cores.iter().map(|&c| per.get(c).copied().flatten()).collect();
    }
    #[allow(unreachable_code)]
    Vec::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn amd_tables() {
        assert_eq!(super::tctl_offset("AMD Ryzen 7 1800X Eight-Core Processor"), 20.0);
        assert_eq!(super::tctl_offset("AMD Ryzen 9 7900X 12-Core Processor"), 0.0);
        assert_eq!(super::ccd_base(0x19, 0x61), Some(0x59B08));
        assert_eq!(super::ccd_base(0x19, 0x21), Some(0x59954));
        assert_eq!(super::ccd_base(0x19, 0x50), None);
    }

    /// Manual, elevated on an AMD machine (PawnIO or ryzen_smu):
    /// `cargo test live_pm_table -- --ignored --nocapture`. Also writes
    /// %TEMP%/cpuzeez-pm-live.txt to diff against CPU-Z's report.
    #[test]
    #[ignore]
    fn live_pm_table() {
        use crate::report::{g, section, to_txt};
        use std::fmt::Write;
        let ctx = crate::Ctx::new(crate::config::Config::default());
        let c = crate::cpu::get(&ctx);
        let groups = super::l3_groups(c);
        let mut s = format!("{} / {}\nL3 groups {groups:?}\n", ctx.hw.name(), c.brand);
        match ctx.hw.amd_smu_pm_table() {
            Some((v, t)) => {
                let _ = writeln!(s, "PM table {v:#x}, {} cells", t.len());
                let _ = writeln!(s, "{:#?}", super::amd_pm::decode(v, &t, &groups));
            }
            None => s += "no PM table (needs admin + PawnIO, or root + ryzen_smu)\n",
        }
        let mut secs = vec![section("Processors Information", super::super::cpu_nodes(&ctx))];
        if let Some(m) = super::pm_monitor(&ctx) {
            let mut rows = super::super::reading_rows(&m.readings);
            rows.push(g("Hardware registers", m.registers));
            secs.push(section(m.name, rows));
        }
        s += &to_txt(&secs);
        println!("{s}");
        let _ = std::fs::write(std::env::temp_dir().join("cpuzeez-pm-live.txt"), s);
    }
}

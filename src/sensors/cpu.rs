//! CPU package/core temperatures, voltage, power and clocks.
//! Intel: IA32_THERM_STATUS (0x19C), IA32_PACKAGE_THERM_STATUS (0x1B1),
//! MSR_TEMPERATURE_TARGET (0x1A2), IA32_PERF_STATUS (0x198), RAPL
//! (0x606 units, 0x610 limits, 0x611/0x639/0x641/0x619/0x64D energy, 0x614 TDP), SDM vol. 4.
//! AMD Zen: SMN THM_TCON_CUR_TMP (0x59800) and per-CCD temperature registers,
//! RAPL MSRs 0xC0010299/0xC001029A/0xC001029B, HW P-state status 0xC0010293 (PPR).

use super::{Kind, Monitor};
use crate::Ctx;
use crate::cpu::Cpu;
use crate::report::{Node, f};

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
    if is_amd(c) {
        amd(ctx, c, &cores, &mut m);
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
    (!m.readings.is_empty()).then_some(m)
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
        if let Some(l) = hw.amd_smu_pm_table().and_then(|(ver, t)| raphael(ver, &t)) {
            v.push(f("Package Power Tracking (PPT)", format!("Current : {:.1} W", l.ppt.0)));
            v.push(f("Thermal Design Current (TDC)", format!("Current : {:.1} A", l.tdc.0)));
            v.push(f("Electrical Design Current (EDC)", format!("Current : {:.1} A", l.edc.0)));
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

fn amd(ctx: &Ctx, c: &Cpu, cores: &[usize], m: &mut Monitor) {
    let hw = &ctx.hw;
    let (fam, model) = (c.sig.0, c.sig.1);
    if fam < 0x17 {
        return;
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
    if let Some(s) = hw.rdmsr(0, 0xC001_0293) {
        let vid = ((s >> 14) & 0xFF) as u32;
        // Zen 4+ (SVI3) VIDs count up from 0 V, SVI2 down from 1.55 V; 6.25 mV steps.
        let svi3 = fam > 0x19 || (fam == 0x19 && matches!(model, 0x10..=0x1F | 0x60..=0x7F | 0xA0..=0xAF));
        let v = if svi3 { vid as f64 * 0.00625 } else { 1.55 - vid as f64 * 0.00625 };
        if v > 0.0 {
            m.push(Kind::Voltage, "VID", v, Some(vid));
        }
    }
    if let Some(units) = hw.rdmsr(0, 0xC001_0299) {
        amd_rapl(ctx, units, cores, m);
    }
    if let Some((ver, t)) = hw.amd_smu_pm_table() {
        if let Some(l) = raphael(ver, &t) {
            m.push(Kind::Power, "PPT Limit", l.ppt.0, Some(l.ppt.0 as u32));
        }
        m.registers.push(f("PMT version", format!("0x{ver:X}")));
        for (i, row) in t.chunks(10).enumerate() {
            let vals: Vec<String> = row.iter().map(|v| format!("{v:7.2}")).collect();
            m.registers.push(f(format!("{:03}", i * 10), vals.join(" ")));
        }
    }
}

/// AMD SMU PPT (W), TDC (A), EDC (A) as (limit, value), when the PM table layout is known.
pub fn amd_limits(ctx: &Ctx) -> Option<[(f64, f64); 3]> {
    if !is_amd(crate::cpu::get(ctx)) {
        return None;
    }
    let (ver, t) = ctx.hw.amd_smu_pm_table()?;
    raphael(ver, &t).map(|l| [l.ppt, l.tdc, l.edc])
}

/// (limit, value) pairs of the SMU PM table.
struct PmLimits {
    ppt: (f64, f64),
    tdc: (f64, f64),
    edc: (f64, f64),
}

/// Zen 4 desktop (PM table 0x54xxxx) layout, as read off the reference
/// report: PPT at 2/3, TDC at 8/9, EDC at 61/62. Other layouts: unknown.
fn raphael(ver: u32, t: &[f32]) -> Option<PmLimits> {
    if ver >> 16 != 0x54 || t.len() < 63 {
        return None;
    }
    let p = |i: usize| (t[i] as f64, t[i + 1] as f64);
    Some(PmLimits { ppt: p(2), tdc: p(8), edc: p(61) })
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

    #[test]
    fn raphael_pm_table() {
        // First 70 values of the reference report's PM table (version 0x540004).
        let t: Vec<f32> = "0 0 230 83.59 0 0 0 0 160 35 95 63.84 0 0 0 0 5371.21 2760.29 1.42 1.36 \
            48.36 20.42 8.19 4.64 1 1 83.6 5.7 5.7 5.7 5.7 5.7 5.7 5.7 5.7 5.44 5.7 5.53 1.42 1.45 \
            1.52 1.47 1.52 1.52 1.47 1.47 1.36 1.36 35.01 48.36 59 1.3 1.3 15.73 20.42 52.12 1.1 1.1 7.45 8.19 \
            52 225 78.5 40.36 208 0 100 0 0 0"
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        let l = super::raphael(0x540004, &t).unwrap();
        assert_eq!((l.ppt.0, l.tdc.0, l.edc.0), (230.0, 160.0, 225.0));
        assert!(super::raphael(0x380005, &t).is_none(), "unknown layout must not be guessed");
    }
}

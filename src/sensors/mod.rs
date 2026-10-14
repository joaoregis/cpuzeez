//! Monitoring, LPCIO and Hardware Monitors sections (FEATURES.md 3.10-3.12),
//! plus the live sensor model the GUI's sensor/Clocks windows poll.
//!
//! Sources, best first: Super I/O chips over raw port I/O (privileged
//! backend), the CPU's own MSR/SMN thermal and RAPL energy registers
//! (privileged), then whatever the OS exposes without privileges (Linux
//! hwmon/powercap/cpufreq, Windows PDH counters + power status, Apple SMC).

mod amd_pm;
mod cpu;
mod hwmon;
#[cfg(target_os = "macos")]
mod macos;
mod superio;
#[cfg(windows)]
mod windows;

use crate::Ctx;
use crate::report::{Node, Section, f, g, gv, hex, section};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Utilization: for GPU/load sources not wired yet
pub enum Kind {
    Voltage,
    Temperature,
    Fan,
    Power,
    Current,
    Clock,
    Utilization,
    Level,
}

/// One sensor value. Units: V, degC, RPM, W, A, MHz, %, %.
#[derive(Debug, Clone)]
pub struct Reading {
    pub kind: Kind,
    pub label: String,
    pub value: f64,
    /// Register value it was decoded from, when there is one.
    pub raw: Option<u32>,
    /// Chip input number; rows are numbered by it when set (CPU-Z keeps
    /// "Voltage 6" for the 7th input even when inputs 4-5 are hidden).
    pub channel: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct Monitor {
    pub name: String,
    pub readings: Vec<Reading>,
    /// Raw register view shown under "Hardware registers" (e.g. SMU PM table).
    pub registers: Vec<Node>,
}

impl Monitor {
    pub fn new(name: impl Into<String>) -> Monitor {
        Monitor { name: name.into(), ..Default::default() }
    }
    pub fn push(&mut self, kind: Kind, label: impl Into<String>, value: f64, raw: Option<u32>) {
        self.push_at(kind, None, label, value, raw);
    }
    /// Like `push`, numbered by the chip's input `ch`.
    pub fn push_ch(&mut self, kind: Kind, ch: usize, label: impl Into<String>, value: f64, raw: u32) {
        self.push_at(kind, Some(ch), label, value, Some(raw));
    }
    fn push_at(&mut self, kind: Kind, channel: Option<usize>, label: impl Into<String>, value: f64, raw: Option<u32>) {
        if value.is_finite() {
            self.readings.push(Reading { kind, label: label.into(), value, raw, channel });
        }
    }
}

/// Super I/O chips, detected once per process (config-mode entry is not
/// something to repeat on every GUI refresh).
fn chips(ctx: &Ctx) -> &'static [superio::Chip] {
    static CHIPS: OnceLock<Vec<superio::Chip>> = OnceLock::new();
    CHIPS.get_or_init(|| if ctx.cfg.sensor && ctx.cfg.lpcio { superio::detect(ctx.hw.as_ref()) } else { Vec::new() })
}

/// Every hardware monitor with its current readings. Cheap enough to call
/// once a second: chip detection is cached and energy counters are diffed
/// against the previous call.
pub fn monitors(ctx: &Ctx) -> Vec<Monitor> {
    if !ctx.cfg.sensor {
        return Vec::new();
    }
    let os = os_monitors(ctx);
    let mut v = Vec::new();
    // When a kernel driver owns the Super I/O we must not poke its ports too.
    if !os.iter().any(|m| superio::is_superio_name(&m.name)) {
        v.extend(chips(ctx).iter().filter_map(|c| superio::read(ctx.hw.as_ref(), c)));
    }
    v.extend(os);
    // DIMM thermal sensors (DDR5 SPD5118 hub / DDR4 TSOD), one monitor per module like CPU-Z.
    if ctx.cfg.spd {
        for (_addr, c) in crate::spd::dimm_temps(ctx) {
            let mut m = Monitor::new("DIMM");
            m.push(Kind::Temperature, "Core", c, None);
            v.push(m);
        }
    }
    v.extend(cpu::monitor(ctx));
    v.extend(cpu::pm_monitor(ctx));
    v.retain(|m| !m.readings.is_empty());
    v
}

fn os_monitors(ctx: &Ctx) -> Vec<Monitor> {
    let _ = ctx;
    #[cfg(target_os = "linux")]
    return hwmon::read(std::path::Path::new(hwmon::ROOT), crate::cpu::get(ctx).sig.0);
    #[cfg(windows)]
    return windows::monitors();
    #[cfg(target_os = "macos")]
    return macos::monitors();
    #[allow(unreachable_code)]
    Vec::new()
}

/// Sensor rows appended to "Processors Information".
pub fn cpu_nodes(ctx: &Ctx) -> Vec<Node> {
    if !ctx.cfg.sensor {
        return Vec::new();
    }
    let mut rows = cpu::fields(ctx);
    let mut m = cpu::monitor(ctx).unwrap_or_default();
    #[cfg(target_os = "linux")]
    if !m.readings.iter().any(|r| r.kind == Kind::Temperature) {
        // No MSR access: the kernel's k10temp/zenpower/coretemp has them.
        let mut t = hwmon::cpu_temps(std::path::Path::new(hwmon::ROOT));
        t.append(&mut m.readings);
        m.readings = t;
    }
    for r in &mut m.readings {
        r.raw = None; // CPU-Z shows raw codes only in the Hardware Monitors section
    }
    rows.extend(reading_rows(&m.readings));
    rows.extend(cpu::ratio_fields(ctx));
    rows
}

pub use amd_pm::FabricClocks;

/// AMD data-fabric (FCLK), memory-controller (UCLK) and memory (MCLK) clocks
/// in MHz, from the SMU PM table; None without SMU access or for unknown
/// table versions.
#[allow(dead_code)] // for the memory/telemetry consumers
pub fn fabric_clocks(ctx: &Ctx) -> Option<FabricClocks> {
    cpu::fabric_clocks(ctx)
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let mut out = Vec::new();
    if let Some(b) = board_model(ctx) {
        out.push(section("Monitoring", vec![f("Mainboard Model", b)]));
    }
    if !ctx.cfg.sensor {
        return out;
    }
    let mut lpc = Vec::new();
    for c in chips(ctx) {
        lpc.extend(lpcio_rows(c));
    }
    out.push(section("LPCIO", lpc));
    let hm = monitors(ctx)
        .into_iter()
        .map(|m| {
            let mut rows = reading_rows(&m.readings);
            if !m.registers.is_empty() {
                rows.push(g("Hardware registers", m.registers));
            }
            gv("Hardware monitor", &m.name, rows)
        })
        .collect();
    out.push(section("Hardware Monitors", hm));
    out
}

fn lpcio_rows(c: &superio::Chip) -> Vec<Node> {
    vec![
        f("LPCIO Vendor", c.vendor),
        f("LPCIO Model", c.model),
        f("LPCIO Vendor ID", format!("0x{:04X}", c.vendor_id)),
        f("LPCIO Chip ID", format!("0x{:02X}", c.chip_id)),
        f("LPCIO Revision ID", format!("0x{:02X}", c.rev)),
        f("Config Mode I/O address", format!("0x{:X}", c.port)),
        f("Config Mode LDN", format!("0x{:X}", c.ldn)),
        hex("Config Mode registers", c.regs.clone()),
        f("Register space", format!("LPC, base address = 0x{:04X}", c.base)),
    ]
}

/// Baseboard product name. SMBIOS type 2 when the SMBIOS module has the
/// table, else the OS's own copy of it.
pub fn board_model(ctx: &Ctx) -> Option<String> {
    crate::smbios::board(ctx).and_then(|b| b.model).filter(|s| !s.trim().is_empty()).or_else(os_board_model)
}

fn os_board_model() -> Option<String> {
    #[cfg(target_os = "linux")]
    return crate::os::read_trim("/sys/class/dmi/id/board_name");
    #[cfg(windows)]
    return crate::os::win::reg_str(r"HARDWARE\DESCRIPTION\System\BIOS", "BaseBoardProduct");
    #[cfg(target_os = "macos")]
    return crate::os::cmd("sysctl", &["-n", "hw.model"]).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    #[allow(unreachable_code)]
    None
}

// ---- formatting (CPU-Z's "Voltage 0  12.05 Volts [0x3EC0] (+12V)" rows) ----

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Voltage => "Voltage",
        Kind::Temperature => "Temperature",
        Kind::Fan => "Fan",
        Kind::Power => "Power",
        Kind::Current => "Current",
        Kind::Clock => "Clock Speed",
        Kind::Utilization => "Utilization",
        Kind::Level => "Level",
    }
}

pub fn value_text(r: &Reading) -> String {
    let v = r.value;
    let mut s = match r.kind {
        Kind::Voltage => format!("{v:.2} Volts"),
        Kind::Temperature => format!("{v:.0} degC ({:.0} degF)", v * 1.8 + 32.0),
        Kind::Fan => format!("{v:.0} RPM"),
        Kind::Power => format!("{v:.2} W"),
        Kind::Current => format!("{v:.2} Amps"),
        Kind::Clock => format!("{v:.2} MHz"),
        Kind::Utilization | Kind::Level => format!("{v:.0} %"),
    };
    if let Some(x) = r.raw {
        s += &format!(" [0x{x:X}]");
    }
    if !r.label.is_empty() {
        s += &format!(" ({})", r.label);
    }
    s
}

/// Rows numbered per kind in insertion order; powers use two digits like CPU-Z.
pub fn reading_rows(rs: &[Reading]) -> Vec<Node> {
    let mut n = [0usize; 8];
    rs.iter()
        .map(|r| {
            let i = &mut n[r.kind as usize];
            let num = r.channel.unwrap_or(*i);
            let key =
                if r.kind == Kind::Power { format!("Power {num:02}") } else { format!("{} {num}", kind_name(r.kind)) };
            *i += 1;
            f(key, value_text(r))
        })
        .collect()
}

/// Average power of energy counters: delta since the previous call, or over
/// a 100 ms window the first time (one sleep for the whole batch).
/// Each counter: (stable key, joules per count, wrap modulus, current count).
pub(crate) fn watts(
    counters: &[(String, f64, u64, Option<u64>)],
    reread: impl Fn(usize) -> Option<u64>,
) -> Vec<Option<f64>> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static LAST: Mutex<Option<HashMap<String, (Instant, u64)>>> = Mutex::new(None);
    let Ok(mut g) = LAST.lock() else { return vec![None; counters.len()] };
    let last = g.get_or_insert_with(HashMap::new);
    let now = Instant::now();
    let fresh = |k: &str| {
        last.get(k)
            .filter(|(t, _)| (Duration::from_millis(20)..Duration::from_secs(10)).contains(&now.duration_since(*t)))
            .copied()
    };
    let mut prev: Vec<Option<(Instant, u64)>> = counters.iter().map(|(k, _, _, v)| v.and(fresh(k))).collect();
    let mut cur: Vec<(Instant, Option<u64>)> = counters.iter().map(|c| (now, c.3)).collect();
    if counters.iter().zip(&prev).any(|(c, p)| c.3.is_some() && p.is_none()) {
        std::thread::sleep(Duration::from_millis(100));
        for (i, c) in counters.iter().enumerate() {
            if c.3.is_some() && prev[i].is_none() {
                prev[i] = c.3.map(|v| (now, v));
                cur[i] = (Instant::now(), reread(i));
            }
        }
    }
    counters
        .iter()
        .enumerate()
        .map(|(i, (k, unit, wrap, _))| {
            let (t1, v1) = (cur[i].0, cur[i].1?);
            last.insert(k.clone(), (t1, v1));
            let (t0, v0) = prev[i]?;
            let dt = t1.duration_since(t0).as_secs_f64();
            let d = if v1 >= v0 { v1 - v0 } else { v1 + wrap.saturating_sub(v0) };
            (dt > 0.0).then(|| d as f64 * unit / dt)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_format() {
        let mut m = Monitor::new("x");
        m.push(Kind::Voltage, "+12V", 12.048, Some(0x3EC0));
        m.push(Kind::Temperature, "Package", 69.6, None);
        m.push(Kind::Power, "Package", 79.61, None);
        m.push(Kind::Fan, "SYS 1", 868.0, Some(0x364));
        m.push_ch(Kind::Voltage, 6, "+3.3V", 3.232, 0xCA00);
        let r = crate::report::to_txt(&[section("S", vec![gv("Hardware monitor", "x", reading_rows(&m.readings))])]);
        assert!(r.contains("\tVoltage 0\t\t12.05 Volts [0x3EC0] (+12V)\n"), "{r}");
        assert!(r.contains("\tVoltage 6\t\t3.23 Volts [0xCA00] (+3.3V)\n"), "{r}");
        assert!(r.contains("\tTemperature 0\t\t70 degC (157 degF) (Package)\n"), "{r}");
        assert!(r.contains("\tPower 00\t\t79.61 W (Package)\n"), "{r}");
        assert!(r.contains("\tFan 0\t\t\t868 RPM [0x364] (SYS 1)\n"), "{r}");
    }

    #[test]
    fn energy_wrap() {
        // 32-bit counter wrapping between samples: 0xFFFF_FF00 -> 0x100 = 0x200 counts.
        let w = watts(&[("t:wrap".into(), 1.0, 1 << 32, Some(0xFFFF_FF00))], |_| Some(0x100));
        let w = w[0].unwrap();
        assert!(w > 0x200 as f64 && w < 0x200 as f64 / 0.09, "{w}");
    }
}

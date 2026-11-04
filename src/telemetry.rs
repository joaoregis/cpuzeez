//! Live sensor telemetry on the Metric Engine (`cpuzeez_core::metrics`).
//!
//! This module provides the engine's sources (CPU + mainboard monitors, GPU,
//! memory, drives, batteries) and two per-tick hooks: throttle detection and
//! CSV logging. The Sensors page, Overview, stress monitor, tray, mini monitor
//! and `-sensors` / `-log=` read from it. Nothing is sampled unless a consumer
//! holds a lease (`Telemetry::want`), so hidden pages cost nothing.
//!
//! Ids are `<device>/<kind>/<name>`: device is `cpu`, `gpu<N>`, `board`,
//! `chipset`, `mem`, `storage`, `battery<N>`; kind is `temp`, `volt`, `power`,
//! `curr`, `clock`, `fan`, `load`, `level` or `size`; name is the sensor label
//! lowercased with only `[a-z0-9]` kept (`+`→`p`, `-`→`m`, `.`→`_`), e.g.
//! `cpu/temp/package`, `cpu/clock/core3`, `board/volt/p12v`, `gpu0/temp/core`.
//! A clashing id within one source gets `_2`, `_3`… in reading order. The CSV
//! format is documented on `cpuzeez_core::metrics::Csv`.

mod os;

use crate::Ctx;
use crate::sensors::Kind;
use cpuzeez_core::metrics::{self, Class, Csv, Engine, Source, Tick};
pub use cpuzeez_core::metrics::{Reading, Series, Snapshot, Unit};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum ThrottleKind {
    /// Temperature at/near Tjmax or the OS/CPU thermal-throttle flag set.
    Thermal,
    /// Package power at its limit (PPT/PL1/PL2) while clocks drop.
    Power,
    /// Current/EDC limit.
    Current,
    /// GPU reports a throttle reason.
    Gpu,
}

#[derive(Debug, Clone)]
pub struct Throttle {
    pub kind: ThrottleKind,
    /// "CPU" / GPU name.
    pub device: String,
    /// Human-readable cause, e.g. "Package 95 °C ≥ Tjmax 95 °C, clock 4.1 GHz → 3.2 GHz".
    pub detail: String,
    /// Seconds since sampler start when it began.
    pub since: f64,
}

// ---------------------------------------------------------------- one pass

/// (unit, id token, display name) of a hardware-monitor reading kind.
fn kind_info(k: Kind) -> (Unit, &'static str, &'static str) {
    match k {
        Kind::Voltage => (Unit::Volt, "volt", "Voltage"),
        Kind::Temperature => (Unit::Celsius, "temp", "Temperature"),
        Kind::Fan => (Unit::Rpm, "fan", "Fan"),
        Kind::Power => (Unit::Watt, "power", "Power"),
        Kind::Current => (Unit::Ampere, "curr", "Current"),
        Kind::Clock => (Unit::Mhz, "clock", "Clock"),
        Kind::Utilization => (Unit::Percent, "load", "Utilization"),
        Kind::Level => (Unit::Percent, "level", "Level"),
    }
}

/// Id form of a label. A trailing " (…)" is a qualifier, not identity
/// ("Charge (AC)" and "Charge" are the same sensor).
fn slug(s: &str) -> String {
    let s = s.split(" (").next().unwrap_or(s);
    s.chars()
        .filter_map(|c| match c {
            'a'..='z' | '0'..='9' => Some(c),
            'A'..='Z' => Some(c.to_ascii_lowercase()),
            '+' => Some('p'),
            '-' => Some('m'),
            '.' => Some('_'),
            _ => None,
        })
        .collect()
}

#[derive(Default)]
struct Out {
    v: Vec<Reading>,
    ids: HashSet<String>,
}

impl Out {
    fn add(&mut self, id: impl Into<String>, group: &str, label: impl Into<String>, unit: Unit, value: f64) {
        if !value.is_finite() {
            return;
        }
        let base = id.into();
        let (mut id, mut n) = (base.clone(), 2);
        while !self.ids.insert(id.clone()) {
            id = format!("{base}_{n}");
            n += 1;
        }
        self.v.push(Reading { id, group: group.into(), label: label.into(), unit, value });
    }

    /// A hardware-monitor reading as `<pre>/<kind>/<label>`.
    fn sensor(&mut self, pre: &str, group: &str, r: &crate::sensors::Reading) {
        let (unit, tok, name) = kind_info(r.kind);
        let label = match (r.label.is_empty(), r.channel) {
            (false, _) => r.label.clone(),
            (true, Some(c)) => format!("{name} {c}"),
            (true, None) => name.to_string(),
        };
        self.add(format!("{pre}/{tok}/{}", slug(&label)), group, label, unit, r.value);
    }
}

fn group_rank(g: &str) -> u8 {
    match g {
        "CPU" => 0,
        _ if g.starts_with("GPU") => 1,
        "Mainboard" => 2,
        "Chipset" => 3,
        "Memory" => 4,
        "Storage" => 5,
        "Battery" => 6,
        _ => 7,
    }
}

/// One pass over every source, in display order (`-sensors`).
pub fn read_all(ctx: &Ctx) -> Vec<Reading> {
    let mut o = Out::default();
    let bats = crate::battery::batteries(ctx);
    monitors(ctx, &mut o, !bats.is_empty());
    cpu_extra(ctx, &mut o);
    gpu(ctx, &crate::gpu::live(ctx), &mut o);
    memory(&mut o);
    storage(ctx, &mut o);
    battery(&bats, &mut o);
    o.v.sort_by_key(|r| group_rank(&r.group));
    o.v
}

/// Every `crate::sensors` monitor, routed to its group.
fn monitors(ctx: &Ctx, o: &mut Out, have_batteries: bool) {
    let c = crate::cpu::get(ctx);
    let cpu_name = if c.name.is_empty() { "CPU" } else { c.name.as_str() };
    let mons = crate::sensors::monitors(ctx);
    #[cfg(target_os = "linux")]
    if !mons.iter().any(|m| m.name == cpu_name && m.readings.iter().any(|r| r.kind == Kind::Temperature)) {
        // No MSR access: the kernel's k10temp/zenpower/coretemp has them.
        for r in crate::sensors::os_cpu_temps() {
            o.sensor("cpu", "CPU", &r);
        }
    }
    let mut clocks = Vec::new();
    let mut dimm = 0;
    for m in &mons {
        let (pre, group) = match m.name.as_str() {
            n if n == cpu_name => ("cpu", "CPU"),
            // Kernel CPU drivers: temperatures were taken (relabelled) above.
            "AMD Ryzen Power Management" | "AMD K10" | "Intel Core DTS" => {
                for r in m.readings.iter().filter(|r| r.kind != Kind::Temperature) {
                    o.sensor("cpu", "CPU", r);
                }
                continue;
            }
            "DIMM" => {
                for r in &m.readings {
                    o.add(format!("mem/temp/dimm{dimm}"), "Memory", format!("DIMM #{dimm}"), Unit::Celsius, r.value);
                }
                dimm += 1;
                continue;
            }
            // The storage and GPU paths below read these same sensors.
            "NVMe" | "Drive" | "AMD GPU" | "NVIDIA GPU" | "Intel GPU" => continue,
            "Battery" if have_batteries => continue,
            "Battery" => ("battery0", "Battery"),
            "Chipset" => ("chipset", "Chipset"),
            _ => ("board", "Mainboard"),
        };
        for r in &m.readings {
            if pre == "cpu" && r.kind == Kind::Clock {
                clocks.push(r.value);
            }
            if pre == "board" && r.label.starts_with("CPU") {
                o.sensor("cpu", "CPU", r); // Apple SMC mixes CPU and board sensors
            } else {
                o.sensor(pre, group, r);
            }
        }
    }
    if !clocks.is_empty() {
        let avg = clocks.iter().sum::<f64>() / clocks.len() as f64;
        o.add("cpu/clock/avg", "CPU", "Average core clock", Unit::Mhz, avg);
    }
}

/// OS load and the AMD SMU limit readings.
fn cpu_extra(ctx: &Ctx, o: &mut Out) {
    let (total, per) = os::cpu_load();
    if let Some(t) = total {
        o.add("cpu/load/total", "CPU", "Total load", Unit::Percent, t);
    }
    for (i, v) in per.iter().enumerate() {
        if let Some(v) = v {
            o.add(format!("cpu/load/cpu{i}"), "CPU", format!("CPU #{i} load"), Unit::Percent, *v);
        }
    }
    if let Some([ppt, tdc, edc]) = crate::sensors::amd_limits(ctx) {
        // "PPT Limit" itself is already a monitor reading (cpu/power/pptlimit).
        o.add("cpu/power/ppt", "CPU", "PPT", Unit::Watt, ppt.1);
        o.add("cpu/curr/tdc", "CPU", "TDC", Unit::Ampere, tdc.1);
        o.add("cpu/curr/tdclimit", "CPU", "TDC Limit", Unit::Ampere, tdc.0);
        o.add("cpu/curr/edc", "CPU", "EDC", Unit::Ampere, edc.1);
        o.add("cpu/curr/edclimit", "CPU", "EDC Limit", Unit::Ampere, edc.0);
    }
}

fn gpu_group(ctx: &Ctx, i: usize) -> String {
    let name = crate::gpu::adapters(ctx).get(i).map(|a| a.name.trim()).filter(|n| !n.is_empty());
    format!("GPU · {}", name.map_or_else(|| format!("GPU {i}"), str::to_string))
}

fn memory(o: &mut Out) {
    if let Some((total, used)) = os::memory() {
        o.add("mem/load/used", "Memory", "Memory load", Unit::Percent, used * 100.0 / total);
        o.add("mem/size/used", "Memory", "Memory used", Unit::Megabytes, used);
    }
}

fn gpu(ctx: &Ctx, live: &[crate::gpu::Live], o: &mut Out) {
    for l in live {
        let (g, p) = (gpu_group(ctx, l.adapter), format!("gpu{}", l.adapter));
        // ponytail: throttle reasons (l.throttle) aren't a numeric series; surface them when the UI has a flag slot.
        for (k, label, unit, v) in [
            ("temp/core", "Temperature", Unit::Celsius, l.temp_c),
            ("temp/hotspot", "Hot spot", Unit::Celsius, l.hotspot_c),
            ("temp/memjunction", "Memory junction", Unit::Celsius, l.mem_junction_c),
            ("clock/core", "Core clock", Unit::Mhz, l.core_mhz),
            ("clock/mem", "Memory clock", Unit::Mhz, l.mem_mhz),
            ("power/board", "Power", Unit::Watt, l.power_w),
            ("load/core", "Load", Unit::Percent, l.load_pct),
            ("load/mem", "Memory load", Unit::Percent, l.mem_load_pct),
            ("fan/rpm", "Fan", Unit::Rpm, l.fan_rpm),
            ("fan/pct", "Fan duty", Unit::Percent, l.fan_pct),
        ] {
            if let Some(v) = v {
                o.add(format!("{p}/{k}"), &g, label, unit, v);
            }
        }
    }
}

/// Drive temperatures. Probing opens every drive, so this is a slow source.
fn storage(ctx: &Ctx, o: &mut Out) {
    for (i, d) in crate::storage::drives(ctx).iter().enumerate() {
        let t = d.health.iter().find(|(k, _)| k == "Temperature");
        let t = t.and_then(|t| t.1.trim_end_matches('C').trim().parse().ok()).unwrap_or(f64::NAN);
        o.add(format!("storage/temp/disk{i}"), "Storage", d.name.clone(), Unit::Celsius, t);
    }
}

fn battery(bats: &[crate::battery::Battery], o: &mut Out) {
    for (i, b) in bats.iter().enumerate() {
        let name = |l: &str| if bats.len() > 1 { format!("{} {l}", b.name) } else { l.to_string() };
        for (k, l, unit, v) in [
            ("level/charge", "Charge", Unit::Percent, b.percent),
            ("power/rate", "Charge rate", Unit::Watt, b.rate_w),
            ("volt/battery", "Voltage", Unit::Volt, b.voltage_v),
            ("temp/battery", "Temperature", Unit::Celsius, b.temperature_c),
            ("level/health", "Health", Unit::Percent, b.health_pct),
        ] {
            if let Some(v) = v {
                o.add(format!("battery{i}/{k}"), "Battery", name(l), unit, v);
            }
        }
    }
}

// ---------------------------------------------------------------- sources

/// Hardware monitors (CPU, mainboard Super I/O, chipset, DIMMs) plus OS load and
/// the AMD SMU limits. One source because `sensors::monitors` reads them in one
/// pass and their ids must stay unique together.
struct CpuBoard {
    ctx: Arc<Ctx>,
    have_batteries: bool,
}

impl Source for CpuBoard {
    fn name(&self) -> &'static str {
        "cpu+board"
    }
    fn prefixes(&self) -> &'static [&'static str] {
        &["cpu/", "board/", "chipset/", "mem/temp/", "battery"]
    }
    fn read(&mut self, out: &mut Vec<Reading>) {
        let mut o = Out::default();
        monitors(&self.ctx, &mut o, self.have_batteries);
        cpu_extra(&self.ctx, &mut o);
        out.append(&mut o.v);
    }
}

struct Gpus {
    ctx: Arc<Ctx>,
    /// The full live structs (throttle reasons included) for the Clocks tool.
    last: Arc<Mutex<Vec<crate::gpu::Live>>>,
}

impl Source for Gpus {
    fn name(&self) -> &'static str {
        "gpu"
    }
    fn prefixes(&self) -> &'static [&'static str] {
        &["gpu"]
    }
    fn read(&mut self, out: &mut Vec<Reading>) {
        let live = crate::gpu::live(&self.ctx);
        let mut o = Out::default();
        gpu(&self.ctx, &live, &mut o);
        *lock(&self.last) = live;
        out.append(&mut o.v);
    }
}

struct Memory;

impl Source for Memory {
    fn name(&self) -> &'static str {
        "memory"
    }
    fn prefixes(&self) -> &'static [&'static str] {
        &["mem/load", "mem/size"]
    }
    fn read(&mut self, out: &mut Vec<Reading>) {
        let mut o = Out::default();
        memory(&mut o);
        out.append(&mut o.v);
    }
}

/// Drive temperatures: opening every drive is slow, so its own worker, every 10 s.
struct Drives {
    ctx: Arc<Ctx>,
}

impl Source for Drives {
    fn name(&self) -> &'static str {
        "storage"
    }
    fn prefixes(&self) -> &'static [&'static str] {
        &["storage/"]
    }
    fn class(&self) -> Class {
        Class::Slow
    }
    fn interval(&self) -> Duration {
        Duration::from_secs(10)
    }
    fn read(&mut self, out: &mut Vec<Reading>) {
        let mut o = Out::default();
        storage(&self.ctx, &mut o);
        out.append(&mut o.v);
    }
}

/// Batteries: macOS spawns `ioreg`, so a slow source (every 5 s).
struct Batteries {
    ctx: Arc<Ctx>,
}

impl Source for Batteries {
    fn name(&self) -> &'static str {
        "battery"
    }
    fn prefixes(&self) -> &'static [&'static str] {
        &["battery"]
    }
    fn class(&self) -> Class {
        Class::Slow
    }
    fn interval(&self) -> Duration {
        Duration::from_secs(5)
    }
    fn read(&mut self, out: &mut Vec<Reading>) {
        let mut o = Out::default();
        battery(&crate::battery::batteries(&self.ctx), &mut o);
        out.append(&mut o.v);
    }
}

/// macOS has no live per-core clock (no MSRs, no cpufreq): time a dependent-add
/// chain on whatever core runs it. It loads that core for ~120 ms, so it is a
/// slow source and only runs while something on screen wants clocks.
#[cfg(target_os = "macos")]
struct MeasuredClock;

#[cfg(target_os = "macos")]
impl Source for MeasuredClock {
    fn name(&self) -> &'static str {
        "measured clock"
    }
    fn prefixes(&self) -> &'static [&'static str] {
        &["cpu/clock/measured"]
    }
    fn class(&self) -> Class {
        Class::Slow
    }
    fn interval(&self) -> Duration {
        Duration::from_secs(5)
    }
    fn read(&mut self, out: &mut Vec<Reading>) {
        let mhz = crate::cpu::clocks::add_chain_mhz(Duration::from_millis(80));
        out.push(Reading {
            id: "cpu/clock/measured".into(),
            group: "CPU".into(),
            label: "Core clock (measured)".into(),
            unit: Unit::Mhz,
            value: mhz,
        });
    }
}

/// Live MHz of physical core `core` (`cpu/clock/core<N>`, else the measured clock).
pub fn core_mhz(snap: &Snapshot, core: usize) -> Option<f64> {
    snap.get(&format!("cpu/clock/core{core}")).or_else(|| snap.get("cpu/clock/measured")).map(|s| s.current)
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------- throttling

/// Consecutive samples a condition must hold before it is reported.
const DEBOUNCE: u32 = 2;
// ponytail: fixed heuristics; these are the calibration knobs if a platform
// reports throttling too eagerly or too late.
/// Package within this many °C of Tjmax counts as "at Tjmax".
const NEAR_TJMAX_C: f64 = 1.0;
/// Average effective clock below this fraction of its recent peak = clock drop.
const CLOCK_DROP: f64 = 0.95;
/// SMU value at this fraction of its limit = limit-bound.
const AT_LIMIT: f64 = 0.98;

/// One raw (not yet debounced) throttle condition.
#[derive(Debug, Clone)]
struct Cond {
    key: String,
    kind: ThrottleKind,
    device: String,
    detail: String,
}

fn cond(key: impl Into<String>, kind: ThrottleKind, device: impl Into<String>, detail: String) -> Cond {
    Cond { key: key.into(), kind, device: device.into(), detail }
}

struct Limits<'a> {
    tjmax: f64,
    /// Highest "cpu/clock/avg" in the kept history.
    clock_peak: Option<f64>,
    /// Slowdown temperature per GPU adapter index.
    gpu_slowdown: &'a [Option<f64>],
}

/// Conditions visible in the readings themselves: package temperature at
/// Tjmax with the clock falling, AMD PPT/TDC/EDC at their limits, GPU at its
/// slowdown temperature.
fn reading_conds(rs: &[Reading], l: &Limits) -> Vec<Cond> {
    let get = |id: &str| rs.iter().find(|r| r.id == id).map(|r| r.value);
    let mut v = Vec::new();
    let pkg = get("cpu/temp/package").or_else(|| {
        rs.iter().filter(|r| r.group == "CPU" && r.unit == Unit::Celsius).map(|r| r.value).reduce(f64::max)
    });
    if let Some(t) = pkg.filter(|&t| t >= l.tjmax - NEAR_TJMAX_C) {
        let clk = get("cpu/clock/avg").zip(l.clock_peak);
        // Without clock data the temperature alone has to do.
        if clk.is_none_or(|(c, p)| c < p * CLOCK_DROP) {
            let mut d = format!("Package {t:.0} °C ≥ Tjmax {:.0} °C", l.tjmax);
            if let Some((c, p)) = clk {
                d += &format!(", clock {:.2} GHz → {:.2} GHz", p / 1000.0, c / 1000.0);
            }
            v.push(cond("cpu/tjmax", ThrottleKind::Thermal, "CPU", d));
        }
    }
    for (val, lim, name, kind, u) in [
        ("cpu/power/ppt", "cpu/power/pptlimit", "PPT", ThrottleKind::Power, "W"),
        ("cpu/curr/tdc", "cpu/curr/tdclimit", "TDC", ThrottleKind::Current, "A"),
        ("cpu/curr/edc", "cpu/curr/edclimit", "EDC", ThrottleKind::Current, "A"),
    ] {
        if let (Some(x), Some(m)) = (get(val), get(lim))
            && m > 0.0
            && x >= m * AT_LIMIT
        {
            v.push(cond(val, kind, "CPU", format!("{name} {x:.1} {u} at its {m:.0} {u} limit")));
        }
    }
    for (i, s) in l.gpu_slowdown.iter().enumerate() {
        let (Some(s), Some(r)) = (s, rs.iter().find(|r| r.id == format!("gpu{i}/temp/core"))) else { continue };
        if r.value >= *s {
            let dev = r.group.trim_start_matches("GPU · ");
            v.push(cond(
                format!("gpu{i}/temp"),
                ThrottleKind::Gpu,
                dev,
                format!("{:.0} °C ≥ slowdown {s:.0} °C", r.value),
            ));
        }
    }
    v
}

/// Current-status bits (Intel SDM vol. 4) of IA32_PACKAGE_THERM_STATUS (0x1B1),
/// IA32_THERM_STATUS (0x19C, OR of every core) and MSR_CORE_PERF_LIMIT_REASONS
/// (0x64F). The odd "log" bits are sticky and ignored.
fn intel_conds(pkg: Option<u64>, cores: Option<u64>, perf: Option<u64>) -> Vec<Cond> {
    let mut why: [Vec<&str>; 3] = Default::default(); // thermal, power, current
    for (msr, name) in [(pkg, "package"), (cores, "core")] {
        let Some(s) = msr else { continue };
        for (bit, slot, what) in [
            (0, 0, "thermal status"),
            (2, 0, "PROCHOT"),
            (4, 0, "critical temperature"),
            (10, 1, "power limit"),
            (12, 2, "current limit"),
        ] {
            if s >> bit & 1 == 1 && !(name == "package" && bit == 12) {
                why[slot].push(what);
            }
        }
    }
    if let Some(p) = perf {
        for (bit, slot, what) in [
            (0, 0, "PROCHOT"),
            (1, 0, "thermal status"),
            (5, 0, "running average thermal limit"),
            (6, 0, "VR thermal alert"),
            (7, 2, "VR TDC"),
            (8, 2, "electrical design point"),
            (10, 1, "PL1"),
            (11, 1, "PL2"),
        ] {
            if p >> bit & 1 == 1 {
                why[slot].push(what);
            }
        }
    }
    let kinds = [ThrottleKind::Thermal, ThrottleKind::Power, ThrottleKind::Current];
    let keys = ["cpu/msr/thermal", "cpu/msr/power", "cpu/msr/current"];
    let mut v = Vec::new();
    for ((w, kind), key) in why.iter_mut().zip(kinds).zip(keys) {
        let mut seen = HashSet::new();
        w.retain(|x| seen.insert(*x));
        if !w.is_empty() {
            v.push(cond(key, kind, "CPU", format!("CPU reports {}", w.join(", "))));
        }
    }
    v
}

/// Conditions only the hardware/OS flags show: Intel MSR status bits and the
/// Linux kernel's thermal_throttle counters (delta since the previous sample).
fn hw_conds(ctx: &Ctx, counts: &mut Option<(u64, u64)>) -> Vec<Cond> {
    let c = crate::cpu::get(ctx);
    let mut v = Vec::new();
    if c.vendor == "GenuineIntel" {
        let hw = &ctx.hw;
        let cores = c.logical.iter().filter(|l| l.smt == 0).filter_map(|l| hw.rdmsr(l.cpu, 0x19C)).reduce(|a, b| a | b);
        v.extend(intel_conds(hw.rdmsr(0, 0x1B1), cores, hw.rdmsr(0, 0x64F)));
    }
    if let Some(now) = os::throttle_counts()
        && let Some(prev) = counts.replace(now)
    {
        if now.0 > prev.0 {
            v.push(cond(
                "cpu/os/thermal",
                ThrottleKind::Thermal,
                "CPU",
                format!("kernel counted {} thermal throttle events", now.0 - prev.0),
            ));
        }
        if now.1 > prev.1 {
            v.push(cond(
                "cpu/os/power",
                ThrottleKind::Power,
                "CPU",
                format!("kernel counted {} power-limit events", now.1 - prev.1),
            ));
        }
    }
    v
}

/// Reports a condition once it held for `DEBOUNCE` consecutive samples;
/// `since` is the first of them.
#[derive(Default)]
struct Debounce {
    live: HashMap<String, (u32, Throttle)>,
}

impl Debounce {
    fn update(&mut self, t: f64, conds: Vec<Cond>) -> Vec<Throttle> {
        let mut next = HashMap::new();
        for c in conds {
            let (n, since) = self.live.get(&c.key).map_or((1, t), |(n, th)| (n + 1, th.since));
            next.insert(c.key, (n, Throttle { kind: c.kind, device: c.device, detail: c.detail, since }));
        }
        self.live = next;
        let mut v: Vec<Throttle> = self.live.values().filter(|(n, _)| *n >= DEBOUNCE).map(|(_, t)| t.clone()).collect();
        v.sort_by(|a, b| a.since.total_cmp(&b.since).then_with(|| a.detail.cmp(&b.detail)));
        v
    }
}

// ---------------------------------------------------------------- Telemetry

/// The app's engine plus its throttle and CSV hooks. Cheap to clone; the engine
/// stops when the last clone is dropped.
#[derive(Clone)]
pub struct Telemetry {
    engine: Engine,
    throttles: Arc<Mutex<Vec<Throttle>>>,
    csv: Arc<Mutex<Option<Csv>>>,
    /// Set while a CSV log runs (the engine then reads every source).
    logging: Arc<AtomicBool>,
    gpu_live: Arc<Mutex<Vec<crate::gpu::Live>>>,
}

impl Telemetry {
    pub fn start(ctx: Arc<Ctx>, period: Duration, keep: Duration) -> Telemetry {
        let gpu_live = Arc::new(Mutex::new(Vec::new()));
        let have_batteries = !crate::battery::batteries(&ctx).is_empty();
        #[allow(unused_mut)]
        let mut sources: Vec<Box<dyn Source>> = vec![
            Box::new(CpuBoard { ctx: ctx.clone(), have_batteries }),
            Box::new(Gpus { ctx: ctx.clone(), last: gpu_live.clone() }),
            Box::new(Memory),
            Box::new(Drives { ctx: ctx.clone() }),
            Box::new(Batteries { ctx: ctx.clone() }),
        ];
        #[cfg(target_os = "macos")]
        sources.push(Box::new(MeasuredClock));
        let engine = Engine::start(sources, metrics::Config { period, keep, ..Default::default() });
        let t = Telemetry { engine, throttles: Arc::default(), csv: Arc::default(), logging: Arc::default(), gpu_live };
        t.engine.add_hook(throttle_hook(ctx, t.throttles.clone()));
        let csv = t.csv.clone();
        t.engine.add_hook(Box::new(move |tick: &Tick| {
            let mut c = lock(&csv);
            if let Some(f) = c.as_mut()
                && let Err(e) = f.write(tick.wall, tick.readings)
            {
                eprintln!("{}: {e}; CSV logging stopped", f.path.display());
                *c = None;
            }
        }));
        t
    }

    /// Keep sampling the metrics under `prefixes` ("" = all) for a few more
    /// seconds. Call it every frame from whatever shows them.
    pub fn want(&self, key: &str, prefixes: &[&str]) {
        self.engine.want(key, prefixes);
    }

    /// Current values and statistics of every metric (no history).
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.engine.snapshot()
    }

    /// Every metric; history only for ids where `with_points(id)` is true.
    pub fn series_with(&self, with_points: impl Fn(&str) -> bool) -> Vec<Series> {
        self.engine.series(with_points)
    }

    /// Clear min/max/avg (keeps the graphs).
    pub fn reset_stats(&self) {
        self.engine.reset_stats();
    }

    /// Active throttling events (empty when none).
    pub fn throttling(&self) -> Vec<Throttle> {
        lock(&self.throttles).clone()
    }

    pub fn set_period(&self, period: Duration) {
        self.engine.set_period(period);
    }

    /// Change how much graph history is kept.
    pub fn set_keep(&self, keep: Duration) {
        self.engine.set_keep(keep);
    }

    /// Latest full GPU live data (clocks, temperatures, throttle reasons).
    pub fn gpu_live(&self) -> Vec<crate::gpu::Live> {
        lock(&self.gpu_live).clone()
    }

    /// Mean read time per source, slowest first.
    pub fn costs(&self) -> Vec<(&'static str, u64, Duration)> {
        self.engine.costs()
    }

    /// Sources that failed recently: (name, error).
    pub fn failures(&self) -> Vec<(&'static str, String)> {
        self.engine.failures()
    }

    /// Append every sample of every metric to `path` as CSV until `stop_csv`.
    pub fn start_csv(&self, path: &std::path::Path) -> std::io::Result<()> {
        *lock(&self.csv) = Some(Csv::create(path)?);
        self.logging.store(true, Ordering::Relaxed);
        self.engine.set_always(true);
        Ok(())
    }

    pub fn stop_csv(&self) {
        *lock(&self.csv) = None;
        self.logging.store(false, Ordering::Relaxed);
        self.engine.set_always(false);
    }

    pub fn csv_path(&self) -> Option<std::path::PathBuf> {
        let path = lock(&self.csv).as_ref().map(|c| c.path.clone());
        if path.is_none() && self.logging.swap(false, Ordering::Relaxed) {
            self.engine.set_always(false); // the hook stopped it after a write error
        }
        path
    }
}

/// Throttle detection after every tick: reading-based rules plus hardware/OS
/// flags, debounced.
fn throttle_hook(ctx: Arc<Ctx>, out: Arc<Mutex<Vec<Throttle>>>) -> metrics::Hook {
    let tjmax = crate::cpu::get(&ctx).info.tjmax_c.unwrap_or_else(|| crate::settings::Settings::load().warn_temp_c);
    let slowdown: Vec<Option<f64>> = crate::gpu::adapters(&ctx).iter().map(|a| a.thermal_limits[1]).collect();
    let (mut counts, mut debounce) = (None, Debounce::default());
    Box::new(move |tick: &Tick| {
        let mut conds = hw_conds(&ctx, &mut counts);
        let l = Limits { tjmax, clock_peak: tick.peak("cpu/clock/avg"), gpu_slowdown: &slowdown };
        conds.extend(reading_conds(tick.readings, &l));
        *lock(&out) = debounce.update(tick.t, conds);
    })
}

// ---------------------------------------------------------------- CLI

/// `-sensors`: print one pass.
pub fn cli_once(ctx: &Ctx) {
    for r in read_all(ctx) {
        println!("{:<28} {:<32} {:>10.2} {:<4} {}", r.group, r.label, r.value, r.unit.suffix(), r.id);
    }
}

/// One-line live status for `-log=`.
fn status(tel: &Telemetry, secs: u64) -> String {
    let s = tel.snapshot();
    let get = |id: &str| s.get(id).map(|x| x.current);
    let mut out = format!("[{secs:>4} s] {} sensors", s.stats.len());
    for (dev, ids) in [
        ("CPU", [("cpu/temp/package", "°C"), ("cpu/clock/avg", "MHz"), ("cpu/load/total", "%")]),
        ("GPU", [("gpu0/temp/core", "°C"), ("gpu0/clock/core", "MHz"), ("gpu0/load/core", "%")]),
    ] {
        let v: Vec<String> = ids.iter().filter_map(|(id, u)| get(id).map(|x| format!("{x:.0} {u}"))).collect();
        if !v.is_empty() {
            out += &format!(" | {dev} {}", v.join(" "));
        }
    }
    let th = tel.throttling();
    if !th.is_empty() {
        let v: Vec<String> = th.iter().map(|t| format!("{} {:?}", t.device, t.kind)).collect();
        out += &format!(" | THROTTLING: {}", v.join(", "));
    }
    out
}

/// `-log=FILE[,SECONDS]`: headless CSV logging (Ctrl+C to stop when SECONDS is omitted).
pub fn cli_log(ctx: Ctx, file: &str, seconds: Option<u64>) {
    let path = crate::report_path(if file.is_empty() { "cpuzeez-log" } else { file }, "csv");
    os::catch_ctrl_c();
    let tel = Telemetry::start(Arc::new(ctx), Duration::from_secs(1), Duration::from_secs(120));
    if let Err(e) = tel.start_csv(&path) {
        return eprintln!("{}: {e}", path.display());
    }
    let until = seconds.map_or("Ctrl+C to stop".to_string(), |s| format!("for {s} s, Ctrl+C stops early"));
    eprintln!("logging to {} every 1 s {until}", path.display());
    let t0 = Instant::now();
    let mut shown = u64::MAX;
    while !os::ctrl_c() && seconds.is_none_or(|s| t0.elapsed() < Duration::from_secs(s)) {
        std::thread::sleep(Duration::from_millis(100));
        let secs = t0.elapsed().as_secs();
        if secs != shown {
            shown = secs;
            eprint!("\r{:<110}", status(&tel, secs));
            let _ = std::io::stderr().flush();
        }
    }
    tel.stop_csv();
    eprintln!("\nwrote {}", path.display());
    let costs: Vec<String> =
        tel.costs().iter().map(|(n, k, t)| format!("{n} {:.1} ms x{k}", t.as_secs_f64() * 1e3)).collect();
    eprintln!("source read time (mean): {}", costs.join(", "));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(id: &str, unit: Unit, value: f64) -> Reading {
        Reading { id: id.into(), group: "CPU".into(), label: id.into(), unit, value }
    }

    #[test]
    fn ids() {
        assert_eq!(slug("Core #3"), "core3");
        assert_eq!(slug("+3.3V"), "p3_3v");
        assert_eq!(slug("Charge (AC)"), "charge");
        let mut o = Out::default();
        o.add("a/b/c", "CPU", "x", Unit::Volt, 1.0);
        o.add("a/b/c", "CPU", "x", Unit::Volt, 2.0);
        o.add("a/b/d", "CPU", "x", Unit::Volt, f64::NAN);
        let ids: Vec<&str> = o.v.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["a/b/c", "a/b/c_2"]);
    }

    fn c(key: &str) -> Cond {
        cond(key, ThrottleKind::Thermal, "CPU", "hot".into())
    }

    #[test]
    fn throttle_debounce() {
        let mut d = Debounce::default();
        assert!(d.update(1.0, vec![c("a")]).is_empty(), "one sample is not enough");
        let v = d.update(2.0, vec![c("a"), c("b")]);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].since, 1.0);
        let v = d.update(3.0, vec![c("a"), c("b")]);
        assert_eq!(v.iter().map(|t| t.since).collect::<Vec<_>>(), [1.0, 2.0]);
        assert!(d.update(4.0, vec![c("b")]).iter().all(|t| t.since == 2.0));
        assert!(d.update(5.0, vec![]).is_empty(), "gap clears it");
        assert!(d.update(6.0, vec![c("a")]).is_empty(), "and it must re-qualify");
    }

    #[test]
    fn throttle_rules() {
        let l = Limits { tjmax: 95.0, clock_peak: Some(5000.0), gpu_slowdown: &[Some(87.0)] };
        let hot = |clk: f64| [r("cpu/temp/package", Unit::Celsius, 94.5), r("cpu/clock/avg", Unit::Mhz, clk)];
        assert!(reading_conds(&hot(4950.0), &l).is_empty(), "at Tjmax but holding clocks");
        let v = reading_conds(&hot(4500.0), &l);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, ThrottleKind::Thermal);
        assert!(v[0].detail.contains("5.00 GHz → 4.50 GHz"), "{}", v[0].detail);
        let ppt = [r("cpu/power/ppt", Unit::Watt, 229.0), r("cpu/power/pptlimit", Unit::Watt, 230.0)];
        assert_eq!(reading_conds(&ppt, &l)[0].kind, ThrottleKind::Power);
        let edc = [r("cpu/curr/edc", Unit::Ampere, 200.0), r("cpu/curr/edclimit", Unit::Ampere, 225.0)];
        assert!(reading_conds(&edc, &l).is_empty());
        let gpu = [Reading { group: "GPU · X".into(), ..r("gpu0/temp/core", Unit::Celsius, 88.0) }];
        let v = reading_conds(&gpu, &l);
        assert_eq!((v[0].kind.clone(), v[0].device.as_str()), (ThrottleKind::Gpu, "X"));
    }

    #[test]
    fn intel_status_bits() {
        assert!(intel_conds(Some(0b1010), Some(1 << 31 | 1 << 13), None).is_empty(), "log bits only");
        let v = intel_conds(Some(1 << 10), Some(1 | 1 << 2), Some(1 << 11 | 1 << 7 | 1));
        let kinds: Vec<ThrottleKind> = v.iter().map(|c| c.kind.clone()).collect();
        assert_eq!(kinds, [ThrottleKind::Thermal, ThrottleKind::Power, ThrottleKind::Current]);
        assert_eq!(v[0].detail, "CPU reports thermal status, PROCHOT");
        assert_eq!(v[1].detail, "CPU reports power limit, PL2");
    }
}

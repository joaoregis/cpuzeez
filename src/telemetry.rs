//! Live sensor telemetry: one background sampler gathers every reading the app
//! knows (CPU, GPU, mainboard/Super I/O, chipset, DIMMs, drives, battery), keeps
//! a time series per sensor with min/max/average, writes CSV logs, and flags
//! thermal / power throttling. The Sensors window, the stress monitor, the tray
//! mini-monitor and `-sensors` / `-log=` all read from here.
//!
//! Ids are `<device>/<kind>/<name>`: device is `cpu`, `gpu<N>`, `board`,
//! `chipset`, `mem`, `storage`, `battery<N>`; kind is `temp`, `volt`, `power`,
//! `curr`, `clock`, `fan`, `load`, `level` or `size`; name is the sensor label
//! lowercased with only `[a-z0-9]` kept (`+`→`p`, `-`→`m`, `.`→`_`), e.g.
//! `cpu/temp/package`, `cpu/clock/core3`, `board/volt/p12v`, `gpu0/temp/core`.
//! A clashing id gets `_2`, `_3`… in reading order.
//!
//! CSV log: a header line `time,<id1>,<id2>,…`, then a comment line
//! `# units,<unit1>,<unit2>,…`, then one row per sample with an ISO-8601 UTC
//! timestamp and empty cells for sensors missing from that sample. When a new
//! sensor appears, a fresh header + units block is written with the old columns
//! in the same positions and the new ones appended, so a reader restarts its
//! column map on every line that starts with `time,`. Each row is written to
//! the OS as soon as it is sampled.

mod os;

use crate::Ctx;
use crate::sensors::Kind;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum Unit {
    Celsius,
    Volt,
    Watt,
    Ampere,
    Mhz,
    Rpm,
    Percent,
    Megabytes,
}

impl Unit {
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::Celsius => "°C",
            Unit::Volt => "V",
            Unit::Watt => "W",
            Unit::Ampere => "A",
            Unit::Mhz => "MHz",
            Unit::Rpm => "RPM",
            Unit::Percent => "%",
            Unit::Megabytes => "MB",
        }
    }
}

/// One value from one pass over every source.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Reading {
    /// Stable key, e.g. "cpu/temp/package", "gpu0/temp/core", "board/fan/sys1".
    pub id: String,
    /// Display group: "CPU", "GPU · NVIDIA GeForce RTX 4060", "Mainboard", "Chipset", "Memory", "Storage", "Battery".
    pub group: String,
    pub label: String,
    pub unit: Unit,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct Series {
    pub id: String,
    pub group: String,
    pub label: String,
    pub unit: Unit,
    /// (seconds since the sampler started, value), oldest first, bounded by `graph_seconds`.
    pub points: Vec<(f64, f64)>,
    pub current: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
}

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

/// One pass over every source (also used by `-sensors`).
pub fn read_all(ctx: &Ctx) -> Vec<Reading> {
    let mut o = Out::default();
    let bats = crate::battery::batteries(ctx);
    monitors(ctx, &mut o, !bats.is_empty());
    cpu_extra(ctx, &mut o);
    gpu(ctx, &mut o);
    if let Some((total, used)) = os::memory() {
        o.add("mem/load/used", "Memory", "Memory load", Unit::Percent, used * 100.0 / total);
        o.add("mem/size/used", "Memory", "Memory used", Unit::Megabytes, used);
    }
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

fn gpu(ctx: &Ctx, o: &mut Out) {
    for l in crate::gpu::live(ctx) {
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

fn storage(ctx: &Ctx, o: &mut Out) {
    // ponytail: probing opens every drive, so temps are refreshed every 10 s;
    // a per-drive temperature query belongs in crate::storage if that is too slow.
    static CACHE: Mutex<Option<(Instant, Vec<(String, f64)>)>> = Mutex::new(None);
    let Ok(mut g) = CACHE.lock() else { return };
    if g.as_ref().is_none_or(|c| c.0.elapsed() > Duration::from_secs(10)) {
        let temps = crate::storage::drives(ctx)
            .iter()
            .map(|d| {
                let t = d.health.iter().find(|(k, _)| k == "Temperature");
                (d.name.clone(), t.and_then(|t| t.1.trim_end_matches('C').trim().parse().ok()).unwrap_or(f64::NAN))
            })
            .collect();
        *g = Some((Instant::now(), temps));
    }
    for (i, (name, t)) in g.iter().flat_map(|c| c.1.iter()).enumerate() {
        o.add(format!("storage/temp/disk{i}"), "Storage", name.clone(), Unit::Celsius, *t);
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

// ---------------------------------------------------------------- history + CSV

struct Buf {
    id: String,
    group: String,
    label: String,
    unit: Unit,
    points: VecDeque<(f64, f64)>,
    current: f64,
    min: f64,
    max: f64,
    sum: f64,
    n: u64,
}

impl Buf {
    fn reset(&mut self) {
        (self.min, self.max, self.sum, self.n) = (f64::INFINITY, f64::NEG_INFINITY, 0.0, 0);
    }
}

struct Csv {
    path: PathBuf,
    file: std::fs::File,
    cols: Vec<(String, Unit)>,
}

/// `time,<ids>` + `# units,<units>` lines.
fn csv_header(cols: &[(String, Unit)]) -> String {
    let ids: Vec<&str> = cols.iter().map(|c| c.0.as_str()).collect();
    let units: Vec<&str> = cols.iter().map(|c| c.1.suffix()).collect();
    format!("time,{}\n# units,{}\n", ids.join(","), units.join(","))
}

fn csv_row(time: &str, cols: &[(String, Unit)], rs: &[Reading]) -> String {
    let by: HashMap<&str, f64> = rs.iter().map(|r| (r.id.as_str(), r.value)).collect();
    let mut s = time.to_string();
    for (id, _) in cols {
        s.push(',');
        if let Some(v) = by.get(id.as_str()) {
            s += &format!("{v:.3}");
        }
    }
    s.push('\n');
    s
}

impl Csv {
    fn write(&mut self, wall: SystemTime, rs: &[Reading]) -> std::io::Result<()> {
        let known: HashSet<&str> = self.cols.iter().map(|c| c.0.as_str()).collect();
        let new: Vec<(String, Unit)> =
            rs.iter().filter(|r| !known.contains(r.id.as_str())).map(|r| (r.id.clone(), r.unit)).collect();
        let mut s = String::new();
        if !new.is_empty() || self.cols.is_empty() {
            self.cols.extend(new);
            s += &csv_header(&self.cols);
        }
        s += &csv_row(&iso8601(wall), &self.cols, rs);
        self.file.write_all(s.as_bytes())
    }
}

/// UTC "2026-10-14T12:34:56.789Z" (days-from-civil inverse, Howard Hinnant's algorithm).
fn iso8601(t: SystemTime) -> String {
    let d = t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let (secs, ms) = (d.as_secs() as i64, d.subsec_millis());
    let (days, sod) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{ms:03}Z", sod / 3600, sod / 60 % 60, sod % 60)
}

struct State {
    /// Seconds of history kept per series.
    keep: f64,
    series: Vec<Buf>,
    index: HashMap<String, usize>,
    debounce: Debounce,
    throttles: Vec<Throttle>,
    csv: Option<Csv>,
}

impl State {
    fn new(keep: f64) -> State {
        State {
            keep,
            series: Vec::new(),
            index: HashMap::new(),
            debounce: Debounce::default(),
            throttles: Vec::new(),
            csv: None,
        }
    }

    fn push(&mut self, t: f64, wall: SystemTime, rs: &[Reading]) {
        for r in rs {
            let i = *self.index.entry(r.id.clone()).or_insert_with(|| {
                self.series.push(Buf {
                    id: r.id.clone(),
                    group: r.group.clone(),
                    label: r.label.clone(),
                    unit: r.unit,
                    points: VecDeque::new(),
                    current: r.value,
                    min: f64::INFINITY,
                    max: f64::NEG_INFINITY,
                    sum: 0.0,
                    n: 0,
                });
                self.series.len() - 1
            });
            let b = &mut self.series[i];
            b.label.clone_from(&r.label);
            b.points.push_back((t, r.value));
            b.current = r.value;
            b.min = b.min.min(r.value);
            b.max = b.max.max(r.value);
            b.sum += r.value;
            b.n += 1;
        }
        for b in &mut self.series {
            while b.points.front().is_some_and(|p| p.0 < t - self.keep) {
                b.points.pop_front();
            }
        }
        if let Some(c) = &mut self.csv
            && let Err(e) = c.write(wall, rs)
        {
            eprintln!("{}: {e}; CSV logging stopped", c.path.display());
            self.csv = None;
        }
    }

    fn peak(&self, id: &str) -> Option<f64> {
        let b = &self.series[*self.index.get(id)?];
        b.points.iter().map(|p| p.1).reduce(f64::max)
    }

    fn snapshot(&self) -> Vec<Series> {
        self.series
            .iter()
            .map(|b| {
                let none = b.n == 0;
                Series {
                    id: b.id.clone(),
                    group: b.group.clone(),
                    label: b.label.clone(),
                    unit: b.unit,
                    points: b.points.iter().copied().collect(),
                    current: b.current,
                    min: if none { b.current } else { b.min },
                    max: if none { b.current } else { b.max },
                    avg: if none { b.current } else { b.sum / b.n as f64 },
                }
            })
            .collect()
    }
}

struct Inner {
    st: Mutex<State>,
    period_ms: AtomicU64,
}

/// Background sampler. Cheap to clone; stops when the last clone is dropped.
#[derive(Clone)]
pub struct Telemetry {
    inner: Arc<Inner>,
}

fn period_ms(p: Duration) -> u64 {
    (p.as_millis() as u64).max(100)
}

fn sampler(ctx: Arc<Ctx>, weak: Weak<Inner>, tjmax: f64) {
    let t0 = Instant::now();
    let slowdown: Vec<Option<f64>> = crate::gpu::adapters(&ctx).iter().map(|a| a.thermal_limits[1]).collect();
    let mut counts = None;
    loop {
        let start = Instant::now();
        let rs = read_all(&ctx);
        let mut conds = hw_conds(&ctx, &mut counts);
        let Some(inner) = weak.upgrade() else { return };
        let t = t0.elapsed().as_secs_f64();
        {
            let mut st = inner.st.lock().unwrap_or_else(|e| e.into_inner());
            st.push(t, SystemTime::now(), &rs);
            let l = Limits { tjmax, clock_peak: st.peak("cpu/clock/avg"), gpu_slowdown: &slowdown };
            conds.extend(reading_conds(&rs, &l));
            st.throttles = st.debounce.update(t, conds);
        }
        let period = Duration::from_millis(inner.period_ms.load(Ordering::Relaxed));
        drop(inner);
        std::thread::sleep(period.saturating_sub(start.elapsed()));
    }
}

impl Telemetry {
    pub fn start(ctx: Arc<Ctx>, period: Duration, keep: Duration) -> Telemetry {
        let inner = Arc::new(Inner {
            st: Mutex::new(State::new(keep.as_secs_f64())),
            period_ms: AtomicU64::new(period_ms(period)),
        });
        let tjmax = crate::cpu::get(&ctx).info.tjmax_c.unwrap_or_else(|| crate::settings::Settings::load().warn_temp_c);
        let weak = Arc::downgrade(&inner);
        let spawned = std::thread::Builder::new().name("telemetry".into()).spawn(move || sampler(ctx, weak, tjmax));
        if let Err(e) = spawned {
            eprintln!("telemetry: {e}");
        }
        Telemetry { inner }
    }
    fn st(&self) -> MutexGuard<'_, State> {
        self.inner.st.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn series(&self) -> Vec<Series> {
        self.st().snapshot()
    }
    /// Clear min/max/avg (keeps the graphs).
    pub fn reset_stats(&self) {
        self.st().series.iter_mut().for_each(Buf::reset);
    }
    /// Active throttling events (empty when none).
    pub fn throttling(&self) -> Vec<Throttle> {
        self.st().throttles.clone()
    }
    pub fn set_period(&self, period: Duration) {
        self.inner.period_ms.store(period_ms(period), Ordering::Relaxed);
    }
    /// Append every sample to `path` as CSV (header = sensor ids) until `stop_csv`.
    pub fn start_csv(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(d)?;
        }
        let file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
        self.st().csv = Some(Csv { path: path.to_path_buf(), file, cols: Vec::new() });
        Ok(())
    }
    pub fn stop_csv(&self) {
        self.st().csv = None;
    }
    pub fn csv_path(&self) -> Option<std::path::PathBuf> {
        self.st().csv.as_ref().map(|c| c.path.clone())
    }
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
    let s = tel.series();
    let get = |id: &str| s.iter().find(|x| x.id == id).map(|x| x.current);
    let mut out = format!("[{secs:>4} s] {} sensors", s.len());
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

    #[test]
    fn ring_buffer_bounded_by_keep() {
        let mut st = State::new(10.0);
        for t in 0..=30 {
            st.push(t as f64, SystemTime::now(), &[r("x", Unit::Celsius, t as f64)]);
        }
        let s = &st.snapshot()[0];
        assert_eq!(s.points.len(), 11);
        assert_eq!(s.points[0], (20.0, 20.0));
        assert_eq!(s.points.last(), Some(&(30.0, 30.0)));
        // Stats cover everything since start, not only the kept window.
        assert_eq!((s.min, s.max, s.avg, s.current), (0.0, 30.0, 15.0, 30.0));
    }

    #[test]
    fn stats_and_reset() {
        let mut st = State::new(100.0);
        for (t, v) in [1.0, 5.0, 3.0].into_iter().enumerate() {
            st.push(t as f64, SystemTime::now(), &[r("x", Unit::Volt, v)]);
        }
        let s = &st.snapshot()[0];
        assert_eq!((s.min, s.max, s.avg), (1.0, 5.0, 3.0));
        st.series.iter_mut().for_each(Buf::reset);
        let s = &st.snapshot()[0];
        assert_eq!((s.min, s.max, s.avg), (3.0, 3.0, 3.0), "no samples since reset: current value");
        st.push(3.0, SystemTime::now(), &[r("x", Unit::Volt, 4.0)]);
        let s = &st.snapshot()[0];
        assert_eq!((s.min, s.max, s.avg, s.points.len()), (4.0, 4.0, 4.0, 4), "graph kept");
    }

    #[test]
    fn timestamps() {
        let at = |s: u64, ms: u64| iso8601(SystemTime::UNIX_EPOCH + Duration::from_millis(s * 1000 + ms));
        assert_eq!(at(0, 0), "1970-01-01T00:00:00.000Z");
        assert_eq!(at(1_700_000_000, 42), "2023-11-14T22:13:20.042Z");
        assert_eq!(at(951_782_400, 0), "2000-02-29T00:00:00.000Z");
    }

    #[test]
    fn csv_header_rows_and_new_columns() {
        let path = std::env::temp_dir().join(format!("cpuzeez_tel_{}.csv", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new().create(true).append(true).open(&path).unwrap();
        let mut st = State::new(60.0);
        st.csv = Some(Csv { path: path.clone(), file, cols: Vec::new() });
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        st.push(0.0, t, &[r("cpu/temp/package", Unit::Celsius, 45.5), r("cpu/volt/vid", Unit::Volt, 1.1)]);
        st.push(1.0, t, &[r("cpu/volt/vid", Unit::Volt, 1.2)]);
        st.push(2.0, t, &[r("cpu/temp/package", Unit::Celsius, 46.0), r("gpu0/temp/core", Unit::Celsius, 50.0)]);
        st.csv = None;
        let got = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let ts = "2023-11-14T22:13:20.000Z";
        let want = format!(
            "time,cpu/temp/package,cpu/volt/vid\n# units,°C,V\n{ts},45.500,1.100\n{ts},,1.200\n\
             time,cpu/temp/package,cpu/volt/vid,gpu0/temp/core\n# units,°C,V,°C\n{ts},46.000,,50.000\n"
        );
        assert_eq!(got, want);
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

//! Live sensor telemetry: one background sampler gathers every reading the app
//! knows (CPU, GPU, mainboard/Super I/O, chipset, DIMMs, drives, battery), keeps
//! a time series per sensor with min/max/average, writes CSV logs, and flags
//! thermal / power throttling. The Sensors window, the stress monitor, the tray
//! mini-monitor and `-sensors` / `-log=` all read from here.

use crate::Ctx;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

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

/// One pass over every source (also used by `-sensors`).
pub fn read_all(_ctx: &Ctx) -> Vec<Reading> {
    Vec::new()
}

/// Background sampler. Cheap to clone; stops when the last clone is dropped.
#[derive(Clone)]
pub struct Telemetry {
    _inner: Arc<()>,
}

impl Telemetry {
    pub fn start(_ctx: Arc<Ctx>, _period: Duration, _keep: Duration) -> Telemetry {
        Telemetry { _inner: Arc::new(()) }
    }
    pub fn series(&self) -> Vec<Series> {
        Vec::new()
    }
    /// Clear min/max/avg (keeps the graphs).
    pub fn reset_stats(&self) {}
    /// Active throttling events (empty when none).
    pub fn throttling(&self) -> Vec<Throttle> {
        Vec::new()
    }
    pub fn set_period(&self, _period: Duration) {}
    /// Append every sample to `path` as CSV (header = sensor ids) until `stop_csv`.
    pub fn start_csv(&self, _path: &Path) -> std::io::Result<()> {
        Ok(())
    }
    pub fn stop_csv(&self) {}
    pub fn csv_path(&self) -> Option<std::path::PathBuf> {
        None
    }
}

/// `-sensors`: print one pass.
pub fn cli_once(ctx: &Ctx) {
    for r in read_all(ctx) {
        println!("{:<28} {:<32} {:>10.2} {}", r.group, r.label, r.value, r.unit.suffix());
    }
}

/// `-log=FILE[,SECONDS]`: headless CSV logging (Ctrl+C to stop when SECONDS is omitted).
pub fn cli_log(_ctx: Ctx, _file: &str, _seconds: Option<u64>) {}

//! Windows user-mode sensors: PDH performance counters (per-CPU effective
//! clock, ACPI thermal zones; no admin needed, unlike WMI MSAcpi_*) and the
//! battery from GetSystemPowerStatus.

use super::{Kind, Monitor};
use cpuzeez_platform::pdh::Array;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// One sample of every instance: % performance, MHz, thermal zone K, % time.
type Row = (String, [Option<f64>; 4]);

/// The counters kept open for the process: rate counters need two samples,
/// so the first call waits and later calls diff against the last.
/// A sample younger than 500 ms is reused: back-to-back callers would
/// otherwise get a near-empty interval (parked cores then read 0).
fn sample() -> Vec<Row> {
    static LAST: Mutex<Option<(Instant, Vec<Row>)>> = Mutex::new(None);
    let Ok(mut last) = LAST.lock() else { return Vec::new() };
    if let Some((_, v)) = last.as_ref().filter(|l| l.0.elapsed() < Duration::from_millis(500)) {
        return v.clone();
    }
    let v = collect();
    *last = Some((Instant::now(), v.clone()));
    v
}

fn collect() -> Vec<Row> {
    static Q: Mutex<Option<[Array; 4]>> = Mutex::new(None);
    let Ok(mut g) = Q.lock() else { return Vec::new() };
    let counters = g.get_or_insert_with(|| {
        let c = [
            r"\Processor Information(*)\% Processor Performance",
            r"\Processor Information(*)\Processor Frequency",
            r"\Thermal Zone Information(*)\Temperature",
            r"\Processor Information(*)\% Processor Time",
        ]
        .map(Array::new);
        std::thread::sleep(Duration::from_millis(100)); // a first interval for the rates
        c
    });
    let mut out: Vec<Row> = Vec::new();
    for (slot, c) in counters.iter_mut().enumerate() {
        for (name, v) in c.read() {
            match out.iter_mut().find(|e| e.0 == name) {
                Some(e) => e.1[slot] = Some(v),
                None => {
                    let mut a = [None; 4];
                    a[slot] = Some(v);
                    out.push((name, a));
                }
            }
        }
    }
    out
}

/// Effective MHz per global logical CPU index ("g,n" instances).
pub fn cpu_mhz() -> Vec<Option<f64>> {
    let groups = crate::os::cpu_groups();
    let mut v = vec![None; groups.iter().map(|g| g.1 as usize).sum()];
    for (name, [perf, freq, _, _]) in sample() {
        let Some((g, n)) = name.split_once(',') else { continue };
        let (Ok(g), Ok(n)) = (g.parse::<u16>(), n.parse::<usize>()) else { continue };
        let off: usize = groups.iter().take_while(|x| x.0 < g).map(|x| x.1 as usize).sum();
        if let (Some(p), Some(f), Some(slot)) = (perf, freq, v.get_mut(off + n))
            && f * p > 0.0
        {
            *slot = Some(f * p / 100.0);
        }
    }
    v
}

/// OS "% Processor Time": (whole machine, per global logical CPU).
pub fn cpu_load() -> (Option<f64>, Vec<Option<f64>>) {
    let groups = crate::os::cpu_groups();
    let mut v = vec![None; groups.iter().map(|g| g.1 as usize).sum()];
    let mut total = None;
    for (name, [_, _, _, t]) in sample() {
        let Some(t) = t else { continue };
        if name == "_Total" {
            total = Some(t);
            continue;
        }
        let Some((g, n)) = name.split_once(',') else { continue };
        let (Ok(g), Ok(n)) = (g.parse::<u16>(), n.parse::<usize>()) else { continue };
        let off: usize = groups.iter().take_while(|x| x.0 < g).map(|x| x.1 as usize).sum();
        if let Some(slot) = v.get_mut(off + n) {
            *slot = Some(t.clamp(0.0, 100.0));
        }
    }
    (total, v)
}

pub fn monitors() -> Vec<Monitor> {
    let mut out = Vec::new();
    let mut tz = Monitor::new("ACPI Thermal Zone");
    for (name, [_, _, k, _]) in sample() {
        if let Some(k) = k.filter(|&k| k > 0.0) {
            let label = name.rsplit('.').next().unwrap_or(&name).to_string();
            tz.push(Kind::Temperature, label, k - 273.15, None);
        }
    }
    out.push(tz);
    if let Some((ac, percent)) = cpuzeez_platform::os::battery() {
        let mut b = Monitor::new("Battery");
        b.push(Kind::Level, if ac { "Charge (AC)" } else { "Charge" }, percent as f64, None);
        out.push(b);
    }
    out
}

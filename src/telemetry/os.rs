//! OS-level telemetry inputs that no hardware monitor covers: per-CPU load,
//! kernel throttle counters and the Ctrl+C flag for `-log=`. Windows load
//! comes from the sensors module's PDH query (one open query per process);
//! Linux reads /proc/stat, macOS uses Mach host statistics. RAM is
//! `cpuzeez_platform::memory`.

/// (whole machine %, per logical CPU %).
#[cfg(windows)]
pub fn cpu_load() -> (Option<f64>, Vec<Option<f64>>) {
    crate::sensors::cpu_load()
}

/// (whole machine %, per logical CPU %) from busy/total tick deltas since the
/// previous call (or over 100 ms the first time).
#[cfg(not(windows))]
pub fn cpu_load() -> (Option<f64>, Vec<Option<f64>>) {
    use std::sync::Mutex;
    static LAST: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());
    let Ok(mut last) = LAST.lock() else { return (None, Vec::new()) };
    let Some(mut now) = ticks() else { return (None, Vec::new()) };
    if last.len() != now.len() {
        *last = now;
        std::thread::sleep(std::time::Duration::from_millis(100));
        let Some(n) = ticks() else { return (None, Vec::new()) };
        now = n;
    }
    let pct: Vec<Option<f64>> = now
        .iter()
        .zip(last.iter())
        .map(|(&(b1, t1), &(b0, t0))| {
            let dt = t1.saturating_sub(t0);
            (dt > 0).then(|| (b1.saturating_sub(b0) as f64 * 100.0 / dt as f64).clamp(0.0, 100.0))
        })
        .collect();
    *last = now;
    match pct.split_first() {
        Some((total, per)) => (*total, per.to_vec()),
        None => (None, Vec::new()),
    }
}

/// (busy, total) ticks: whole machine first, then every logical CPU.
#[cfg(target_os = "linux")]
fn ticks() -> Option<Vec<(u64, u64)>> {
    let s = std::fs::read_to_string("/proc/stat").ok()?;
    let v = parse_proc_stat(&s);
    (!v.is_empty()).then_some(v)
}

/// `cpu`/`cpuN` lines: busy = everything but idle + iowait (proc(5)).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn parse_proc_stat(s: &str) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for l in s.lines().filter(|l| l.starts_with("cpu")) {
        // guest/guest_nice are already counted in user/nice.
        let f: Vec<u64> = l.split_whitespace().skip(1).take(8).filter_map(|x| x.parse().ok()).collect();
        if f.len() < 4 {
            continue;
        }
        let total: u64 = f.iter().sum();
        let idle = f[3] + f.get(4).copied().unwrap_or(0);
        out.push((total - idle, total));
    }
    out
}

/// host_processor_info: user, system, idle, nice ticks per CPU; the total first.
#[cfg(target_os = "macos")]
fn ticks() -> Option<Vec<(u64, u64)>> {
    let mut per: Vec<(u64, u64)> =
        crate::os::cpu_load_ticks()?.into_iter().map(|[u, s, i, ni]| (u + s + ni, u + s + i + ni)).collect();
    let total = per.iter().fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
    per.insert(0, total);
    Some(per)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn ticks() -> Option<Vec<(u64, u64)>> {
    None
}

/// Sum of the kernel's thermal_throttle counters over all CPUs:
/// (`*_throttle_count`, `*_power_limit_count`). Intel-only on current kernels.
#[cfg(target_os = "linux")]
pub fn throttle_counts() -> Option<(u64, u64)> {
    let rd = std::fs::read_dir("/sys/devices/system/cpu").ok()?;
    let (mut th, mut pw, mut any) = (0, 0, false);
    for e in rd.flatten() {
        let Ok(files) = std::fs::read_dir(e.path().join("thermal_throttle")) else { continue };
        for f in files.flatten() {
            let name = f.file_name().to_string_lossy().into_owned();
            let Some(v) = crate::os::read_u64(f.path()) else { continue };
            any = true;
            if name.ends_with("_throttle_count") {
                th += v;
            } else if name.ends_with("_power_limit_count") {
                pw += v;
            }
        }
    }
    any.then_some((th, pw))
}

#[cfg(not(target_os = "linux"))]
pub fn throttle_counts() -> Option<(u64, u64)> {
    None
}

pub use crate::os::{catch_ctrl_c, ctrl_c};

#[cfg(test)]
mod tests {
    #[test]
    fn proc_stat() {
        let s = "cpu  10 0 10 70 10 0 0 0 0 0\ncpu0 5 0 5 35 5 0 0 0 0 0\nintr 1 2 3\n";
        assert_eq!(super::parse_proc_stat(s), vec![(20, 100), (10, 50)]);
    }
}

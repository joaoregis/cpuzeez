//! ZeeZ Live's frame source (S2b-05, S2b-06; docs/v1/zeez-live.md §3.1).
//!
//! One hub holds the target app's recent frames, whatever produced them:
//! - Windows: the bundled PresentMon console (Intel, MIT) as a sidecar. It
//!   reads the graphics kernel's ETW events, so nothing touches the game
//!   (P4). It needs an administrator. It runs only while something reads
//!   `frames/*` (the overlay, the Live tab, a capture) and stops 5 s later.
//!   Its SHA-256 is checked against `packaging/pins.txt` before it runs.
//! - Linux: MangoHud's CSV logs (S2b-06) feed the same hub.
//!
//! The telemetry [`Source`](cpuzeez_core::metrics::Source) `frames` turns the
//! hub into `frames/fps`, `frames/ms`, `frames/low1`, `frames/low01` and
//! `frames/gpu_busy`, so layouts, logs and captures read them like any metric.

use cpuzeez_core::frames::{self, Frame, LowConvention};
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Seconds of frames kept for the rolling lows.
pub const WINDOW_S: f64 = 30.0;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Status {
    #[default]
    Off,
    Starting,
    Running,
    /// Frame timing needs an administrator on Windows.
    NeedsAdmin,
    /// The sidecar isn't next to CpuZeeZ (a dev build, or a damaged install).
    Missing,
    /// It is there but its SHA-256 isn't the pinned one: it was not run.
    BadChecksum,
    Failed(String),
}

/// An app presenting frames, for the target picker.
#[derive(Debug, Clone, PartialEq)]
pub struct App {
    pub pid: u32,
    pub name: String,
    pub fps: f64,
}

#[derive(Default)]
struct Hub {
    status: Status,
    /// Manual target; None picks the foreground app.
    pinned: Option<u32>,
    target: Option<(u32, String)>,
    frames: VecDeque<Frame>,
    /// Frames per app in the last 2 s: (name, times).
    seen: HashMap<u32, (String, VecDeque<f64>)>,
    wanted_at: Option<Instant>,
    running: bool,
    /// Present mode of the target (exclusive fullscreen hides the overlay).
    present_mode: String,
}

fn hub() -> &'static Mutex<Hub> {
    static H: OnceLock<Mutex<Hub>> = OnceLock::new();
    H.get_or_init(Default::default)
}

fn lock() -> std::sync::MutexGuard<'static, Hub> {
    hub().lock().unwrap_or_else(|e| e.into_inner())
}

/// Keep the frame source running for the next 5 s.
pub fn want() {
    let start = {
        let mut h = lock();
        h.wanted_at = Some(Instant::now());
        !h.running
    };
    if start {
        imp::start();
    }
}

fn wanted() -> bool {
    lock().wanted_at.is_some_and(|t| t.elapsed() < Duration::from_secs(5))
}

pub fn status() -> Status {
    lock().status.clone()
}

/// Follow `pid` (None: the foreground app).
pub fn pin_target(pid: Option<u32>) {
    let mut h = lock();
    h.pinned = pid;
    h.frames.clear();
}

/// (pid, name) of the app being measured.
pub fn target() -> Option<(u32, String)> {
    lock().target.clone()
}

pub fn present_mode() -> String {
    lock().present_mode.clone()
}

/// Apps that presented frames in the last 2 s, busiest first.
pub fn apps() -> Vec<App> {
    let h = lock();
    let mut v: Vec<App> = h
        .seen
        .iter()
        .map(|(pid, (name, t))| App { pid: *pid, name: name.clone(), fps: t.len() as f64 / 2.0 })
        .collect();
    v.sort_by(|a, b| b.fps.total_cmp(&a.fps));
    v
}

/// The target's frames of the last `secs` seconds.
pub fn recent(secs: f64) -> Vec<Frame> {
    let h = lock();
    let Some(end) = h.frames.back().map(|f| f.t_s) else { return Vec::new() };
    h.frames.iter().filter(|f| f.t_s >= end - secs).cloned().collect()
}

/// Processes that present but are never the game.
fn ignored(name: &str, pid: u32) -> bool {
    pid == std::process::id()
        || ["dwm.exe", "explorer.exe", "searchhost.exe", "startmenuexperiencehost.exe", "textinputhost.exe"]
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name))
}

/// Add frames from a source; picks the target (pinned, else the foreground
/// app if it presents, else the busiest presenter) and keeps its frames.
pub fn push(batch: Vec<Frame>, foreground: Option<u32>) {
    let mut h = lock();
    for f in batch {
        if ignored(&f.app, f.pid) {
            continue;
        }
        let e = h.seen.entry(f.pid).or_insert_with(|| (f.app.clone(), VecDeque::new()));
        e.1.push_back(f.t_s);
        while e.1.front().is_some_and(|t| *t < f.t_s - 2.0) {
            e.1.pop_front();
        }
        let target = h.pinned.or_else(|| {
            let busiest = || h.seen.iter().max_by_key(|(_, (_, t))| t.len()).map(|(p, _)| *p);
            match foreground {
                Some(fg) if h.seen.contains_key(&fg) => Some(fg),
                _ => h.target.as_ref().map(|t| t.0).filter(|p| h.seen.contains_key(p)).or_else(busiest),
            }
        });
        if target != h.target.as_ref().map(|t| t.0) {
            h.frames.clear();
            h.target = target.map(|p| (p, h.seen.get(&p).map(|s| s.0.clone()).unwrap_or_default()));
        }
        if Some(f.pid) == target {
            h.present_mode = f.present_mode.clone();
            h.frames.push_back(f);
        }
    }
    let end = h.frames.back().map_or(0.0, |f| f.t_s);
    while h.frames.front().is_some_and(|f| f.t_s < end - WINDOW_S) {
        h.frames.pop_front();
    }
    // Apps that stopped presenting leave the picker.
    let latest = h.seen.values().filter_map(|s| s.1.back().copied()).fold(0.0, f64::max);
    h.seen.retain(|_, s| s.1.back().is_some_and(|t| *t >= latest - 5.0));
}

fn set_status(s: Status) {
    lock().status = s;
}

/// The readings `frames/*` for the telemetry engine.
pub fn readings() -> Vec<(&'static str, &'static str, cpuzeez_core::metrics::Unit, f64)> {
    use cpuzeez_core::metrics::Unit;
    let all = recent(WINDOW_S);
    let last1 = frames::last_secs(&all, 1.0);
    if last1.len() < 2 {
        return Vec::new();
    }
    let span = (last1[last1.len() - 1].t_s - last1[0].t_s).max(1e-3);
    let fps = (last1.len() - 1) as f64 / span;
    let ms = last1.iter().map(|f| f.ms).sum::<f64>() / last1.len() as f64;
    let s = frames::summarize(&all, LowConvention::Percentile);
    let mut v = vec![
        ("frames/fps", "FPS", Unit::Count, fps),
        ("frames/ms", "Frame time", Unit::Millis, ms),
        ("frames/low1", "1% low", Unit::Count, s.low1_fps),
        ("frames/low01", "0.1% low", Unit::Count, s.low01_fps),
    ];
    let busy: Vec<f64> = last1.iter().filter_map(|f| f.gpu_busy_ms).collect();
    if !busy.is_empty() {
        v.push(("frames/gpu_busy", "GPU Busy", Unit::Millis, busy.iter().sum::<f64>() / busy.len() as f64));
    }
    v
}

#[cfg(windows)]
mod imp {
    use super::{Status, set_status};
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    /// Where the packages put the sidecar (next to cpuzeez.exe); a dev
    /// build can point `CPUZEEZ_PRESENTMON` at one.
    fn sidecar() -> Option<std::path::PathBuf> {
        if let Some(p) = std::env::var_os("CPUZEEZ_PRESENTMON") {
            return Some(p.into());
        }
        let p = std::env::current_exe().ok()?.with_file_name("PresentMon.exe");
        p.exists().then_some(p)
    }

    pub fn start() {
        let Some(exe) = sidecar() else {
            set_status(Status::Missing);
            return;
        };
        // Checked every start: a swapped binary next to us is never run.
        match crate::update::sha256_file(&exe) {
            Ok(h) if h.eq_ignore_ascii_case(crate::executor::pin("PRESENTMON_SHA256")) => {}
            _ => {
                set_status(Status::BadChecksum);
                return;
            }
        }
        if !cpuzeez_platform::elevation::is_elevated() {
            set_status(Status::NeedsAdmin);
            return;
        }
        super::lock().running = true;
        set_status(Status::Starting);
        let spawned = std::thread::Builder::new().name("cz-presentmon".into()).spawn(move || {
            run(&exe);
            super::lock().running = false;
        });
        if spawned.is_err() {
            super::lock().running = false;
            set_status(Status::Failed("no thread".into()));
        }
    }

    fn run(exe: &std::path::Path) {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let child = Command::new(exe)
            .args([
                "--output_stdout",
                "--no_console_stats",
                "--session_name",
                "CpuZeeZ-PresentMon",
                "--stop_existing_session",
                "--no_track_input",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => return set_status(Status::Failed(e.to_string())),
        };
        let Some(out) = child.stdout.take() else { return };
        let mut cols = None;
        let mut batch = Vec::new();
        let mut last_push = std::time::Instant::now();
        for line in BufReader::new(out).lines() {
            let Ok(line) = line else { break };
            match &cols {
                None => {
                    cols = cpuzeez_core::frames::PresentMonColumns::parse(&line);
                    if cols.is_some() {
                        set_status(Status::Running);
                    }
                }
                Some(c) => {
                    if let Some(f) = c.frame(&line) {
                        batch.push(f);
                    }
                }
            }
            if last_push.elapsed().as_millis() >= 100 && !batch.is_empty() {
                super::push(std::mem::take(&mut batch), cpuzeez_platform::desktop::foreground_pid());
                last_push = std::time::Instant::now();
                if !super::wanted() {
                    break;
                }
            }
        }
        let _ = child.kill();
        let status = child.wait_with_output().ok();
        let err = status.as_ref().map(|o| String::from_utf8_lossy(&o.stderr).trim().to_string()).unwrap_or_default();
        if cols.is_none() {
            set_status(if err.to_ascii_lowercase().contains("access") {
                Status::NeedsAdmin
            } else {
                Status::Failed(err.lines().next().unwrap_or("PresentMon stopped").to_string())
            });
        } else {
            set_status(Status::Off);
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn start() {
        // Linux: MangoHud logs are read by `live_mangohud` (S2b-06); macOS: experimental (S2b-07).
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(pid: u32, app: &str, t: f64) -> Frame {
        Frame { pid, app: app.into(), t_s: t, ms: 10.0, ..Default::default() }
    }

    /// Hub state is global: one test walks every case in order.
    #[test]
    fn hub_picks_the_target_and_reports() {
        pin_target(None);
        // A game and the compositor present; the compositor is never a target.
        let mut batch = Vec::new();
        for i in 0..200 {
            batch.push(f(10, "game.exe", i as f64 * 0.01));
            batch.push(f(20, "dwm.exe", i as f64 * 0.01));
        }
        push(batch, None);
        assert_eq!(target().map(|t| t.0), Some(10));
        assert!(apps().iter().all(|a| a.name != "dwm.exe"));
        let r = readings();
        let fps = r.iter().find(|x| x.0 == "frames/fps").unwrap().3;
        assert!((fps - 100.0).abs() < 1.0, "10 ms frames: {fps}");
        // The foreground app wins once it presents.
        push((0..50).map(|i| f(30, "other.exe", 2.0 + i as f64 * 0.01)).collect(), Some(30));
        assert_eq!(target().map(|t| t.1), Some("other.exe".to_string()));
        // A pinned target holds against the foreground.
        pin_target(Some(10));
        push((0..10).map(|i| f(10, "game.exe", 2.5 + i as f64 * 0.01)).collect(), Some(30));
        assert_eq!(target().map(|t| t.0), Some(10));
        pin_target(None);
    }
}

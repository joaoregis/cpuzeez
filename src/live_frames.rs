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
    #[cfg_attr(target_os = "macos", allow(dead_code))] // no frame source on macOS yet
    Starting,
    Running,
    /// Frame timing needs an administrator on Windows.
    NeedsAdmin,
    /// The sidecar isn't next to CPU ZeeZ (a dev build, or a damaged install).
    #[cfg_attr(not(windows), allow(dead_code))] // PresentMon (Windows)
    Missing,
    /// It is there but its SHA-256 isn't the pinned one: it was not run.
    #[cfg_attr(not(windows), allow(dead_code))] // PresentMon (Windows)
    BadChecksum,
    #[cfg_attr(not(windows), allow(dead_code))] // PresentMon (Windows)
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
    /// Every target frame since `begin_capture` (None: not capturing).
    captured: Option<Vec<Frame>>,
    /// The running sidecar's PID (Windows), to stop it on exit.
    #[cfg_attr(not(windows), allow(dead_code))] // PresentMon (Windows)
    child: Option<u32>,
    /// When the target last presented a frame.
    last_frame: Option<Instant>,
    /// `-screenshots`: a recorded capture replayed in a loop instead of a
    /// frame source, and how far it has been played.
    demo: Option<Demo>,
}

struct Demo {
    frames: Vec<Frame>,
    started: Instant,
    /// Frames already pushed, counting every loop.
    played: usize,
}

/// At most this many frames per capture (about 2.3 h at 240 FPS).
#[cfg_attr(target_os = "macos", allow(dead_code))] // no frame source on macOS yet
const CAPTURE_MAX: usize = 2_000_000;

/// Start keeping every frame of the target.
pub fn begin_capture() {
    lock().captured = Some(Vec::new());
}

/// Stop, and return the frames with times from the capture's first frame.
pub fn end_capture() -> Vec<Frame> {
    let mut v = lock().captured.take().unwrap_or_default();
    if let Some(t0) = v.first().map(|f| f.t_s) {
        for f in &mut v {
            f.t_s -= t0;
        }
    }
    v
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
        h.demo.is_none() && !h.running
    };
    if start {
        imp::start();
    }
    replay();
}

/// `-screenshots`: replay `frames` (a recorded capture) in a loop, as if a
/// game presented them now, and never start the real frame source (10.4).
pub fn demo(frames: Vec<Frame>) {
    let mut h = lock();
    if frames.len() < 2 {
        return;
    }
    h.demo = Some(Demo { frames, started: Instant::now(), played: 0 });
    h.status = Status::Running;
}

/// Push the demo frames due by now.
fn replay() {
    let batch = {
        let mut h = lock();
        let Some(d) = h.demo.as_mut() else { return };
        let now = d.started.elapsed().as_secs_f64();
        due(d, now)
    };
    if !batch.is_empty() {
        push(batch, None);
    }
}

/// The demo frames with a time up to `now` (seconds since it started) not
/// pushed yet, looping over the capture with times that keep growing.
fn due(d: &mut Demo, now: f64) -> Vec<Frame> {
    let n = d.frames.len();
    let t0 = d.frames[0].t_s;
    let period = d.frames[n - 1].t_s - t0 + d.frames[n - 1].ms / 1e3;
    let mut batch = Vec::new();
    loop {
        let (lap, i) = (d.played / n, d.played % n);
        let t = lap as f64 * period + d.frames[i].t_s - t0;
        // At most a few seconds per call, if the UI stalled for a long time.
        if t > now || batch.len() > 10_000 {
            break;
        }
        batch.push(Frame { t_s: t, ..d.frames[i].clone() });
        d.played += 1;
    }
    batch
}

#[cfg_attr(target_os = "macos", allow(dead_code))] // no frame source on macOS yet
pub fn wanted() -> bool {
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

/// How long ago the target last presented a frame (None: never).
pub fn frame_age() -> Option<Duration> {
    lock().last_frame.map(|t| t.elapsed())
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

/// Only the frame times of the last `secs` seconds (cheap: no strings copied).
pub fn recent_ms(secs: f64) -> Vec<f64> {
    let h = lock();
    let Some(end) = h.frames.back().map(|f| f.t_s) else { return Vec::new() };
    h.frames.iter().filter(|f| f.t_s >= end - secs).map(|f| f.ms).collect()
}

/// Processes that present but are never the game.
#[cfg_attr(target_os = "macos", allow(dead_code))] // no frame source on macOS yet
fn ignored(name: &str, pid: u32) -> bool {
    pid == std::process::id()
        || ["dwm.exe", "explorer.exe", "searchhost.exe", "startmenuexperiencehost.exe", "textinputhost.exe"]
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name))
}

/// Add frames from a source; picks the target (pinned, else the foreground
/// app if it presents, else the busiest presenter) and keeps its frames.
#[cfg_attr(target_os = "macos", allow(dead_code))] // no frame source on macOS yet
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
            if let Some(c) = h.captured.as_mut().filter(|c| c.len() < CAPTURE_MAX) {
                c.push(f.clone());
            }
            h.frames.push_back(f);
            h.last_frame = Some(Instant::now());
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

/// A source delivered frames (the Linux log reader).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn set_running() {
    set_status(Status::Running);
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

/// Stop the frame source now (on exit: a sidecar outlives its parent otherwise).
pub fn stop() {
    imp::stop();
}

#[cfg(windows)]
mod imp {
    use super::{Status, set_status};
    use std::io::Read;
    use std::process::{Command, Stdio};

    const SESSION: &str = "CpuZeeZ-PresentMon";

    /// The verified PresentMon.exe, held open with read-only sharing for the
    /// whole session. In a user-writable (portable) install folder nobody can
    /// modify, rename or delete the file while we hold it, so nothing can be
    /// swapped in between the SHA-256 check and any later run of it — the
    /// reader loop, a restart after idle, or the
    /// `--terminate_existing_session` call at stop. The same pattern as the
    /// PawnIO installer (src/executor.rs): this runs elevated, so a swapped
    /// binary would run with the administrator token.
    ///
    /// Scope: this pins the sidecar *binary*. An install folder writable by
    /// other users remains unsupported — such a user could plant a DLL beside
    /// either exe or replace cpuzeez.exe itself; only a per-user (or
    /// administrator-owned) install folder is a supported configuration.
    /// (verified handle, resolved final path) — the handle and the path are
    /// set together and only replaced by a later verified `start()`.
    fn held() -> &'static std::sync::Mutex<Option<(std::fs::File, std::path::PathBuf)>> {
        static H: std::sync::OnceLock<std::sync::Mutex<Option<(std::fs::File, std::path::PathBuf)>>> =
            std::sync::OnceLock::new();
        H.get_or_init(|| std::sync::Mutex::new(None))
    }

    fn hold(f: std::fs::File, run_path: std::path::PathBuf) {
        *held().lock().unwrap_or_else(|e| e.into_inner()) = Some((f, run_path));
    }

    fn release() {
        *held().lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// The resolved path of the verified sidecar — what every later run uses.
    fn verified() -> Option<std::path::PathBuf> {
        held().lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(_, p)| p.clone())
    }

    /// The file's final path from the held handle itself, so later executions
    /// cannot be redirected by retargeting a junction (reparse point) that
    /// sits in the original path: the handle pins the real file, and this
    /// returns the real location it pinned — not whatever the original path
    /// resolves to now.
    fn final_path_of(f: &std::fs::File) -> Option<std::path::PathBuf> {
        use std::os::windows::ffi::OsStringExt;
        use std::os::windows::io::AsRawHandle;
        // SAFETY: kernel32 is always loaded on Windows; the declaration
        // matches GetFinalPathNameByHandleW (HANDLE, PWSTR buffer, DWORD
        // length in chars, DWORD flags; returns chars written, or the needed
        // length when the buffer is too small, or 0 on failure).
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetFinalPathNameByHandleW(
                hfile: *mut core::ffi::c_void,
                lpszfile: *mut u16,
                cchfile: u32,
                dwflags: u32,
            ) -> u32;
        }
        const CAP: u32 = 1024;
        let mut buf = vec![0u16; CAP as usize];
        // SAFETY: `f` owns a valid file handle; buf/CAP describe a writable
        // buffer of CAP wide chars; flags 0 = DOS volume name + normalized.
        let n = unsafe { GetFinalPathNameByHandleW(f.as_raw_handle(), buf.as_mut_ptr(), CAP, 0) };
        if n == 0 || n >= CAP {
            return None;
        }
        let s =
            std::path::PathBuf::from(std::ffi::OsString::from_wide(&buf[..n as usize])).to_string_lossy().into_owned();
        // \\?\C:\… → C:\… and \\?\UNC\server\share → \\server\share
        // (CreateProcess prefers the plain drive form).
        let plain = if let Some(p) = s.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{p}")
        } else if let Some(p) = s.strip_prefix(r"\\?\") {
            p.to_string()
        } else {
            s
        };
        Some(std::path::PathBuf::from(plain))
    }

    /// A killed PresentMon leaves its ETW session running: end it by name.
    fn end_session(exe: &std::path::Path) {
        use std::os::windows::process::CommandExt;
        let _ = Command::new(exe)
            .args(["--terminate_existing_session", "--session_name", SESSION])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .status();
    }

    pub fn stop() {
        let pid = super::lock().child.take();
        if let Some(pid) = pid {
            let _ = cpuzeez_platform::proc_ops::kill(pid);
            // Teardown runs the *verified* path from the held handle, not a
            // fresh lookup of the original (junction-able) path.
            if let Some(exe) = verified() {
                end_session(&exe);
            }
        }
        // The verified sidecar stays held until process exit (the OS then
        // closes it): the reader thread may run its own end_session after us,
        // and that run must be protected by the same verified handle.
    }

    /// Where the packages put the sidecar (next to cpuzeez.exe); a dev
    /// build can point `CPUZEEZ_PRESENTMON` at one — absolute paths only:
    /// a bare or relative name would resolve differently for the hash open
    /// (current directory) than for CreateProcess (app dir, System32, PATH),
    /// so the file that was verified and the file that runs could differ.
    fn sidecar() -> Option<std::path::PathBuf> {
        if let Some(p) = std::env::var_os("CPUZEEZ_PRESENTMON") {
            let p = std::path::PathBuf::from(p);
            return p.is_absolute().then_some(p);
        }
        let p = std::env::current_exe().ok()?.with_file_name("PresentMon.exe");
        p.exists().then_some(p)
    }

    pub fn start() {
        use std::os::windows::fs::OpenOptionsExt;
        let Some(exe) = sidecar() else {
            set_status(Status::Missing);
            return;
        };
        // Held open without write/delete sharing from the hash check until
        // stop(): a swapped binary next to us is never run — not even at the
        // session teardown, which the hash check used to skip entirely.
        const FILE_SHARE_READ: u32 = 1;
        let mut held = match std::fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ).open(&exe) {
            Ok(f) => f,
            Err(e) => {
                set_status(Status::Failed(e.to_string()));
                return;
            }
        };
        match crate::update::sha256_of(&mut held) {
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
        // Resolve what the handle actually pinned and run that from now on:
        // re-resolving the original path on every spawn would re-traverse any
        // junction in it, which an attacker with write access to that part of
        // the path could retarget between the check and a later run. Falls
        // back to the (already absolute) original path only if the OS cannot
        // answer.
        let run_path = final_path_of(&held).unwrap_or_else(|| exe.clone());
        hold(held, run_path.clone());
        super::lock().running = true;
        set_status(Status::Starting);
        let spawned = std::thread::Builder::new().name("cz-presentmon".into()).spawn(move || {
            run(&run_path);
            super::lock().running = false;
        });
        if spawned.is_err() {
            super::lock().running = false;
            release();
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
                SESSION,
                "--stop_existing_session",
                "--no_track_input",
                // Marks Frame Generation frames (FrameType), where the driver reports them.
                "--track_frame_type",
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
        super::lock().child = Some(child.id());
        let Some(mut out) = child.stdout.take() else { return };
        let mut cols = None;
        let mut batch = Vec::new();
        let mut last_push = std::time::Instant::now();
        // UTF-8 through a pipe, UTF-16LE to a file: the decoder tells which.
        let mut dec = cpuzeez_core::frames::OutputLines::default();
        let mut buf = [0u8; 16 * 1024];
        'read: loop {
            let n = match out.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            for line in dec.feed(&buf[..n]) {
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
            }
            if last_push.elapsed().as_millis() >= 100 && !batch.is_empty() {
                super::push(std::mem::take(&mut batch), cpuzeez_platform::desktop::foreground_pid());
                last_push = std::time::Instant::now();
            }
            if !super::wanted() {
                break 'read;
            }
        }
        let _ = child.kill();
        super::lock().child = None;
        end_session(exe);
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

#[cfg(target_os = "linux")]
mod imp {
    pub fn stop() {}

    pub fn start() {
        super::lock().running = true;
        super::set_status(super::Status::Starting);
        let ok = std::thread::Builder::new().name("cz-mangohud".into()).spawn(|| {
            crate::live_mangohud::tail();
            super::lock().running = false;
            super::set_status(super::Status::Off);
        });
        if ok.is_err() {
            super::lock().running = false;
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn stop() {}

    pub fn start() {
        // macOS has no frame timing for other apps (zeez-live §3.1): see live.rs's Metal HUD launcher.
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

    /// A file held the way the sidecar is held (read + FILE_SHARE_READ) can
    /// neither be written nor renamed by anyone else until it is dropped —
    /// the guarantee the verify→execute fix rests on.
    #[test]
    #[cfg(windows)]
    fn held_handle_blocks_writes_and_renames() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = std::env::temp_dir().join(format!("cz-share-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("held.exe");
        std::fs::write(&p, b"MZ").unwrap();
        const FILE_SHARE_READ: u32 = 1;
        let held = std::fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ).open(&p).unwrap();
        assert!(
            std::fs::OpenOptions::new().write(true).open(&p).is_err(),
            "a write open must be refused while the verified file is held"
        );
        assert!(std::fs::rename(&p, dir.join("gone.exe")).is_err(), "renaming the held file away must be refused");
        drop(held);
        assert!(std::fs::OpenOptions::new().write(true).open(&p).is_ok(), "after release the file is writable again");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The real sidecar end to end (Windows, administrator, a GPU presenting):
    /// `CPUZEEZ_PRESENTMON=path\PresentMon.exe cargo test sidecar -- --ignored`.
    #[test]
    #[ignore]
    #[cfg(windows)]
    fn sidecar_delivers_frames_and_cleans_up() {
        for _ in 0..40 {
            want();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let (st, apps) = (status(), apps());
        stop();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let sessions = std::process::Command::new("logman").args(["query", "-ets"]).output().unwrap();
        let left = String::from_utf8_lossy(&sessions.stdout).contains("CpuZeeZ-PresentMon");
        assert_eq!(st, Status::Running, "apps seen: {apps:?}");
        assert!(!apps.is_empty(), "something presented frames");
        assert!(!left, "the ETW session was ended");
        eprintln!("apps presenting: {apps:?}");
    }

    /// The screenshot demo plays a capture in a loop, in time order.
    #[test]
    fn the_demo_replays_in_a_loop() {
        let frames: Vec<Frame> = (0..10).map(|i| f(7, "game.exe", 5.0 + i as f64 * 0.01)).collect();
        let mut d = Demo { frames, started: Instant::now(), played: 0 };
        let first = due(&mut d, 0.045);
        assert_eq!(first.len(), 5, "frames at 0, 10 … 40 ms");
        assert!(due(&mut d, 0.045).is_empty(), "nothing twice");
        let more = due(&mut d, 0.25);
        assert_eq!(first.len() + more.len(), 26, "two and a half laps of 100 ms");
        assert!(more.windows(2).all(|w| w[1].t_s > w[0].t_s), "times keep growing across laps");
    }
}

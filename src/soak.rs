//! The soak-test hooks (S3-02), compiled only with `--features soak`, which
//! no release has: a synthetic game that feeds the frame hub, so that the
//! overlay, the capture, the detector and the reports run for hours without a
//! person or a GPU. `.github/scripts/soak.py` builds and drives it, then
//! checks that memory, handles and threads plateau (docs/testing/soak.md).
//!
//! It only runs when `CPUZEEZ_SOAK=1` is set too, so a soak build started by
//! mistake behaves like the normal app. `CPUZEEZ_SOAK=overlay` runs the same
//! synthetic game with no tour and the main window minimised: the overlay next
//! to a game, the case the 2 % CPU budget is for (docs/performance.md).

use cpuzeez_core::frames::Frame;
use std::time::{Duration, Instant};

/// The pid and name the synthetic game reports.
pub const PID: u32 = 4242;
pub const NAME: &str = "soak-game.exe";

pub fn enabled() -> bool {
    std::env::var_os("CPUZEEZ_SOAK").is_some_and(|v| v == "1" || v == "overlay")
}

/// The overlay-only run: no page tour, main window minimised.
pub fn overlay_only() -> bool {
    std::env::var_os("CPUZEEZ_SOAK").is_some_and(|v| v == "overlay")
}

/// A frame of the synthetic game at `t` seconds. The scene changes every 40 s
/// through the cases the detector tells apart: GPU-bound, CPU-bound, locked to
/// a 60 Hz refresh, a steady cap, and a loading stretch with stutters; one
/// frame in 20 s is a hitch.
pub fn frame(t: f64, n: u64) -> Frame {
    let scene = (t / 40.0) as u64 % 5;
    let (ms, gpu, cpu) = match scene {
        0 => (6.9, 6.8, 3.0),                   // GPU-bound at ~145 FPS
        1 => (7.5, 2.0, 7.4),                   // CPU-bound
        2 => (16.667, 6.0, 5.0),                // V-Sync at 60 Hz
        3 => (8.333, 4.0, 3.0),                 // a 120 FPS cap
        _ => (12.0 + (n % 7) as f64, 8.0, 6.0), // loading: uneven
    };
    let hitch = (t % 20.0) < 0.012;
    Frame {
        t_s: t,
        ms: if hitch { 60.0 } else { ms },
        gpu_busy_ms: Some(gpu),
        cpu_busy_ms: Some(cpu),
        present_mode: "Hardware: Independent Flip".into(),
        app: NAME.into(),
        pid: PID,
        ..Default::default()
    }
}

/// Feed the frame hub in 100 ms batches, forever.
pub fn start() {
    if !enabled() {
        return;
    }
    std::thread::Builder::new()
        .name("cz-soak-frames".into())
        .spawn(|| {
            let t0 = Instant::now();
            let (mut t, mut n) = (0.0f64, 0u64);
            loop {
                std::thread::sleep(Duration::from_millis(100));
                let now = t0.elapsed().as_secs_f64();
                let mut batch = Vec::new();
                while t < now {
                    let f = frame(t, n);
                    t += f.ms / 1000.0;
                    n += 1;
                    batch.push(f);
                }
                crate::live_frames::push(batch, Some(PID));
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each scene is what its name says, to the detector.
    #[test]
    fn the_scenes_are_what_they_claim() {
        use cpuzeez_core::bottleneck::{Class, LimiterKind, Thresholds, Window, classify};
        let t = Thresholds::default();
        for (secs, class, limiter) in [
            (10.0, Class::Gpu, None),
            (50.0, Class::Cpu, None),
            (90.0, Class::Limiter, Some(LimiterKind::VSync)),
            (130.0, Class::Limiter, Some(LimiterKind::FpsCap)),
        ] {
            let frames: Vec<Frame> = (0..120).map(|i| frame(secs + 0.005 * i as f64 * 0.0 + 1.0, i)).collect();
            // A second of the scene, with no hitch (they come at t % 20 < 12 ms).
            let mut w = Window::from_frames(&frames);
            w.refresh_hz = Some(60.0);
            w.focused = Some(true);
            let v = classify(&w, &t);
            assert_eq!((v.primary.class, v.primary.limiter), (class, limiter), "scene at {secs}: {w:?}");
        }
    }
}

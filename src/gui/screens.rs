//! The monitors, in egui points, for placing the overlay and the mini monitors
//! (the geometry is `cpuzeez_core::screens`). Where the OS gives no monitor
//! positions the screen is the one egui reports, at the origin, as before.

use super::cache::Cached;
use crate::Ctx;
use cpuzeez_core::screens::Screen;
use cpuzeez_platform::display::{ScreenRect, screens};
use eframe::egui;
use std::sync::Arc;
use std::time::Duration;

/// Every monitor, in points. `fallback` is the size egui reports for the
/// window's own monitor (used until the first read, or where the OS lists none).
///
/// ponytail: physical pixels are divided by this viewport's pixels-per-point,
/// so with monitors of different scale factors the other monitors' rectangles
/// are approximate; the window still lands on the right one.
pub fn points(ectx: &egui::Context, ctx: &Arc<Ctx>, fallback: egui::Vec2) -> Vec<Screen> {
    static RECTS: Cached<Vec<ScreenRect>> = Cached::new(Duration::from_secs(5));
    let ppp = ectx.pixels_per_point().max(0.5);
    let rects = RECTS.get(ctx, |_| screens()).map(|a| (*a).clone()).unwrap_or_default();
    if rects.is_empty() {
        return vec![Screen { x: 0.0, y: 0.0, w: fallback.x, h: fallback.y, primary: true }];
    }
    rects
        .iter()
        .map(|r| Screen {
            x: r.x as f32 / ppp,
            y: r.y as f32 / ppp,
            w: r.w as f32 / ppp,
            h: r.h as f32 / ppp,
            primary: r.primary,
        })
        .collect()
}

/// The centre of the game's window in points, when it is the foreground window.
pub fn game_centre(ectx: &egui::Context) -> Option<[f32; 2]> {
    let (pid, _) = crate::live_frames::target()?;
    let c = cpuzeez_platform::desktop::window_centre_of(pid)?;
    let ppp = ectx.pixels_per_point().max(0.5);
    Some([c[0] as f32 / ppp, c[1] as f32 / ppp])
}

/// "Monitor 2 (2560 x 1440)" for the pick list; `n` counts from 1.
pub fn label(n: usize, s: &Screen, ppp: f32) -> String {
    format!(
        "{} {n} ({} × {}){}",
        super::i18n::tr("Monitor"),
        (s.w * ppp).round(),
        (s.h * ppp).round(),
        if s.primary { " ★" } else { "" }
    )
}

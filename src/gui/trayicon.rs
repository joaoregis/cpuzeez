//! Tray icon pictures (S1-M01): number, bar, mini graph and ring, drawn at the
//! icon's real pixel size (16/20/24/32 px for 100–200 % scaling) with text
//! from the app's own font (egui's glyph atlas), coloured by the Widget
//! Engine's thresholds and always paired with a shape cue: a corner notch for
//! a warning, a frame for danger.

use cpuzeez_core::layout::Tone;
use eframe::egui;

pub use cpuzeez_core::layout::tray::Style;

const OK: [u8; 4] = [0x2e, 0x7d, 0x32, 255];
const WARN: [u8; 4] = [0xef, 0x6c, 0x00, 255];
const HOT: [u8; 4] = [0xc6, 0x28, 0x28, 255];
const NONE: [u8; 4] = [0x45, 0x5a, 0x64, 255];
const DARK: [u8; 4] = [0x20, 0x24, 0x28, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

fn tone_color(t: Option<Tone>) -> [u8; 4] {
    match t {
        None => NONE,
        Some(Tone::Danger) => HOT,
        Some(Tone::Warning) => WARN,
        Some(_) => OK,
    }
}

/// The tray's icon size here: the OS small-icon size (DPI-aware) on Windows,
/// 32 px elsewhere (the hosts scale it).
pub fn size() -> usize {
    cpuzeez_platform::display::small_icon_px().map_or(32, |p| p.min(64))
}

/// An alpha mask of `text` in the app font, as large as fits `max_w × max_h`.
pub struct Mask {
    pub w: usize,
    pub h: usize,
    pub a: Vec<u8>,
}

pub fn text_mask(ctx: &egui::Context, text: &str, max_w: usize, max_h: usize) -> Option<Mask> {
    // Masks repeat (the same few numbers): keep the last ones.
    type Cache = Vec<((String, usize, usize, u32), std::sync::Arc<Mask>)>;
    static CACHE: std::sync::Mutex<Cache> = std::sync::Mutex::new(Vec::new());
    let ppp = ctx.pixels_per_point();
    let key = (text.to_string(), max_w, max_h, ppp.to_bits());
    if let Some(m) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|c| c.0 == key) {
        return Some(Mask { w: m.1.w, h: m.1.h, a: m.1.a.clone() });
    }
    // Lay out every candidate size first, then read the atlas once.
    let sizes: Vec<usize> = (6..=max_h * 4 / 3 + 2).rev().collect();
    let galleys: Vec<_> = sizes
        .iter()
        .map(|&px| {
            let id = egui::FontId::new(px as f32 / ppp, egui::FontFamily::Proportional);
            ctx.fonts_mut(|f| f.layout_no_wrap(text.into(), id, egui::Color32::WHITE))
        })
        .collect();
    let img = ctx.fonts(|f| f.image());
    for galley in galleys {
        let (cw, ch) = (256i32, 128i32);
        let mut canvas = vec![0u8; (cw * ch) as usize];
        for row in &galley.rows {
            for g in &row.row.glyphs {
                let uv = g.uv_rect;
                if uv.is_nothing() {
                    continue;
                }
                let lt = row.pos + g.pos.to_vec2() + uv.offset;
                let (x0, y0) = ((lt.x * ppp).round() as i32 + 8, (lt.y * ppp).round() as i32 + 8);
                for y in uv.min[1]..uv.max[1] {
                    for x in uv.min[0]..uv.max[0] {
                        let a = img.pixels[y as usize * img.size[0] + x as usize].a();
                        let (tx, ty) = (x0 + (x - uv.min[0]) as i32, y0 + (y - uv.min[1]) as i32);
                        if (0..cw).contains(&tx) && (0..ch).contains(&ty) {
                            let c = &mut canvas[(ty * cw + tx) as usize];
                            *c = (*c).max(a);
                        }
                    }
                }
            }
        }
        // Tight box around the ink.
        let ink: Vec<(usize, usize)> =
            (0..canvas.len()).filter(|&i| canvas[i] > 24).map(|i| (i % cw as usize, i / cw as usize)).collect();
        let (x0, x1) = (ink.iter().map(|p| p.0).min()?, ink.iter().map(|p| p.0).max()?);
        let (y0, y1) = (ink.iter().map(|p| p.1).min()?, ink.iter().map(|p| p.1).max()?);
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        if w <= max_w && h <= max_h {
            let canvas = &canvas;
            let a: Vec<u8> = (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| canvas[y * cw as usize + x])).collect();
            let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
            if c.len() >= 64 {
                c.remove(0);
            }
            c.push((key, std::sync::Arc::new(Mask { w, h, a: a.clone() })));
            return Some(Mask { w, h, a });
        }
    }
    None
}

struct Canvas {
    n: usize,
    px: Vec<u8>,
}

impl Canvas {
    fn new(n: usize) -> Canvas {
        Canvas { n, px: vec![0; n * n * 4] }
    }

    fn set(&mut self, x: usize, y: usize, c: [u8; 4]) {
        if x < self.n && y < self.n {
            self.px[(y * self.n + x) * 4..][..4].copy_from_slice(&c);
        }
    }

    fn blend(&mut self, x: usize, y: usize, c: [u8; 4], a: u8) {
        if x >= self.n || y >= self.n || a == 0 {
            return;
        }
        let p = &mut self.px[(y * self.n + x) * 4..][..4];
        let k = a as u32;
        for i in 0..3 {
            p[i] = ((c[i] as u32 * k + p[i] as u32 * (255 - k)) / 255) as u8;
        }
        p[3] = p[3].max(a);
    }

    /// Rounded square.
    fn fill(&mut self, c: [u8; 4]) {
        let n = self.n;
        let r = (n / 8).max(2);
        for y in 0..n {
            for x in 0..n {
                let (dx, dy) = (x.min(n - 1 - x), y.min(n - 1 - y));
                if dx + dy >= r {
                    self.set(x, y, c);
                }
            }
        }
    }

    fn mask(&mut self, m: &Mask, c: [u8; 4]) {
        let (ox, oy) = ((self.n.saturating_sub(m.w)) / 2, (self.n.saturating_sub(m.h)) / 2);
        for y in 0..m.h {
            for x in 0..m.w {
                self.blend(ox + x, oy + y, c, m.a[y * m.w + x]);
            }
        }
    }

    /// Shape cue that doesn't rely on colour: warning = corner notch, danger = frame.
    fn cue(&mut self, t: Option<Tone>) {
        let n = self.n;
        match t {
            Some(Tone::Warning) => {
                let k = n / 4;
                for y in 0..k {
                    for x in n - k + y..n {
                        self.set(x, y, WHITE);
                    }
                }
            }
            Some(Tone::Danger) => {
                let k = (n / 16).max(1);
                for i in 0..n {
                    for j in 0..k {
                        self.set(i, j, WHITE);
                        self.set(i, n - 1 - j, WHITE);
                        self.set(j, i, WHITE);
                        self.set(n - 1 - j, i, WHITE);
                    }
                }
            }
            _ => {}
        }
    }
}

/// What one icon shows.
pub struct Picture<'a> {
    pub style: Style,
    /// "62" (no unit: the tooltip has it).
    pub text: Option<String>,
    /// 0..=1 for bar and ring.
    pub fill: f32,
    pub tone: Option<Tone>,
    /// Recent values (oldest first) for the mini graph.
    pub history: &'a [f64],
}

/// RGBA pixels, `n × n`.
pub fn draw(ctx: Option<&egui::Context>, p: &Picture, n: usize) -> Vec<u8> {
    let mut c = Canvas::new(n);
    let color = tone_color(p.tone);
    let text = |w: usize, h: usize| -> Option<Mask> {
        let t = p.text.as_deref()?;
        ctx.and_then(|ctx| text_mask(ctx, t, w, h))
    };
    match p.style {
        Style::Number => {
            c.fill(color);
            if let Some(m) = text(n - 2, n * 3 / 4) {
                c.mask(&m, WHITE);
            }
        }
        Style::Bar => {
            c.fill(DARK);
            let h = ((n - 4) as f32 * p.fill.clamp(0.0, 1.0)).round() as usize;
            for y in n - 2 - h..n - 2 {
                for x in 2..n - 2 {
                    c.set(x, y, color);
                }
            }
            if let Some(m) = text(n - 4, n / 2) {
                c.mask(&m, WHITE);
            }
        }
        Style::Graph => {
            c.fill(DARK);
            let v = &p.history[p.history.len().saturating_sub(n - 2)..];
            let (lo, hi) = v.iter().fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)));
            let span = (hi - lo).max(1e-9);
            for (i, &x) in v.iter().enumerate() {
                let top = n - 2 - (((x - lo) / span) * (n as f64 - 5.0)).round() as usize - 1;
                for y in top..n - 2 {
                    c.set(1 + i, y, if y == top { WHITE } else { color });
                }
            }
        }
        Style::Ring => {
            let (cx, r_out) = (n as f32 / 2.0, n as f32 / 2.0 - 0.5);
            let r_in = r_out - (n as f32 / 6.0).max(2.0);
            for y in 0..n {
                for x in 0..n {
                    let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cx);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d <= r_out && d >= r_in {
                        // Clockwise from 12 o'clock.
                        let ang = (dx.atan2(-dy) / std::f32::consts::TAU + 1.0) % 1.0;
                        c.set(x, y, if ang <= p.fill.clamp(0.0, 1.0) { color } else { DARK });
                    } else if d < r_in {
                        c.set(x, y, [0x10, 0x12, 0x14, 230]);
                    }
                }
            }
            let inner = (r_in * 1.4) as usize;
            if let Some(m) = text(inner, inner * 3 / 4) {
                c.mask(&m, WHITE);
            }
        }
    }
    c.cue(p.tone);
    c.px
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
        ctx
    }

    #[test]
    fn text_is_real_and_fits() {
        let ctx = ctx();
        for n in [16, 20, 24, 32] {
            let m = text_mask(&ctx, "62", n - 2, n * 3 / 4).expect("digits render");
            assert!(m.w <= n - 2 && m.h <= n * 3 / 4 && m.h >= n / 3, "{n} px: {}×{}", m.w, m.h);
            assert!(m.a.iter().any(|&a| a > 200), "solid ink");
        }
        assert!(text_mask(&ctx, "100", 14, 12).is_some(), "three digits fit a 16 px icon");
    }

    #[test]
    fn every_style_draws_with_a_shape_cue() {
        let ctx = ctx();
        let hist: Vec<f64> = (0..40).map(|i| i as f64).collect();
        for style in Style::ALL {
            for (tone, cue) in [(Some(Tone::Normal), false), (Some(Tone::Warning), true), (Some(Tone::Danger), true)] {
                let p = Picture { style, text: Some("71".into()), fill: 0.6, tone, history: &hist };
                let px = draw(Some(&ctx), &p, 24);
                assert_eq!(px.len(), 24 * 24 * 4);
                let top_right = &px[(23) * 4..][..4];
                assert_eq!(top_right == WHITE, cue, "{style:?} {tone:?}: the cue marks the corner");
            }
        }
        assert_eq!(
            draw(None, &Picture { style: Style::Number, text: None, fill: 0.0, tone: None, history: &[] }, 16).len(),
            16 * 16 * 4
        );
    }
}

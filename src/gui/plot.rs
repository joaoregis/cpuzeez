//! Generic time-series line chart drawn with egui's painter (series in,
//! painter out). One chart = one unit; the caller splits mixed units.

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, pos2, vec2};

const LINE_WIDTH: f32 = 1.6;
const RADIUS: f32 = 6.0;
const MARGIN_LEFT: f32 = 46.0;
const MARGIN_BOTTOM: f32 = 14.0;
const AXIS_FONT: f32 = 10.0;

pub struct Line<'a> {
    pub label: &'a str,
    pub color: Color32,
    /// (seconds, value), oldest first.
    pub points: &'a [(f64, f64)],
}

/// Draw `lines` over the last `span` seconds ending at `now`. `fmt` formats a
/// value with its unit for the hover tooltip.
pub fn chart(
    ui: &mut egui::Ui,
    lines: &[Line],
    now: f64,
    span: f64,
    height: f32,
    fmt: &dyn Fn(f64) -> String,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(rect);
    let p = super::theme::palette(ui);
    let (grid, cursor, text) = (p.border, p.text_muted, p.text_secondary);
    painter.rect_filled(rect, RADIUS, ui.visuals().extreme_bg_color);
    let area = Rect::from_min_max(
        pos2(rect.left() + MARGIN_LEFT, rect.top() + 4.0),
        pos2(rect.right() - 4.0, rect.bottom() - MARGIN_BOTTOM),
    );
    let font = FontId::proportional(AXIS_FONT);
    let t0 = now - span;

    let vis: Vec<&[(f64, f64)]> = lines.iter().map(|l| &l.points[l.points.partition_point(|p| p.0 < t0)..]).collect();
    let (mut lo, mut hi) =
        vis.iter().flat_map(|s| s.iter()).fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
    if lo > hi {
        painter.text(area.center(), Align2::CENTER_CENTER, "…", font, text);
        return resp;
    }
    if hi - lo < 1e-9 {
        lo -= 1.0;
        hi += 1.0;
    }
    let step = nice_step((hi - lo) / 4.0);
    let (lo, hi) = ((lo / step).floor() * step, (hi / step).ceil() * step);
    let x = |t: f64| area.left() + ((t - t0) / span) as f32 * area.width();
    let y = |val: f64| area.bottom() - ((val - lo) / (hi - lo)) as f32 * area.height();

    // Y grid + labels.
    let decimals = if step >= 1.0 { 0 } else { (-step.log10()).ceil() as usize };
    let mut val = lo;
    while val <= hi + step * 0.5 {
        let yy = y(val);
        painter.line_segment([pos2(area.left(), yy), pos2(area.right(), yy)], Stroke::new(1.0, grid));
        painter.text(
            pos2(area.left() - 4.0, yy),
            Align2::RIGHT_CENTER,
            format!("{val:.decimals$}"),
            font.clone(),
            text,
        );
        val += step;
    }
    // X grid + labels (seconds/minutes ago).
    let xs = [5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0, 3600.0];
    let xstep = xs.into_iter().find(|s| span / s <= 6.0).unwrap_or(3600.0);
    let mut ago = 0.0;
    while ago <= span + 0.1 {
        let xx = x(now - ago);
        painter.line_segment([pos2(xx, area.top()), pos2(xx, area.bottom())], Stroke::new(1.0, grid));
        painter.text(pos2(xx, area.bottom() + 1.0), Align2::CENTER_TOP, ago_text(ago), font.clone(), text);
        ago += xstep;
    }

    for (l, pts) in lines.iter().zip(&vis) {
        let screen: Vec<Pos2> = pts.iter().map(|p| pos2(x(p.0), y(p.1))).collect();
        painter.add(Shape::line(screen, Stroke::new(LINE_WIDTH, l.color)));
    }

    // Hover: cursor line, dots and a tooltip with every series' value there.
    if let Some(p) = resp.hover_pos().filter(|p| area.x_range().contains(p.x)) {
        let t = t0 + ((p.x - area.left()) / area.width()) as f64 * span;
        painter.line_segment([pos2(p.x, area.top()), pos2(p.x, area.bottom())], Stroke::new(1.0, cursor));
        let hits: Vec<(&Line, (f64, f64))> =
            lines.iter().zip(&vis).filter_map(|(l, pts)| nearest(pts, t).map(|pt| (l, pt))).collect();
        for (l, pt) in &hits {
            painter.circle_filled(pos2(x(pt.0), y(pt.1)), 3.0, l.color);
        }
        if !hits.is_empty() {
            resp.clone().on_hover_ui_at_pointer(|ui| {
                ui.label(egui::RichText::new(ago_text(now - t)).weak());
                for (l, pt) in hits {
                    ui.colored_label(l.color, format!("{}: {}", l.label, fmt(pt.1)));
                }
            });
        }
    }
    resp
}

fn nearest(pts: &[(f64, f64)], t: f64) -> Option<(f64, f64)> {
    let i = pts.partition_point(|p| p.0 < t);
    [i.checked_sub(1), Some(i)]
        .into_iter()
        .flatten()
        .filter_map(|i| pts.get(i))
        .min_by(|a, b| (a.0 - t).abs().total_cmp(&(b.0 - t).abs()))
        .copied()
}

/// 1, 2, 2.5 or 5 × 10^n just above `raw`.
fn nice_step(raw: f64) -> f64 {
    let mag = 10f64.powf(raw.max(1e-9).log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0].into_iter().map(|m| m * mag).find(|s| *s >= raw).unwrap_or(10.0 * mag)
}

fn ago_text(secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    if s == 0 {
        "0".into()
    } else if s < 60 {
        format!("-{s}s")
    } else if s.is_multiple_of(60) {
        format!("-{}m", s / 60)
    } else {
        format!("-{}:{:02}", s / 60, s % 60)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn steps_and_nearest() {
        assert_eq!(super::nice_step(0.7), 1.0);
        assert_eq!(super::nice_step(3.0), 5.0);
        assert_eq!(super::nice_step(22.0), 25.0);
        let pts = [(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)];
        assert_eq!(super::nearest(&pts, 1.4), Some((1.0, 2.0)));
        assert_eq!(super::nearest(&pts, 9.0), Some((2.0, 3.0)));
        assert_eq!(super::ago_text(90.0), "-1:30");
    }
}

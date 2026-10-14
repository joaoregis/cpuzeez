//! Reusable building blocks of the modern layout: sidebar items, page header,
//! cards, key/value rows, metric tiles with sparklines, status pills and
//! empty states. Colors come from `theme::palette`, never literals.

use super::theme::{self, Palette};
use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, Response, RichText, Sense, Stroke, StrokeKind, Ui, Vec2,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

fn tone_colors(p: &Palette, t: Tone) -> (Color32, Color32) {
    match t {
        Tone::Neutral => (p.sidebar, p.text_secondary),
        Tone::Accent => (p.accent_soft, p.accent),
        Tone::Success => (p.success_soft, p.success),
        Tone::Warning => (p.warning_soft, p.warning),
        Tone::Danger => (p.danger_soft, p.danger),
    }
}

/// Navigation entry: icon + label, full width, highlighted when selected.
pub fn sidebar_item(ui: &mut Ui, icon: &str, label: &str, selected: bool) -> Response {
    let p = theme::palette(ui);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 34.0), Sense::click());
    let fill = if selected {
        p.accent_soft
    } else if resp.hovered() {
        p.card_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(theme::RADIUS_CONTROL), fill);
    let color = if selected { p.accent } else { p.text_secondary };
    let text_color = if selected { p.text } else { p.text_secondary };
    let font = egui::FontId::proportional(16.0);
    ui.painter().text(rect.left_center() + egui::vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, icon, font, color);
    let fid = if selected { egui::FontId::new(14.0, theme::semibold()) } else { egui::FontId::proportional(14.0) };
    ui.painter().text(rect.left_center() + egui::vec2(38.0, 0.0), egui::Align2::LEFT_CENTER, label, fid, text_color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Page title, subtitle and optional right-aligned actions.
pub fn page_header(ui: &mut Ui, title: &str, subtitle: &str, actions: impl FnOnce(&mut Ui)) {
    let p = theme::palette(ui);
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).text_style(theme::title()).color(p.text));
            if !subtitle.is_empty() {
                ui.label(RichText::new(subtitle).text_style(theme::subtitle()).color(p.text_secondary));
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), actions);
    });
    ui.add_space(theme::GAP);
}

/// Raised card with an icon + title header.
pub fn card<R>(ui: &mut Ui, icon: &str, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    let p = theme::palette(ui);
    Frame::default()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(theme::RADIUS_CARD))
        .inner_margin(Margin::same(theme::PAD_CARD))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                ui.horizontal(|ui| {
                    if !icon.is_empty() {
                        ui.label(RichText::new(icon).size(17.0).color(p.accent));
                    }
                    ui.label(RichText::new(title).text_style(egui::TextStyle::Heading).color(p.text));
                });
                ui.add_space(6.0);
            }
            add(ui)
        })
        .inner
}

/// Label on the left, value on the right; wraps long values. Empty values show "–".
pub fn kv(ui: &mut Ui, label: &str, value: &str) {
    let p = theme::palette(ui);
    let w = ui.available_width();
    ui.horizontal_top(|ui| {
        let lw = (w * 0.42).clamp(90.0, 220.0);
        ui.allocate_ui_with_layout(egui::vec2(lw, 20.0), Layout::left_to_right(Align::Center), |ui| {
            ui.set_width(lw);
            ui.add(egui::Label::new(RichText::new(label).color(p.text_secondary)).truncate());
        });
        let v = if value.is_empty() { "–" } else { value };
        ui.with_layout(Layout::right_to_left(Align::Min).with_main_wrap(true), |ui| {
            ui.add(egui::Label::new(RichText::new(v).color(p.text)).wrap());
        });
    });
}

/// `kv` rows separated by hairlines.
pub fn kv_list(ui: &mut Ui, rows: &[(&str, String)]) {
    let p = theme::palette(ui);
    for (i, (k, v)) in rows.iter().enumerate() {
        if i > 0 {
            let y = ui.cursor().top() - 2.0;
            let x = ui.max_rect().x_range();
            ui.painter().hline(x, y, Stroke::new(1.0, p.border.gamma_multiply(0.6)));
        }
        kv(ui, k, v);
    }
}

/// Small rounded status label.
pub fn pill(ui: &mut Ui, text: &str, tone: Tone) -> Response {
    let p = theme::palette(ui);
    let (bg, fg) = tone_colors(p, tone);
    Frame::default()
        .fill(bg)
        .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| ui.label(RichText::new(text).size(12.5).color(fg)))
        .response
}

/// Summary tile: label, big value, optional sparkline.
pub fn metric(ui: &mut Ui, label: &str, value: &str, spark: &[f64], color: Color32, tone: Tone) {
    let p = theme::palette(ui);
    let (bg, _) = tone_colors(p, tone);
    let fill = if tone == Tone::Neutral { p.card } else { bg };
    Frame::default()
        .fill(fill)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(theme::RADIUS_CARD))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).text_style(theme::subtitle()).color(p.text_secondary));
            ui.label(RichText::new(value).text_style(theme::metric()).color(p.text));
            if spark.len() >= 2 {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), Sense::hover());
                sparkline(ui.painter(), rect, spark, color);
            }
        });
}

/// Polyline scaled to `rect`.
pub fn sparkline(painter: &egui::Painter, rect: egui::Rect, values: &[f64], color: Color32) {
    let (lo, hi) = values.iter().fold((f64::MAX, f64::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    let span = if (hi - lo).abs() < 1e-9 { 1.0 } else { hi - lo };
    let n = values.len().max(2) - 1;
    let pts: Vec<egui::Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = rect.left() + rect.width() * i as f32 / n as f32;
            let y = rect.bottom() - rect.height() * ((v - lo) / span) as f32;
            egui::pos2(x, y)
        })
        .collect();
    painter.add(egui::Shape::line(pts, Stroke::new(1.6, color)));
}

/// Lay `n` equal-width columns that fall back to fewer on narrow windows.
pub fn columns(ui: &mut Ui, max_cols: usize, min_width: f32, items: usize, mut add: impl FnMut(&mut Ui, usize)) {
    let avail = ui.available_width();
    let cols = ((avail + theme::GAP) / (min_width + theme::GAP)).floor().clamp(1.0, max_cols as f32) as usize;
    let w = (avail - theme::GAP * (cols as f32 - 1.0)) / cols as f32;
    for row in (0..items).collect::<Vec<_>>().chunks(cols) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = theme::GAP;
            for &i in row {
                ui.allocate_ui_with_layout(Vec2::new(w, 0.0), Layout::top_down(Align::Min), |ui| {
                    ui.set_width(w);
                    add(ui, i);
                });
            }
        });
        ui.add_space(theme::GAP);
    }
}

/// Centered icon, title and one line of text.
pub fn empty_state(ui: &mut Ui, icon: &str, title: &str, body: &str) {
    let p = theme::palette(ui);
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        ui.label(RichText::new(icon).size(36.0).color(p.text_muted));
        ui.label(RichText::new(title).text_style(egui::TextStyle::Heading).color(p.text));
        ui.label(RichText::new(body).color(p.text_secondary));
        ui.add_space(24.0);
    });
}

/// Accent-filled primary button (one per view).
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let p = theme::palette(ui);
    ui.add(
        egui::Button::new(RichText::new(text).color(p.on_accent))
            .fill(p.accent)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL)),
    )
}

/// Thin progress bar in the accent color.
pub fn progress(ui: &mut Ui, frac: f32, text: &str) -> Response {
    let p = theme::palette(ui);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), Sense::hover());
    let r = CornerRadius::same(6);
    ui.painter().rect_filled(rect, r, p.sidebar);
    let mut fill = rect;
    fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
    ui.painter().rect_filled(fill, r, p.accent);
    ui.painter().rect_stroke(rect, r, Stroke::new(1.0, p.border), StrokeKind::Inside);
    if !text.is_empty() {
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(12.5), p.text);
    }
    resp
}

/// Pill-style segmented control (one choice among a few views).
pub fn segmented<T: PartialEq + Copy>(ui: &mut Ui, current: &mut T, items: &[(T, &str, &str)]) {
    let p = theme::palette(ui);
    Frame::default()
        .fill(p.sidebar)
        .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL + 2))
        .inner_margin(Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (v, icon, label) in items {
                    let on = *current == *v;
                    let text =
                        RichText::new(format!("{icon}  {label}")).color(if on { p.text } else { p.text_secondary });
                    let b = egui::Button::new(text)
                        .fill(if on { p.card } else { Color32::TRANSPARENT })
                        .stroke(if on { Stroke::new(1.0, p.border) } else { Stroke::NONE })
                        .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL));
                    if ui.add(b).clicked() {
                        *current = *v;
                    }
                }
            });
        });
    ui.add_space(theme::GAP);
}

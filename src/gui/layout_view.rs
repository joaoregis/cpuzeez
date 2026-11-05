//! Draws a `cpuzeez_core::layout::Layout` from live metrics: the renderer the
//! mini monitor, the tray, the Overview tiles and the ZeeZ Live overlay share.

use super::i18n::tr;
use super::sensors;
use super::theme::{self, icon};
use super::widgets;
use crate::telemetry::Series;
use cpuzeez_core::layout::{Direction, Kind, Layout, Tone, Widget};
use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, RichText, Ui};

pub const PT_BR: &[(&str, &str)] = &[
    ("Clock", "Clock"),
    ("Power", "Consumo"),
    ("Load", "Carga"),
    ("Temperature", "Temperatura"),
    ("Memory", "Memória"),
];
pub const ES: &[(&str, &str)] = &[
    ("Clock", "Reloj"),
    ("Power", "Consumo"),
    ("Load", "Carga"),
    ("Temperature", "Temperatura"),
    ("Memory", "Memoria"),
];

/// Aliases a layout may use instead of exact ids, resolved to the best sensor
/// on this machine. (alias, what it means), for the layout editor.
pub const ALIASES: &[(&str, &str)] = &[
    ("@cpu_temp", "CPU temperature (package, Tctl/Tdie)"),
    ("@cpu_clock", "CPU clock (average of the cores)"),
    ("@cpu_load", "CPU load (all cores)"),
    ("@cpu_power", "CPU package power"),
    ("@gpu_temp", "GPU temperature"),
    ("@gpu_load", "GPU load"),
    ("@chipset_temp", "Chipset temperature"),
    ("@mem_load", "Memory load"),
    ("@cpu_vid", "CPU voltage (VID)"),
    ("@gpu_power", "GPU power"),
    ("@battery_charge", "Battery charge"),
    ("@battery_rate", "Battery charge rate"),
    ("@disk_active", "First disk's active time"),
    ("@net_rx", "First adapter's download"),
];

/// The series a widget's metric (exact id or alias) points to.
pub fn resolve<'a>(metric: &str, series: &'a [Series]) -> Option<&'a Series> {
    let id = |want: &str| series.iter().find(|s| s.id == want);
    match metric {
        "@cpu_temp" => sensors::cpu_temp(series),
        "@cpu_clock" => sensors::avg_clock(series),
        "@cpu_load" => id("cpu/load/total"),
        "@cpu_power" => sensors::package_power(series),
        "@gpu_temp" => sensors::gpu_temp(series),
        "@gpu_load" => id("gpu0/load/core"),
        "@chipset_temp" => sensors::chipset_temp(series),
        "@mem_load" => id("mem/load/used"),
        "@cpu_vid" => id("cpu/volt/vid"),
        "@gpu_power" => id("gpu0/power/board"),
        "@battery_charge" => id("battery0/level/charge"),
        "@battery_rate" => id("battery0/power/rate"),
        "@disk_active" => id("disk0/load/active"),
        "@net_rx" => id("net0/rate/rx"),
        exact => id(exact),
    }
}

fn tone_color(p: &theme::Palette, t: Tone) -> Color32 {
    match t {
        Tone::Normal => p.text,
        Tone::Good => p.success,
        Tone::Warning => p.warning,
        Tone::Danger => p.danger,
    }
}

/// Colour is never the only cue (colour-blind users): warn/danger add an
/// icon (Phosphor: the UI fonts don't have ▲ on every OS).
fn tone_mark(t: Tone) -> String {
    match t {
        Tone::Warning => format!("{} ", theme::icon::WARNING),
        Tone::Danger => format!("{} ", theme::icon::WARNING_OCTAGON),
        _ => String::new(),
    }
}

/// The value in the chosen units; a widget's own decimals win.
pub fn value_text(w: &Widget, s: &Series) -> String {
    let u = super::units::get();
    let u = cpuzeez_core::units::Units { decimals: w.decimals.or(u.decimals), ..u };
    u.format(s.current, s.unit)
}

/// The metric prefixes `widgets` read (aliases included), for a telemetry lease.
pub fn prefixes(widgets: &[Widget]) -> Vec<String> {
    let mut v: Vec<String> = widgets
        .iter()
        .filter_map(|w| w.metric.as_deref())
        .map(|m| match m {
            m if m.starts_with("@cpu") => "cpu/".to_string(),
            m if m.starts_with("@gpu") => "gpu".to_string(),
            "@chipset_temp" => "chipset/".to_string(),
            "@mem_load" => "mem/".to_string(),
            m if m.starts_with("@battery") => "battery".to_string(),
            "@disk_active" => "disk".to_string(),
            "@net_rx" => "net".to_string(),
            exact => exact.to_string(),
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Draw `layout`. `series` has current values; `points(id)` returns history,
/// asked only for the sparkline widgets.
pub fn show(
    ui: &mut Ui,
    layout: &Layout,
    series: &[Series],
    points: &dyn Fn(&str) -> Vec<(f64, f64)>,
) -> egui::Response {
    ui.scope(|ui| {
        let st = &layout.style;
        match st.theme {
            cpuzeez_core::layout::Theme::Dark => ui.style_mut().visuals = egui::Visuals::dark(),
            cpuzeez_core::layout::Theme::Light => ui.style_mut().visuals = egui::Visuals::light(),
            cpuzeez_core::layout::Theme::Follow => {}
        }
        let p = theme::palette(ui);
        let bg = p.card.gamma_multiply(st.opacity.clamp(0.0, 1.0));
        // ponytail: "outline" is a dark plate behind each reading, not a glyph
        // stroke; it reads over any background. A real stroke needs painted text.
        let one = |ui: &mut Ui, w: &Widget| {
            if st.outline {
                Frame::default()
                    .fill(Color32::from_black_alpha(150))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(4, 1))
                    .show(ui, |ui| widget(ui, w, series, points, st.scale));
            } else {
                widget(ui, w, series, points, st.scale);
            }
        };
        Frame::default()
            .fill(bg)
            .corner_radius(CornerRadius::same(st.corner))
            .inner_margin(Margin::same(st.padding as i8))
            .show(ui, |ui| match layout.direction {
                Direction::Vertical => {
                    ui.vertical(|ui| layout.widgets.iter().for_each(|w| one(ui, w)));
                }
                Direction::Horizontal => {
                    ui.horizontal(|ui| {
                        for (i, w) in layout.widgets.iter().enumerate() {
                            if i > 0 && w.kind != Kind::Separator {
                                ui.label(RichText::new("·").color(p.text_muted));
                            }
                            one(ui, w);
                        }
                    });
                }
                Direction::Grid => {
                    let cols = layout.columns.max(1) as usize;
                    egui::Grid::new(ui.id().with("layout-grid")).spacing([12.0, 4.0]).show(ui, |ui| {
                        for (i, w) in layout.widgets.iter().enumerate() {
                            one(ui, w);
                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });
                }
            })
            .response
    })
    .inner
}

fn widget(ui: &mut Ui, w: &Widget, series: &[Series], points: &dyn Fn(&str) -> Vec<(f64, f64)>, scale: f32) {
    let p = theme::palette(ui);
    let size = ui.text_style_height(&egui::TextStyle::Body) * scale;
    let s = w.metric.as_deref().and_then(|m| resolve(m, series));
    let label =
        w.label.as_deref().map(tr).map(str::to_string).or_else(|| s.map(|s| s.label.clone())).unwrap_or_default();
    let tone = s.map(|s| w.tone(s.current)).unwrap_or_default();
    let color = tone_color(p, tone);
    let value = || match s {
        Some(s) => {
            RichText::new(format!("{}{}", tone_mark(tone), value_text(w, s))).size(size).monospace().color(color)
        }
        None => RichText::new("–").size(size).color(p.text_muted),
    };
    let label_text = || RichText::new(&label).size(size * 0.9).color(p.text_secondary);
    match w.kind {
        Kind::Value => {
            ui.label(value());
        }
        Kind::LabelValue => {
            ui.horizontal(|ui| {
                ui.label(label_text());
                ui.label(value());
            });
        }
        Kind::Bar => {
            ui.horizontal(|ui| {
                ui.label(label_text());
                ui.label(value());
            });
            if let Some(s) = s {
                let c = if tone == Tone::Normal { p.accent } else { color };
                bar(ui, w.fill(s.current), c, size * 0.45);
            }
        }
        Kind::Sparkline => {
            ui.horizontal(|ui| {
                ui.label(label_text());
                ui.label(value());
            });
            if let Some(s) = s {
                let pts: Vec<f64> = points(&s.id).iter().rev().take(60).rev().map(|p| p.1).collect();
                let (rect, _) = ui
                    .allocate_exact_size(egui::vec2(ui.available_width().min(160.0), size * 1.4), egui::Sense::hover());
                if pts.len() >= 2 {
                    let c = if tone == Tone::Normal { p.accent } else { color };
                    widgets::sparkline(ui.painter(), rect, &pts, c);
                }
            }
        }
        Kind::Header => {
            ui.label(RichText::new(label).size(size).strong().color(p.text));
        }
        Kind::Separator => {
            ui.separator();
        }
        Kind::Clock => {
            ui.label(
                RichText::new(format!("{} {}", icon::CLOCK, sensors::local_time()))
                    .size(size)
                    .monospace()
                    .color(p.text),
            );
        }
    }
}

fn bar(ui: &mut Ui, frac: f32, color: Color32, h: f32) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width().clamp(60.0, 160.0), h), egui::Sense::hover());
    let r = CornerRadius::same(3);
    ui.painter().rect_filled(rect, r, theme::palette(ui).border);
    let mut fill = rect;
    fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
    ui.painter().rect_filled(fill, r, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::Unit;

    fn s(id: &str, group: &str, label: &str, unit: Unit, v: f64) -> Series {
        Series {
            id: id.into(),
            group: group.into(),
            label: label.into(),
            unit,
            points: vec![],
            current: v,
            min: v,
            max: v,
            avg: v,
        }
    }

    #[test]
    fn aliases_resolve_to_this_machines_sensors() {
        let series = vec![
            s("cpu/temp/ccd0", "CPU", "CCD #0", Unit::Celsius, 70.0),
            s("cpu/temp/package", "CPU", "Package", Unit::Celsius, 80.0),
            s("gpu0/temp/core", "GPU · RTX", "Temperature", Unit::Celsius, 50.0),
            s("mem/load/used", "Memory", "Memory load", Unit::Percent, 40.0),
        ];
        assert_eq!(resolve("@cpu_temp", &series).map(|x| x.id.as_str()), Some("cpu/temp/package"));
        assert_eq!(resolve("@gpu_temp", &series).map(|x| x.current), Some(50.0));
        assert_eq!(resolve("cpu/temp/ccd0", &series).map(|x| x.current), Some(70.0), "exact ids work too");
        assert!(resolve("@chipset_temp", &series).is_none(), "missing sensors read as –");
        for (a, _) in ALIASES {
            let _ = resolve(a, &series); // every documented alias is handled
        }
    }

    #[test]
    fn every_preset_alias_is_known() {
        let known: Vec<&str> = ALIASES.iter().map(|a| a.0).collect();
        for l in cpuzeez_core::layout::presets::all(95.0) {
            for m in l.metrics().into_iter().filter(|m| m.starts_with('@')) {
                assert!(known.contains(&m), "{} uses unknown alias {m}", l.name);
            }
        }
    }
}

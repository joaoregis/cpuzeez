//! A table on the left and a panel on the right, with a handle between them
//! (M2-05). The handle can be dragged, and when focused (Tab) it moves with
//! the arrow keys; a double-click puts the default width back. The panel is
//! at least 280 points and at most 60 % of the space; the width is saved per
//! panel in the settings (`panel_widths`), which the app publishes here each
//! frame and saves when a width changes.

use super::theme;
use eframe::egui::{self, Ui};
use std::collections::BTreeMap;

pub const MIN: f32 = 280.0;
/// The table's least useful width next to the panel; below MIN + this, the
/// panel is a drawer over the table (M2-07).
pub const TABLE_MIN: f32 = 360.0;
const MAX_FRAC: f32 = 0.6;
const HANDLE: f32 = 8.0;
const STEP: f32 = 16.0;

fn widths_id() -> egui::Id {
    egui::Id::new("split-widths")
}

fn changed_id() -> egui::Id {
    egui::Id::new("split-changed")
}

/// The saved widths, published by the app each frame (not while a change
/// waits to be saved: that one is newer).
pub fn publish(ctx: &egui::Context, w: &BTreeMap<String, f32>) {
    ctx.data_mut(|d| {
        if d.get_temp::<bool>(changed_id()).is_none() {
            d.insert_temp(widths_id(), w.clone());
        }
    });
}

/// The widths to save, when one changed and the drag is over (one save per
/// drag, not one per frame).
pub fn take_changes(ctx: &egui::Context) -> Option<BTreeMap<String, f32>> {
    if ctx.input(|i| i.pointer.any_down()) {
        return None;
    }
    ctx.data_mut(|d| d.remove_temp::<bool>(changed_id()))
        .and_then(|_| ctx.data(|d| d.get_temp::<BTreeMap<String, f32>>(widths_id())))
}

/// The panel width that fits `avail`: between MIN and 60 %. Below MIN / 0.6
/// there is no room for both; the caller shows the panel some other way.
pub fn clamp(w: f32, avail: f32) -> f32 {
    w.min(avail * MAX_FRAC).max(MIN)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    /// The table.
    Left,
    /// The panel.
    Right,
}

/// Draw the two sides next to each other: `body` is called once for each
/// (one closure, so both sides may borrow the same state). `key` names the
/// panel in the settings and `default` is its width before the user moves it.
pub fn show(ui: &mut Ui, key: &str, default: f32, mut body: impl FnMut(&mut Ui, Side)) {
    let ctx = ui.ctx().clone();
    let avail = ui.available_width();
    let saved = ctx.data(|d| d.get_temp::<BTreeMap<String, f32>>(widths_id())).and_then(|m| m.get(key).copied());
    let mut w = clamp(saved.unwrap_or(default), avail);
    let height = ui.available_height();
    if !side_by_side(avail) {
        // A drawer may be wider than a side panel: it only covers the table.
        return drawer(ui, key, w.max(TABLE_MIN).min(avail), height, &mut body);
    }
    // Each side in its own fixed rectangle: a side wider than its share is
    // clipped there instead of pushing the other one sideways.
    let top = ui.cursor().min;
    let side = |ui: &mut Ui, x0: f32, x1: f32, s: Side, body: &mut dyn FnMut(&mut Ui, Side)| {
        let r = egui::Rect::from_min_max(egui::pos2(x0, top.y), egui::pos2(x1, top.y + height));
        ui.scope_builder(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
            ui.set_clip_rect(r.intersect(ui.clip_rect()));
            ui.set_max_width(r.width());
            body(ui, s);
        })
        .response
        .rect
        .height()
    };
    let h_left = side(ui, top.x, top.x + avail - w - HANDLE, Side::Left, &mut body);
    let rect =
        egui::Rect::from_min_size(egui::pos2(top.x + avail - w - HANDLE, top.y), egui::vec2(HANDLE, h_left.max(40.0)));
    let resp = ui.interact(rect, ui.id().with(("split", key)), egui::Sense::click_and_drag());
    resp.widget_info(|| egui::WidgetInfo::slider(true, f64::from(w.round()), crate::gui::i18n::tr("Panel width")));
    let mut next = w;
    if resp.dragged() {
        next -= resp.drag_delta().x;
    }
    if resp.has_focus() {
        ui.input(|i| {
            if i.key_pressed(egui::Key::ArrowLeft) {
                next += STEP;
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                next -= STEP;
            }
        });
    }
    if resp.double_clicked() {
        next = default;
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    next = clamp(next, avail);
    if next != w {
        w = next;
        ctx.data_mut(|d| {
            d.get_temp_mut_or_default::<BTreeMap<String, f32>>(widths_id()).insert(key.to_string(), w);
            d.insert_temp(changed_id(), true);
        });
    }
    let h_right = side(ui, top.x + avail - w, top.x + avail, Side::Right, &mut body);
    let h = h_left.max(h_right).max(40.0);
    let p = theme::palette(ui);
    let active = resp.hovered() || resp.dragged() || resp.has_focus();
    let x = rect.center().x;
    ui.painter().line_segment(
        [egui::pos2(x, top.y + 4.0), egui::pos2(x, top.y + h - 4.0)],
        egui::Stroke::new(if active { 2.0 } else { 1.0 }, if active { p.accent } else { p.border }),
    );
    ui.allocate_rect(egui::Rect::from_min_size(top, egui::vec2(avail, h)), egui::Sense::hover());
}

/// A narrow window: the table takes the whole width and the panel opens
/// over its right side, on a layer above the page (so it scrolls and is
/// clipped with the page, unlike a floating area).
fn drawer(ui: &mut Ui, key: &str, w: f32, height: f32, body: &mut dyn FnMut(&mut Ui, Side)) {
    let top = ui.cursor().min;
    let right = top.x + ui.available_width();
    let clip = ui.clip_rect();
    let left_h = ui.scope(|ui| body(ui, Side::Left)).response.rect.height();
    let h = left_h.max(height.min(clip.bottom() - top.y)).max(40.0);
    let r = egui::Rect::from_min_size(egui::pos2(right - w, top.y), egui::vec2(w, h));
    let p = theme::palette(ui);
    let layer = egui::LayerId::new(egui::Order::Middle, ui.id().with(("drawer", key)));
    ui.scope_builder(
        egui::UiBuilder::new().layer_id(layer).max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| {
            ui.set_clip_rect(r.intersect(clip));
            ui.painter().rect_filled(r, 0.0, p.bg);
            ui.painter().add(ui.style().visuals.popup_shadow.as_shape(r, 0.0));
            ui.painter().rect_filled(r, 0.0, p.bg);
            ui.set_max_width(w);
            body(ui, Side::Right);
        },
    );
}

/// True when a panel of the saved or `default` width fits next to its table
/// in `avail` (otherwise it opens as a drawer).
pub fn side_by_side(avail: f32) -> bool {
    avail >= MIN + TABLE_MIN + HANDLE
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    fn harness() -> Harness<'static, BTreeMap<String, f32>> {
        Harness::builder().with_size(egui::vec2(1000.0, 300.0)).with_step_dt(1.0 / 60.0).build_ui_state(
            |ui, saved: &mut BTreeMap<String, f32>| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                publish(ui.ctx(), saved);
                show(ui, "test", 345.0, |ui, side| drop(ui.label(format!("{side:?}"))));
                if let Some(w) = take_changes(ui.ctx()) {
                    *saved = w;
                }
            },
            BTreeMap::new(),
        )
    }

    fn width(h: &Harness<'_, BTreeMap<String, f32>>) -> f32 {
        h.state().get("test").copied().unwrap_or(345.0)
    }

    #[test]
    fn the_handle_drags_and_moves_with_the_keys() {
        let mut h = harness();
        h.run();
        h.run();
        let handle = h.get_by_label("Panel width").rect().center();
        h.drag_at(handle);
        h.run();
        h.hover_at(handle - egui::vec2(100.0, 0.0));
        h.run();
        h.drop_at(handle - egui::vec2(100.0, 0.0));
        h.run();
        assert!((width(&h) - 445.0).abs() < 2.0, "dragging left widens the panel: {}", width(&h));

        h.get_by_label("Panel width").focus();
        h.run();
        h.key_press(egui::Key::ArrowRight);
        h.run();
        assert!((width(&h) - 429.0).abs() < 2.0, "→ narrows it by a step: {}", width(&h));

        // Never past 60 % of the space, nor under the minimum.
        for _ in 0..40 {
            h.key_press(egui::Key::ArrowLeft);
            h.run();
        }
        assert!(width(&h) <= 0.6 * 1000.0 + 1.0, "{}", width(&h));
        for _ in 0..40 {
            h.key_press(egui::Key::ArrowRight);
            h.run();
        }
        assert_eq!(width(&h), MIN);

        // Two pointer clicks in a row (the harness's clock does not move between them).
        let at = h.get_by_label("Panel width").rect().center();
        h.hover_at(at);
        h.step();
        for pressed in [true, false, true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
            h.step();
        }
        h.run();
        assert_eq!(width(&h), 345.0, "a double-click resets it");
    }

    #[test]
    fn the_width_survives_a_settings_round_trip() {
        let mut s = crate::settings::Settings::default();
        s.panel_widths.insert("services".into(), 410.0);
        let back: crate::settings::Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.panel_widths.get("services"), Some(&410.0));
    }

    #[test]
    fn the_width_is_clamped_to_the_window() {
        assert_eq!(clamp(900.0, 1000.0), 600.0);
        assert_eq!(clamp(100.0, 1000.0), MIN);
        assert_eq!(clamp(400.0, 1000.0), 400.0);
    }

    #[test]
    fn a_narrow_window_opens_the_panel_over_the_table() {
        let mut h = Harness::builder().with_size(egui::vec2(600.0, 300.0)).build_ui(|ui| {
            show(ui, "test", 345.0, |ui, side| drop(ui.label(format!("{side:?}"))));
        });
        h.run();
        let (left, right) = (h.get_by_label("Left").rect(), h.get_by_label("Right").rect());
        assert!(h.query_by_label("Panel width").is_none(), "no handle in a drawer");
        assert!(right.left() < 600.0 - 340.0 && left.left() < 20.0, "the panel covers the table's right side");
        assert!(right.right() <= 600.0 + 0.5, "and stays in the window");
    }
}

//! Mini monitors (S1-M02): Widget Engine layouts in small always-on-top
//! windows. Several at once, each remembered (layout, position, behaviour);
//! they snap to the screen edges, can be locked (click-through: part of the
//! desktop), step aside while a full-screen app runs (Windows), expand on
//! hover and grow with Ctrl+scroll. Right-click for the menu; the editor is
//! in Settings and shares layouts with the overlay.

use super::i18n::tr;
use super::sensors::Monitor;
use cpuzeez_core::layout::mini::{self, Mini};
use cpuzeez_core::layout::presets;
use eframe::egui;
use std::time::{Duration, Instant};

pub const PT_BR: &[(&str, &str)] = &[
    ("Mini monitors", "Mini monitores"),
    (
        "Small always-on-top windows with the readings you choose. Drag to move (they snap to the screen edges), Ctrl+scroll to resize, right-click for options.",
        "Janelas pequenas sempre no topo com as leituras que você escolher. Arraste para mover (elas grudam nas bordas da tela), Ctrl+rolagem para redimensionar, clique direito para opções.",
    ),
    ("Shown", "Visível"),
    ("Edit", "Editar"),
    ("Done", "Concluir"),
    ("Add mini monitor", "Adicionar mini monitor"),
    (
        "Locked monitors ignore the mouse: unlock them here or from the tray.",
        "Monitores travados ignoram o mouse: destrave aqui ou pela bandeja.",
    ),
    ("Hiding during full-screen apps works on Windows.", "Ocultar durante apps em tela cheia funciona no Windows."),
    ("Mini monitor", "Mini monitor"),
    ("Layout", "Layout"),
    ("Edit…", "Editar…"),
    ("Lock (click-through)", "Travar (clique atravessa)"),
    ("Hide while a full-screen app runs", "Ocultar com um app em tela cheia"),
    ("Expand on hover", "Expandir ao passar o mouse"),
    ("New mini monitor", "Novo mini monitor"),
    ("Show CpuZeeZ", "Mostrar CpuZeeZ"),
    ("Close", "Fechar"),
    ("Mini", "Mini"),
    ("Compact", "Compacto"),
    ("Detailed", "Detalhado"),
    ("Grid", "Grade"),
    ("Big number", "Número grande"),
];

pub const ES: &[(&str, &str)] = &[
    ("Mini monitors", "Minimonitores"),
    (
        "Small always-on-top windows with the readings you choose. Drag to move (they snap to the screen edges), Ctrl+scroll to resize, right-click for options.",
        "Ventanas pequeñas siempre visibles con las lecturas que elija. Arrastre para mover (se ajustan a los bordes), Ctrl+rueda para cambiar el tamaño, clic derecho para opciones.",
    ),
    ("Shown", "Visible"),
    ("Edit", "Editar"),
    ("Done", "Listo"),
    ("Add mini monitor", "Añadir minimonitor"),
    (
        "Locked monitors ignore the mouse: unlock them here or from the tray.",
        "Los monitores bloqueados ignoran el ratón: desbloquéelos aquí o desde la bandeja.",
    ),
    (
        "Hiding during full-screen apps works on Windows.",
        "Ocultar durante apps a pantalla completa funciona en Windows.",
    ),
    ("Mini monitor", "Minimonitor"),
    ("Layout", "Diseño"),
    ("Edit…", "Editar…"),
    ("Lock (click-through)", "Bloquear (clic atraviesa)"),
    ("Hide while a full-screen app runs", "Ocultar con una app a pantalla completa"),
    ("Expand on hover", "Ampliar al pasar el ratón"),
    ("New mini monitor", "Nuevo minimonitor"),
    ("Show CpuZeeZ", "Mostrar CpuZeeZ"),
    ("Close", "Cerrar"),
    ("Mini", "Mini"),
    ("Compact", "Compacto"),
    ("Detailed", "Detallado"),
    ("Grid", "Cuadrícula"),
    ("Big number", "Número grande"),
];

/// Snap distance to the screen edges (points).
const SNAP: f32 = 20.0;

/// Frame-to-frame state that isn't saved.
#[derive(Default)]
pub struct Runtime {
    fullscreen: Option<(bool, Instant)>,
}

enum Change {
    Pos(usize, [f32; 2]),
    Scale(usize, f32),
    Layout(usize, cpuzeez_core::layout::Layout),
    Toggle(usize, fn(&mut Mini) -> &mut bool),
    New(usize),
    Close(usize),
    Edit(usize),
    ShowApp,
}

/// Draws every open mini monitor and applies what the user did to them.
pub fn show_all(mon: &mut Monitor, ectx: &egui::Context) {
    let minis = mon.settings.minis.clone();
    if !minis.iter().any(|m| m.open) {
        return;
    }
    let full = if minis.iter().any(|m| m.open && m.hide_fullscreen) {
        let rt = &mut mon.mini_rt;
        if rt.fullscreen.is_none_or(|(_, t)| t.elapsed() > Duration::from_secs(1)) {
            rt.fullscreen = Some((cpuzeez_platform::display::fullscreen_app(), Instant::now()));
        }
        rt.fullscreen.is_some_and(|f| f.0)
    } else {
        false
    };
    let tjmax = mon.tjmax();
    let tel = mon.tel();
    let ws: Vec<_> = minis.iter().filter(|m| m.open).flat_map(|m| m.layout.widgets.clone()).collect();
    let lease = super::layout_view::prefixes(&ws);
    tel.want("mini", &lease.iter().map(String::as_str).collect::<Vec<_>>());
    let series = tel.series_with(|_| false);
    let mut changes = Vec::new();
    for (i, m) in minis.iter().enumerate() {
        if !m.open || (m.hide_fullscreen && full) {
            continue;
        }
        let mut builder = egui::ViewportBuilder::default()
            .with_title(format!("{} · {}", crate::DISPLAY_NAME, tr("Mini monitor")))
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_taskbar(false)
            .with_mouse_passthrough(m.locked);
        if let Some(p) = m.pos {
            builder = builder.with_position(p);
        }
        let vid = egui::ViewportId::from_hash_of(("mini-monitor", i));
        ectx.show_viewport_immediate(vid, builder, |ui, _| {
            if ui.ctx().input(|i| i.viewport().close_requested()) {
                changes.push(Change::Close(i));
            }
            let frame = egui::Frame::NONE.inner_margin(egui::Margin::same(0));
            egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                let bg = ui.interact(ui.max_rect(), ui.id().with("drag"), egui::Sense::click_and_drag());
                if bg.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                bg.context_menu(|ui| menu(ui, i, m, tjmax, &mut changes));
                let hover = m.expand_hover && ui.rect_contains_pointer(ui.max_rect());
                let layout = if hover { mini::expanded(&m.layout) } else { m.layout.clone() };
                let drawn = super::layout_view::show(ui, &layout, &series, &|id| tel.points(id)).rect;
                // Fit the window to what was drawn (fonts and DPI vary by OS).
                let want = drawn.size() + egui::vec2(2.0, 2.0);
                if (ui.ctx().content_rect().size() - want).length() > 2.0 {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(want));
                }
                // Ctrl+scroll (egui reads it as zoom): text scale.
                let zoom = ui.input(|i| i.zoom_delta());
                if zoom != 1.0 && ui.rect_contains_pointer(ui.max_rect()) {
                    changes.push(Change::Scale(i, if zoom > 1.0 { 0.1 } else { -0.1 }));
                }
                // Remember where it went, snapped to the screen edges.
                let (outer, screen, down) =
                    ui.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size, i.pointer.any_down()));
                if let (Some(r), Some(scr), false) = (outer, screen, down) {
                    let pos = [r.min.x, r.min.y];
                    // ponytail: snaps against the monitor size from the origin, so
                    // only on the primary display; per-monitor origins need winit.
                    let snapped = mini::snap(pos, [r.width(), r.height()], [scr.x, scr.y], SNAP);
                    if snapped != pos {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::OuterPosition(snapped.into()));
                    }
                    if m.pos.is_none_or(|p| (p[0] - snapped[0]).abs() + (p[1] - snapped[1]).abs() > 1.0) {
                        changes.push(Change::Pos(i, snapped));
                    }
                }
            });
        });
    }
    apply(mon, ectx, changes);
}

fn menu(ui: &mut egui::Ui, i: usize, m: &Mini, tjmax: f64, changes: &mut Vec<Change>) {
    ui.menu_button(tr("Layout"), |ui| {
        for l in presets::all(tjmax) {
            if ui.button(tr(&l.name)).clicked() {
                changes.push(Change::Layout(i, l));
                ui.close();
            }
        }
    });
    if ui.button(tr("Edit…")).clicked() {
        changes.push(Change::Edit(i));
        ui.close();
    }
    let mut toggle = |ui: &mut egui::Ui, on: bool, label: &str, f: fn(&mut Mini) -> &mut bool| {
        if ui.selectable_label(on, tr(label)).clicked() {
            changes.push(Change::Toggle(i, f));
            ui.close();
        }
    };
    toggle(ui, m.locked, "Lock (click-through)", |m| &mut m.locked);
    toggle(ui, m.hide_fullscreen, "Hide while a full-screen app runs", |m| &mut m.hide_fullscreen);
    toggle(ui, m.expand_hover, "Expand on hover", |m| &mut m.expand_hover);
    ui.separator();
    if ui.button(tr("New mini monitor")).clicked() {
        changes.push(Change::New(i));
        ui.close();
    }
    if ui.button(tr("Show CpuZeeZ")).clicked() {
        changes.push(Change::ShowApp);
        ui.close();
    }
    if ui.button(tr("Close")).clicked() {
        changes.push(Change::Close(i));
        ui.close();
    }
}

fn apply(mon: &mut Monitor, ectx: &egui::Context, changes: Vec<Change>) {
    for c in changes {
        match c {
            Change::Pos(i, p) => mon.set_setting(move |s| {
                if let Some(m) = s.minis.get_mut(i) {
                    m.pos = Some(p);
                }
            }),
            Change::Scale(i, d) => mon.set_setting(move |s| {
                if let Some(m) = s.minis.get_mut(i) {
                    m.layout.style.scale = (m.layout.style.scale + d).clamp(0.6, 4.0);
                }
            }),
            Change::Layout(i, l) => mon.set_setting(move |s| {
                if let Some(m) = s.minis.get_mut(i) {
                    m.layout = l.clone();
                }
            }),
            Change::Toggle(i, f) => mon.set_setting(move |s| {
                if let Some(m) = s.minis.get_mut(i) {
                    let v = f(m);
                    *v = !*v;
                }
            }),
            Change::New(i) => mon.set_setting(move |s| {
                let mut m = s.minis.get(i).cloned().unwrap_or_default();
                m.pos = m.pos.map(|p| [p[0] + 24.0, p[1] + 24.0]);
                m.locked = false;
                s.minis.push(m);
            }),
            Change::Close(i) => {
                mon.set_setting(move |s| {
                    if s.minis.len() > 1 {
                        s.minis.remove(i);
                    } else if let Some(m) = s.minis.get_mut(i) {
                        m.open = false;
                    }
                });
                mon.show_mini = mon.settings.minis.iter().any(|m| m.open);
            }
            Change::Edit(i) => {
                mon.edit_mini = Some(i);
                show_app(ectx);
            }
            Change::ShowApp => show_app(ectx),
        }
    }
}

fn show_app(ectx: &egui::Context) {
    ectx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Visible(true));
    ectx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Minimized(false));
    ectx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Focus);
}

/// Settings › Mini monitors: one row per monitor (shown, layout, behaviour,
/// edit, remove), the inline editor and "Add".
pub fn settings_card(
    ui: &mut egui::Ui,
    minis: &mut Vec<Mini>,
    edit: &mut Option<usize>,
    import: &mut String,
    tjmax: f64,
) {
    use super::theme::{self, icon};
    let p = theme::palette(ui);
    ui.label(egui::RichText::new(tr("Small always-on-top windows with the readings you choose. Drag to move (they snap to the screen edges), Ctrl+scroll to resize, right-click for options.")).color(p.text_secondary));
    let mut remove = None;
    for (i, m) in minis.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut m.open, tr("Shown"));
            ui.label(egui::RichText::new(tr(&m.layout.name)).strong());
            ui.checkbox(&mut m.locked, tr("Lock (click-through)"));
            ui.checkbox(&mut m.hide_fullscreen, tr("Hide while a full-screen app runs"));
            ui.checkbox(&mut m.expand_hover, tr("Expand on hover"));
            let editing = *edit == Some(i);
            if ui.button(tr(if editing { "Done" } else { "Edit" })).clicked() {
                *edit = if editing { None } else { Some(i) };
            }
            if ui.small_button(icon::TRASH).clicked() {
                remove = Some(i);
            }
        });
        if *edit == Some(i) {
            egui::Frame::group(ui.style())
                .show(ui, |ui| super::layout_editor::editor(ui, ("mini", i), &mut m.layout, import, tjmax));
        }
    }
    if let Some(i) = remove {
        minis.remove(i);
        *edit = None;
    }
    if ui.button(format!("{}  {}", icon::PLUS, tr("Add mini monitor"))).clicked() {
        minis.push(Mini { layout: presets::mini(tjmax), ..Default::default() });
    }
    let mut notes = vec![tr("Locked monitors ignore the mouse: unlock them here or from the tray.")];
    if !cfg!(windows) {
        notes.push(tr("Hiding during full-screen apps works on Windows."));
    }
    for n in notes {
        ui.label(egui::RichText::new(n).small().color(p.text_muted));
    }
}

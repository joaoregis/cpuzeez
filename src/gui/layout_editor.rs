//! The Widget Engine layout editor (S1-M02): presets, direction, style and
//! the widget list (metric by alias, kind, label, order). The mini monitors
//! use it now; the ZeeZ Live overlay will share it.

use super::i18n::tr;
use super::layout_view::ALIASES;
use super::theme::icon;
use super::widgets::Control;
use cpuzeez_core::layout::{Direction, Kind, Layout, Theme, Widget, presets};
use eframe::egui;

pub const PT_BR: &[(&str, &str)] = &[
    ("Preset", "Predefinição"),
    ("Arrangement", "Disposição"),
    ("Vertical", "Vertical"),
    ("Horizontal", "Horizontal"),
    ("Grid", "Grade"),
    ("Columns", "Colunas"),
    ("Text size", "Tamanho do texto"),
    ("Background", "Fundo"),
    ("Corners", "Cantos"),
    ("Padding", "Margem"),
    ("Outline", "Contorno"),
    ("Theme", "Tema"),
    ("Follow the app", "Seguir o app"),
    ("Dark", "Escuro"),
    ("Light", "Claro"),
    ("Value", "Valor"),
    ("Label + value", "Rótulo + valor"),
    ("Bar", "Barra"),
    ("Graph", "Gráfico"),
    ("Header", "Título"),
    ("Separator", "Separador"),
    ("Clock", "Relógio"),
    ("Throttling", "Redução de clock"),
    ("CPU and GPU", "CPU e GPU"),
    ("CPU only", "Só a CPU"),
    ("GPU only", "Só a GPU"),
    ("Label", "Rótulo"),
    ("Add reading", "Adicionar leitura"),
    ("Shown as", "Exibir como"),
    ("Device", "Dispositivo"),
    ("Reading", "Leitura"),
    ("Copy as JSON", "Copiar como JSON"),
    ("Import JSON", "Importar JSON"),
    ("Paste a layout's JSON here", "Cole aqui o JSON de um layout"),
];

pub const ES: &[(&str, &str)] = &[
    ("Preset", "Predefinido"),
    ("Arrangement", "Disposición"),
    ("Vertical", "Vertical"),
    ("Horizontal", "Horizontal"),
    ("Grid", "Cuadrícula"),
    ("Columns", "Columnas"),
    ("Text size", "Tamaño del texto"),
    ("Background", "Fondo"),
    ("Corners", "Esquinas"),
    ("Padding", "Margen"),
    ("Outline", "Contorno"),
    ("Theme", "Tema"),
    ("Follow the app", "Seguir la app"),
    ("Dark", "Oscuro"),
    ("Light", "Claro"),
    ("Value", "Valor"),
    ("Label + value", "Etiqueta + valor"),
    ("Bar", "Barra"),
    ("Graph", "Gráfico"),
    ("Header", "Título"),
    ("Separator", "Separador"),
    ("Clock", "Reloj"),
    ("Throttling", "Reducción de reloj"),
    ("CPU and GPU", "CPU y GPU"),
    ("CPU only", "Solo la CPU"),
    ("GPU only", "Solo la GPU"),
    ("Label", "Etiqueta"),
    ("Add reading", "Añadir lectura"),
    ("Shown as", "Mostrar como"),
    ("Device", "Dispositivo"),
    ("Reading", "Lectura"),
    ("Copy as JSON", "Copiar como JSON"),
    ("Import JSON", "Importar JSON"),
    ("Paste a layout's JSON here", "Pegue aquí el JSON de un diseño"),
];

/// What a Throttle widget watches: (metric, label).
const THROTTLE_DEVICES: [(&str, &str); 3] =
    [("", "CPU and GPU"), ("@throttle_cpu", "CPU only"), ("@throttle_gpu", "GPU only")];

const KINDS: [(Kind, &str); 8] = [
    (Kind::Value, "Value"),
    (Kind::LabelValue, "Label + value"),
    (Kind::Bar, "Bar"),
    (Kind::Sparkline, "Graph"),
    (Kind::Header, "Header"),
    (Kind::Separator, "Separator"),
    (Kind::Clock, "Clock"),
    (Kind::Throttle, "Throttling"),
];

/// Edits `l` in place; `import` holds the JSON box's text between frames.
pub fn editor(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + Copy + std::fmt::Debug,
    l: &mut Layout,
    import: &mut String,
    tjmax: f64,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Preset"));
        for pre in presets::all(tjmax) {
            if ui.button(tr(&pre.name)).clicked() {
                *l = pre;
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Arrangement"));
        for (d, name) in
            [(Direction::Vertical, "Vertical"), (Direction::Horizontal, "Horizontal"), (Direction::Grid, "Grid")]
        {
            ui.selectable_value(&mut l.direction, d, tr(name));
        }
        if l.direction == Direction::Grid {
            let c = ui.label(tr("Columns")).id;
            ui.add(egui::DragValue::new(&mut l.columns).range(1..=4)).name_by(c);
        }
    });
    let st = &mut l.style;
    ui.horizontal_wrapped(|ui| {
        let l = ui.label(tr("Text size")).id;
        ui.add(egui::Slider::new(&mut st.scale, 0.6..=4.0).fixed_decimals(1)).name_by(l);
        let l = ui.label(tr("Background")).id;
        ui.add(egui::Slider::new(&mut st.opacity, 0.0..=1.0).fixed_decimals(2)).name_by(l);
    });
    ui.horizontal_wrapped(|ui| {
        let l = ui.label(tr("Corners")).id;
        ui.add(egui::DragValue::new(&mut st.corner).range(0..=24)).name_by(l);
        let l = ui.label(tr("Padding")).id;
        ui.add(egui::DragValue::new(&mut st.padding).range(0..=24)).name_by(l);
        ui.checkbox(&mut st.outline, tr("Outline"));
        ui.label(tr("Theme"));
        for (t, name) in [(Theme::Follow, "Follow the app"), (Theme::Dark, "Dark"), (Theme::Light, "Light")] {
            ui.selectable_value(&mut st.theme, t, tr(name));
        }
    });
    ui.add_space(4.0);
    let (mut up, mut remove) = (None, None);
    let n = l.widgets.len();
    for (i, w) in l.widgets.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            let kind = KINDS.iter().find(|k| k.0 == w.kind).map_or("", |k| k.1);
            egui::ComboBox::from_id_salt((salt, "kind", i))
                .selected_text(tr(kind))
                .width(110.0)
                .show_ui(ui, |ui| {
                    for (k, name) in KINDS {
                        ui.selectable_value(&mut w.kind, k, tr(name));
                    }
                })
                .response
                .accessible_name(tr("Shown as"));
            if w.kind == Kind::Throttle {
                // Which device to watch; the metric is only a filter here.
                let m = w.metric.clone().unwrap_or_default();
                let shown = THROTTLE_DEVICES.iter().find(|a| a.0 == m).map_or(THROTTLE_DEVICES[0].1, |a| a.1);
                egui::ComboBox::from_id_salt((salt, "throttle", i))
                    .selected_text(tr(shown))
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for (alias, what) in THROTTLE_DEVICES {
                            ui.selectable_value(
                                &mut w.metric,
                                Some(alias.to_string()).filter(|a| !a.is_empty()),
                                tr(what),
                            );
                        }
                    })
                    .response
                    .accessible_name(tr("Device"));
            } else if !matches!(w.kind, Kind::Header | Kind::Separator | Kind::Clock) {
                let m = w.metric.clone().unwrap_or_default();
                let shown = ALIASES.iter().find(|a| a.0 == m).map_or(m.clone(), |a| tr(a.1).to_string());
                egui::ComboBox::from_id_salt((salt, "metric", i))
                    .selected_text(shown)
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for (alias, what) in ALIASES {
                            ui.selectable_value(&mut w.metric, Some(alias.to_string()), tr(what));
                        }
                    })
                    .response
                    .accessible_name(tr("Reading"));
            }
            if w.kind != Kind::Separator {
                let mut label = w.label.clone().unwrap_or_default();
                if ui.add(egui::TextEdit::singleline(&mut label).hint_text(tr("Label")).desired_width(90.0)).changed() {
                    w.label = (!label.is_empty()).then_some(label);
                }
            }
            if ui.add_enabled(i > 0, egui::Button::new(icon::ARROW_UP).small()).clicked() {
                up = Some(i);
            }
            if ui.add_enabled(i + 1 < n, egui::Button::new(icon::ARROW_DOWN).small()).clicked() {
                up = Some(i + 1);
            }
            if ui.small_button(icon::TRASH).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = up {
        l.widgets.swap(i - 1, i);
    }
    if let Some(i) = remove {
        l.widgets.remove(i);
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button(format!("{}  {}", icon::PLUS, tr("Add reading"))).clicked() {
            l.widgets.push(Widget::metric(Kind::LabelValue, "@cpu_load", "CPU %"));
        }
        if ui.button(tr("Copy as JSON")).clicked() {
            ui.ctx().copy_text(l.to_json());
        }
    });
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(import).hint_text(tr("Paste a layout's JSON here")).desired_width(260.0));
        if ui.add_enabled(!import.is_empty(), egui::Button::new(tr("Import JSON"))).clicked() {
            match Layout::from_json(import) {
                Ok(new) => {
                    *l = new;
                    import.clear();
                }
                // The box shows why it was refused.
                Err(e) => *import = e,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use eframe::egui;
    use egui_kittest::{Harness, kittest::Queryable};

    /// S1-M02: the editor adds a reading, switches to a grid and applies a preset.
    #[test]
    fn layout_editor_edits() {
        use cpuzeez_core::layout::{Direction, Layout, presets};
        let mut h = Harness::builder().with_size(egui::vec2(1000.0, 600.0)).build_ui_state(
            |ui, st: &mut (Layout, String)| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                let (l, import) = st;
                super::editor(ui, "t", l, import, 95.0);
            },
            (presets::mini(95.0), String::new()),
        );
        h.run();
        let before = h.state().0.widgets.len();
        h.get_by_label_contains("Add reading").click();
        h.run();
        assert_eq!(h.state().0.widgets.len(), before + 1);
        // The second "Grid" is the arrangement (the first is the preset).
        h.get_all_by_label("Grid").nth(1).expect("arrangement button").click();
        h.run();
        assert_eq!(h.state().0.direction, Direction::Grid);
        assert_eq!(h.state().0.widgets.len(), before + 1, "the arrangement keeps the readings");
        h.get_by_label("Big number").click();
        h.run();
        assert_eq!(h.state().0, presets::big(95.0));
    }
}

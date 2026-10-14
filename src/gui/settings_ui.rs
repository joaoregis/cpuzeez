//! Settings page: language, theme, tray, sampling, graphs, warning
//! temperature; changes apply live and are saved to settings.json at once.

use super::i18n::{self, tr};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::settings::{Settings, Theme};
use eframe::egui::{self, RichText};

pub const PT_BR: &[(&str, &str)] = &[
    ("Settings", "Configurações"),
    (
        "Changes apply immediately and are saved automatically.",
        "As alterações valem na hora e são salvas automaticamente.",
    ),
    ("Appearance", "Aparência"),
    ("Monitoring", "Monitoramento"),
    ("Updates", "Atualizações"),
    ("Data", "Dados"),
    ("Language", "Idioma"),
    ("System default", "Padrão do sistema"),
    ("Theme", "Tema"),
    ("System", "Sistema"),
    ("Light", "Claro"),
    ("Dark", "Escuro"),
    ("Show tray / menu-bar monitor", "Mostrar monitor na bandeja / barra de menus"),
    ("Close to tray", "Fechar para a bandeja"),
    ("Sensor sampling period", "Intervalo de leitura dos sensores"),
    ("Graph history", "Histórico dos gráficos"),
    ("Warning temperature", "Temperatura de alerta"),
    (
        "Used by the stress monitor when the CPU's Tjmax is unknown.",
        "Usada pelo monitor de estresse quando o Tjmax da CPU é desconhecido.",
    ),
    ("Check for updates at startup", "Procurar atualizações ao iniciar"),
    ("On", "Ligado"),
    ("Off", "Desligado"),
    (
        "Set CheckUpdates=0 (or 1) in cpuzeez.ini next to the executable to change this.",
        "Defina CheckUpdates=0 (ou 1) no cpuzeez.ini, ao lado do executável, para mudar isso.",
    ),
    ("Open data folder", "Abrir pasta de dados"),
    ("Settings, benchmark history and logs are kept in:", "Configurações, histórico de benchmarks e logs ficam em:"),
    ("ms", "ms"),
    ("s", "s"),
];

pub const ES: &[(&str, &str)] = &[
    ("Settings", "Configuración"),
    (
        "Changes apply immediately and are saved automatically.",
        "Los cambios se aplican al instante y se guardan automáticamente.",
    ),
    ("Appearance", "Apariencia"),
    ("Monitoring", "Monitorización"),
    ("Updates", "Actualizaciones"),
    ("Data", "Datos"),
    ("Language", "Idioma"),
    ("System default", "Predeterminado del sistema"),
    ("Theme", "Tema"),
    ("System", "Sistema"),
    ("Light", "Claro"),
    ("Dark", "Oscuro"),
    ("Show tray / menu-bar monitor", "Mostrar monitor en la bandeja / barra de menús"),
    ("Close to tray", "Cerrar a la bandeja"),
    ("Sensor sampling period", "Intervalo de muestreo de sensores"),
    ("Graph history", "Historial de los gráficos"),
    ("Warning temperature", "Temperatura de aviso"),
    (
        "Used by the stress monitor when the CPU's Tjmax is unknown.",
        "La usa el monitor de estrés cuando se desconoce el Tjmax de la CPU.",
    ),
    ("Check for updates at startup", "Buscar actualizaciones al iniciar"),
    ("On", "Activado"),
    ("Off", "Desactivado"),
    (
        "Set CheckUpdates=0 (or 1) in cpuzeez.ini next to the executable to change this.",
        "Ponga CheckUpdates=0 (o 1) en cpuzeez.ini, junto al ejecutable, para cambiarlo.",
    ),
    ("Open data folder", "Abrir carpeta de datos"),
    (
        "Settings, benchmark history and logs are kept in:",
        "La configuración, el historial de pruebas y los registros se guardan en:",
    ),
    ("ms", "ms"),
    ("s", "s"),
];

pub fn theme_pref(t: Theme) -> egui::ThemePreference {
    match t {
        Theme::System => egui::ThemePreference::System,
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    }
}

/// Language from the settings, else the OS's.
pub fn lang_of(s: &Settings) -> i18n::Lang {
    s.language.as_deref().map_or_else(i18n::os_lang, i18n::from_code)
}

/// A label + control row inside a card.
fn row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        let p = theme::palette(ui);
        let w = 220.0_f32.min(ui.available_width() * 0.5);
        ui.allocate_ui_with_layout(egui::vec2(w, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_width(w);
            ui.label(RichText::new(label).color(p.text_secondary));
        });
        add(ui);
    });
}

/// The Settings page. `persist` = save changes (off while taking screenshots).
pub fn page(ui: &mut egui::Ui, s: &mut Settings, check_updates: bool, persist: bool) {
    let before = serde_json::to_string(s).unwrap_or_default();
    widgets::page_header(ui, tr("Settings"), tr("Changes apply immediately and are saved automatically."), |_| {});

    widgets::card(ui, icon::PALETTE, tr("Appearance"), |ui| {
        row(ui, tr("Language"), |ui| {
            let name = |code: Option<&str>| match code {
                None => tr("System default").to_string(),
                Some(c) => i18n::LANGS.iter().find(|l| l.1 == c).map_or(c.to_string(), |l| l.2.to_string()),
            };
            egui::ComboBox::from_id_salt("lang").selected_text(name(s.language.as_deref())).show_ui(ui, |ui| {
                ui.selectable_value(&mut s.language, None, name(None));
                for (_, code, label) in i18n::LANGS {
                    ui.selectable_value(&mut s.language, Some(code.to_string()), label);
                }
            });
        });
        row(ui, tr("Theme"), |ui| {
            for (t, label) in [(Theme::System, "System"), (Theme::Light, "Light"), (Theme::Dark, "Dark")] {
                ui.selectable_value(&mut s.theme, t, tr(label));
            }
        });
    });
    ui.add_space(theme::GAP);

    widgets::card(ui, icon::GAUGE, tr("Monitoring"), |ui| {
        row(ui, tr("Show tray / menu-bar monitor"), |ui| {
            ui.checkbox(&mut s.tray, "");
        });
        row(ui, tr("Close to tray"), |ui| {
            ui.add_enabled(s.tray, egui::Checkbox::without_text(&mut s.close_to_tray));
        });
        row(ui, tr("Sensor sampling period"), |ui| {
            ui.add(
                egui::DragValue::new(&mut s.sample_ms).range(100..=10_000).speed(10).suffix(format!(" {}", tr("ms"))),
            );
        });
        row(ui, tr("Graph history"), |ui| {
            ui.add(
                egui::DragValue::new(&mut s.graph_seconds).range(30..=86_400).speed(5).suffix(format!(" {}", tr("s"))),
            );
        });
        row(ui, tr("Warning temperature"), |ui| {
            ui.add(egui::DragValue::new(&mut s.warn_temp_c).range(40.0..=125.0).speed(0.5).suffix(" °C"))
                .on_hover_text(tr("Used by the stress monitor when the CPU's Tjmax is unknown."));
        });
    });
    ui.add_space(theme::GAP);

    widgets::card(ui, icon::ARROW_CIRCLE_UP, tr("Updates"), |ui| {
        row(ui, tr("Check for updates at startup"), |ui| {
            let (tone, text) = if check_updates { (Tone::Success, "On") } else { (Tone::Neutral, "Off") };
            widgets::pill(ui, tr(text), tone);
        });
        let p = theme::palette(ui);
        ui.label(
            RichText::new(tr("Set CheckUpdates=0 (or 1) in cpuzeez.ini next to the executable to change this."))
                .small()
                .color(p.text_muted),
        );
    });
    ui.add_space(theme::GAP);

    if let Some(dir) = crate::settings::dir() {
        widgets::card(ui, icon::FOLDER_OPEN, tr("Data"), |ui| {
            ui.label(tr("Settings, benchmark history and logs are kept in:"));
            ui.label(RichText::new(dir.display().to_string()).monospace().small());
            if ui.button(format!("{} {}", icon::FOLDER_OPEN, tr("Open data folder"))).clicked() {
                crate::update::open_path(&dir);
            }
        });
    }

    if !s.tray {
        s.close_to_tray = false;
    }
    if serde_json::to_string(s).unwrap_or_default() != before {
        i18n::set(lang_of(s));
        ui.ctx().set_theme(theme_pref(s.theme));
        if persist {
            s.save();
        }
    }
}

//! Settings page: language, theme, tray, sampling, graphs, warning
//! temperature; changes apply live and are saved to settings.json at once.

use super::i18n::{self, tr};
use super::theme::{self, icon};
use super::widgets;
use crate::report::Section;
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
    (
        "One anonymous request to GitHub's releases page at startup; nothing about your computer is sent.",
        "Uma consulta anônima à página de versões do GitHub ao iniciar; nada sobre o seu computador é enviado.",
    ),
    ("Turned off in cpuzeez.ini (CheckUpdates=0).", "Desativado no cpuzeez.ini (CheckUpdates=0)."),
    ("Privacy & data", "Privacidade e dados"),
    (
        "These settings were saved by a newer CpuZeeZ, so changes here aren't saved (that would lose its newer options).",
        "Estas configurações foram salvas por um CpuZeeZ mais novo, então mudanças aqui não são salvas (isso perderia as opções mais novas).",
    ),
    (
        "Everything CpuZeeZ keeps is on this computer, in this folder. Nothing is uploaded.",
        "Tudo o que o CpuZeeZ guarda está neste computador, nesta pasta. Nada é enviado.",
    ),
    ("Open data folder", "Abrir pasta de dados"),
    ("Sensor logs", "Logs de sensores"),
    ("Benchmark history", "Histórico de benchmarks"),
    ("Action log", "Registro de ações"),
    ("App log", "Log do app"),
    ("Diagnostics bundles", "Pacotes de diagnóstico"),
    ("Create diagnostics bundle", "Criar pacote de diagnóstico"),
    (
        "A folder with the app log and this computer's report, with serial numbers, MAC and IP addresses removed. It stays here; attach it to an issue only if you want to.",
        "Uma pasta com o log do app e o relatório deste computador, sem números de série, endereços MAC e IP. Ela fica aqui; anexe a um issue só se quiser.",
    ),
    ("{} files · {}", "{} arquivos · {}"),
    ("Delete", "Apagar"),
    ("Confirm", "Confirmar"),
    ("Delete sensor logs older than", "Apagar logs de sensores com mais de"),
    ("days (0 = keep)", "dias (0 = manter)"),
    ("Delete everything", "Apagar tudo"),
    ("Recent actions", "Ações recentes"),
    ("No actions yet.", "Nenhuma ação ainda."),
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
    (
        "One anonymous request to GitHub's releases page at startup; nothing about your computer is sent.",
        "Una consulta anónima a la página de versiones de GitHub al iniciar; no se envía nada sobre tu equipo.",
    ),
    ("Turned off in cpuzeez.ini (CheckUpdates=0).", "Desactivado en cpuzeez.ini (CheckUpdates=0)."),
    ("Privacy & data", "Privacidad y datos"),
    (
        "These settings were saved by a newer CpuZeeZ, so changes here aren't saved (that would lose its newer options).",
        "Estos ajustes los guardó un CpuZeeZ más nuevo, así que los cambios aquí no se guardan (se perderían sus opciones nuevas).",
    ),
    (
        "Everything CpuZeeZ keeps is on this computer, in this folder. Nothing is uploaded.",
        "Todo lo que CpuZeeZ guarda está en este equipo, en esta carpeta. No se sube nada.",
    ),
    ("Open data folder", "Abrir carpeta de datos"),
    ("Sensor logs", "Registros de sensores"),
    ("Benchmark history", "Historial de pruebas"),
    ("Action log", "Registro de acciones"),
    ("App log", "Registro de la app"),
    ("Diagnostics bundles", "Paquetes de diagnóstico"),
    ("Create diagnostics bundle", "Crear paquete de diagnóstico"),
    (
        "A folder with the app log and this computer's report, with serial numbers, MAC and IP addresses removed. It stays here; attach it to an issue only if you want to.",
        "Una carpeta con el registro de la app y el informe de este equipo, sin números de serie ni direcciones MAC e IP. Se queda aquí; adjúntala a un issue solo si quieres.",
    ),
    ("{} files · {}", "{} archivos · {}"),
    ("Delete", "Borrar"),
    ("Confirm", "Confirmar"),
    ("Delete sensor logs older than", "Borrar registros de sensores con más de"),
    ("days (0 = keep)", "días (0 = conservar)"),
    ("Delete everything", "Borrar todo"),
    ("Recent actions", "Acciones recientes"),
    ("No actions yet.", "Aún no hay acciones."),
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
/// Privacy & data: what is kept, its size, deleting it, retention, the action log.
fn privacy(ui: &mut egui::Ui, s: &mut Settings, dir: &std::path::Path, report: Option<&[Section]>) {
    use crate::data::{self, Category};
    let p = theme::palette(ui);
    ui.label(
        RichText::new(tr("Everything CpuZeeZ keeps is on this computer, in this folder. Nothing is uploaded."))
            .color(p.text_secondary),
    );
    ui.horizontal(|ui| {
        ui.label(RichText::new(dir.display().to_string()).monospace().small());
        if ui.small_button(format!("{} {}", icon::FOLDER_OPEN, tr("Open data folder"))).clicked() {
            crate::update::open_path(dir);
        }
    });
    ui.add_space(4.0);
    // A delete needs a second click; which button is armed lives in egui's memory.
    let armed_id = egui::Id::new("privacy-delete-armed");
    let armed: Option<usize> = ui.data(|d| d.get_temp(armed_id)).flatten();
    let mut arm = armed;
    for (i, u) in data::usage().iter().enumerate() {
        row(ui, tr(u.cat.label()), |ui| {
            ui.label(
                RichText::new(i18n::trf("{} files · {}", &[&u.files, &super::bench_tab::bytes_text(u.bytes as f64)]))
                    .color(p.text_muted),
            );
            if u.files > 0 {
                let label = if armed == Some(i) { tr("Confirm") } else { tr("Delete") };
                if ui.small_button(label).clicked() {
                    if armed == Some(i) {
                        let _ = data::delete(u.cat);
                        if u.cat == Category::Settings {
                            *s = Settings::default(); // the in-memory copy too, so nothing is written back
                        }
                        arm = None;
                    } else {
                        arm = Some(i);
                    }
                }
            }
        });
    }
    row(ui, tr("Delete sensor logs older than"), |ui| {
        ui.add(
            egui::DragValue::new(&mut s.log_retention_days)
                .range(0..=3650)
                .suffix(format!(" {}", tr("days (0 = keep)"))),
        );
    });
    let all = usize::MAX;
    let label = if armed == Some(all) { tr("Confirm") } else { tr("Delete everything") };
    if ui.add(egui::Button::new(RichText::new(label).color(p.danger))).clicked() {
        if armed == Some(all) {
            for c in Category::ALL {
                let _ = data::delete(c);
            }
            *s = Settings::default();
            arm = None;
        } else {
            arm = Some(all);
        }
    }
    ui.data_mut(|d| d.insert_temp(armed_id, arm));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.add_enabled(report.is_some(), egui::Button::new(tr("Create diagnostics bundle"))).clicked()
            && let Some(r) = report
        {
            match crate::applog::diagnostics_bundle(r) {
                Ok(d) => {
                    crate::update::open_path(&d);
                }
                Err(e) => crate::applog::warn(&format!("diagnostics bundle: {e}")),
            }
        }
    });
    ui.label(RichText::new(tr("A folder with the app log and this computer's report, with serial numbers, MAC and IP addresses removed. It stays here; attach it to an issue only if you want to.")).small().color(p.text_muted));
    ui.add_space(4.0);
    egui::CollapsingHeader::new(tr("Recent actions")).id_salt("audit").show(ui, |ui| {
        let log = crate::executor::audit_log();
        if log.is_empty() {
            ui.label(RichText::new(tr("No actions yet.")).color(p.text_muted));
        }
        for e in log.iter().rev().take(20) {
            let mark = if e.ok { "✓" } else { "✗" };
            let how = if e.elevated { " (admin)" } else { "" };
            ui.label(
                RichText::new(format!(
                    "{mark} {} {}{how}{}",
                    e.time,
                    e.description,
                    if e.detail.is_empty() { String::new() } else { format!(" · {}", e.detail) }
                ))
                .small()
                .monospace(),
            );
        }
    });
}

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

/// The Settings page. `updates_allowed` is false when cpuzeez.ini turns update
/// checks off; `persist` = save changes (off while taking screenshots).
pub fn page(ui: &mut egui::Ui, s: &mut Settings, updates_allowed: bool, persist: bool, report: Option<&[Section]>) {
    let before = serde_json::to_string(s).unwrap_or_default();
    if s.read_only {
        widgets::pill(
            ui,
            tr(
                "These settings were saved by a newer CpuZeeZ, so changes here aren't saved (that would lose its newer options).",
            ),
            widgets::Tone::Warning,
        );
        ui.add_space(theme::GAP);
    }
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
            let mut on = s.update_check == Some(true) && updates_allowed;
            if ui.add_enabled(updates_allowed, egui::Checkbox::without_text(&mut on)).changed() {
                s.update_check = Some(on);
            }
        });
        let p = theme::palette(ui);
        let note = if updates_allowed {
            "One anonymous request to GitHub's releases page at startup; nothing about your computer is sent."
        } else {
            "Turned off in cpuzeez.ini (CheckUpdates=0)."
        };
        ui.label(RichText::new(tr(note)).small().color(p.text_muted));
    });
    ui.add_space(theme::GAP);

    if let Some(dir) = crate::settings::dir() {
        widgets::card(ui, icon::LOCK_SIMPLE, tr("Privacy & data"), |ui| privacy(ui, s, &dir, report));
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

//! Settings page: language, theme, tray, sampling, graphs, warning
//! temperature; changes apply live and are saved to settings.json at once.

use super::i18n::{self, tr};
use super::theme::{self, icon};
use super::widgets;
use crate::report::Section;
use crate::settings::{Settings, Theme};
use eframe::egui::{self, RichText};

pub const PT_BR: &[(&str, &str)] = &[
    ("Tray icons", "Ícones da bandeja"),
    ("Add icon", "Adicionar ícone"),
    ("Remove", "Remover"),
    ("Left click opens", "Clique esquerdo abre"),
    ("The app", "O app"),
    ("Number", "Número"),
    ("Bar", "Barra"),
    ("Mini graph", "Minigráfico"),
    ("Ring", "Anel"),
    ("Label", "Rótulo"),
    (
        "Windows 11 puts new tray icons in the overflow menu (^). Drag them to the taskbar, or turn them on in Settings › Personalization › Taskbar › Other system tray icons. Apps can't do it for you.",
        "O Windows 11 coloca ícones novos no menu de excedentes (^). Arraste-os para a barra de tarefas, ou ative em Configurações › Personalização › Barra de tarefas › Outros ícones da bandeja do sistema. Apps não podem fazer isso por você.",
    ),
    (
        "GNOME shows tray icons only with the AppIndicator extension; without it, the mini monitor opens instead.",
        "O GNOME só mostra ícones da bandeja com a extensão AppIndicator; sem ela, o mini monitor abre no lugar.",
    ),
    (
        "macOS shows one menu-bar item with every metric as text.",
        "O macOS mostra um item na barra de menus com todas as métricas em texto.",
    ),
    ("Alerts", "Alertas"),
    ("Show local alerts on the Overview and in the tray", "Mostrar alertas locais na Visão geral e na bandeja"),
    (
        "Checked on this computer, from its own sensor history; never sent anywhere.",
        "Verificados neste computador, pelo histórico dos próprios sensores; nunca enviados a lugar nenhum.",
    ),
    ("above", "acima de"),
    ("below", "abaixo de"),
    ("for", "por"),
    ("CPU temperature", "Temperatura da CPU"),
    ("GPU temperature", "Temperatura da GPU"),
    ("Memory load", "Uso da memória"),
    ("Disk active time", "Tempo ativo do disco"),
    ("Units", "Unidades"),
    ("Temperature", "Temperatura"),
    ("Clocks", "Clocks"),
    ("Memory sizes", "Tamanhos de memória"),
    ("Decimal places", "Casas decimais"),
    ("Automatic", "Automático"),
    ("MB (as Windows counts)", "MB (como o Windows conta)"),
    ("MB (decimal, as drive makers count)", "MB (decimal, como os fabricantes de discos contam)"),
    ("As each page shows it", "Como cada página mostra"),
    (
        "Only how values are shown changes: logs, reports and thresholds keep °C, MHz and MiB.",
        "Só muda como os valores aparecem: logs, relatórios e limites continuam em °C, MHz e MiB.",
    ),
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
    ("Update channel", "Canal de atualização"),
    ("Stable", "Estável"),
    ("Beta", "Beta"),
    ("Alpha", "Alfa"),
    (
        "Stable: releases only. Beta: also betas and release candidates. Alpha: everything that is published. Pre-releases may be unfinished.",
        "Estável: só versões finais. Beta: também betas e candidatas a versão final. Alfa: tudo o que foi publicado. Pré-versões podem estar inacabadas.",
    ),
    (
        "One anonymous request to GitHub's releases page at startup; nothing about your computer is sent.",
        "Uma consulta anônima à página de versões do GitHub ao iniciar; nada sobre o seu computador é enviado.",
    ),
    ("Turned off in cpuzeez.ini (CheckUpdates=0).", "Desativado no cpuzeez.ini (CheckUpdates=0)."),
    ("Privacy & data", "Privacidade e dados"),
    (
        "These settings were saved by a newer CPU ZeeZ, so changes here aren't saved (that would lose its newer options).",
        "Estas configurações foram salvas por um CPU ZeeZ mais novo, então mudanças aqui não são salvas (isso perderia as opções mais novas).",
    ),
    (
        "Everything CPU ZeeZ keeps is on this computer, in this folder. Nothing is uploaded.",
        "Tudo o que o CPU ZeeZ guarda está neste computador, nesta pasta. Nada é enviado.",
    ),
    ("Open data folder", "Abrir pasta de dados"),
    ("Sensor logs", "Logs de sensores"),
    ("ZeeZ Live sessions", "Sessões do ZeeZ Live"),
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
    ("System (Task Manager)", "Sistema (Gerenciador de Tarefas)"),
    ("Default start page", "Página inicial padrão"),
    ("Always on top", "Sempre no topo"),
    ("Minimize on use (Switch to / Run task)", "Minimizar ao usar (Alternar para / Executar tarefa)"),
    ("Hide when minimized", "Ocultar quando minimizado"),
    ("Show full account name", "Mostrar nome completo da conta"),
    ("Confirm before enabling Efficiency mode", "Confirmar antes de ativar o Modo de eficiência"),
    ("App history retention", "Retenção do histórico de apps"),
];

pub const ES: &[(&str, &str)] = &[
    ("Tray icons", "Iconos de la bandeja"),
    ("Add icon", "Añadir icono"),
    ("Remove", "Quitar"),
    ("Left click opens", "El clic izquierdo abre"),
    ("The app", "La app"),
    ("Number", "Número"),
    ("Bar", "Barra"),
    ("Mini graph", "Minigráfico"),
    ("Ring", "Anillo"),
    ("Label", "Etiqueta"),
    (
        "Windows 11 puts new tray icons in the overflow menu (^). Drag them to the taskbar, or turn them on in Settings › Personalization › Taskbar › Other system tray icons. Apps can't do it for you.",
        "Windows 11 pone los iconos nuevos en el menú de desbordamiento (^). Arrástrelos a la barra de tareas o actívelos en Configuración › Personalización › Barra de tareas › Otros iconos de la bandeja del sistema. Las apps no pueden hacerlo por usted.",
    ),
    (
        "GNOME shows tray icons only with the AppIndicator extension; without it, the mini monitor opens instead.",
        "GNOME solo muestra iconos de la bandeja con la extensión AppIndicator; sin ella, se abre el minimonitor.",
    ),
    (
        "macOS shows one menu-bar item with every metric as text.",
        "macOS muestra un elemento en la barra de menús con todas las métricas como texto.",
    ),
    ("Alerts", "Alertas"),
    ("Show local alerts on the Overview and in the tray", "Mostrar alertas locales en el Resumen y en la bandeja"),
    (
        "Checked on this computer, from its own sensor history; never sent anywhere.",
        "Se comprueban en este equipo, con el historial de sus sensores; nunca se envían a ningún sitio.",
    ),
    ("above", "por encima de"),
    ("below", "por debajo de"),
    ("for", "durante"),
    ("CPU temperature", "Temperatura de la CPU"),
    ("GPU temperature", "Temperatura de la GPU"),
    ("Memory load", "Uso de memoria"),
    ("Disk active time", "Tiempo activo del disco"),
    ("Units", "Unidades"),
    ("Temperature", "Temperatura"),
    ("Clocks", "Relojes"),
    ("Memory sizes", "Tamaños de memoria"),
    ("Decimal places", "Decimales"),
    ("Automatic", "Automático"),
    ("MB (as Windows counts)", "MB (como cuenta Windows)"),
    ("MB (decimal, as drive makers count)", "MB (decimal, como cuentan los fabricantes de discos)"),
    ("As each page shows it", "Como lo muestra cada página"),
    (
        "Only how values are shown changes: logs, reports and thresholds keep °C, MHz and MiB.",
        "Solo cambia cómo se muestran los valores: registros, informes y umbrales siguen en °C, MHz y MiB.",
    ),
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
    ("Update channel", "Canal de actualización"),
    ("Stable", "Estable"),
    ("Beta", "Beta"),
    ("Alpha", "Alfa"),
    (
        "Stable: releases only. Beta: also betas and release candidates. Alpha: everything that is published. Pre-releases may be unfinished.",
        "Estable: solo versiones finales. Beta: también betas y candidatas a versión final. Alfa: todo lo publicado. Las preversiones pueden estar inacabadas.",
    ),
    (
        "One anonymous request to GitHub's releases page at startup; nothing about your computer is sent.",
        "Una consulta anónima a la página de versiones de GitHub al iniciar; no se envía nada sobre tu equipo.",
    ),
    ("Turned off in cpuzeez.ini (CheckUpdates=0).", "Desactivado en cpuzeez.ini (CheckUpdates=0)."),
    ("Privacy & data", "Privacidad y datos"),
    (
        "These settings were saved by a newer CPU ZeeZ, so changes here aren't saved (that would lose its newer options).",
        "Estos ajustes los guardó un CPU ZeeZ más nuevo, así que los cambios aquí no se guardan (se perderían sus opciones nuevas).",
    ),
    (
        "Everything CPU ZeeZ keeps is on this computer, in this folder. Nothing is uploaded.",
        "Todo lo que CPU ZeeZ guarda está en este equipo, en esta carpeta. No se sube nada.",
    ),
    ("Open data folder", "Abrir carpeta de datos"),
    ("Sensor logs", "Registros de sensores"),
    ("ZeeZ Live sessions", "Sesiones de ZeeZ Live"),
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
    ("System (Task Manager)", "Sistema (Administrador de tareas)"),
    ("Default start page", "Página de inicio predeterminada"),
    ("Always on top", "Siempre visible"),
    ("Minimize on use (Switch to / Run task)", "Minimizar al usar (Cambiar a / Ejecutar tarea)"),
    ("Hide when minimized", "Ocultar al minimizar"),
    ("Show full account name", "Mostrar nombre completo de la cuenta"),
    ("Confirm before enabling Efficiency mode", "Confirmar antes de habilitar el Modo de eficiencia"),
    ("App history retention", "Retención del historial de aplicaciones"),
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
        RichText::new(tr("Everything CPU ZeeZ keeps is on this computer, in this folder. Nothing is uploaded."))
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

/// Tray icons (S1-M01): one row per icon (metric, style, label), add/remove,
/// the left-click action and the platform's honest limits.
pub(super) fn tray_items(ui: &mut egui::Ui, s: &mut Settings) {
    use cpuzeez_core::layout::tray::{Click, Item, Style};
    let p = theme::palette(ui);
    let mut remove = None;
    for (i, it) in s.tray_items.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            let metric = it.widget.metric.clone().unwrap_or_default();
            let shown = super::layout_view::ALIASES
                .iter()
                .find(|a| a.0 == metric)
                .map_or(metric.clone(), |a| tr(a.1).to_string());
            egui::ComboBox::from_id_salt(("tray-metric", i)).selected_text(shown).width(220.0).show_ui(ui, |ui| {
                for (alias, what) in super::layout_view::ALIASES {
                    ui.selectable_value(&mut it.widget.metric, Some(alias.to_string()), tr(what));
                }
            });
            for st in Style::ALL {
                ui.selectable_value(&mut it.style, st, tr(st.label()));
            }
            let mut label = it.widget.label.clone().unwrap_or_default();
            if ui.add(egui::TextEdit::singleline(&mut label).hint_text(tr("Label")).desired_width(70.0)).changed() {
                it.widget.label = (!label.is_empty()).then_some(label);
            }
            if ui.small_button(icon::TRASH).on_hover_text(tr("Remove")).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove.filter(|_| s.tray_items.len() > 1) {
        s.tray_items.remove(i);
    }
    if s.tray_items.len() < 8 && ui.button(format!("{}  {}", icon::PLUS, tr("Add icon"))).clicked() {
        let mut it = Item::default();
        it.widget.metric = Some("@gpu_temp".into());
        it.widget.label = Some("GPU".into());
        s.tray_items.push(it);
    }
    row(ui, tr("Left click opens"), |ui| {
        ui.selectable_value(&mut s.tray_click, Click::App, tr("The app"));
        ui.selectable_value(&mut s.tray_click, Click::Mini, tr("Mini monitor"));
    });
    let limit = if cfg!(windows) {
        "Windows 11 puts new tray icons in the overflow menu (^). Drag them to the taskbar, or turn them on in Settings › Personalization › Taskbar › Other system tray icons. Apps can't do it for you."
    } else if cfg!(target_os = "macos") {
        "macOS shows one menu-bar item with every metric as text."
    } else {
        "GNOME shows tray icons only with the AppIndicator extension; without it, the mini monitor opens instead."
    };
    ui.label(RichText::new(tr(limit)).small().color(p.text_muted));
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
/// Opt-in local alert rules (S1-G01).
fn alerts(ui: &mut egui::Ui, s: &mut Settings) {
    ui.checkbox(&mut s.alerts_on, tr("Show local alerts on the Overview and in the tray"));
    ui.add_enabled_ui(s.alerts_on, |ui| {
        for (i, r) in s.alerts.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut r.enabled, tr(&r.label));
                egui::ComboBox::from_id_salt(("alert-op", i))
                    .width(90.0)
                    .selected_text(tr(if r.above { "above" } else { "below" }))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut r.above, true, tr("above"));
                        ui.selectable_value(&mut r.above, false, tr("below"));
                    });
                ui.add(egui::DragValue::new(&mut r.threshold).speed(0.5));
                ui.label(tr("for"));
                ui.add(egui::DragValue::new(&mut r.secs).range(1..=3600).suffix(" s"));
            });
        }
    });
    let p = theme::palette(ui);
    ui.label(
        RichText::new(tr("Checked on this computer, from its own sensor history; never sent anywhere."))
            .small()
            .color(p.text_muted),
    );
}

/// °C/°F, MHz/GHz, MB/MiB, decimals (S1-U04).
fn units(ui: &mut egui::Ui, u: &mut cpuzeez_core::units::Units) {
    use cpuzeez_core::units::{Clock, Size, Temp};
    row(ui, tr("Temperature"), |ui| {
        ui.selectable_value(&mut u.temp, Temp::Celsius, "°C");
        ui.selectable_value(&mut u.temp, Temp::Fahrenheit, "°F");
    });
    row(ui, tr("Clocks"), |ui| {
        ui.selectable_value(&mut u.clock, Clock::Auto, tr("Automatic"));
        ui.selectable_value(&mut u.clock, Clock::Mhz, "MHz");
        ui.selectable_value(&mut u.clock, Clock::Ghz, "GHz");
    });
    row(ui, tr("Memory sizes"), |ui| {
        let label = |s: Size| match s {
            Size::Windows => tr("MB (as Windows counts)").to_string(),
            Size::Iec => "MiB / GiB".to_string(),
            Size::Decimal => tr("MB (decimal, as drive makers count)").to_string(),
        };
        egui::ComboBox::from_id_salt("size-unit").selected_text(label(u.size)).show_ui(ui, |ui| {
            for s in [Size::Windows, Size::Iec, Size::Decimal] {
                ui.selectable_value(&mut u.size, s, label(s));
            }
        });
    });
    row(ui, tr("Decimal places"), |ui| {
        let label = |d: Option<u8>| d.map_or_else(|| tr("As each page shows it").to_string(), |d| d.to_string());
        egui::ComboBox::from_id_salt("decimals").selected_text(label(u.decimals)).show_ui(ui, |ui| {
            for d in [None, Some(0), Some(1), Some(2), Some(3)] {
                ui.selectable_value(&mut u.decimals, d, label(d));
            }
        });
    });
    let p = theme::palette(ui);
    ui.label(
        RichText::new(tr("Only how values are shown changes: logs, reports and thresholds keep °C, MHz and MiB."))
            .small()
            .color(p.text_muted),
    );
}

/// System (Task Manager) preferences (S2a-11, docs/v1/task-manager.md §9).
pub(super) fn system_settings(ui: &mut egui::Ui, s: &mut Settings) {
    const PAGES: &[&str] = &[
        "Overview",
        "System",
        "Processor",
        "Mainboard",
        "Memory",
        "Graphics",
        "Devices",
        "Sensors",
        "Benchmarks",
        "Tools",
    ];
    row(ui, tr("Default start page"), |ui| {
        let cur = if s.default_start_page.is_empty() { "Overview" } else { &s.default_start_page };
        egui::ComboBox::from_id_salt("sys_default_start_page").selected_text(tr(cur)).show_ui(ui, |ui| {
            for &p in PAGES {
                ui.selectable_value(&mut s.default_start_page, p.to_string(), tr(p));
            }
        });
    });
    row(ui, tr("Always on top"), |ui| {
        ui.checkbox(&mut s.always_on_top, "");
    });
    row(ui, tr("Minimize on use (Switch to / Run task)"), |ui| {
        ui.checkbox(&mut s.minimize_on_use, "");
    });
    row(ui, tr("Hide when minimized"), |ui| {
        ui.checkbox(&mut s.hide_when_minimized, "");
    });
    row(ui, tr("Show full account name"), |ui| {
        ui.checkbox(&mut s.show_full_account_name, "");
    });
    row(ui, tr("Confirm before enabling Efficiency mode"), |ui| {
        ui.checkbox(&mut s.confirm_eco_mode, "");
    });
    row(ui, tr("Record app resource history"), |ui| {
        ui.checkbox(&mut s.app_history_enabled, "");
    });
    row(ui, tr("App history retention"), |ui| {
        ui.add(
            egui::DragValue::new(&mut s.app_history_retention_days)
                .range(1..=365)
                .suffix(format!(" {}", tr("days (0 = keep)"))),
        );
    });
}

/// checks off; `persist` = save changes (off while taking screenshots).
pub fn page(ui: &mut egui::Ui, s: &mut Settings, updates_allowed: bool, persist: bool, report: Option<&[Section]>) {
    let before = serde_json::to_string(s).unwrap_or_default();
    if s.read_only {
        widgets::pill(
            ui,
            tr(
                "These settings were saved by a newer CPU ZeeZ, so changes here aren't saved (that would lose its newer options).",
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

    widgets::card(ui, icon::RULER, tr("Units"), |ui| units(ui, &mut s.units));
    ui.add_space(theme::GAP);

    widgets::card(ui, icon::LIST_BULLETS, tr("System (Task Manager)"), |ui| system_settings(ui, s));
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

    if s.tray {
        widgets::card(ui, icon::TRAY, tr("Tray icons"), |ui| tray_items(ui, s));
        ui.add_space(theme::GAP);
    }

    widgets::card(ui, icon::BELL, tr("Alerts"), |ui| alerts(ui, s));
    ui.add_space(theme::GAP);

    widgets::card(ui, icon::ARROW_CIRCLE_UP, tr("Updates"), |ui| {
        row(ui, tr("Check for updates at startup"), |ui| {
            let mut on = s.update_check == Some(true) && updates_allowed;
            if ui.add_enabled(updates_allowed, egui::Checkbox::without_text(&mut on)).changed() {
                s.update_check = Some(on);
            }
        });
        row(ui, tr("Update channel"), |ui| {
            let cur = s.channel();
            egui::ComboBox::from_id_salt("update-channel").selected_text(tr(cur.name())).show_ui(ui, |ui| {
                for c in crate::update::Channel::ALL {
                    if ui.selectable_label(cur == c, tr(c.name())).clicked() {
                        s.update_channel = Some(c);
                    }
                }
            });
        });
        let p = theme::palette(ui);
        ui.label(RichText::new(tr(
            "Stable: releases only. Beta: also betas and release candidates. Alpha: everything that is published. Pre-releases may be unfinished.",
        ))
        .small()
        .color(p.text_muted));
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

#[cfg(test)]
mod tests {
    use eframe::egui;
    use egui_kittest::{Harness, kittest::Queryable};

    /// S1-M01: icons are added, restyled and removed from the settings card.
    #[test]
    fn tray_icons_settings() {
        use cpuzeez_core::layout::tray::Style;
        let mut h = Harness::builder().with_size(egui::vec2(900.0, 500.0)).build_ui_state(
            |ui, s: &mut crate::settings::Settings| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                super::tray_items(ui, s);
            },
            crate::settings::Settings::default(),
        );
        h.run();
        h.get_by_label_contains("Add icon").click();
        h.run();
        assert_eq!(h.state().tray_items.len(), 2);
        h.get_all_by_label("Ring").next().expect("style buttons").click();
        h.run();
        assert_eq!(h.state().tray_items[0].style, Style::Ring);
        h.get_by_label("Mini monitor").click();
        h.run();
        assert_eq!(h.state().tray_click, cpuzeez_core::layout::tray::Click::Mini);
    }

    /// S2a-11: System (Task Manager) settings card renders and binds options.
    #[test]
    fn system_task_manager_settings() {
        let mut h = Harness::builder().with_size(egui::vec2(900.0, 500.0)).build_ui_state(
            |ui, s: &mut crate::settings::Settings| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                super::system_settings(ui, s);
            },
            crate::settings::Settings::default(),
        );
        h.run();
        h.run();
        h.get_by_label("Default start page");
        h.get_by_label("Always on top");
    }
}

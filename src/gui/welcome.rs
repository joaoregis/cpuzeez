//! First-run screen (docs/v1/README.md P1–P3): what CPU ZeeZ does and doesn't
//! do with your data, the update opt-in, recording and permissions. Shown once
//! per `PRIVACY_VERSION`, so it comes back if any of this ever changes.

use super::i18n::tr;
use super::theme::{self, icon};
use super::widgets;
use eframe::egui::{self, RichText};

/// Bump when what this screen promises changes.
pub const PRIVACY_VERSION: u32 = 1;

pub const PT_BR: &[(&str, &str)] = &[
    ("Welcome to CPU ZeeZ", "Bem-vindo ao CPU ZeeZ"),
    ("A few things to know before you start.", "Algumas coisas antes de começar."),
    ("Your data stays on this computer", "Seus dados ficam neste computador"),
    (
        "No telemetry, no account, no cloud. Everything CPU ZeeZ saves stays in its data folder, and you can see and delete it in Settings › Privacy & data.",
        "Sem telemetria, sem conta, sem nuvem. Tudo o que o CPU ZeeZ salva fica na pasta de dados dele, e você pode ver e apagar em Configurações › Privacidade e dados.",
    ),
    ("Updates", "Atualizações"),
    ("Check for new versions at startup", "Procurar novas versões ao iniciar"),
    (
        "One anonymous request to GitHub's releases page. Nothing about your computer is sent. You can change this any time in Settings.",
        "Uma consulta anônima à página de versões do GitHub. Nada sobre o seu computador é enviado. Você pode mudar isso a qualquer momento em Configurações.",
    ),
    ("Turned off in cpuzeez.ini (CheckUpdates=0).", "Desativado no cpuzeez.ini (CheckUpdates=0)."),
    ("Nothing is recorded until you start it", "Nada é gravado até você iniciar"),
    (
        "Sensor logs, benchmark history and, later, game sessions are only saved when you ask. While something records, ● REC shows in the window title and the sidebar.",
        "Logs de sensores, histórico de benchmarks e, no futuro, sessões de jogo só são salvos quando você pede. Enquanto algo grava, ● REC aparece no título da janela e na barra lateral.",
    ),
    ("Permissions", "Permissões"),
    (
        "CPU ZeeZ starts without administrator rights. When a page needs more access, it says exactly what is missing and offers to restart with it.",
        "O CPU ZeeZ inicia sem permissão de administrador. Quando uma página precisa de mais acesso, ela diz exatamente o que falta e oferece reiniciar com ele.",
    ),
    ("Continue", "Continuar"),
];

pub const ES: &[(&str, &str)] = &[
    ("Welcome to CPU ZeeZ", "Bienvenido a CPU ZeeZ"),
    ("A few things to know before you start.", "Algunas cosas antes de empezar."),
    ("Your data stays on this computer", "Tus datos se quedan en este equipo"),
    (
        "No telemetry, no account, no cloud. Everything CPU ZeeZ saves stays in its data folder, and you can see and delete it in Settings › Privacy & data.",
        "Sin telemetría, sin cuenta, sin nube. Todo lo que CPU ZeeZ guarda se queda en su carpeta de datos, y puedes verlo y borrarlo en Ajustes › Privacidad y datos.",
    ),
    ("Updates", "Actualizaciones"),
    ("Check for new versions at startup", "Buscar nuevas versiones al iniciar"),
    (
        "One anonymous request to GitHub's releases page. Nothing about your computer is sent. You can change this any time in Settings.",
        "Una consulta anónima a la página de versiones de GitHub. No se envía nada sobre tu equipo. Puedes cambiarlo cuando quieras en Ajustes.",
    ),
    ("Turned off in cpuzeez.ini (CheckUpdates=0).", "Desactivado en cpuzeez.ini (CheckUpdates=0)."),
    ("Nothing is recorded until you start it", "No se graba nada hasta que lo inicies"),
    (
        "Sensor logs, benchmark history and, later, game sessions are only saved when you ask. While something records, ● REC shows in the window title and the sidebar.",
        "Los registros de sensores, el historial de pruebas y, más adelante, las sesiones de juego solo se guardan cuando lo pides. Mientras algo graba, ● REC aparece en el título de la ventana y en la barra lateral.",
    ),
    ("Permissions", "Permisos"),
    (
        "CPU ZeeZ starts without administrator rights. When a page needs more access, it says exactly what is missing and offers to restart with it.",
        "CPU ZeeZ se inicia sin permisos de administrador. Cuando una página necesita más acceso, indica exactamente qué falta y ofrece reiniciar con él.",
    ),
    ("Continue", "Continuar"),
];

/// The screen's one choice.
#[derive(Default)]
pub struct Welcome {
    pub updates: bool,
}

/// Draws the screen; returns Some(update_check) once the user continues.
pub fn page(ui: &mut egui::Ui, w: &mut Welcome, updates_allowed: bool) -> Option<bool> {
    let p = theme::palette(ui);
    let mut done = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(720.0);
        widgets::page_header(ui, tr("Welcome to CPU ZeeZ"), tr("A few things to know before you start."), |_| {});
        let text = |ui: &mut egui::Ui, s: &str| {
            ui.label(RichText::new(tr(s)).color(p.text_secondary));
        };
        widgets::card(ui, icon::LOCK_SIMPLE, tr("Your data stays on this computer"), |ui| {
            text(ui, "No telemetry, no account, no cloud. Everything CPU ZeeZ saves stays in its data folder, and you can see and delete it in Settings › Privacy & data.");
        });
        ui.add_space(theme::GAP);
        widgets::card(ui, icon::ARROW_CIRCLE_UP, tr("Updates"), |ui| {
            ui.add_enabled(updates_allowed, egui::Checkbox::new(&mut w.updates, tr("Check for new versions at startup")));
            text(ui, "One anonymous request to GitHub's releases page. Nothing about your computer is sent. You can change this any time in Settings.");
            if !updates_allowed {
                text(ui, "Turned off in cpuzeez.ini (CheckUpdates=0).");
            }
        });
        ui.add_space(theme::GAP);
        widgets::card(ui, icon::RECORD, tr("Nothing is recorded until you start it"), |ui| {
            text(ui, "Sensor logs, benchmark history and, later, game sessions are only saved when you ask. While something records, ● REC shows in the window title and the sidebar.");
        });
        ui.add_space(theme::GAP);
        widgets::card(ui, icon::SHIELD_CHECK, tr("Permissions"), |ui| {
            text(ui, "CPU ZeeZ starts without administrator rights. When a page needs more access, it says exactly what is missing and offers to restart with it.");
        });
        ui.add_space(theme::GAP);
        if widgets::primary_button(ui, tr("Continue")).clicked() {
            done = Some(w.updates && updates_allowed);
        }
    });
    done
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    /// (screen state, the answer once Continue is pressed)
    type St = (Welcome, Option<bool>);

    fn harness<'a>() -> Harness<'a, St> {
        // Tall enough that Continue is on screen (the page scrolls in the app).
        Harness::builder().with_size(egui::vec2(900.0, 1400.0)).build_ui_state(
            |ui, st: &mut St| {
                // The harness draws a frame while it is built: install the app's
                // text styles then, and draw from the next frame on.
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                if let Some(d) = page(ui, &mut st.0, true) {
                    st.1 = Some(d);
                }
            },
            (Welcome::default(), None),
        )
    }

    #[test]
    fn continuing_without_ticking_means_no_update_check() {
        let mut h = harness();
        h.run();
        h.get_by_label("Continue").click();
        h.run();
        assert_eq!(h.state().1, Some(false), "the update check is opt-in (P1)");
    }

    #[test]
    fn ticking_the_box_opts_in() {
        let mut h = harness();
        h.run();
        h.get_by_label("Check for new versions at startup").click();
        h.run();
        h.get_by_label("Continue").click();
        h.run();
        assert_eq!(h.state().1, Some(true));
    }
}

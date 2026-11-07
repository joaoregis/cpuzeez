//! About page.

#[allow(unused_imports)]
use crate::gui::*;

impl App {
    pub(crate) fn about_tab(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let logo = self.logo_texture(ui);
        ui.horizontal(|ui| {
            ui.image((logo, egui::vec2(64.0, 64.0)));
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(format!("{} {}", crate::DISPLAY_NAME, crate::VERSION)).text_style(theme::title()),
                );
                ui.label(
                    RichText::new(tr("Clean-room, open-source reimplementation of CPU-Z's feature set."))
                        .color(theme::palette(ui).text_secondary),
                );
            });
        });
        ui.add_space(theme::GAP);
        ui.label(trf("Hardware access: {}", &[&self.ctx.hw.name()]));
        let sw = sec(r, "Software")
            .map(|s| report::fields(&s.nodes).map(|(k, v)| format!("{}: {v}", tr(k))).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default();
        if !sw.is_empty() {
            ui.label(sw);
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("Save Report (.TXT)")).clicked() {
                self.save(&Some(r.to_vec()), "txt");
            }
            if ui.button(tr("Save Report (.HTML)")).clicked() {
                self.save(&Some(r.to_vec()), "html");
            }
            if ui.button(tr("Clocks")).clicked() {
                self.page = Page::Tools;
                self.tool_view = ToolView::Clocks;
            }
            if ui.button(tr("Timers")).clicked() {
                self.page = Page::Tools;
                self.tool_view = ToolView::Timers;
            }
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.hyperlink_to(tr("Project page"), crate::REPO_URL);
            if ui.button(tr("Check for updates")).clicked() {
                let slot = self.update_manual.clone();
                *slot.lock().unwrap() = Some(tr("Checking...").into());
                let found = self.update.clone();
                let channel = self.mon.settings.channel();
                self.update_dismissed = false;
                std::thread::spawn(move || {
                    let r = crate::update::check(channel);
                    // Translated when the check ends (the language can't change mid-check in practice).
                    let text = match &r {
                        Some(r) => trf("{} {} is available.", &[&crate::DISPLAY_NAME, &r.version]),
                        None => trf("You have the latest version ({}).", &[&crate::VERSION]),
                    };
                    *found.lock().unwrap() = r;
                    *slot.lock().unwrap() = Some(text);
                });
            }
            if let Some(t) = self.update_manual.lock().unwrap().as_deref() {
                ui.label(t);
            }
        });
        ui.separator();
        ui.collapsing(tr("Third-party components"), |ui| {
            if cfg!(windows) {
                use crate::executor::pin;
                ui.label(trf("PawnIO driver installer {}, by namazso (pawnio.eu)", &[&pin("PAWNIO_SETUP_VERSION")]));
                ui.label(trf("PawnIO modules {}, LGPL-2.1-or-later", &[&pin("PAWNIO_MODULES_VERSION")]));
                ui.label(trf("PresentMon {}, by Intel, MIT (ZeeZ Live frame timing)", &[&pin("PRESENTMON_VERSION")]));
            } else if cfg!(target_os = "linux") {
                ui.label(tr(
                    "MangoHud (optional, not bundled): installed by your package manager when you ask for it.",
                ));
            }
            ui.label(tr("Rust crates under MIT, Apache-2.0 and compatible licenses."));
            ui.hyperlink_to(
                tr("Full list and licenses"),
                format!("{}/blob/v{}/THIRD_PARTY.md", crate::REPO_URL, crate::VERSION),
            );
        });
        ui.label(RichText::new(tr("Not affiliated with CPUID. CPU-Z is a trademark of CPUID.")).small().weak());
    }
}

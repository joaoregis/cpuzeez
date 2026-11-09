//! The update banner (download, verify, install).

#[allow(unused_imports)]
use crate::gui::*;

impl App {
    pub(crate) fn update_banner(&mut self, ui: &mut egui::Ui) {
        if self.installer.as_ref().is_some_and(|i| matches!(i.stage(), Stage::Quit)) {
            // The update helper waits for us: quit for real (not to the tray),
            // through on_exit, which stops PresentMon and the ETW session; a
            // bare process exit left them running and the installer blocked.
            self.installer = None;
            self.mon.quit(ui.ctx());
            return;
        }
        if let Some(&code) = crate::update::FAILED.get()
            && !self.update_failed_seen
        {
            egui::Panel::top("update-failed").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    widgets::pill(ui, &failed_text(code), widgets::Tone::Danger);
                    if ui.small_button(icon::X).on_hover_text(tr("Dismiss")).clicked() {
                        self.update_failed_seen = true;
                    }
                });
            });
        }
        if self.update_dismissed {
            return;
        }
        let Some(rel) = self.update.lock().unwrap().clone() else { return };
        egui::Panel::top("update").show(ui, |ui| update_bar(ui, &rel, &mut self.installer, &mut self.update_dismissed));
    }
}

/// The update notice: version, one-click install with progress, errors, notes.
pub(crate) fn update_bar(ui: &mut egui::Ui, rel: &Release, installer: &mut Option<Installer>, dismissed: &mut bool) {
    let p = theme::palette(ui);
    let stage = installer.as_ref().map(Installer::stage);
    if matches!(stage, Some(Stage::Downloading { .. } | Stage::Verifying | Stage::Installing)) {
        ui.ctx().request_repaint_after(Duration::from_millis(150));
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(icon::ARROW_CIRCLE_UP).color(p.accent));
        ui.label(RichText::new(trf("{} {} is available", &[&crate::DISPLAY_NAME, &rel.version])).strong());
        ui.label(RichText::new(trf("(you have {})", &[&crate::VERSION])).color(p.text_secondary));
        let start = |installer: &mut Option<Installer>| *installer = Some(Installer::start(rel));
        match &stage {
            None => {
                if rel.asset.is_some()
                    && widgets::primary_button(ui, &format!("{} {}", icon::DOWNLOAD_SIMPLE, tr("Download & install")))
                        .clicked()
                {
                    start(installer);
                }
            }
            Some(Stage::Downloading { done, total }) => {
                let mb = |b: u64| format!("{:.1}", b as f64 / 1e6);
                let frac = if *total > 0 { *done as f32 / *total as f32 } else { 0.0 };
                let text = trf("Downloading… {} / {} MB", &[&mb(*done), &mb(*total)]);
                ui.allocate_ui(egui::vec2(220.0, 22.0), |ui| widgets::progress(ui, frac, &text));
            }
            Some(Stage::Verifying) => {
                ui.spinner();
                ui.label(tr("Verifying checksum…"));
            }
            Some(Stage::Installing) => {
                ui.spinner();
                ui.label(tr("Starting the installer…"));
            }
            Some(Stage::Installed) => {
                widgets::pill(ui, tr("Installed. Restart CPU ZeeZ to use the new version."), widgets::Tone::Success);
            }
            Some(Stage::OpenedInstaller) => {
                widgets::pill(
                    ui,
                    tr("The installer was opened: follow its steps, then restart CPU ZeeZ."),
                    widgets::Tone::Accent,
                );
            }
            Some(Stage::OpenedFolder(d)) => {
                widgets::pill(ui, &trf("Downloaded and verified. Opened {}", &[&d.display()]), widgets::Tone::Success);
            }
            Some(Stage::Failed(e)) => {
                widgets::pill(ui, &trf("Update failed: {}", &[e]), widgets::Tone::Danger);
                if ui.button(tr("Retry")).clicked() {
                    start(installer);
                }
                if let Some(a) = &rel.asset
                    && ui.button(tr("Open download page")).clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(&a.url));
                }
            }
            Some(Stage::Quit) => {
                ui.spinner();
                ui.label(tr("Closing to install…"));
            }
        }
        if ui.button(tr("What's new")).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(&rel.page));
        }
        let busy = matches!(stage, Some(Stage::Downloading { .. } | Stage::Verifying | Stage::Installing));
        if !busy && ui.small_button(icon::X).on_hover_text(tr("Dismiss")).clicked() {
            *dismissed = true;
        }
    });
}

/// Why the update helper reopened the old version (msiexec's exit code).
fn failed_text(code: i32) -> String {
    match code {
        1602 | 1223 => trf("The update was cancelled; CPU ZeeZ {} is still installed.", &[&crate::VERSION]),
        1618 => trf(
            "Another installation was running, so the update did not start; CPU ZeeZ {} is still installed.",
            &[&crate::VERSION],
        ),
        _ => trf(
            "The update did not finish (installer code {}); CPU ZeeZ {} is still installed.",
            &[&code, &crate::VERSION],
        ),
    }
}

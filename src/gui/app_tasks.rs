//! The frame loop's chores (M4-06): pins and panel widths handed back by
//! the pages, global shortcuts, the update check and the window title.

#[allow(unused_imports)]
use crate::gui::*;

impl App {
    /// Pins asked for by any row last frame (S1-U03) go into the settings.
    pub(super) fn take_pins(&mut self, ectx: &egui::Context) {
        let mut changed = false;
        for p in pins::take_requests(ectx) {
            let s = &mut self.mon.settings.pins;
            if !s.iter().any(|q| q.card == p.card && q.label == p.label && q.metric == p.metric) {
                s.push(p);
                changed = true;
            }
        }
        if changed && self.auto.is_none() {
            self.mon.settings.save();
        }
        pins::publish(ectx, &self.mon.settings.pins);
    }

    /// A side panel resized last frame: save its width (M2-05).
    pub(super) fn take_widths(&mut self, ectx: &egui::Context) {
        if let Some(w) = split::take_changes(ectx) {
            self.mon.settings.panel_widths = w;
            if self.auto.is_none() {
                self.mon.settings.save();
            }
        }
        split::publish(ectx, &self.mon.settings.panel_widths);
    }

    pub(super) fn hotkeys(&mut self, ectx: &egui::Context) {
        let (f5, f6, f7) =
            ectx.input(|i| (i.key_pressed(egui::Key::F5), i.key_pressed(egui::Key::F6), i.key_pressed(egui::Key::F7)));
        if f5 || f6 {
            self.shot = Some(if f6 { Shot::Clipboard } else { Shot::Bmp });
            shots::request(ectx);
        }
        if f7 {
            let r = self.report.lock().unwrap().clone();
            self.save(&r, "cvf");
        }
        if ectx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) && self.welcome.is_none() {
            let r = self.report.lock().unwrap().clone().unwrap_or_default();
            let pages: Vec<(Page, &str)> = NAV.iter().chain(NAV_BOTTOM.iter()).map(|n| (n.0, n.2)).collect();
            self.palette.open(&r, &pages);
        }
        if let Some(t) = self.palette.show(ectx) {
            self.run_palette(t);
        }
    }

    /// The update check, only when the user opted in and cpuzeez.ini allows it (P1).
    pub(super) fn check_updates_if_allowed(&self) {
        if crate::update::startup_check_allowed(self.ctx.cfg.check_updates, self.mon.settings.update_check) {
            let slot = self.updates.release.clone();
            let channel = self.mon.settings.channel();
            std::thread::spawn(move || *slot.lock().unwrap() = crate::update::check(channel));
        }
    }

    /// Window title: ● REC while recording, and the core clock while
    /// minimized (as CPU-Z does). Sent only when it changes.
    pub(super) fn window_title(&mut self, ectx: &egui::Context) {
        let mut t = String::new();
        if self.mon.recording() || self.live.capturing() {
            t += "● REC · ";
        }
        if ectx.input(|i| i.viewport().minimized.unwrap_or(false)) {
            let tel = self.mon.tel();
            tel.want("title", &["cpu/clock"]);
            if let Some(m) = crate::telemetry::core_mhz(&tel.snapshot(), 0) {
                t += &format!("{m:.0} MHz - ");
            }
        }
        t += crate::DISPLAY_NAME;
        if t != self.title {
            ectx.send_viewport_cmd(egui::ViewportCommand::Title(t.clone()));
            self.title = t;
        }
    }
}

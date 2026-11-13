//! Settings: the settings cards, then the mini monitors.

#[allow(unused_imports)]
use crate::gui::*;

impl App {
    pub(in crate::gui) fn settings_page(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let s = &self.mon.settings;
        let (tray, sampling) = (s.tray, (s.sample_ms, s.graph_seconds));
        let persist = self.auto.is_none();
        settings_ui::page(ui, &mut self.mon.settings, self.ctx.cfg.check_updates, persist, Some(r));
        if self.mon.settings.tray != tray {
            let on = self.mon.settings.tray;
            self.mon.set_tray(on, ui.ctx());
        }
        if (self.mon.settings.sample_ms, self.mon.settings.graph_seconds) != sampling {
            self.mon.apply_sampling();
        }
        ui.add_space(theme::GAP);
        let tjmax = self.mon.tjmax();
        let mut minis = self.mon.settings.minis.clone();
        widgets::card(ui, icon::MONITOR, tr("Mini monitors"), |ui| {
            mini::settings_card(ui, &mut minis, &mut self.minis.edit, &mut self.minis.import, tjmax);
        });
        if minis != self.mon.settings.minis {
            let on = minis.iter().any(|m| m.open);
            if persist {
                self.mon.set_setting(move |s| s.minis = minis.clone());
            } else {
                self.mon.settings.minis = minis;
            }
            self.mon.show_mini = on;
            if on {
                self.mon.tel();
            }
        }
    }
}

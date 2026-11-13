//! Navigation (M4-07): the pages, the sidebar and the command palette's targets.

#[allow(unused_imports)]
use crate::gui::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Page {
    Overview,
    System,
    Live,
    Processor,
    Mainboard,
    Memory,
    Graphics,
    Devices,
    Sensors,
    Benchmarks,
    Tools,
    Settings,
    About,
}

/// (page, icon, English name: translated when shown, used as-is for file names).
pub(super) const NAV: [(Page, &str, &str); 11] = [
    (Page::Overview, icon::SQUARES_FOUR, "Overview"),
    (Page::System, icon::LIST_BULLETS, "System"),
    (Page::Live, icon::PULSE, "ZeeZ Live"),
    (Page::Processor, icon::CPU, "Processor"),
    (Page::Mainboard, icon::CIRCUITRY, "Mainboard"),
    (Page::Memory, icon::MEMORY, "Memory"),
    (Page::Graphics, icon::GRAPHICS_CARD, "Graphics"),
    (Page::Devices, icon::HARD_DRIVES, "Devices"),
    (Page::Sensors, icon::THERMOMETER, "Sensors"),
    (Page::Benchmarks, icon::GAUGE, "Benchmarks"),
    (Page::Tools, icon::TOOLBOX, "Tools"),
];

pub(super) const NAV_BOTTOM: [(Page, &str, &str); 2] =
    [(Page::Settings, icon::GEAR_SIX, "Settings"), (Page::About, icon::INFO, "About")];

pub(super) fn page_name(p: Page) -> &'static str {
    NAV.iter().chain(NAV_BOTTOM.iter()).find(|n| n.0 == p).map_or("Overview", |n| n.2)
}

impl App {
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(ui);
        let logo = self.logo_texture(ui);
        ui.horizontal(|ui| {
            ui.image((logo, egui::vec2(28.0, 28.0)));
            ui.label(RichText::new(crate::DISPLAY_NAME).font(egui::FontId::new(17.0, theme::semibold())).color(p.text));
        });
        ui.add_space(14.0);
        ui.spacing_mut().item_spacing.y = 2.0;
        for (pg, ic, name) in NAV {
            if widgets::sidebar_item(ui, ic, tr(name), self.page == pg).clicked() {
                self.page = pg;
            }
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.label(RichText::new(format!("v{}", crate::VERSION)).small().color(p.text_muted));
            if self.mon.recording() {
                let rec = widgets::pill_button(ui, &format!("{} REC", icon::RECORD), widgets::Tone::Danger)
                    .on_hover_text(tr("Recording · click to stop"));
                if rec.clicked() {
                    self.mon.stop_recording();
                }
            }
            if !self.status.is_empty() {
                ui.add(egui::Label::new(RichText::new(&self.status).small().color(p.text_secondary)).wrap());
            }
            ui.add_space(6.0);
            for (pg, ic, name) in NAV_BOTTOM.iter().rev() {
                if widgets::sidebar_item(ui, ic, tr(name), self.page == *pg).clicked() {
                    self.page = *pg;
                }
            }
        });
    }

    pub(super) fn logo_texture(&mut self, ui: &egui::Ui) -> egui::TextureId {
        self.logo
            .get_or_insert_with(|| {
                let icon = decode_png(include_bytes!("../../assets/icon-128.png")).unwrap_or_default();
                let img =
                    egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
                ui.ctx().load_texture("logo", img, egui::TextureOptions::LINEAR)
            })
            .id()
    }

    /// What the command palette picked (S1-U02).
    pub(super) fn run_palette(&mut self, t: palette::Target) {
        use palette::{Act, Target};
        match t {
            Target::Page(p) => self.page = p,
            Target::Act(Act::SaveTxt | Act::SaveHtml) => {
                let r = self.report.lock().unwrap().clone();
                self.save(&r, if t == Target::Act(Act::SaveTxt) { "txt" } else { "html" });
            }
            Target::Act(Act::ToggleCsv) => {
                let tel = self.mon.tel();
                if tel.csv_path().is_some() {
                    tel.stop_csv();
                } else if let Some(p) = sensors::log_path()
                    && let Err(e) = tel.start_csv(&p)
                {
                    self.status = format!("{}: {e}", p.display());
                }
            }
            Target::Act(Act::ToggleMini) => {
                let on = !self.mon.show_mini;
                self.mon.set_mini(on);
            }
            Target::Act(Act::ResetMinMax) => self.mon.tel().reset_stats(),
            Target::Act(Act::Stress) => self.mon.show_stress = true,
        }
    }
}

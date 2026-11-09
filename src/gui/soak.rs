//! The soak tour (S3-02): with the synthetic game running, visit every page
//! and tab, record and analyse now and then, and show and hide the overlay, so
//! that a leak anywhere shows as growth over hours. Compiled only with
//! `--features soak` and active only with `CPUZEEZ_SOAK=1`.

use super::*;
use std::time::Instant;

/// What the tour is doing, and when it last did each thing.
pub struct Tour {
    started: Instant,
    page: Instant,
    capture: Instant,
    analysis: Instant,
    overlay: Instant,
    visits: usize,
}

impl Default for Tour {
    fn default() -> Self {
        let now = Instant::now();
        Tour { started: now, page: now, capture: now, analysis: now, overlay: now, visits: 0 }
    }
}

impl App {
    pub(crate) fn soak_tour(&mut self, ectx: &egui::Context) {
        if !crate::soak::enabled() {
            return;
        }
        let first = self.soak.is_none();
        let tour = self.soak.get_or_insert_with(Tour::default);
        if crate::soak::overlay_only() {
            if first {
                ectx.send_viewport_cmd_to(egui::ViewportId::ROOT, egui::ViewportCommand::Minimized(true));
            }
            return;
        }
        let every = |t: &Instant, secs: u64| t.elapsed() >= Duration::from_secs(secs);
        // The synthetic game is the target; keep the overlay and the detector on.
        if tour.page.elapsed() >= Duration::from_secs(15) {
            tour.page = Instant::now();
            tour.visits += 1;
            let pages: Vec<Page> = NAV.iter().chain(NAV_BOTTOM.iter()).map(|n| n.0).collect();
            self.page = pages[tour.visits % pages.len()];
            // Each visit to a tabbed page shows the next tab.
            self.system.tab = [
                system::SysTab::Processes,
                system::SysTab::Performance,
                system::SysTab::AppHistory,
                system::SysTab::Startup,
                system::SysTab::Users,
                system::SysTab::Services,
            ][tour.visits / pages.len() % 6];
            self.live.tab = [live::Tab::Live, live::Tab::Overlay, live::Tab::Sessions, live::Tab::Bottleneck]
                [tour.visits / pages.len() % 4];
        }
        if every(&tour.capture, 9 * 60) {
            tour.capture = Instant::now();
            self.live.soak_capture(&mut self.mon, 60);
        }
        if every(&tour.analysis, 11 * 60) {
            tour.analysis = Instant::now();
            self.live.detector.length = Some(60);
            self.live.detector.start();
        }
        if every(&tour.overlay, 30 * 60) {
            tour.overlay = Instant::now();
            let on = !self.mon.settings.overlay.shown;
            self.mon.set_setting(move |s| s.overlay.shown = on);
        }
        if tour.started.elapsed().as_secs() % 600 == 0 {
            ectx.request_repaint();
        }
    }
}

//! Tools page (compare, clocks, timers, reports).

#[allow(unused_imports)]
use crate::gui::*;

impl App {
    pub(crate) fn tools_page(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        widgets::page_header(ui, tr("Tools"), "", |_| {});
        widgets::segmented(
            ui,
            &mut self.tool_view,
            &[
                (ToolView::Compare, icon::GIT_DIFF, tr("Compare reports")),
                (ToolView::Clocks, icon::SPEEDOMETER, tr("Clocks")),
                (ToolView::Timers, icon::TIMER, tr("Timers")),
                (ToolView::Reports, icon::EXPORT, tr("Reports")),
            ],
        );
        match self.tool_view {
            ToolView::Compare => self.compare.page(ui, Some(r)),
            ToolView::Clocks => self.clocks_body(ui),
            ToolView::Timers => timers_body(ui, r),
            ToolView::Reports => {
                widgets::card(ui, icon::FILE_TEXT, tr("Save a report"), |ui| {
                    ui.label(
                        RichText::new(tr("Same format as CPU-Z's -txt / -html reports, or our JSON."))
                            .color(theme::palette(ui).text_secondary),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        let rep = Some(r.to_vec());
                        if widgets::primary_button(ui, &format!("{} {}", icon::FLOPPY_DISK, tr("Save .TXT"))).clicked()
                        {
                            self.save(&rep, "txt");
                        }
                        if ui.button(tr("Save .HTML")).clicked() {
                            self.save(&rep, "html");
                        }
                        if ui.button(tr("Save .cvf (JSON)")).clicked() {
                            self.save(&rep, "cvf");
                        }
                    });
                });
            }
        }
    }

    pub(crate) fn clocks_body(&mut self, ui: &mut egui::Ui) {
        let c = crate::cpu::get(&self.ctx);
        let tel = self.mon.tel();
        tel.want("clocks", &["cpu/clock", "gpu"]);
        let snap = tel.snapshot();
        let mut rows: Vec<(String, String)> = Vec::new();
        for (core, l) in c.logical.iter().filter(|l| l.smt == 0).enumerate() {
            let kind = l.kind.as_deref().map(|k| format!(" ({k})")).unwrap_or_default();
            let v = crate::telemetry::core_mhz(&snap, core).map_or("…".into(), |m| units::clock(m, false, 1));
            rows.push((format!("{}{kind}", trf("Core #{}", &[&core])), v));
        }
        let names = crate::gpu::adapters(&self.ctx);
        let mut gpus: Vec<(String, String)> = Vec::new();
        for g in &tel.gpu_live() {
            let name = names.get(g.adapter).map_or("GPU", |a| a.name.as_str());
            let opt = |v: Option<f64>, unit: &str| v.map_or(String::new(), |v| format!("{v:.0} {unit}"));
            let mut s: Vec<String> = [
                opt(g.core_mhz, "MHz core"),
                opt(g.mem_mhz, "MHz mem"),
                opt(g.temp_c, "°C"),
                opt(g.hotspot_c, &format!("°C {}", tr("hot spot"))),
                opt(g.fan_rpm, "RPM"),
                opt(g.power_w, "W"),
                opt(g.load_pct, "%"),
            ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
            if g.throttling() {
                s.push(tr("throttling").into());
            }
            gpus.push((name.to_string(), s.join(", ")));
        }
        let source = crate::cpu::clocks::live_source(&self.ctx).label();
        let refs: Vec<(&str, String)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        widgets::card(ui, icon::CPU, tr("Cores"), |ui| {
            ui.label(RichText::new(trf("source: {}", &[&source])).small().color(theme::palette(ui).text_muted));
            widgets::kv_list(ui, &refs);
        });
        if !gpus.is_empty() {
            ui.add_space(theme::GAP);
            let refs: Vec<(&str, String)> = gpus.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            widgets::card(ui, icon::GRAPHICS_CARD, tr("Graphics"), |ui| widgets::kv_list(ui, &refs));
        }
    }
}

pub(crate) fn timers_body(ui: &mut egui::Ui, r: &[Section]) {
    let rows: Vec<(String, String)> = sec(r, "Timers")
        .map(|s| report::fields(&s.nodes).map(|(k, v)| (k.to_string(), v.to_string())).collect())
        .unwrap_or_default();
    widgets::card(ui, icon::TIMER, tr("Timers"), |ui| {
        if rows.is_empty() {
            ui.label(tr("No timer information."));
        }
        let refs: Vec<(&str, String)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        widgets::kv_list(ui, &refs);
    });
}

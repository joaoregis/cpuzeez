//! Graphics page.

#[allow(unused_imports)]
use crate::gui::*;

impl App {
    pub(crate) fn graphics_tab(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let gpus: Vec<(&str, &[Node])> =
            sec(r, "Display Adapters").map(|s| report::groups(&s.nodes, "Display adapter")).unwrap_or_default();
        if gpus.is_empty() {
            ui.label(no_data(ui, "graphics", &self.ctx));
            return;
        }
        self.gpu = self.gpu.min(gpus.len() - 1);
        ui.horizontal(|ui| {
            ui.label(tr("Display Device Selection"));
            egui::ComboBox::from_id_salt("gpu").selected_text(gpu_name(gpus[self.gpu].1)).show_ui(ui, |ui| {
                for (i, (_, n)) in gpus.iter().enumerate() {
                    ui.selectable_value(&mut self.gpu, i, gpu_name(n));
                }
            });
        });
        let rows: Vec<(&str, String)> = report::fields(gpus[self.gpu].1).map(|(k, v)| (k, v.to_string())).collect();
        boxed(ui, "GPU", &rows);
        for (_, perf) in report::groups(gpus[self.gpu].1, "Performance Level") {
            let rows: Vec<(&str, String)> = report::fields(perf).map(|(k, v)| (k, v.to_string())).collect();
            boxed(ui, "Clocks", &rows);
        }
    }
}

pub(crate) fn gpu_name(n: &[Node]) -> String {
    report::find(n, "Name").unwrap_or("GPU").to_string()
}

//! Memory page (SPD + memory controller).

#[allow(unused_imports)]
use crate::gui::*;

impl App {
    pub(crate) fn spd_tab(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        // Empty "DIMM #" groups are the headers of the raw dumps at the end.
        let mut dimms: Vec<(&str, &[Node])> =
            sec(r, "Memory SPD").map(|s| report::groups(&s.nodes, "DIMM #")).unwrap_or_default();
        dimms.retain(|(_, c)| !c.is_empty());
        if dimms.is_empty() {
            ui.label(no_data(ui, "SPD", &self.ctx));
            return;
        }
        self.slot = self.slot.min(dimms.len() - 1);
        ui.horizontal(|ui| {
            ui.label(tr("Memory Slot Selection"));
            egui::ComboBox::from_id_salt("slot").selected_text(trf("Slot #{}", &[&dimms[self.slot].0])).show_ui(
                ui,
                |ui| {
                    for (i, (v, _)) in dimms.iter().enumerate() {
                        ui.selectable_value(&mut self.slot, i, trf("Slot #{}", &[v]));
                    }
                },
            );
        });
        let rows: Vec<(&str, String)> = report::fields(dimms[self.slot].1).map(|(k, v)| (k, v.to_string())).collect();
        boxed(ui, "Module", &rows);
        // CPU-Z style timings table for the selected module: one column per
        // JEDEC bin / XMP / EXPO profile.
        let mods = crate::spd::modules(&self.ctx);
        let Some(m) = mods.get(self.slot) else { return };
        let cols: Vec<(String, &crate::spd::decode::Timing)> = m
            .jedec
            .iter()
            .enumerate()
            .map(|(i, t)| (trf("JEDEC #{}", &[&(i + 1)]), t))
            .chain(
                m.profiles
                    .iter()
                    .map(|p| (if p.name.is_empty() { p.kind.to_string() } else { p.name.clone() }, &p.timing)),
            )
            .collect();
        ui.group(|ui| {
            ui.label(RichText::new(tr("Timings Table")).strong());
            egui::ScrollArea::horizontal().show(ui, |ui| {
                egui::Grid::new("spd-timings").striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
                    ui.label("");
                    for (h, _) in &cols {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    type Cell = fn(&crate::spd::decode::Timing) -> String;
                    let rows: [(&str, Cell); 8] = [
                        ("Frequency", |t| format!("{:.0} MHz", t.freq)),
                        ("CAS# Latency", |t| format!("{:.1}", t.cl)),
                        ("RAS# to CAS#", |t| t.rcd.to_string()),
                        ("RAS# Precharge", |t| t.rp.to_string()),
                        ("tRAS", |t| t.ras.to_string()),
                        ("tRC", |t| t.rc.to_string()),
                        ("Command Rate", |t| t.cr.map_or("-".into(), |c| format!("{c}T"))),
                        ("Voltage", |t| t.volts.map_or("-".into(), |v| format!("{v:.3} V"))),
                    ];
                    for (name, cell) in rows {
                        ui.label(tr(name));
                        for (_, t) in &cols {
                            value(ui, &cell(t));
                        }
                        ui.end_row();
                    }
                });
            });
        });
    }
}

pub(crate) fn memory_tab(ui: &mut egui::Ui, r: &[Section]) {
    let k = |key: &str| sget(r, "Chipset", key);
    boxed(
        ui,
        "General",
        &[
            ("Type", k("Memory Type")),
            ("Size", k("Memory Size")),
            ("Channel #", k("Channels")),
            ("Uncore Frequency", k("Uncore Frequency")),
        ],
    );
    boxed(
        ui,
        "Timings",
        &[
            ("DRAM Frequency", k("Memory Frequency")),
            ("CAS# Latency (CL)", k("CAS# latency (CL)")),
            ("RAS# to CAS# Delay (tRCD)", k("RAS# to CAS# delay (tRCD)")),
            ("RAS# Precharge (tRP)", k("RAS# Precharge (tRP)")),
            ("Cycle Time (tRAS)", k("Cycle Time (tRAS)")),
            ("Bank Cycle Time (tRC)", k("Bank Cycle Time (tRC)")),
            ("Command Rate (CR)", k("Command Rate (CR)")),
            ("Row Refresh Cycle Time (tRFC)", k("Row Refresh Cycle Time (tRFC)")),
        ],
    );
    #[cfg(target_os = "macos")]
    if k("CAS# latency (CL)").is_empty() {
        let muted = theme::palette(ui).text_muted;
        ui.label(
            RichText::new(tr("macOS does not expose live DRAM timings. Dashes indicate unavailable fields."))
                .color(muted),
        );
    }
}

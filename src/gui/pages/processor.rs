//! Processor page (CPU + caches).

#[allow(unused_imports)]
use crate::gui::widgets::Control;
use crate::gui::*;

impl App {
    pub(crate) fn cpu_tab(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let c = crate::cpu::get(&self.ctx);
        let pi = |k: &str| sget(r, "Processors Information", k);
        let (fam, model, step) = c.sig;
        let tel = self.mon.tel();
        tel.want("processor", &["cpu/clock"]);
        let core_mhz = crate::telemetry::core_mhz(&tel.snapshot(), self.core);
        let bus = crate::cpu::bus_mhz(c);
        boxed(
            ui,
            "Processor",
            &[
                ("Name", c.name.clone()),
                ("Code Name", c.info.codename.clone().unwrap_or_default()),
                ("Max TDP", c.info.tdp_w.map(|w| format!("{w:.1} W")).unwrap_or_default()),
                ("Package", c.info.package.clone().unwrap_or_default()),
                ("Technology", c.info.technology.clone().unwrap_or_default()),
                ("Specification", c.brand.clone()),
                (
                    "Family / Model / Stepping",
                    c.cpuid
                        .as_deref()
                        .map_or_else(|| format!("{fam:X} / {model:X} / {step:X}"), |s| s.replace('.', " / ")),
                ),
                ("Ext. Family / Model", c.ext_cpuid.clone().unwrap_or_default()),
                ("Revision", c.info.core_stepping.clone().unwrap_or_default()),
                ("Instructions", c.instructions.join(", ")),
                ("Microcode", c.microcode.clone().unwrap_or_default()),
            ],
        );
        ui.columns(2, |cols| {
            boxed(
                &mut cols[0],
                &trf("Clocks (Core #{})", &[&self.core]),
                &[
                    ("Core Speed", core_mhz.map(|m| units::clock(m, false, 2)).unwrap_or_else(|| pi("Core Speed"))),
                    ("Multiplier", core_mhz.zip(bus).map(|(m, b)| format!("x {:.1}", m / b)).unwrap_or_default()),
                    ("Bus Speed", bus.map(|b| units::clock(b, false, 2)).unwrap_or_default()),
                    ("Base", c.base_mhz.map(|b| units::clock(b, false, 0)).unwrap_or_default()),
                ],
            );
            let cache = |lvl: u32, kind: char| {
                c.caches
                    .iter()
                    .find(|x| x.level == lvl && (x.kind == kind || kind == 'U'))
                    .map(|x| format!("{} x {}", x.count, crate::cpu::size_text(x.size)))
                    .unwrap_or_default()
            };
            boxed(
                &mut cols[1],
                "Cache",
                &[
                    ("L1 Data", cache(1, 'D')),
                    ("L1 Inst.", cache(1, 'I')),
                    ("Level 2", cache(2, 'U')),
                    ("Level 3", cache(3, 'U')),
                ],
            );
        });
        ui.horizontal(|ui| {
            let l = ui.label(tr("Selection")).id;
            let n = c.cores.max(1);
            egui::ComboBox::from_id_salt("core")
                .selected_text(trf("Core #{}", &[&self.core]))
                .show_ui(ui, |ui| {
                    for i in 0..n {
                        ui.selectable_value(&mut self.core, i, trf("Core #{}", &[&i]));
                    }
                })
                .name_by(l);
            ui.label(trf("Cores {}   Threads {}", &[&c.cores, &c.threads]));
        });
        for cl in &c.clusters {
            ui.label(trf("{}: {} cores, {} threads", &[&cl.kind, &cl.cores, &cl.threads]));
        }
    }

    pub(crate) fn caches_tab(&mut self, ui: &mut egui::Ui) {
        let c = crate::cpu::get(&self.ctx);
        for ca in &c.caches {
            let title = match (ca.level, ca.kind) {
                (1, 'D') => tr("L1 D-Cache").to_string(),
                (1, 'I') => tr("L1 I-Cache").to_string(),
                (l, _) => trf("Level {}", &[&l]),
            };
            let ways = if ca.ways == 0 {
                tr("fully associative").to_string()
            } else {
                trf("{}-way set associative", &[&ca.ways])
            };
            let mut rows = vec![
                ("Size", format!("{} x {}", ca.count, crate::cpu::size_text(ca.size))),
                ("Descriptor", trf("{}, {}-byte line size", &[&ways, &ca.line])),
                ("Shared by", trf("{} thread(s)", &[&ca.sharing])),
            ];
            if let Some(n) = &ca.note {
                rows.push(("Core set", n.clone()));
            }
            boxed(ui, &title, &rows);
        }
    }
}

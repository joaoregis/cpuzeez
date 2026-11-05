//! Benchmarks › Storage (S1-K06).

use super::i18n::{tr, trf};
use super::theme::{self, icon};
use super::widgets;
use crate::Ctx;
use crate::diskbench::{Run, Score, TESTS};
use eframe::egui::{self, RichText};
use std::path::PathBuf;
use std::sync::atomic::Ordering;

pub const PT_BR: &[(&str, &str)] = &[
    ("Storage", "Armazenamento"),
    ("Where", "Onde"),
    ("App data folder", "Pasta de dados do app"),
    ("Test file", "Arquivo de teste"),
    ("Run", "Executar"),
    ("Cancel", "Cancelar"),
    ("Read", "Leitura"),
    ("Write", "Gravação"),
    (
        "Writes a temporary file (deleted afterwards) and bypasses the OS cache. T = requests in flight. SSDs wear a tiny amount with every write.",
        "Grava um arquivo temporário (apagado depois) e ignora o cache do sistema. T = requisições simultâneas. SSDs se desgastam um pouco a cada gravação.",
    ),
    ("Stopped: {}", "Interrompido: {}"),
];

pub const ES: &[(&str, &str)] = &[
    ("Storage", "Almacenamiento"),
    ("Where", "Dónde"),
    ("App data folder", "Carpeta de datos de la app"),
    ("Test file", "Archivo de prueba"),
    ("Run", "Ejecutar"),
    ("Cancel", "Cancelar"),
    ("Read", "Lectura"),
    ("Write", "Escritura"),
    (
        "Writes a temporary file (deleted afterwards) and bypasses the OS cache. T = requests in flight. SSDs wear a tiny amount with every write.",
        "Escribe un archivo temporal (se borra después) y evita la caché del sistema. T = peticiones simultáneas. Los SSD se desgastan un poco con cada escritura.",
    ),
    ("Stopped: {}", "Detenido: {}"),
];

const SIZES: [(u64, &str); 3] = [(256 << 20, "256 MiB"), (1 << 30, "1 GiB"), (4 << 30, "4 GiB")];

pub struct State {
    dir: Option<PathBuf>,
    size: u64,
    run: Option<Run>,
    scores: Vec<Score>,
    error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        State { dir: None, size: 1 << 30, run: None, scores: Vec::new(), error: None }
    }
}

impl State {
    pub fn page(&mut self, ui: &mut egui::Ui, ctx: &Ctx) {
        let p = theme::palette(ui);
        let finished = self.run.as_ref().and_then(|run| run.result.lock().unwrap_or_else(|e| e.into_inner()).take());
        if let Some(r) = finished {
            match r {
                Ok(s) => self.scores = s,
                Err(e) => self.error = Some(e),
            }
            self.run = None;
        }
        // The system drive's root needs admin: there, use the app's data folder.
        let data = crate::settings::dir();
        let mut places: Vec<(Option<PathBuf>, String)> = vec![(None, tr("App data folder").to_string())];
        for d in crate::storage::drives(ctx).iter().filter(|d| !d.removable) {
            for v in &d.volumes {
                places.push((Some(PathBuf::from(&v.name)), format!("{} · {}", v.name, d.name)));
            }
        }
        widgets::card(ui, icon::HARD_DRIVES, tr("Storage"), |ui| {
            ui.label(RichText::new(tr("Writes a temporary file (deleted afterwards) and bypasses the OS cache. T = requests in flight. SSDs wear a tiny amount with every write.")).color(p.text_secondary));
            ui.add_space(4.0);
            ui.add_enabled_ui(self.run.is_none(), |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("Where"));
                    let shown = places
                        .iter()
                        .find(|x| x.0 == self.dir)
                        .map_or_else(|| tr("App data folder").to_string(), |x| x.1.clone());
                    egui::ComboBox::from_id_salt("disk-where").selected_text(shown).width(320.0).show_ui(ui, |ui| {
                        for (path, label) in &places {
                            ui.selectable_value(&mut self.dir, path.clone(), label);
                        }
                    });
                    ui.label(tr("Test file"));
                    for (s, l) in SIZES {
                        ui.selectable_value(&mut self.size, s, l);
                    }
                });
            });
            ui.add_space(4.0);
            match &self.run {
                Some(run) => {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    let (stage, f) = run.stage.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    ui.horizontal(|ui| {
                        widgets::progress(ui, f, &stage);
                        if ui.button(tr("Cancel")).clicked() {
                            run.cancel.store(true, Ordering::Relaxed);
                        }
                    });
                }
                None => {
                    if widgets::primary_button(ui, &format!("{}  {}", icon::PLAY, tr("Run"))).clicked()
                        && let Some(dir) = self.dir.clone().or(data.clone())
                    {
                        self.error = None;
                        self.run = Some(crate::diskbench::start(&dir, self.size, 5.0));
                    }
                }
            }
            if let Some(e) = &self.error {
                ui.label(RichText::new(trf("Stopped: {}", &[e])).color(p.danger));
            }
        });
        ui.add_space(theme::GAP);
        if !self.scores.is_empty() {
            widgets::card(ui, "", "", |ui| {
                egui::Grid::new("disk-scores").striped(true).spacing([24.0, 6.0]).show(ui, |ui| {
                    ui.label("");
                    ui.label(RichText::new(tr("Read")).strong());
                    ui.label(RichText::new(tr("Write")).strong());
                    ui.end_row();
                    for s in &self.scores {
                        let random = TESTS.iter().find(|t| t.name == s.name).is_some_and(|t| t.random);
                        ui.label(RichText::new(s.name).monospace().color(p.text));
                        for (mbps, iops) in [(s.read_mbps, s.read_iops), (s.write_mbps, s.write_iops)] {
                            let t = if random {
                                format!("{mbps:.1} MB/s · {iops:.0} IOPS")
                            } else {
                                format!("{mbps:.0} MB/s")
                            };
                            ui.label(RichText::new(t).monospace());
                        }
                        ui.end_row();
                    }
                });
            });
        }
    }
}

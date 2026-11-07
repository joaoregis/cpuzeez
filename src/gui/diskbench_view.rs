//! Benchmarks › Storage (S1-K06).

use super::benchkit_view as kit;
use super::i18n::{tr, trf};
use super::theme::{self, icon};
use super::widgets;
use crate::Ctx;
use crate::diskbench::{Run, Score, TESTS};
use eframe::egui::{self, RichText};
use std::path::PathBuf;

pub const PT_BR: &[(&str, &str)] = &[
    ("Sustained write", "Gravação sustentada"),
    (
        "Writes continuously until the size below is reached. Fast SSDs write into a cache first; the curve shows where it runs out. The file is deleted afterwards.",
        "Grava sem parar até atingir o tamanho abaixo. SSDs rápidos gravam primeiro num cache; a curva mostra onde ele acaba. O arquivo é apagado depois.",
    ),
    (
        "This writes {} GB to the drive. SSDs wear a little with every write.",
        "Isto grava {} GB no disco. SSDs se desgastam um pouco a cada gravação.",
    ),
    ("Start", "Início"),
    ("End", "Fim"),
    ("Average", "Média"),
    ("Cache", "Cache"),
    ("Storage sustained", "Armazenamento sustentado"),
    ("Storage", "Armazenamento"),
    ("Where", "Onde"),
    ("App data folder", "Pasta de dados do app"),
    ("Test file", "Arquivo de teste"),
    ("Run", "Executar"),
    ("Read", "Leitura"),
    ("Write", "Gravação"),
    (
        "Writes a temporary file (deleted afterwards) and bypasses the OS cache. T = requests in flight. SSDs wear a tiny amount with every write.",
        "Grava um arquivo temporário (apagado depois) e ignora o cache do sistema. T = requisições simultâneas. SSDs se desgastam um pouco a cada gravação.",
    ),
    ("Stopped: {}", "Interrompido: {}"),
];

pub const ES: &[(&str, &str)] = &[
    ("Sustained write", "Escritura sostenida"),
    (
        "Writes continuously until the size below is reached. Fast SSDs write into a cache first; the curve shows where it runs out. The file is deleted afterwards.",
        "Escribe sin parar hasta alcanzar el tamaño de abajo. Los SSD rápidos escriben primero en una caché; la curva muestra dónde se agota. El archivo se borra después.",
    ),
    (
        "This writes {} GB to the drive. SSDs wear a little with every write.",
        "Esto escribe {} GB en la unidad. Los SSD se desgastan un poco con cada escritura.",
    ),
    ("Start", "Inicio"),
    ("End", "Fin"),
    ("Average", "Promedio"),
    ("Cache", "Caché"),
    ("Storage sustained", "Almacenamiento sostenido"),
    ("Storage", "Almacenamiento"),
    ("Where", "Dónde"),
    ("App data folder", "Carpeta de datos de la app"),
    ("Test file", "Archivo de prueba"),
    ("Run", "Ejecutar"),
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
    conditions: Option<crate::benchkit::Conditions>,
    saved: Option<kit::Saved>,
    label: String,
    status: String,
    sus_gb: u64,
    sus: Option<crate::benchkit::Job<Vec<(f64, f64)>>>,
    sus_result: Option<kit::Saved>,
    sus_error: Option<String>,
    sus_label: String,
    sus_status: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            dir: None,
            size: 1 << 30,
            run: None,
            scores: Vec::new(),
            error: None,
            conditions: None,
            saved: None,
            label: String::new(),
            status: String::new(),
            sus_gb: 32,
            sus: None,
            sus_result: None,
            sus_error: None,
            sus_label: String::new(),
            sus_status: String::new(),
        }
    }
}

impl State {
    pub fn page(&mut self, ui: &mut egui::Ui, ctx: &std::sync::Arc<Ctx>, start_temp: Option<f64>) {
        let p = theme::palette(ui);
        // The system drive's root needs admin: there, use the app's data folder.
        let data = crate::settings::dir();
        let mut places: Vec<(Option<PathBuf>, String)> = vec![(None, tr("App data folder").to_string())];
        for d in super::cache::drives(ctx).iter().filter(|d| !d.removable) {
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
            if let Some(r) = kit::poll(ui, &mut self.run) {
                match r {
                    Ok(s) => {
                        let subs = Score::subs(&s);
                        self.saved = Some(kit::Saved {
                            kind: "Storage",
                            score: subs.first().map_or(0.0, |x| x.value),
                            unit: "MB/s",
                            variant: format!("{} MiB", self.size >> 20),
                            threads: 8,
                            subs,
                            conditions: self.conditions.take().unwrap_or_default(),
                            series: Vec::new(),
                        });
                        self.scores = s;
                        self.status.clear();
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            if self.run.is_none()
                && widgets::primary_button(ui, &format!("{}  {}", icon::PLAY, tr("Run"))).clicked()
                && let Some(dir) = self.dir.clone().or(data.clone())
            {
                self.error = None;
                self.conditions = Some(crate::benchkit::conditions(start_temp, 1));
                self.run = Some(crate::diskbench::start(&dir, self.size, 5.0));
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
            ui.add_space(theme::GAP);
            if let Some(r) = &self.saved {
                kit::save(ui, ctx, r, &mut self.label, &mut self.status);
            }
        }
        ui.add_space(theme::GAP);
        self.sustained(ui, ctx, start_temp, data);
    }

    /// Sustained write (S1-K06): how long the drive keeps its speed.
    fn sustained(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &std::sync::Arc<Ctx>,
        start_temp: Option<f64>,
        data: Option<PathBuf>,
    ) {
        let p = theme::palette(ui);
        let dir = self.dir.clone().or(data);
        // Free space of the volume holding `dir` (longest matching root).
        let free = dir.as_ref().and_then(|d| {
            let d = d.to_string_lossy().to_lowercase();
            super::cache::drives(ctx)
                .iter()
                .flat_map(|x| x.volumes.iter())
                .filter(|v| !v.name.is_empty() && d.starts_with(&v.name.to_lowercase()))
                .max_by_key(|v| v.name.len())
                .map(|v| v.free)
        });
        widgets::card(ui, icon::TRAY_ARROW_DOWN, tr("Sustained write"), |ui| {
            ui.label(RichText::new(tr("Writes continuously until the size below is reached. Fast SSDs write into a cache first; the curve shows where it runs out. The file is deleted afterwards.")).color(p.text_secondary));
            ui.add_enabled_ui(self.sus.is_none(), |ui| {
                ui.horizontal(|ui| {
                    for gb in [8u64, 32, 64] {
                        let fits = free.is_some_and(|f| gb << 30 < f / 10 * 8);
                        ui.add_enabled_ui(fits, |ui| ui.selectable_value(&mut self.sus_gb, gb, format!("{gb} GB")));
                    }
                });
            });
            let fits = free.is_some_and(|f| self.sus_gb << 30 < f / 10 * 8);
            ui.label(
                RichText::new(trf(
                    "This writes {} GB to the drive. SSDs wear a little with every write.",
                    &[&self.sus_gb],
                ))
                .color(p.warning),
            );
            if let Some(r) = kit::poll(ui, &mut self.sus) {
                match r {
                    Ok(series) if !series.is_empty() => {
                        let subs = crate::diskbench::sustained_subs(&series);
                        self.sus_result = Some(kit::Saved {
                            kind: "Storage sustained",
                            score: subs.get(2).map_or(0.0, |x| x.value),
                            unit: "MB/s",
                            variant: format!("{} GB", self.sus_gb),
                            threads: 4,
                            subs,
                            conditions: crate::benchkit::conditions(start_temp, 1),
                            series,
                        });
                        self.sus_status.clear();
                    }
                    Ok(_) => {}
                    Err(e) => self.sus_error = Some(e),
                }
            }
            if self.sus.is_none()
                && self.run.is_none()
                && let Some(dir) = dir.clone()
                && ui.add_enabled(fits, egui::Button::new(format!("{}  {}", icon::PLAY, tr("Run")))).clicked()
            {
                self.sus_error = None;
                self.sus_result = None;
                let bytes = self.sus_gb << 30;
                self.sus = Some(crate::benchkit::Job::spawn(move |ctl| crate::diskbench::sustained(&dir, bytes, ctl)));
            }
            if let Some(e) = &self.sus_error {
                ui.label(RichText::new(trf("Stopped: {}", &[e])).color(p.danger));
            }
        });
        if let Some(r) = &self.sus_result {
            ui.add_space(theme::GAP);
            widgets::card(ui, "", "", |ui| {
                let s = [super::tools::Series { name: String::new(), color: p.series[0], points: r.series.clone() }];
                super::tools::line_chart(ui, &s, false, &|x| format!("{x:.0} s"), &|y| format!("{y:.0} MB/s"));
                kit::table(ui, "sustained", &r.subs);
            });
            ui.add_space(theme::GAP);
            kit::save(ui, ctx, r, &mut self.sus_label, &mut self.sus_status);
        }
    }
}

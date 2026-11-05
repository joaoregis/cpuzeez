//! Tools › Drive health (S1-K08): health panel (life left, written vs rated
//! TBW, temperature over time, error counts, alerts), SMART self-tests with
//! progress and history, and a read-only surface scan.

use super::i18n::{tr, trf};
use super::plot;
use super::theme::{self, icon};
use super::widgets;
use crate::Ctx;
use crate::benchkit::Job;
use crate::drivetest::{self, BUCKETS, Scan, Status};
use crate::storage::Drive;
use eframe::egui::{self, RichText};
use std::time::{Duration, Instant};

pub const PT_BR: &[(&str, &str)] = &[
    ("Drive health", "Saúde dos discos"),
    ("Drive", "Disco"),
    ("Life left", "Vida restante"),
    ("Written", "Gravado"),
    ("Rated endurance (TBW)", "Resistência nominal (TBW)"),
    ("{} of the rated endurance", "{} da resistência nominal"),
    ("From the drive's datasheet; drives don't report it.", "Da ficha técnica do disco; os discos não informam isso."),
    ("Power-on time", "Tempo ligado"),
    ("Temperature", "Temperatura"),
    ("Media errors", "Erros de mídia"),
    ("Reallocated sectors", "Setores realocados"),
    ("Pending sectors", "Setores pendentes"),
    ("Uncorrectable sectors", "Setores incorrigíveis"),
    ("Unsafe shutdowns", "Desligamentos inseguros"),
    ("No SMART data for this drive.", "Sem dados SMART para este disco."),
    ("Self-test", "Autoteste"),
    ("Short", "Curto"),
    ("Extended", "Estendido"),
    ("Conveyance", "Transporte"),
    ("Abort", "Abortar"),
    ("Running: {} · {}%", "Em andamento: {} · {}%"),
    ("History", "Histórico"),
    ("No self-tests recorded.", "Nenhum autoteste registrado."),
    ("{} h", "{} h"),
    (
        "The drive tests itself (short: ~2 min, extended: up to hours) and keeps working meanwhile. Needs administrator rights.",
        "O disco testa a si mesmo (curto: ~2 min, estendido: até horas) e continua funcionando enquanto isso. Requer direitos de administrador.",
    ),
    ("Self-tests aren't available for this drive here.", "Autotestes não estão disponíveis para este disco aqui."),
    ("Surface scan", "Varredura de superfície"),
    (
        "Reads the whole drive (or a sample spread over it) and times every 1 MiB block. Read-only: nothing is written. Needs administrator rights.",
        "Lê o disco inteiro (ou uma amostra espalhada por ele) e mede cada bloco de 1 MiB. Só leitura: nada é gravado. Requer direitos de administrador.",
    ),
    ("Sample (4096 blocks)", "Amostra (4096 blocos)"),
    ("Whole drive", "Disco inteiro"),
    ("Scan", "Varrer"),
    ("Unreadable blocks: {}", "Blocos ilegíveis: {}"),
    ("{} blocks · {} MB/s", "{} blocos · {} MB/s"),
    ("Stopped: {}", "Interrompido: {}"),
    ("Temperature history", "Histórico de temperatura"),
];

pub const ES: &[(&str, &str)] = &[
    ("Drive health", "Salud de las unidades"),
    ("Drive", "Unidad"),
    ("Life left", "Vida restante"),
    ("Written", "Escrito"),
    ("Rated endurance (TBW)", "Resistencia nominal (TBW)"),
    ("{} of the rated endurance", "{} de la resistencia nominal"),
    (
        "From the drive's datasheet; drives don't report it.",
        "De la hoja de datos de la unidad; las unidades no lo informan.",
    ),
    ("Power-on time", "Tiempo encendido"),
    ("Temperature", "Temperatura"),
    ("Media errors", "Errores de medio"),
    ("Reallocated sectors", "Sectores reasignados"),
    ("Pending sectors", "Sectores pendientes"),
    ("Uncorrectable sectors", "Sectores incorregibles"),
    ("Unsafe shutdowns", "Apagados inseguros"),
    ("No SMART data for this drive.", "Sin datos SMART para esta unidad."),
    ("Self-test", "Autoprueba"),
    ("Short", "Corta"),
    ("Extended", "Extendida"),
    ("Conveyance", "Transporte"),
    ("Abort", "Abortar"),
    ("Running: {} · {}%", "En curso: {} · {}%"),
    ("History", "Historial"),
    ("No self-tests recorded.", "No hay autopruebas registradas."),
    ("{} h", "{} h"),
    (
        "The drive tests itself (short: ~2 min, extended: up to hours) and keeps working meanwhile. Needs administrator rights.",
        "La unidad se prueba a sí misma (corta: ~2 min, extendida: hasta horas) y sigue funcionando mientras tanto. Requiere derechos de administrador.",
    ),
    ("Self-tests aren't available for this drive here.", "Las autopruebas no están disponibles para esta unidad aquí."),
    ("Surface scan", "Exploración de superficie"),
    (
        "Reads the whole drive (or a sample spread over it) and times every 1 MiB block. Read-only: nothing is written. Needs administrator rights.",
        "Lee toda la unidad (o una muestra repartida) y mide cada bloque de 1 MiB. Solo lectura: no se escribe nada. Requiere derechos de administrador.",
    ),
    ("Sample (4096 blocks)", "Muestra (4096 bloques)"),
    ("Whole drive", "Unidad completa"),
    ("Scan", "Explorar"),
    ("Unreadable blocks: {}", "Bloques ilegibles: {}"),
    ("{} blocks · {} MB/s", "{} bloques · {} MB/s"),
    ("Stopped: {}", "Detenido: {}"),
    ("Temperature history", "Historial de temperatura"),
];

pub struct State {
    pick: usize,
    status: Option<(Result<Status, String>, Instant)>,
    action: Option<Result<(), String>>,
    sample: bool,
    scan: Option<Job<Scan>>,
    scanned: Option<Result<Scan, String>>,
}

impl Default for State {
    fn default() -> Self {
        State { pick: 0, status: None, action: None, sample: true, scan: None, scanned: None }
    }
}

fn tb(bytes: u64) -> String {
    format!("{:.2} TB", bytes as f64 / 1e12)
}

impl State {
    pub fn page(&mut self, ui: &mut egui::Ui, ctx: &std::sync::Arc<Ctx>, mon: &mut super::sensors::Monitor) {
        let drives = super::cache::drives(ctx);
        if drives.is_empty() {
            widgets::empty_state(ui, icon::HARD_DRIVES, tr("Drive health"), tr("No SMART data for this drive."));
            return;
        }
        self.pick = self.pick.min(drives.len() - 1);
        let before = self.pick;
        ui.add_enabled_ui(self.scan.is_none(), |ui| {
            ui.horizontal(|ui| {
                ui.label(tr("Drive"));
                egui::ComboBox::from_id_salt("health-drive")
                    .selected_text(&drives[self.pick].name)
                    .width(360.0)
                    .show_ui(ui, |ui| {
                        for (i, d) in drives.iter().enumerate() {
                            ui.selectable_value(&mut self.pick, i, &d.name);
                        }
                    });
            });
        });
        if self.pick != before {
            (self.status, self.action, self.scanned) = (None, None, None);
        }
        let d = &drives[self.pick];
        ui.add_space(theme::GAP / 2.0);
        self.health(ui, d, mon);
        ui.add_space(theme::GAP);
        self.selftest(ui, d);
        ui.add_space(theme::GAP);
        self.surface(ui, d);
    }

    fn health(&mut self, ui: &mut egui::Ui, d: &Drive, mon: &mut super::sensors::Monitor) {
        let p = theme::palette(ui);
        widgets::card(ui, icon::HEARTBEAT, tr("Drive health"), |ui| {
            let Some(s) = &d.smart else {
                ui.label(RichText::new(tr("No SMART data for this drive.")).color(p.text_muted));
                return;
            };
            let mut rows: Vec<(&str, String)> = Vec::new();
            if let Some(u) = s.percent_used {
                rows.push(("Life left", format!("{}%", 100u32.saturating_sub(u))));
            }
            if let Some(w) = s.data_written_bytes {
                rows.push(("Written", tb(w)));
            }
            if let Some(h) = s.power_on_hours {
                rows.push(("Power-on time", trf("{} h", &[&h])));
            }
            if let Some(t) = s.temperature_c {
                rows.push(("Temperature", super::units::temp(t, 0)));
            }
            for (k, v) in [
                ("Media errors", s.media_errors),
                ("Reallocated sectors", s.reallocated_sectors),
                ("Pending sectors", s.pending_sectors),
                ("Uncorrectable sectors", s.uncorrectable_sectors),
                ("Unsafe shutdowns", s.unsafe_shutdowns),
            ] {
                if let Some(v) = v {
                    rows.push((k, v.to_string()));
                }
            }
            let rows: Vec<(&str, String)> = rows.into_iter().map(|(k, v)| (tr(k), v)).collect();
            widgets::kv_list(ui, &rows);
            for r in &s.reasons {
                ui.label(RichText::new(format!("{}  {r}", icon::WARNING)).color(p.warning));
            }
            // Rated TBW: typed by the user, kept per serial number.
            let key = d.serial.clone().unwrap_or_else(|| d.name.clone());
            let mut tbw = mon.settings.tbw.get(&key).copied().unwrap_or(0.0);
            ui.horizontal(|ui| {
                ui.label(tr("Rated endurance (TBW)"));
                if ui.add(egui::DragValue::new(&mut tbw).range(0.0..=100_000.0).suffix(" TB")).changed() {
                    let k = key.clone();
                    mon.set_setting(move |st| {
                        if tbw > 0.0 {
                            st.tbw.insert(k.clone(), tbw);
                        } else {
                            st.tbw.remove(&k);
                        }
                    });
                }
                match s.data_written_bytes.filter(|_| tbw > 0.0) {
                    Some(w) => {
                        let pct = w as f64 / 1e12 / tbw * 100.0;
                        let c = if pct >= 90.0 {
                            p.danger
                        } else if pct >= 70.0 {
                            p.warning
                        } else {
                            p.text
                        };
                        ui.label(RichText::new(trf("{} of the rated endurance", &[&format!("{pct:.1}%")])).color(c));
                    }
                    None => {
                        ui.label(
                            RichText::new(tr("From the drive's datasheet; drives don't report it."))
                                .small()
                                .color(p.text_muted),
                        );
                    }
                }
            });
        });
        // Temperature over time from the sampler (`storage/temp/diskN`).
        let tel = mon.tel();
        tel.want("drive-health", &["storage/"]);
        let series = tel.series_with(|id| id.starts_with("storage/temp/"));
        if let Some(s) = series.iter().find(|s| s.label == d.name) {
            ui.add_space(theme::GAP);
            widgets::card(ui, icon::THERMOMETER, tr("Temperature history"), |ui| {
                let now = s.points.last().map_or(0.0, |x| x.0);
                let line = [plot::Line { label: &s.label, color: p.series[0], points: &s.points }];
                plot::chart(ui, &line, now, mon.settings.graph_seconds as f64, 90.0, &|v| super::units::temp(v, 0));
            });
        }
    }

    fn selftest(&mut self, ui: &mut egui::Ui, d: &Drive) {
        let p = theme::palette(ui);
        let kinds = drivetest::kinds(d);
        widgets::card(ui, icon::STETHOSCOPE, tr("Self-test"), |ui| {
            if kinds.is_empty() {
                ui.label(RichText::new(tr("Self-tests aren't available for this drive here.")).color(p.text_muted));
                return;
            }
            ui.label(RichText::new(tr("The drive tests itself (short: ~2 min, extended: up to hours) and keeps working meanwhile. Needs administrator rights.")).color(p.text_secondary));
            // Refresh every 2 s while a test runs, else every 30 s.
            let running = matches!(&self.status, Some((Ok(s), _)) if s.running.is_some());
            let every = if running { 2 } else { 30 };
            if self.status.as_ref().is_none_or(|(_, t)| t.elapsed() > Duration::from_secs(every)) {
                self.status = Some((drivetest::status(d), Instant::now()));
            }
            ui.ctx().request_repaint_after(Duration::from_secs(every));
            ui.horizontal(|ui| {
                for k in &kinds {
                    if ui.add_enabled(!running, egui::Button::new(tr(k.label()))).clicked() {
                        self.action = Some(drivetest::start(d, Some(*k)));
                        self.status = None;
                    }
                }
                if running && ui.button(tr("Abort")).clicked() {
                    self.action = Some(drivetest::start(d, None));
                    self.status = None;
                }
            });
            if let Some(Err(e)) = &self.action {
                ui.label(RichText::new(trf("Stopped: {}", &[e])).color(p.danger));
            }
            match &self.status {
                Some((Ok(s), _)) => {
                    if let Some((k, pct)) = &s.running {
                        widgets::progress(ui, *pct as f32 / 100.0, &trf("Running: {} · {}%", &[&tr(k), pct]));
                    }
                    ui.add_space(4.0);
                    ui.label(RichText::new(tr("History")).strong());
                    if s.history.is_empty() {
                        ui.label(RichText::new(tr("No self-tests recorded.")).color(p.text_muted));
                    }
                    egui::Grid::new("selftests").striped(true).spacing([18.0, 4.0]).show(ui, |ui| {
                        for e in s.history.iter().take(10) {
                            ui.label(tr(&e.kind));
                            ui.label(RichText::new(&e.result).color(if e.ok { p.success } else { p.danger }));
                            ui.label(e.hours.map_or(String::new(), |h| trf("{} h", &[&h])));
                            ui.end_row();
                        }
                    });
                }
                Some((Err(e), _)) => {
                    ui.label(RichText::new(e).color(p.text_muted));
                }
                None => {}
            }
        });
    }

    fn surface(&mut self, ui: &mut egui::Ui, d: &Drive) {
        let p = theme::palette(ui);
        widgets::card(ui, icon::SCAN, tr("Surface scan"), |ui| {
            ui.label(RichText::new(tr("Reads the whole drive (or a sample spread over it) and times every 1 MiB block. Read-only: nothing is written. Needs administrator rights.")).color(p.text_secondary));
            ui.add_enabled_ui(self.scan.is_none(), |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.sample, true, tr("Sample (4096 blocks)"));
                    ui.selectable_value(&mut self.sample, false, tr("Whole drive"));
                });
            });
            if let Some(r) = super::benchkit_view::poll(ui, &mut self.scan) {
                self.scanned = Some(r);
            }
            if self.scan.is_none() && widgets::primary_button(ui, &format!("{}  {}", icon::PLAY, tr("Scan"))).clicked()
            {
                let (d, sample) = (d.clone(), self.sample.then_some(4096));
                self.scanned = None;
                self.scan = Some(Job::spawn(move |ctl| drivetest::scan(&d, sample, ctl)));
            }
            match &self.scanned {
                Some(Ok(s)) => {
                    ui.label(trf("{} blocks · {} MB/s", &[&s.blocks, &format!("{:.0}", s.mbps)]));
                    let labels: Vec<String> = BUCKETS
                        .iter()
                        .map(|b| format!("< {b:.0} ms"))
                        .chain(std::iter::once(format!("≥ {:.0} ms", BUCKETS[BUCKETS.len() - 1])))
                        .collect();
                    egui::Grid::new("scan-buckets").spacing([18.0, 3.0]).show(ui, |ui| {
                        let tones = [p.success, p.success, p.text, p.warning, p.warning, p.danger];
                        for ((l, n), c) in labels.iter().zip(s.buckets).zip(tones) {
                            ui.label(RichText::new(l).monospace());
                            ui.label(RichText::new(n.to_string()).monospace().color(c));
                            ui.end_row();
                        }
                    });
                    let c = if s.unreadable.is_empty() { p.success } else { p.danger };
                    ui.label(RichText::new(trf("Unreadable blocks: {}", &[&s.unreadable.len()])).color(c));
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(trf("Stopped: {}", &[e])).color(p.danger));
                }
                None => {}
            }
        });
    }
}

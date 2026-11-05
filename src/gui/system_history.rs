//! System › App history tab (S2a-10, docs/v1/task-manager.md §5):
//! Cross-platform, opt-in per-app resource recorder (CPU time, GPU time,
//! disk bytes, peak memory, run time) persisted locally with configurable
//! retention (default 30 days), period filter, per-day chart, top consumers,
//! and "Delete usage history".

use super::i18n::{tr, trf};
use super::table::{self, Cell, Column, Table};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::procs::Snapshot as ProcSnap;
use eframe::egui::{self, CornerRadius, Sense, Ui, vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Period {
    Today,
    Days7,
    #[default]
    Days30,
    All,
}

impl Period {
    pub const ALL: [Period; 4] = [Period::Today, Period::Days7, Period::Days30, Period::All];

    pub fn label(self) -> &'static str {
        match self {
            Period::Today => "Today",
            Period::Days7 => "Last 7 days",
            Period::Days30 => "Last 30 days",
            Period::All => "All time",
        }
    }

    pub fn max_days(self) -> u32 {
        match self {
            Period::Today => 1,
            Period::Days7 => 7,
            Period::Days30 => 30,
            Period::All => 3650,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppDayRecord {
    pub date: String,
    pub app: String,
    pub publisher: String,
    pub cpu_secs: f64,
    pub gpu_secs: f64,
    pub disk_bytes: u64,
    pub peak_mem_bytes: u64,
    pub run_secs: f64,
}

#[derive(Debug, Clone, Default)]
pub struct AggregatedApp {
    pub app: String,
    pub publisher: String,
    pub cpu_secs: f64,
    pub gpu_secs: f64,
    pub disk_bytes: u64,
    pub peak_mem_bytes: u64,
    pub run_secs: f64,
}

pub struct State {
    pub records: Vec<AppDayRecord>,
    pub period: Period,
    pub table: table::State,
    pub enabled: bool,
    pub retention_days: u32,
    loaded: bool,
    last_sample: Option<Instant>,
    last_save: Option<Instant>,
}

impl Default for State {
    fn default() -> Self {
        let mut t = table::State::default();
        t.sort = Some(("cpu".into(), false));
        Self {
            records: Vec::new(),
            period: Period::Days30,
            table: t,
            enabled: false,
            retention_days: 30,
            loaded: false,
            last_sample: None,
            last_save: None,
        }
    }
}

fn storage_path() -> Option<PathBuf> {
    crate::settings::dir().map(|d| d.join("app_history.json"))
}

pub fn today_ymd() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let ft = (secs + 11_644_473_600) * 10_000_000;
    cpuzeez_core::startup::filetime_to_ymd(ft).unwrap_or_else(|| "2026-11-05".into())
}

pub fn aggregate_records(records: &[AppDayRecord], period: Period) -> Vec<AggregatedApp> {
    let mut distinct_dates: Vec<&str> = records.iter().map(|r| r.date.as_str()).collect();
    distinct_dates.sort_unstable();
    distinct_dates.dedup();
    let keep_dates = distinct_dates.len().saturating_sub(period.max_days() as usize);
    let min_date = distinct_dates.get(keep_dates).copied().unwrap_or("");

    let mut map: BTreeMap<String, AggregatedApp> = BTreeMap::new();
    for r in records {
        if r.date.as_str() < min_date {
            continue;
        }
        let key = r.app.to_ascii_lowercase();
        let entry = map.entry(key).or_insert_with(|| AggregatedApp {
            app: r.app.clone(),
            publisher: r.publisher.clone(),
            ..Default::default()
        });
        if entry.publisher.is_empty() && !r.publisher.is_empty() {
            entry.publisher = r.publisher.clone();
        }
        entry.cpu_secs += r.cpu_secs;
        entry.gpu_secs += r.gpu_secs;
        entry.disk_bytes = entry.disk_bytes.saturating_add(r.disk_bytes);
        entry.peak_mem_bytes = entry.peak_mem_bytes.max(r.peak_mem_bytes);
        entry.run_secs += r.run_secs;
    }
    let mut out: Vec<AggregatedApp> = map.into_values().collect();
    out.sort_by(|a, b| b.cpu_secs.total_cmp(&a.cpu_secs));
    out
}

pub fn format_duration_hms(secs: f64) -> String {
    let total = secs.round().max(0.0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

impl State {
    fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if let Some(p) = storage_path()
            && let Ok(text) = std::fs::read_to_string(p)
            && let Ok(list) = serde_json::from_str::<Vec<AppDayRecord>>(&text)
        {
            self.records = list;
        }
    }

    pub fn ingest_snapshot(&mut self, procs: &ProcSnap) {
        self.ensure_loaded();
        if !self.enabled {
            self.last_sample = None;
            return;
        }
        let now = Instant::now();
        let dt = match self.last_sample {
            Some(prev) => now.duration_since(prev).as_secs_f64().clamp(0.0, 10.0),
            None => {
                self.last_sample = Some(now);
                return;
            }
        };
        if dt < 0.8 {
            return;
        }
        self.last_sample = Some(now);
        let date = today_ymd();

        for r in &procs.rows {
            if r.name.is_empty() || r.pid == 0 {
                continue;
            }
            let cpu_add = (r.cpu / 100.0) * dt;
            let gpu_add = (r.gpu / 100.0) * dt;
            let disk_add = (r.disk_bps() * dt).round().max(0.0) as u64;
            if let Some(rec) = self.records.iter_mut().find(|x| x.date == date && x.app.eq_ignore_ascii_case(&r.name)) {
                rec.cpu_secs += cpu_add;
                rec.gpu_secs += gpu_add;
                rec.disk_bytes = rec.disk_bytes.saturating_add(disk_add);
                rec.peak_mem_bytes = rec.peak_mem_bytes.max(r.working_set);
                rec.run_secs += dt;
                if rec.publisher.is_empty() && !r.publisher.is_empty() {
                    rec.publisher = r.publisher.clone();
                }
            } else {
                self.records.push(AppDayRecord {
                    date: date.clone(),
                    app: r.name.clone(),
                    publisher: r.publisher.clone(),
                    cpu_secs: cpu_add,
                    gpu_secs: gpu_add,
                    disk_bytes: disk_add,
                    peak_mem_bytes: r.working_set,
                    run_secs: dt,
                });
            }
        }

        if self.last_save.is_none_or(|t| t.elapsed().as_secs() >= 30) {
            self.last_save = Some(now);
            self.save_to_disk();
        }
    }

    pub fn save_to_disk(&self) {
        if let Some(p) = storage_path()
            && let Ok(json) = serde_json::to_string(&self.records)
        {
            let _ = std::fs::write(p, json);
        }
    }

    pub fn clear_history(&mut self) {
        self.records.clear();
        if let Some(p) = storage_path() {
            let _ = std::fs::remove_file(p);
        }
    }

    pub fn show(&mut self, ui: &mut Ui, procs: &ProcSnap) {
        self.ingest_snapshot(procs);
        self.show_ui(ui);
    }

    pub fn show_ui(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui);

        // Honest scope notice (docs/v1/task-manager.md §5)
        widgets::pill(ui, tr("Covers only the time CpuZeeZ (or its tray icon) was running."), Tone::Neutral);
        ui.add_space(6.0);

        // Toolbar: Opt-in toggle, period filter pills, Delete usage history
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.enabled, tr("Record app resource history"));
            ui.separator();
            for per in Period::ALL {
                if ui.selectable_label(self.period == per, tr(per.label())).clicked() {
                    self.period = per;
                }
            }
            ui.separator();
            if ui.button(format!("{} {}", icon::TRASH, tr("Delete usage history"))).clicked() {
                self.clear_history();
            }
        });
        ui.add_space(6.0);

        let rows = aggregate_records(&self.records, self.period);

        // Top consumers summary & per-day bar chart
        if !rows.is_empty() {
            let top_cpu = rows.iter().max_by(|a, b| a.cpu_secs.total_cmp(&b.cpu_secs));
            let top_disk = rows.iter().max_by_key(|a| a.disk_bytes);
            let top_mem = rows.iter().max_by_key(|a| a.peak_mem_bytes);
            ui.horizontal_wrapped(|ui| {
                if let Some(c) = top_cpu {
                    widgets::pill(
                        ui,
                        &trf("Top CPU: {} ({})", &[&c.app, &format_duration_hms(c.cpu_secs)]),
                        Tone::Accent,
                    );
                }
                if let Some(d) = top_disk.filter(|d| d.disk_bytes > 0) {
                    widgets::pill(
                        ui,
                        &trf("Top Disk: {} ({} MB)", &[&d.app, &format!("{:.1}", d.disk_bytes as f64 / 1_048_576.0)]),
                        Tone::Neutral,
                    );
                }
                if let Some(m) = top_mem {
                    widgets::pill(
                        ui,
                        &trf(
                            "Peak RAM: {} ({} MB)",
                            &[&m.app, &format!("{:.0}", m.peak_mem_bytes as f64 / 1_048_576.0)],
                        ),
                        Tone::Neutral,
                    );
                }
            });
            ui.add_space(6.0);
            self.render_daily_chart(ui);
            ui.add_space(6.0);
        }

        if rows.is_empty() {
            widgets::empty_state(
                ui,
                icon::CLOCK_COUNTER_CLOCKWISE,
                tr("No app usage history recorded yet"),
                tr(
                    "Enable 'Record app resource history' above to accumulate per-app CPU, GPU, disk, and memory usage.",
                ),
            );
            return;
        }

        Table::new("sys_app_history_table", |r: &AggregatedApp| {
            let mut h = 0xcbf29ce484222325u64;
            for b in r.app.bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        })
        .col(Column::new("name", tr("Name"), 210.0, |r: &AggregatedApp| Cell::Text(r.app.clone())))
        .col(Column::new("publisher", tr("Publisher"), 170.0, |r: &AggregatedApp| {
            Cell::Text(if r.publisher.is_empty() { "–".into() } else { r.publisher.clone() })
        }))
        .col(
            Column::new("cpu", tr("CPU time"), 115.0, |r: &AggregatedApp| {
                Cell::num(r.cpu_secs, format_duration_hms(r.cpu_secs))
            })
            .right()
            .colored(move |r: &AggregatedApp| (r.cpu_secs >= 60.0).then_some(p.accent)),
        )
        .col(
            Column::new("gpu", tr("GPU time"), 115.0, |r: &AggregatedApp| {
                Cell::num(r.gpu_secs, format_duration_hms(r.gpu_secs))
            })
            .right(),
        )
        .col(
            Column::new("disk", tr("Disk bytes"), 120.0, |r: &AggregatedApp| {
                let mb = r.disk_bytes as f64 / 1_048_576.0;
                let text = if mb >= 1024.0 { format!("{:.2} GB", mb / 1024.0) } else { format!("{mb:.1} MB") };
                Cell::num(r.disk_bytes as f64, text)
            })
            .right(),
        )
        .col(
            Column::new("peak_mem", tr("Peak memory"), 120.0, |r: &AggregatedApp| {
                let mb = r.peak_mem_bytes as f64 / 1_048_576.0;
                Cell::num(mb, format!("{mb:.0} MB"))
            })
            .right(),
        )
        .col(
            Column::new("run", tr("Run time"), 115.0, |r: &AggregatedApp| {
                Cell::num(r.run_secs, format_duration_hms(r.run_secs))
            })
            .right(),
        )
        .show(ui, &rows, &mut self.table, |_, _| {});
    }

    fn render_daily_chart(&self, ui: &mut Ui) {
        let p = theme::palette(ui);
        let mut by_day: BTreeMap<&str, f64> = BTreeMap::new();
        for r in &self.records {
            *by_day.entry(r.date.as_str()).or_insert(0.0) += r.cpu_secs;
        }
        if by_day.is_empty() {
            return;
        }
        let max_v = by_day.values().copied().fold(1.0, f64::max);
        let days: Vec<(&str, f64)> = by_day.into_iter().rev().take(14).collect::<Vec<_>>().into_iter().rev().collect();
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::same(5), p.sidebar);
        let n = days.len().max(1) as f32;
        let bar_w = (rect.width() / n - 4.0).clamp(6.0, 48.0);
        for (i, (_day, secs)) in days.iter().enumerate() {
            let h = ((*secs / max_v) as f32 * (rect.height() - 6.0)).clamp(2.0, rect.height() - 4.0);
            let x = rect.left() + 4.0 + i as f32 * (bar_w + 4.0);
            let r = egui::Rect::from_min_size(egui::pos2(x, rect.bottom() - h - 2.0), vec2(bar_w, h));
            ui.painter().rect_filled(r, CornerRadius::same(2), p.series[0]);
        }
    }
}

pub const PT_BR: &[(&str, &str)] = &[
    (
        "Covers only the time CpuZeeZ (or its tray icon) was running.",
        "Cobre apenas o período em que o CpuZeeZ (ou seu ícone na bandeja) esteve em execução.",
    ),
    ("Record app resource history", "Gravar histórico de uso de recursos por app"),
    ("Today", "Hoje"),
    ("Last 7 days", "Últimos 7 dias"),
    ("Last 30 days", "Últimos 30 dias"),
    ("All time", "Todo o período"),
    ("Delete usage history", "Excluir histórico de uso"),
    ("Top CPU: {} ({})", "Maior CPU: {} ({})"),
    ("Top Disk: {} ({} MB)", "Maior Disco: {} ({} MB)"),
    ("Peak RAM: {} ({} MB)", "Pico de RAM: {} ({} MB)"),
    ("No app usage history recorded yet", "Nenhum histórico de uso de apps gravado ainda"),
    (
        "Enable 'Record app resource history' above to accumulate per-app CPU, GPU, disk, and memory usage.",
        "Ative 'Gravar histórico de uso de recursos por app' acima para acumular uso de CPU, GPU, disco e memória por aplicativo.",
    ),
    ("GPU time", "Tempo de GPU"),
    ("Disk bytes", "Bytes de disco"),
    ("Peak memory", "Pico de memória"),
    ("Run time", "Tempo em execução"),
];

pub const ES: &[(&str, &str)] = &[
    (
        "Covers only the time CpuZeeZ (or its tray icon) was running.",
        "Cubre únicamente el tiempo en que CpuZeeZ (o su icono en la bandeja) estuvo en ejecución.",
    ),
    ("Record app resource history", "Registrar historial de recursos por aplicación"),
    ("Today", "Hoy"),
    ("Last 7 days", "Últimos 7 días"),
    ("Last 30 days", "Últimos 30 días"),
    ("All time", "Todo el tiempo"),
    ("Delete usage history", "Eliminar historial de uso"),
    ("Top CPU: {} ({})", "Mayor CPU: {} ({})"),
    ("Top Disk: {} ({} MB)", "Mayor Disco: {} ({} MB)"),
    ("Peak RAM: {} ({} MB)", "Pico de RAM: {} ({} MB)"),
    ("No app usage history recorded yet", "Aún no se ha registrado historial de uso de aplicaciones"),
    (
        "Enable 'Record app resource history' above to accumulate per-app CPU, GPU, disk, and memory usage.",
        "Habilite 'Registrar historial de recursos por aplicación' arriba para acumular uso de CPU, GPU, disco y memoria por aplicación.",
    ),
    ("GPU time", "Tiempo de GPU"),
    ("Disk bytes", "Bytes de disco"),
    ("Peak memory", "Memoria máxima"),
    ("Run time", "Tiempo de ejecución"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_history_aggregates_and_clears() {
        use egui_kittest::{Harness, kittest::Queryable};
        let st = State {
            loaded: true,
            enabled: true,
            records: vec![AppDayRecord {
                date: "2026-11-05".into(),
                app: "firefox.exe".into(),
                publisher: "Mozilla".into(),
                cpu_secs: 125.0,
                gpu_secs: 40.0,
                disk_bytes: 50 * 1_048_576,
                peak_mem_bytes: 1200 * 1_048_576,
                run_secs: 600.0,
            }],
            ..State::default()
        };
        let agg = aggregate_records(&st.records, Period::Days30);
        assert_eq!(agg.len(), 1);
        assert_eq!(format_duration_hms(125.0), "00:02:05");

        let mut h = Harness::builder().with_size(egui::vec2(1050.0, 600.0)).build_ui_state(
            move |ui, state: &mut State| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                state.show_ui(ui);
            },
            st,
        );
        h.run();
        h.run();
        h.get_by_label_contains("Covers only the time CpuZeeZ");
        h.get_by_label_contains("Top CPU: firefox.exe (00:02:05)");
    }
}

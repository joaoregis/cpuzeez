//! System › App history tab (S2a-10, docs/v1/task-manager.md §5):
//! Cross-platform, opt-in per-app resource recorder (CPU time, GPU time,
//! disk bytes, peak memory, run time) in a local SQLite file with configurable
//! retention (default 30 days), period filter, per-day chart, top consumers,
//! and "Delete usage history". It records whatever page is shown, while
//! CpuZeeZ runs and recording is on.

use super::i18n::{tr, trf};
use super::table::{self, Cell, Column, Table};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::procs::{Sampler, Snapshot as ProcSnap};
use eframe::egui::{self, CornerRadius, RichText, Sense, Ui, vec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AggregatedApp {
    pub app: String,
    pub publisher: String,
    pub cpu_secs: f64,
    pub gpu_secs: f64,
    pub disk_bytes: u64,
    pub peak_mem_bytes: u64,
    pub run_secs: f64,
}

/// What the tab shows, answered by the recorder thread.
#[derive(Debug, Clone, Default)]
pub struct View {
    pub period: Period,
    pub apps: Vec<AggregatedApp>,
    /// (date, CPU seconds) for the last 14 days with data, oldest first.
    pub days: Vec<(String, f64)>,
    pub error: Option<String>,
}

/// The SQLite store (`app_history.sqlite` next to the settings): one row per
/// app per day, accumulated with UPSERTs, queried by period, pruned by the
/// retention setting. Every function takes the connection so tests run on
/// an in-memory database.
pub mod db {
    use super::{AggregatedApp, AppDayRecord, Period};
    use rusqlite::{Connection, params};

    pub fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
        let c = Connection::open(path)?;
        init(&c)?;
        Ok(c)
    }

    pub fn init(c: &Connection) -> rusqlite::Result<()> {
        c.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS app_day (
                 date TEXT NOT NULL,
                 app TEXT NOT NULL COLLATE NOCASE,
                 publisher TEXT NOT NULL DEFAULT '',
                 cpu_secs REAL NOT NULL DEFAULT 0,
                 gpu_secs REAL NOT NULL DEFAULT 0,
                 disk_bytes INTEGER NOT NULL DEFAULT 0,
                 peak_mem_bytes INTEGER NOT NULL DEFAULT 0,
                 run_secs REAL NOT NULL DEFAULT 0,
                 PRIMARY KEY (date, app)
             );",
        )
    }

    /// Add `recs` (deltas for CPU, GPU, disk and run time; a peak for memory).
    pub fn add(c: &mut Connection, recs: &[AppDayRecord]) -> rusqlite::Result<()> {
        let tx = c.transaction()?;
        {
            let mut st = tx.prepare_cached(
                "INSERT INTO app_day (date, app, publisher, cpu_secs, gpu_secs, disk_bytes, peak_mem_bytes, run_secs)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT (date, app) DO UPDATE SET
                     publisher = CASE WHEN publisher = '' THEN excluded.publisher ELSE publisher END,
                     cpu_secs = cpu_secs + excluded.cpu_secs,
                     gpu_secs = gpu_secs + excluded.gpu_secs,
                     disk_bytes = disk_bytes + excluded.disk_bytes,
                     peak_mem_bytes = max(peak_mem_bytes, excluded.peak_mem_bytes),
                     run_secs = run_secs + excluded.run_secs",
            )?;
            for r in recs {
                st.execute(params![
                    r.date,
                    r.app,
                    r.publisher,
                    r.cpu_secs,
                    r.gpu_secs,
                    r.disk_bytes as i64,
                    r.peak_mem_bytes as i64,
                    r.run_secs
                ])?;
            }
        }
        tx.commit()
    }

    /// `period` ending at `today` (YYYY-MM-DD), heaviest CPU first.
    pub fn apps(c: &Connection, period: Period, today: &str) -> rusqlite::Result<Vec<AggregatedApp>> {
        let from = format!("-{} days", period.max_days().saturating_sub(1));
        let mut st = c.prepare_cached(
            "SELECT app, max(publisher), sum(cpu_secs), sum(gpu_secs), sum(disk_bytes), max(peak_mem_bytes), sum(run_secs)
             FROM app_day WHERE date >= date(?1, ?2) AND date <= ?1
             GROUP BY app ORDER BY sum(cpu_secs) DESC",
        )?;
        let rows = st.query_map(params![today, from], |r| {
            Ok(AggregatedApp {
                app: r.get(0)?,
                publisher: r.get(1)?,
                cpu_secs: r.get(2)?,
                gpu_secs: r.get(3)?,
                disk_bytes: r.get::<_, i64>(4)?.max(0) as u64,
                peak_mem_bytes: r.get::<_, i64>(5)?.max(0) as u64,
                run_secs: r.get(6)?,
            })
        })?;
        rows.collect()
    }

    /// CPU seconds per day, the last `n` days with data, oldest first.
    pub fn days(c: &Connection, n: usize) -> rusqlite::Result<Vec<(String, f64)>> {
        let mut st =
            c.prepare_cached("SELECT date, sum(cpu_secs) FROM app_day GROUP BY date ORDER BY date DESC LIMIT ?1")?;
        let mut v: Vec<(String, f64)> =
            st.query_map(params![n as i64], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        v.reverse();
        Ok(v)
    }

    /// Drop days older than `keep_days` before `today`.
    pub fn retain(c: &Connection, keep_days: u32, today: &str) -> rusqlite::Result<usize> {
        let from = format!("-{} days", keep_days.max(1).saturating_sub(1));
        c.execute("DELETE FROM app_day WHERE date < date(?1, ?2)", params![today, from])
    }

    pub fn clear(c: &Connection) -> rusqlite::Result<()> {
        c.execute("DELETE FROM app_day", []).map(|_| ())
    }
}

enum Cmd {
    Query(Period),
    Clear,
    Flush,
}

/// The recorder thread: samples processes (through the System hub's own
/// sampler) while recording is on, whatever page is shown, writes to SQLite
/// every 30 s and answers the tab's queries. No disk I/O on the UI thread.
struct Recorder {
    tx: mpsc::Sender<Cmd>,
    view: Arc<Mutex<View>>,
    enabled: Arc<AtomicBool>,
    retention: Arc<AtomicU32>,
}

fn storage_path() -> Option<PathBuf> {
    crate::settings::dir().map(|d| d.join("app_history.sqlite"))
}

/// The 1.0.0-beta.3/4 file, moved into SQLite once.
fn legacy_path() -> Option<PathBuf> {
    crate::settings::dir().map(|d| d.join("app_history.json"))
}

/// Import the old JSON file into `c` and delete it (kept if anything fails).
fn migrate_json(c: &mut rusqlite::Connection, old: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(old) else { return };
    if let Ok(list) = serde_json::from_str::<Vec<AppDayRecord>>(&text)
        && db::add(c, &list).is_ok()
    {
        let _ = std::fs::remove_file(old);
    }
}

impl Recorder {
    fn start(sampler: Sampler) -> Recorder {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let view = Arc::new(Mutex::new(View::default()));
        let enabled = Arc::new(AtomicBool::new(false));
        let retention = Arc::new(AtomicU32::new(30));
        let (v, en, ret) = (view.clone(), enabled.clone(), retention.clone());
        std::thread::Builder::new()
            .name("cz-app-history".into())
            .spawn(move || {
                let mut conn = storage_path().and_then(|p| db::open(&p).ok());
                if let (Some(c), Some(old)) = (&mut conn, legacy_path()) {
                    migrate_json(c, &old);
                }
                let mut pending: HashMap<(String, String), AppDayRecord> = HashMap::new();
                let mut last_snap: Option<(Arc<ProcSnap>, Instant)> = None;
                let mut last_flush = Instant::now();
                let mut period = Period::Days30;
                let mut dirty_view = true;
                loop {
                    let cmd = match rx.recv_timeout(Duration::from_millis(1000)) {
                        Ok(c) => Some(c),
                        Err(mpsc::RecvTimeoutError::Timeout) => None,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    let on = en.load(Ordering::Relaxed);
                    if on {
                        sampler.want();
                        let snap = sampler.snapshot();
                        let now = Instant::now();
                        match &last_snap {
                            Some((prev, at)) if !Arc::ptr_eq(prev, &snap) => {
                                let dt = now.duration_since(*at).as_secs_f64().clamp(0.0, 10.0);
                                accumulate(&mut pending, &snap, dt, &today_ymd());
                                last_snap = Some((snap, now));
                            }
                            None => last_snap = Some((snap, now)),
                            _ => {}
                        }
                    } else {
                        last_snap = None;
                    }
                    let flush = matches!(cmd, Some(Cmd::Flush))
                        || (!pending.is_empty() && (last_flush.elapsed() >= Duration::from_secs(30) || !on));
                    if let Some(c) = &mut conn {
                        if flush && !pending.is_empty() {
                            let recs: Vec<AppDayRecord> = pending.drain().map(|(_, r)| r).collect();
                            let _ = db::add(c, &recs);
                            let _ = db::retain(c, ret.load(Ordering::Relaxed), &today_ymd());
                            last_flush = Instant::now();
                            dirty_view = true;
                        }
                        match cmd {
                            Some(Cmd::Query(p)) => {
                                period = p;
                                dirty_view = true;
                            }
                            Some(Cmd::Clear) => {
                                pending.clear();
                                let _ = db::clear(c);
                                dirty_view = true;
                            }
                            _ => {}
                        }
                        if dirty_view {
                            dirty_view = false;
                            let today = today_ymd();
                            let res = db::apps(c, period, &today).and_then(|a| Ok((a, db::days(c, 14)?)));
                            let mut out = v.lock().unwrap_or_else(|e| e.into_inner());
                            out.period = period;
                            match res {
                                Ok((apps, days)) => {
                                    out.apps = apps;
                                    out.days = days;
                                    out.error = None;
                                }
                                Err(e) => out.error = Some(e.to_string()),
                            }
                        }
                    } else if cmd.is_some() {
                        v.lock().unwrap_or_else(|e| e.into_inner()).error =
                            Some("the history database could not be opened".into());
                    }
                }
            })
            .ok();
        Recorder { tx, view, enabled, retention }
    }
}

/// Add one snapshot's usage over `dt` seconds to the per-(day, app) deltas.
pub fn accumulate(pending: &mut HashMap<(String, String), AppDayRecord>, procs: &ProcSnap, dt: f64, date: &str) {
    // Run time is the app's wall time: once per app, however many processes it has.
    let mut ran: std::collections::HashSet<String> = std::collections::HashSet::new();
    for r in &procs.rows {
        if r.name.is_empty() || r.pid == 0 {
            continue;
        }
        let rec = pending.entry((date.to_string(), r.name.to_ascii_lowercase())).or_insert_with(|| AppDayRecord {
            date: date.to_string(),
            app: r.name.clone(),
            ..Default::default()
        });
        if rec.publisher.is_empty() {
            rec.publisher = r.publisher.clone();
        }
        rec.cpu_secs += r.cpu / 100.0 * dt;
        rec.gpu_secs += r.gpu / 100.0 * dt;
        rec.disk_bytes = rec.disk_bytes.saturating_add((r.disk_bps() * dt).round().max(0.0) as u64);
        rec.peak_mem_bytes = rec.peak_mem_bytes.max(r.working_set);
        if ran.insert(r.name.to_ascii_lowercase()) {
            rec.run_secs += dt;
        }
    }
}

pub struct State {
    pub period: Period,
    pub table: table::State,
    pub enabled: bool,
    pub retention_days: u32,
    recorder: Option<Recorder>,
    asked: Option<Period>,
    confirm_clear: bool,
    /// For tests and screenshots: a fixed view instead of the recorder's.
    pub preview: Option<View>,
}

impl Default for State {
    fn default() -> Self {
        let mut t = table::State::default();
        t.sort = Some(("cpu".into(), false));
        Self {
            period: Period::Days30,
            table: t,
            enabled: false,
            retention_days: 30,
            recorder: None,
            asked: None,
            confirm_clear: false,
            preview: None,
        }
    }
}

pub fn today_ymd() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let ft = (secs + 11_644_473_600) * 10_000_000;
    cpuzeez_core::startup::filetime_to_ymd(ft).unwrap_or_else(|| "1970-01-01".into())
}

pub fn format_duration_hms(secs: f64) -> String {
    let total = secs.round().max(0.0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

impl State {
    /// Every frame, on any page: start the recorder and pass it the settings.
    pub fn background(&mut self, sampler: &Sampler) {
        let r = self.recorder.get_or_insert_with(|| Recorder::start(sampler.clone()));
        r.enabled.store(self.enabled, Ordering::Relaxed);
        r.retention.store(self.retention_days.max(1), Ordering::Relaxed);
    }

    /// Write what is pending now (on exit).
    pub fn flush(&self) {
        if let Some(r) = &self.recorder {
            let _ = r.tx.send(Cmd::Flush);
        }
    }

    fn view(&mut self) -> View {
        if let Some(v) = &self.preview {
            return v.clone();
        }
        let Some(r) = &self.recorder else { return View::default() };
        if self.asked != Some(self.period) {
            self.asked = Some(self.period);
            let _ = r.tx.send(Cmd::Query(self.period));
        }
        r.view.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn clear_history(&mut self) {
        if let Some(r) = &self.recorder {
            let _ = r.tx.send(Cmd::Clear);
        }
        if let Some(v) = &mut self.preview {
            *v = View::default();
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        self.show_ui(ui);
        // The recorder answers queries and writes in the background.
        ui.ctx().request_repaint_after(Duration::from_secs(2));
    }

    pub fn show_ui(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui);

        // Honest scope notice (docs/v1/task-manager.md §5)
        widgets::pill(ui, tr("Covers only the time CpuZeeZ (or its tray icon) was running."), Tone::Neutral);
        let view = self.view();
        if let Some(e) = &view.error {
            widgets::pill(ui, &trf("History unavailable: {}", &[e]), Tone::Danger);
        }
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
            if self.confirm_clear {
                ui.label(RichText::new(tr("Delete every recorded day?")).color(p.danger));
                if ui.button(tr("Delete")).clicked() {
                    self.clear_history();
                    self.confirm_clear = false;
                }
                if ui.button(tr("Cancel")).clicked() {
                    self.confirm_clear = false;
                }
            } else if ui.button(format!("{} {}", icon::TRASH, tr("Delete usage history"))).clicked() {
                self.confirm_clear = true;
            }
        });
        ui.add_space(6.0);

        let rows = if view.period == self.period { view.apps.clone() } else { Vec::new() };

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
            render_daily_chart(ui, &view.days);
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
}

fn render_daily_chart(ui: &mut Ui, days: &[(String, f64)]) {
    {
        let p = theme::palette(ui);
        if days.is_empty() {
            return;
        }
        let max_v = days.iter().map(|d| d.1).fold(1.0, f64::max);
        let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::same(5), p.sidebar);
        let n = days.len().max(1) as f32;
        let bar_w = (rect.width() / n - 4.0).clamp(6.0, 48.0);
        let mut tip = None;
        for (i, (day, secs)) in days.iter().enumerate() {
            let h = ((*secs / max_v) as f32 * (rect.height() - 6.0)).clamp(2.0, rect.height() - 4.0);
            let x = rect.left() + 4.0 + i as f32 * (bar_w + 4.0);
            let r = egui::Rect::from_min_size(egui::pos2(x, rect.bottom() - h - 2.0), vec2(bar_w, h));
            ui.painter().rect_filled(r, CornerRadius::same(2), p.series[0]);
            if resp.hover_pos().is_some_and(|h| h.x >= r.left() && h.x <= r.right()) {
                tip = Some(format!("{day}: {} CPU", format_duration_hms(*secs)));
            }
        }
        if let Some(t) = tip {
            resp.on_hover_text(t);
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

    fn rec(date: &str, app: &str, cpu: f64, mem: u64) -> AppDayRecord {
        AppDayRecord {
            date: date.into(),
            app: app.into(),
            cpu_secs: cpu,
            peak_mem_bytes: mem,
            run_secs: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn sqlite_store_accumulates_queries_and_prunes() {
        let mut c = rusqlite::Connection::open_in_memory().unwrap();
        db::init(&c).unwrap();
        db::add(&mut c, &[rec("2026-11-05", "firefox.exe", 10.0, 100), rec("2026-11-05", "game.exe", 50.0, 900)])
            .unwrap();
        // The same app (any case) on the same day accumulates; memory keeps the peak.
        db::add(&mut c, &[rec("2026-11-05", "Firefox.exe", 5.0, 300), rec("2026-10-01", "firefox.exe", 7.0, 50)])
            .unwrap();
        let today = db::apps(&c, Period::Today, "2026-11-05").unwrap();
        assert_eq!(today.len(), 2);
        assert_eq!(today[0].app, "game.exe", "heaviest CPU first");
        let ff = today.iter().find(|a| a.app.eq_ignore_ascii_case("firefox.exe")).unwrap();
        assert_eq!((ff.cpu_secs, ff.peak_mem_bytes, ff.run_secs), (15.0, 300, 2.0));
        let all = db::apps(&c, Period::All, "2026-11-05").unwrap();
        assert_eq!(all.iter().find(|a| a.app.eq_ignore_ascii_case("firefox.exe")).unwrap().cpu_secs, 22.0);
        assert_eq!(db::days(&c, 14).unwrap(), [("2026-10-01".to_string(), 7.0), ("2026-11-05".to_string(), 65.0)]);
        assert_eq!(db::retain(&c, 30, "2026-11-05").unwrap(), 1, "October 1 is older than 30 days");
        assert_eq!(db::days(&c, 14).unwrap().len(), 1);
        db::clear(&c).unwrap();
        assert!(db::apps(&c, Period::All, "2026-11-05").unwrap().is_empty());
    }

    #[test]
    fn old_json_history_moves_into_sqlite() {
        let dir = std::env::temp_dir().join(format!("cz-hist-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join("app_history.json");
        std::fs::write(&old, serde_json::to_string(&[rec("2026-11-04", "x.exe", 3.0, 1)]).unwrap()).unwrap();
        let mut c = rusqlite::Connection::open_in_memory().unwrap();
        db::init(&c).unwrap();
        migrate_json(&mut c, &old);
        assert!(!old.exists(), "the JSON file is gone once imported");
        assert_eq!(db::apps(&c, Period::All, "2026-11-05").unwrap()[0].cpu_secs, 3.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn snapshots_accumulate_per_app_and_day() {
        let row = |pid, name: &str, cpu| cpuzeez_core::procs::Row {
            pid,
            name: name.into(),
            cpu,
            working_set: 10,
            ..Default::default()
        };
        let snap = ProcSnap {
            rows: vec![row(1, "a.exe", 50.0), row(2, "A.EXE", 50.0), row(0, "idle", 99.0)],
            ..Default::default()
        };
        let mut pending = HashMap::new();
        accumulate(&mut pending, &snap, 2.0, "2026-11-05");
        let a = &pending[&("2026-11-05".to_string(), "a.exe".to_string())];
        assert_eq!(a.cpu_secs, 2.0, "two processes of one app at 50 % for 2 s: 2 CPU-seconds");
        assert_eq!(a.run_secs, 2.0, "run time is wall time, not per process");
        assert_eq!(pending.len(), 1, "PID 0 is skipped");
    }

    #[test]
    fn app_history_tab_shows_the_view() {
        use egui_kittest::{Harness, kittest::Queryable};
        let st = State {
            enabled: true,
            preview: Some(View {
                period: Period::Days30,
                apps: vec![AggregatedApp {
                    app: "firefox.exe".into(),
                    publisher: "Mozilla".into(),
                    cpu_secs: 125.0,
                    gpu_secs: 40.0,
                    disk_bytes: 50 * 1_048_576,
                    peak_mem_bytes: 1200 * 1_048_576,
                    run_secs: 600.0,
                }],
                days: vec![("2026-11-05".into(), 125.0)],
                error: None,
            }),
            ..State::default()
        };
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

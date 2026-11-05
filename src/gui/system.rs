//! System hub (`Page::System`, S2a): cross-platform Task Manager with tabs for
//! Processes (S2a-02, S2a-03, S2a-04, S2a-05), Performance (S2a-06),
//! App history (S2a-10), Startup (S2a-08), Users (S2a-09) and Services (S2a-07).

use super::i18n::{tr, trf};
use super::table::{self, Cell, Column, Table};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::executor::{self, Done};
use crate::procs::{Sampler, Snapshot};
use cpuzeez_core::actions::{Action, Priority, ProcessOp};
use cpuzeez_core::procs::{self, Kind, Problem, Row};
use cpuzeez_platform::{desktop, proc_ops, procs as os_procs};
use eframe::egui::{self, Color32, RichText, Ui};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SysTab {
    #[default]
    Processes,
    Performance,
    AppHistory,
    Startup,
    Users,
    Services,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ProcMode {
    #[default]
    Grouped,
    Detailed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Preset {
    #[default]
    Essential,
    Gaming,
    LeakHunting,
    Security,
    Everything,
}

impl Preset {
    pub const ALL: [Preset; 5] =
        [Preset::Essential, Preset::Gaming, Preset::LeakHunting, Preset::Security, Preset::Everything];

    pub fn label(self) -> &'static str {
        match self {
            Preset::Essential => "Essential",
            Preset::Gaming => "Gaming",
            Preset::LeakHunting => "Leak hunting",
            Preset::Security => "Security",
            Preset::Everything => "Everything",
        }
    }

    pub fn hidden(self) -> BTreeSet<String> {
        let keep: &[&str] = match self {
            Preset::Essential => &["name", "pid", "status", "cpu", "trend", "mem", "disk", "gpu"],
            Preset::Gaming => &["name", "pid", "cpu", "trend", "gpu", "vram", "top_thread", "priority", "mem"],
            Preset::LeakHunting => &["name", "pid", "private", "mem", "peak_mem", "faults", "handles", "threads"],
            Preset::Security => &["name", "pid", "publisher", "user", "session", "path"],
            Preset::Everything => return BTreeSet::new(),
        };
        ALL_COL_IDS.iter().filter(|id| !keep.contains(id)).map(|s| (*s).to_string()).collect()
    }
}

const ALL_COL_IDS: &[&str] = &[
    "name",
    "pid",
    "status",
    "cpu",
    "trend",
    "mem",
    "private",
    "peak_mem",
    "disk",
    "gpu",
    "vram",
    "top_thread",
    "threads",
    "handles",
    "faults",
    "priority",
    "user",
    "session",
    "publisher",
    "path",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DetailTab {
    #[default]
    Overview,
    Threads,
    Network,
    Windows,
    Security,
}

/// Snapshot diff entry ("Compare with snapshot", S2a-05).
#[derive(Debug, Clone, PartialEq)]
pub struct DiffEntry {
    pub pid: u32,
    pub name: String,
    pub kind: DiffKind,
    pub mem_delta: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Added,
    Exited,
    MemoryChanged,
}

pub fn diff_snapshots(base: &[Row], now: &[Row]) -> Vec<DiffEntry> {
    let mut out = Vec::new();
    for r in now {
        match base.iter().find(|b| b.key() == r.key()) {
            None => out.push(DiffEntry {
                pid: r.pid,
                name: r.name.clone(),
                kind: DiffKind::Added,
                mem_delta: r.private as i64,
            }),
            Some(b) => {
                let d = r.private as i64 - b.private as i64;
                if d.unsigned_abs() >= (16 << 20) {
                    out.push(DiffEntry {
                        pid: r.pid,
                        name: r.name.clone(),
                        kind: DiffKind::MemoryChanged,
                        mem_delta: d,
                    });
                }
            }
        }
    }
    for b in base {
        if !now.iter().any(|r| r.key() == b.key()) {
            out.push(DiffEntry {
                pid: b.pid,
                name: b.name.clone(),
                kind: DiffKind::Exited,
                mem_delta: -(b.private as i64),
            });
        }
    }
    out
}

/// CSV export of process rows (S2a-05).
pub fn export_csv(rows: &[Row]) -> String {
    let mut s = String::from(
        "pid,ppid,name,cpu_pct,working_set_bytes,private_bytes,disk_bps,gpu_pct,vram_bytes,threads,handles,user,publisher,path\n",
    );
    let esc = |t: &str| format!("\"{}\"", t.replace('"', "\"\""));
    for r in rows {
        s.push_str(&format!(
            "{},{},{},{:.2},{},{},{:.0},{:.2},{},{},{},{},{},{}\n",
            r.pid,
            r.ppid,
            esc(&r.name),
            r.cpu,
            r.working_set,
            r.private,
            r.disk_bps(),
            r.gpu,
            r.vram,
            r.threads,
            r.handles,
            esc(&r.user),
            esc(&r.publisher),
            esc(&r.path),
        ));
    }
    s
}

/// JSON export of process rows (S2a-05).
pub fn export_json(rows: &[Row]) -> String {
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "pid": r.pid,
                "ppid": r.ppid,
                "name": r.name,
                "cpu_pct": r.cpu,
                "working_set": r.working_set,
                "private": r.private,
                "disk_bps": r.disk_bps(),
                "gpu_pct": r.gpu,
                "vram": r.vram,
                "threads": r.threads,
                "handles": r.handles,
                "user": r.user,
                "publisher": r.publisher,
                "path": r.path,
            })
        })
        .collect();
    serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".into())
}

#[derive(Clone)]
struct ConfirmEnd {
    pids: Vec<u32>,
    names: Vec<String>,
    tree: bool,
    force: bool,
    critical: bool,
    elevate: bool,
}

struct AffinityModal {
    pid: u32,
    name: String,
    mask: u64,
    elevate: bool,
}

struct WaitChainModal {
    pid: u32,
    name: String,
    lines: Vec<String>,
}

struct RunTaskModal {
    cmd: String,
    admin: bool,
}

/// Cached on-demand details for the selected PID (S2a-04).
#[derive(Default)]
struct InspectorCache {
    pid: u32,
    cmdline: String,
    threads: Vec<os_procs::Thread>,
    conns: Vec<proc_ops::NetConn>,
    sha256: Option<String>,
    last_refresh: Option<std::time::Instant>,
    /// Threads and connections read on a background thread (a full snapshot
    /// with threads plus the socket table took longer than a frame).
    pending: Option<Arc<Mutex<Option<(u32, Vec<os_procs::Thread>, Vec<proc_ops::NetConn>)>>>>,
}

pub struct State {
    pub tab: SysTab,
    pub sampler: Sampler,
    pub history: super::system_history::State,
    pub perf: super::system_perf::State,
    pub services: super::system_services::State,
    pub startup: super::system_startup::State,
    pub users: super::system_users::State,
    pub mode: ProcMode,
    pub tree: bool,
    pub preset: Preset,
    pub problem_filter: Option<Problem>,
    pub pinned: BTreeSet<String>,
    pub table: table::State,
    pub show_details: bool,
    pub detail_tab: DetailTab,
    inspector: InspectorCache,
    pub baseline: Option<Vec<Row>>,
    pub show_diff: bool,
    confirm_end: Option<ConfirmEnd>,
    affinity: Option<AffinityModal>,
    wait_chain: Option<WaitChainModal>,
    run_task: Option<RunTaskModal>,
    running: Option<Arc<Mutex<Option<Done>>>>,
    pub message: Option<(String, Tone)>,
}

impl Default for State {
    fn default() -> Self {
        let preset = Preset::Essential;
        let mut table = table::State::default();
        table.sort = Some(("cpu".into(), false));
        table.hidden = preset.hidden();
        State {
            tab: SysTab::Processes,
            sampler: Sampler::new(1000),
            history: Default::default(),
            perf: Default::default(),
            services: Default::default(),
            startup: Default::default(),
            users: Default::default(),
            mode: ProcMode::Grouped,
            tree: false,
            preset,
            problem_filter: None,
            pinned: BTreeSet::new(),
            table,
            show_details: false,
            detail_tab: DetailTab::Overview,
            inspector: InspectorCache::default(),
            baseline: None,
            show_diff: false,
            confirm_end: None,
            affinity: None,
            wait_chain: None,
            run_task: None,
            running: None,
            message: None,
        }
    }
}

impl State {
    fn dispatch(&mut self, mut a: Action) {
        // Pin each target to the process the user saw (PID + start time).
        if let Action::Process { target, identity, .. } = &mut a {
            let snap = self.sampler.snapshot();
            *identity = target
                .pids()
                .into_iter()
                .filter_map(|pid| snap.rows.iter().find(|r| r.pid == pid).map(|r| (pid, r.start_ms)))
                .collect();
        }
        let slot = Arc::new(Mutex::new(None));
        let s = slot.clone();
        std::thread::spawn(move || {
            *s.lock().unwrap_or_else(|e| e.into_inner()) = Some(executor::run(&a));
        });
        self.running = Some(slot);
    }

    fn poll_running(&mut self, ctx: &egui::Context) {
        if let Some(slot) = &self.running {
            let done = slot.lock().unwrap_or_else(|e| e.into_inner()).take();
            match done {
                None => ctx.request_repaint_after(std::time::Duration::from_millis(120)),
                Some(Done::Ok(msg)) => {
                    self.running = None;
                    let text = if msg.is_empty() { tr("Action completed.").to_string() } else { msg };
                    self.message = Some((text, Tone::Success));
                }
                Some(Done::Cancelled) => {
                    self.running = None;
                    self.message = Some((tr("Action cancelled.").to_string(), Tone::Neutral));
                }
                Some(Done::Failed(e)) => {
                    self.running = None;
                    self.message = Some((trf("Action failed: {}", &[&e]), Tone::Danger));
                }
            }
        }
    }

    pub fn page(&mut self, ui: &mut Ui, ctx: &Arc<crate::Ctx>, mon: &mut super::sensors::Monitor) {
        self.sampler.want();
        self.poll_running(ui.ctx());
        self.history.enabled = mon.settings.app_history_enabled;
        self.history.retention_days = mon.settings.app_history_retention_days;
        let snap = self.sampler.snapshot();
        if self.tab != SysTab::AppHistory {
            self.history.ingest_snapshot(&snap);
        }
        if self.tab == SysTab::Performance {
            self.header_and_tabs(ui, &snap);
            self.perf.show(ui, ctx, mon, &snap);
            self.modals(ui, &snap);
        } else {
            self.page_with_snapshot(ui, &snap);
        }
        if self.history.enabled != mon.settings.app_history_enabled {
            let on = self.history.enabled;
            mon.set_setting(|s| s.app_history_enabled = on);
        }
    }

    fn header_and_tabs(&mut self, ui: &mut Ui, snap: &Snapshot) {
        let sub = trf(
            "{} processes · {} threads · {} handles",
            &[&snap.totals.processes, &snap.totals.threads, &snap.totals.handles],
        );
        widgets::page_header(ui, tr("System"), &sub, |ui| {
            let paused = self.sampler.is_paused();
            let pause_label = if paused {
                format!("{} {}", icon::PLAY, tr("Resume"))
            } else {
                format!("{} {}", icon::PAUSE, tr("Pause"))
            };
            if ui.button(pause_label).on_hover_text(tr("Pause or resume live updates (Space)")).clicked() {
                self.sampler.set_paused(!paused);
            }
            let cur_ms = self.sampler.period_ms();
            let speed_label = match cur_ms {
                0..=600 => tr("High (0.5 s)"),
                601..=1500 => tr("Normal (1 s)"),
                _ => tr("Low (4 s)"),
            };
            egui::ComboBox::from_id_salt("sys_speed").selected_text(speed_label).show_ui(ui, |ui| {
                for (ms, label) in [(500, "High (0.5 s)"), (1000, "Normal (1 s)"), (4000, "Low (4 s)")] {
                    if ui.selectable_label(cur_ms == ms, tr(label)).clicked() {
                        self.sampler.set_period_ms(ms);
                    }
                }
            });
            if ui.button(format!("{} {}", icon::TERMINAL_WINDOW, tr("Run new task"))).clicked() {
                self.run_task = Some(RunTaskModal { cmd: String::new(), admin: false });
            }
        });

        widgets::segmented(
            ui,
            &mut self.tab,
            &[
                (SysTab::Processes, icon::LIST_BULLETS, tr("Processes")),
                (SysTab::Performance, icon::CHART_LINE_UP, tr("Performance")),
                (SysTab::AppHistory, icon::CLOCK_COUNTER_CLOCKWISE, tr("App history")),
                (SysTab::Startup, icon::ROCKET_LAUNCH, tr("Startup")),
                (SysTab::Users, icon::USERS, tr("Users")),
                (SysTab::Services, icon::GEAR_SIX, tr("Services")),
            ],
        );

        if let Some((msg, tone)) = &self.message {
            let mut clear = false;
            ui.horizontal(|ui| {
                widgets::pill(ui, msg, *tone);
                if ui.small_button(icon::X).clicked() {
                    clear = true;
                }
            });
            if clear {
                self.message = None;
            }
            ui.add_space(4.0);
        }
    }

    /// Renders the System hub using a provided snapshot (used by `page` and unit/GUI tests).
    pub fn page_with_snapshot(&mut self, ui: &mut Ui, snap: &Snapshot) {
        self.header_and_tabs(ui, snap);

        match self.tab {
            SysTab::Processes => self.processes_tab(ui, snap),
            SysTab::Performance => {
                let ctx = Arc::new(crate::Ctx::new(Default::default()));
                self.perf.show_with_series(ui, &ctx, &[], 0.0, snap);
            }
            SysTab::AppHistory => self.history.show(ui, snap),
            SysTab::Startup => self.startup.show(ui),
            SysTab::Users => self.users.show(ui, snap),
            SysTab::Services => self.services.show(ui),
        }

        self.modals(ui, snap);
    }

    fn processes_tab(&mut self, ui: &mut Ui, snap: &Snapshot) {
        // Spacebar pauses/resumes when no text box has focus.
        if ui.ctx().memory(|m| m.focused().is_none()) && ui.input(|i| i.key_pressed(egui::Key::Space)) {
            self.sampler.set_paused(!self.sampler.is_paused());
        }

        // Toolbar row 1: Mode (Grouped / Detailed), Tree, Preset, Details panel, Snapshot compare & Export.
        ui.horizontal_wrapped(|ui| {
            let grouped = self.mode == ProcMode::Grouped;
            if ui.selectable_label(grouped, tr("Grouped")).clicked() {
                self.mode = ProcMode::Grouped;
            }
            if ui.selectable_label(!grouped, tr("Detailed")).clicked() {
                self.mode = ProcMode::Detailed;
            }
            ui.separator();
            if ui
                .selectable_label(self.tree, format!("{} {}", icon::GIT_BRANCH, tr("Tree")))
                .on_hover_text(tr("Show parent → child hierarchy"))
                .clicked()
            {
                self.tree = !self.tree;
                if self.tree {
                    self.table.sort = None;
                }
            }
            ui.separator();
            egui::ComboBox::from_id_salt("proc_preset")
                .selected_text(format!("{}: {}", tr("Preset"), tr(self.preset.label())))
                .show_ui(ui, |ui| {
                    for p in Preset::ALL {
                        if ui.selectable_value(&mut self.preset, p, tr(p.label())).changed() {
                            self.table.hidden = p.hidden();
                        }
                    }
                });
            ui.separator();
            if ui
                .selectable_label(self.show_details, format!("{} {}", icon::SIDEBAR_SIMPLE, tr("Details panel")))
                .clicked()
            {
                self.show_details = !self.show_details;
            }
            if ui.button(format!("{} {}", icon::CAMERA, tr("Snapshot"))).clicked() {
                self.baseline = Some(snap.rows.clone());
                self.show_diff = true;
            }
            if self.baseline.is_some() && ui.selectable_label(self.show_diff, tr("Compare")).clicked() {
                self.show_diff = !self.show_diff;
            }
            ui.menu_button(format!("{} {}", icon::EXPORT, tr("Export")), |ui| {
                if ui.button(tr("Export CSV")).clicked() {
                    let path = "processes.csv";
                    match std::fs::write(path, export_csv(&snap.rows)) {
                        Ok(()) => self.message = Some((trf("Saved {}", &[&path]), Tone::Success)),
                        Err(e) => self.message = Some((e.to_string(), Tone::Danger)),
                    }
                    ui.close();
                }
                if ui.button(tr("Export JSON")).clicked() {
                    let path = "processes.json";
                    match std::fs::write(path, export_json(&snap.rows)) {
                        Ok(()) => self.message = Some((trf("Saved {}", &[&path]), Tone::Success)),
                        Err(e) => self.message = Some((e.to_string(), Tone::Danger)),
                    }
                    ui.close();
                }
            });

            // Quick End task button for current selection.
            let sel_rows: Vec<&Row> = snap.rows.iter().filter(|r| self.table.selected.contains(&r.key())).collect();
            if !sel_rows.is_empty() {
                ui.separator();
                let label = if sel_rows.len() == 1 {
                    tr("End task").to_string()
                } else {
                    trf("End {} tasks", &[&sel_rows.len()])
                };
                if widgets::primary_button(ui, &label).clicked() {
                    self.request_end(&sel_rows, &snap.rows, false, false);
                }
            }
        });

        // Toolbar row 2: Smart "Problems" filter pills (S2a-05).
        let spm = 60;
        let counts = [
            (Problem::NotResponding, "Not responding"),
            (Problem::MemoryGrowing, "Memory growing"),
            (Problem::HighCpu, "Sustained high CPU"),
            (Problem::FromTemp, "Running from temp"),
        ];
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("Problems:")).small().color(theme::palette(ui).text_secondary));
            if ui.selectable_label(self.problem_filter.is_none(), tr("All")).clicked() {
                self.problem_filter = None;
            }
            for (prob, label) in counts {
                let n = snap.rows.iter().filter(|r| procs::problems(r, spm).contains(&prob)).count();
                let text = format!("{} ({n})", tr(label));
                if ui
                    .selectable_label(self.problem_filter == Some(prob), text)
                    .on_hover_text(tr(prob.reason()))
                    .clicked()
                {
                    self.problem_filter = if self.problem_filter == Some(prob) { None } else { Some(prob) };
                }
            }
        });

        if self.show_diff
            && let Some(base) = &self.baseline
        {
            let diffs = diff_snapshots(base, &snap.rows);
            widgets::card(ui, icon::GIT_DIFF, tr("Changes since snapshot"), |ui| {
                if diffs.is_empty() {
                    ui.label(RichText::new(tr("No processes started, exited or changed memory significantly.")).weak());
                } else {
                    for d in diffs.iter().take(25) {
                        let (tag, tone) = match d.kind {
                            DiffKind::Added => (tr("Started"), Tone::Success),
                            DiffKind::Exited => (tr("Exited"), Tone::Danger),
                            DiffKind::MemoryChanged => (tr("Memory Δ"), Tone::Warning),
                        };
                        ui.horizontal(|ui| {
                            widgets::pill(ui, tag, tone);
                            ui.label(format!("{} (PID {})", d.name, d.pid));
                            if d.mem_delta != 0 {
                                let mb = d.mem_delta as f64 / (1024.0 * 1024.0);
                                ui.label(RichText::new(format!("{mb:+.1} MB")).monospace());
                            }
                        });
                    }
                }
            });
            ui.add_space(6.0);
        }

        // Prepare rows: filter by problem if active, apply tree ordering if enabled.
        let mut rows: Vec<Row> = snap
            .rows
            .iter()
            .filter(|r| self.problem_filter.is_none_or(|p| procs::problems(r, spm).contains(&p)))
            .cloned()
            .collect();
        if self.tree {
            rows = procs::tree(rows);
        }

        // Split view if details panel is open.
        let selected_row = rows.iter().find(|r| self.table.selected.contains(&r.key())).cloned();
        if self.show_details {
            let avail = ui.available_width();
            let panel_w = (avail * 0.36).clamp(280.0, 440.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(avail - panel_w - theme::GAP, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.render_table(ui, &rows, snap);
                    },
                );
                ui.add_space(theme::GAP);
                ui.allocate_ui_with_layout(
                    egui::vec2(panel_w, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.render_details_panel(ui, selected_row.as_ref(), snap);
                    },
                );
            });
        } else {
            self.render_table(ui, &rows, snap);
        }
    }

    fn render_table(&mut self, ui: &mut Ui, rows: &[Row], snap: &Snapshot) {
        let p = theme::palette(ui);
        let pinned = self.pinned.clone();
        let tree_mode = self.tree;

        let heat = |frac: f64| -> Option<Color32> {
            let f = frac.clamp(0.0, 1.0) as f32;
            if f < 0.03 {
                None
            } else {
                let alpha = (25.0 + f * 95.0) as u8;
                let base = if f > 0.7 {
                    p.danger
                } else if f > 0.3 {
                    p.warning
                } else {
                    p.accent
                };
                Some(Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha))
            }
        };

        let mut t = Table::new("cz_procs", Row::key)
            .col(Column::new("name", tr("Name"), 210.0, move |r: &Row| {
                let pin = if pinned.contains(&r.name) { "📌 " } else { "" };
                let indent =
                    if tree_mode && r.depth > 0 { "  ".repeat(r.depth.min(8)) + "└ " } else { String::new() };
                Cell::Text(format!("{pin}{indent}{}", r.name))
            }))
            .col(Column::new("pid", "PID", 64.0, |r: &Row| Cell::num(r.pid as f64, r.pid.to_string())).right())
            .col(
                Column::new("status", tr("Status"), 135.0, |r: &Row| {
                    let s = if r.not_responding {
                        tr("Not responding")
                    } else if r.suspended {
                        tr("Suspended")
                    } else if r.efficiency {
                        tr("Efficiency mode")
                    } else {
                        ""
                    };
                    Cell::Text(s.into())
                })
                .colored(move |r: &Row| {
                    if r.not_responding {
                        Some(p.danger)
                    } else if r.suspended {
                        Some(p.warning)
                    } else if r.efficiency {
                        Some(p.success)
                    } else {
                        None
                    }
                }),
            )
            .col(
                Column::new("cpu", "CPU", 72.0, |r: &Row| Cell::num(r.cpu, format!("{:.1} %", r.cpu)))
                    .right()
                    .tinted(move |r: &Row| heat(r.cpu / 50.0)),
            )
            .col(Column::new("trend", tr("60s"), 74.0, |r: &Row| Cell::num(r.cpu, format!("{:.0}%", r.cpu))).drawn(
                move |painter, rect, r: &Row| {
                    if r.cpu_history.len() >= 2 {
                        let vals: Vec<f64> = r.cpu_history.iter().map(|&v| v as f64).collect();
                        widgets::sparkline(painter, rect, &vals, p.accent);
                    }
                },
            ))
            .col(
                Column::new("mem", tr("Memory"), 92.0, |r: &Row| {
                    let mb = r.working_set as f64 / (1024.0 * 1024.0);
                    Cell::num(r.working_set as f64, format!("{mb:.1} MB"))
                })
                .right()
                .tinted(move |r: &Row| heat(r.working_set as f64 / (4.0 * 1024.0 * 1024.0 * 1024.0))),
            )
            .col(
                Column::new("private", tr("Private"), 92.0, |r: &Row| {
                    let mb = r.private as f64 / (1024.0 * 1024.0);
                    Cell::num(r.private as f64, format!("{mb:.1} MB"))
                })
                .right(),
            )
            .col(
                Column::new("peak_mem", tr("Peak Mem"), 92.0, |r: &Row| {
                    let mb = r.peak_working_set as f64 / (1024.0 * 1024.0);
                    Cell::num(r.peak_working_set as f64, format!("{mb:.1} MB"))
                })
                .right(),
            )
            .col(
                Column::new("disk", tr("Disk"), 86.0, |r: &Row| {
                    let mb = r.disk_bps() / (1024.0 * 1024.0);
                    Cell::num(r.disk_bps(), format!("{mb:.1} MB/s"))
                })
                .right()
                .tinted(move |r: &Row| heat(r.disk_bps() / (50.0 * 1024.0 * 1024.0))),
            )
            .col(
                Column::new("gpu", "GPU", 70.0, |r: &Row| Cell::num(r.gpu, format!("{:.1} %", r.gpu)))
                    .right()
                    .tinted(move |r: &Row| heat(r.gpu / 80.0)),
            )
            .col(
                Column::new("vram", "VRAM", 84.0, |r: &Row| {
                    let mb = r.vram as f64 / (1024.0 * 1024.0);
                    Cell::num(r.vram as f64, format!("{mb:.0} MB"))
                })
                .right(),
            )
            .col(
                Column::new("top_thread", tr("Top thread"), 100.0, |r: &Row| match r.top_thread {
                    Some((tid, pct)) => Cell::num(pct, format!("T{tid} ({pct:.0}%)")),
                    None => Cell::num(0.0, "–"),
                })
                .right(),
            )
            .col(
                Column::new("threads", tr("Threads"), 68.0, |r: &Row| {
                    Cell::num(r.threads as f64, r.threads.to_string())
                })
                .right(),
            )
            .col(
                Column::new("handles", tr("Handles"), 72.0, |r: &Row| {
                    Cell::num(r.handles as f64, r.handles.to_string())
                })
                .right(),
            )
            .col(
                Column::new("faults", tr("Page faults"), 90.0, |r: &Row| {
                    Cell::num(r.page_faults as f64, r.page_faults.to_string())
                })
                .right(),
            )
            .col(Column::new("priority", tr("Priority"), 95.0, |r: &Row| {
                let pr = Priority::from_base(r.priority, cfg!(unix));
                Cell::Text(tr(pr.label()).into())
            }))
            .col(Column::new("user", tr("User"), 120.0, |r: &Row| Cell::Text(r.user.clone())))
            .col(
                Column::new("session", tr("Session"), 64.0, |r: &Row| match r.session {
                    Some(s) => Cell::num(s as f64, s.to_string()),
                    None => Cell::Text(String::new()),
                })
                .right(),
            )
            .col(Column::new("publisher", tr("Publisher"), 150.0, |r: &Row| Cell::Text(r.publisher.clone())))
            .col(Column::new("path", tr("Path"), 260.0, |r: &Row| Cell::Text(r.path.clone())));

        if self.mode == ProcMode::Grouped && !self.tree {
            let pinned_set = self.pinned.clone();
            t = t.grouped(move |r: &Row| {
                if pinned_set.contains(&r.name) {
                    return tr("Pinned").to_string();
                }
                match r.kind.unwrap_or(Kind::Background) {
                    Kind::App => tr("Apps").to_string(),
                    Kind::Background => tr("Background processes").to_string(),
                    Kind::System => tr("System processes").to_string(),
                }
            });
        }

        let mut ctx_action: Option<CtxCmd> = None;
        let pinned_ref = &self.pinned;
        t.show(ui, rows, &mut self.table, |ui, sel| {
            if sel.is_empty() {
                return;
            }
            let first = sel[0];
            let multi = sel.len() > 1;
            let end_label = if multi { trf("End {} tasks", &[&sel.len()]) } else { tr("End task").to_string() };
            if ui.button(end_label).clicked() {
                ctx_action = Some(CtxCmd::End { tree: false, force: false });
                ui.close();
            }
            if ui.button(tr("End process tree")).clicked() {
                ctx_action = Some(CtxCmd::End { tree: true, force: false });
                ui.close();
            }
            if !multi {
                ui.separator();
                let susp_label = if first.suspended { tr("Resume process") } else { tr("Suspend process") };
                if ui.button(susp_label).clicked() {
                    ctx_action = Some(CtxCmd::Suspend(first.pid, !first.suspended));
                    ui.close();
                }
                let mut eco = first.efficiency;
                if ui.checkbox(&mut eco, tr("Efficiency mode")).changed() {
                    ctx_action = Some(CtxCmd::Eco(first.pid, eco));
                    ui.close();
                }
                ui.menu_button(tr("Set priority"), |ui| {
                    let cur = Priority::from_base(first.priority, cfg!(unix));
                    for pr in Priority::ALL {
                        if ui.selectable_label(cur == pr, tr(pr.label())).clicked() {
                            ctx_action = Some(CtxCmd::Priority(first.pid, pr));
                            ui.close();
                        }
                    }
                });
                if ui.button(tr("Set affinity…")).clicked() {
                    ctx_action = Some(CtxCmd::Affinity(first.pid, first.name.clone()));
                    ui.close();
                }
                ui.separator();
                if ui.button(tr("Analyze wait chain…")).clicked() {
                    ctx_action = Some(CtxCmd::WaitChain(first.pid, first.name.clone()));
                    ui.close();
                }
                // P4: never read an anti-cheat-protected process's memory.
                let protected = procs::anti_cheat(&first.name);
                if ui
                    .add_enabled(!protected, egui::Button::new(tr("Create dump file")))
                    .on_disabled_hover_text(tr("Protected by an anti-cheat: CpuZeeZ never reads its memory."))
                    .clicked()
                {
                    ctx_action = Some(CtxCmd::Dump(first.pid));
                    ui.close();
                }
                if cfg!(windows) && ui.button(tr("Toggle UAC virtualization")).clicked() {
                    ctx_action = Some(CtxCmd::Virtualization(first.pid));
                    ui.close();
                }
                ui.separator();
                let is_pinned = pinned_ref.contains(&first.name);
                let pin_label = if is_pinned { tr("Unpin process") } else { tr("Pin to top") };
                if ui.button(pin_label).clicked() {
                    ctx_action = Some(CtxCmd::TogglePin(first.name.clone()));
                    ui.close();
                }
                if !first.path.is_empty() && ui.button(tr("Open file location")).clicked() {
                    ctx_action = Some(CtxCmd::Reveal(first.path.clone()));
                    ui.close();
                }
                if first.window.is_some() {
                    ui.menu_button(tr("Window"), |ui| {
                        if ui.button(tr("Switch to")).clicked() {
                            ctx_action = Some(CtxCmd::Window(first.pid, desktop::WindowOp::SwitchTo));
                            ui.close();
                        }
                        if ui.button(tr("Minimize")).clicked() {
                            ctx_action = Some(CtxCmd::Window(first.pid, desktop::WindowOp::Minimize));
                            ui.close();
                        }
                        if ui.button(tr("Maximize")).clicked() {
                            ctx_action = Some(CtxCmd::Window(first.pid, desktop::WindowOp::Maximize));
                            ui.close();
                        }
                    });
                }
                if ui.button(tr("Inspect details")).clicked() {
                    ctx_action = Some(CtxCmd::Inspect);
                    ui.close();
                }
            }
        });
        // Double-click (or Enter) opens the details panel on that process.
        if self.table.activated.is_some() {
            self.show_details = true;
        }

        if let Some(cmd) = ctx_action {
            let sel: Vec<&Row> = rows.iter().filter(|r| self.table.selected.contains(&r.key())).collect();
            match cmd {
                CtxCmd::End { tree, force } => self.request_end(&sel, &snap.rows, tree, force),
                CtxCmd::Suspend(pid, on) => {
                    self.dispatch(Action::Process {
                        target: ProcessOp::Suspend { pid, on },
                        elevate: false,
                        identity: Vec::new(),
                    });
                }
                CtxCmd::Eco(pid, on) => {
                    self.dispatch(Action::Process {
                        target: ProcessOp::Eco { pid, on },
                        elevate: false,
                        identity: Vec::new(),
                    });
                }
                CtxCmd::Priority(pid, priority) => {
                    self.dispatch(Action::Process {
                        target: ProcessOp::Priority { pid, priority },
                        elevate: false,
                        identity: Vec::new(),
                    });
                }
                CtxCmd::Affinity(pid, name) => {
                    let n_cpus = std::thread::available_parallelism().map_or(1, usize::from).min(64);
                    let mask = if n_cpus == 64 { u64::MAX } else { (1u64 << n_cpus) - 1 };
                    self.affinity = Some(AffinityModal { pid, name, mask, elevate: false });
                }
                CtxCmd::WaitChain(pid, name) => {
                    let tids: Vec<u32> = os_procs::snapshot(true)
                        .into_iter()
                        .find(|p| p.pid == pid)
                        .map(|p| p.thread_list.into_iter().map(|t| t.tid).collect())
                        .unwrap_or_default();
                    let lines = proc_ops::wait_chain(pid, &tids).unwrap_or_else(|e| vec![e]);
                    self.wait_chain = Some(WaitChainModal { pid, name, lines });
                }
                CtxCmd::Dump(pid) => {
                    self.dispatch(Action::Process {
                        target: ProcessOp::Dump { pid },
                        elevate: false,
                        identity: Vec::new(),
                    });
                }
                CtxCmd::Virtualization(pid) => {
                    self.dispatch(Action::Process {
                        target: ProcessOp::Virtualization { pid, on: true },
                        elevate: false,
                        identity: Vec::new(),
                    });
                }
                CtxCmd::TogglePin(name) => {
                    if !self.pinned.remove(&name) {
                        self.pinned.insert(name);
                    }
                }
                CtxCmd::Reveal(path) => {
                    if let Err(e) = proc_ops::reveal(&path) {
                        self.message = Some((e, Tone::Danger));
                    }
                }
                CtxCmd::Window(pid, op) => {
                    if let Some(w) = snap.windows.iter().find(|w| w.pid == pid)
                        && !desktop::window_op(w.handle, op)
                    {
                        self.message = Some((tr("Window operation failed.").into(), Tone::Danger));
                    }
                }
                CtxCmd::Inspect => {
                    self.show_details = true;
                }
            }
        }
    }

    fn request_end(&mut self, sel: &[&Row], all: &[Row], tree: bool, force: bool) {
        if sel.is_empty() {
            return;
        }
        let mut pids: Vec<u32> = sel.iter().map(|r| r.pid).collect();
        let mut names: Vec<String> = sel.iter().map(|r| format!("{} ({})", r.name, r.pid)).collect();
        if tree {
            // Collect descendants recursively.
            let mut added = true;
            while added {
                added = false;
                for r in all {
                    if pids.contains(&r.ppid) && !pids.contains(&r.pid) {
                        pids.push(r.pid);
                        names.push(format!("{} ({})", r.name, r.pid));
                        added = true;
                    }
                }
            }
        }
        let critical = all.iter().filter(|r| pids.contains(&r.pid)).any(|r| procs::critical(&r.name, r.pid));
        self.confirm_end = Some(ConfirmEnd { pids, names, tree, force, critical, elevate: false });
    }

    fn render_details_panel(&mut self, ui: &mut Ui, row: Option<&Row>, snap: &Snapshot) {
        widgets::card(ui, icon::INFO, tr("Process details"), |ui| {
            let Some(r) = row else {
                ui.label(
                    RichText::new(tr("Select a process to inspect its threads, network sockets and security details."))
                        .weak(),
                );
                return;
            };

            // Refresh on-demand inspector cache when PID changes or every 2s.
            let stale = self.inspector.pid != r.pid
                || self.inspector.last_refresh.is_none_or(|t| t.elapsed() > std::time::Duration::from_secs(2));
            if stale {
                let pid_changed = self.inspector.pid != r.pid;
                self.inspector.pid = r.pid;
                self.inspector.last_refresh = Some(std::time::Instant::now());
                if pid_changed {
                    self.inspector.cmdline = os_procs::command_line(r.pid).unwrap_or_default();
                    self.inspector.sha256 = None;
                }
                if self.inspector.pending.is_none() {
                    let slot = Arc::new(Mutex::new(None));
                    let (out, pid) = (slot.clone(), r.pid);
                    std::thread::spawn(move || {
                        let threads = os_procs::snapshot(true)
                            .into_iter()
                            .find(|p| p.pid == pid)
                            .map(|p| p.thread_list)
                            .unwrap_or_default();
                        *out.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some((pid, threads, proc_ops::connections(pid)));
                    });
                    self.inspector.pending = Some(slot);
                }
            }
            if let Some(slot) = self.inspector.pending.clone() {
                let done = slot.lock().unwrap_or_else(|e| e.into_inner()).take();
                match done {
                    Some((pid, threads, conns)) => {
                        if pid == self.inspector.pid {
                            (self.inspector.threads, self.inspector.conns) = (threads, conns);
                        }
                        self.inspector.pending = None;
                    }
                    None => ui.ctx().request_repaint_after(std::time::Duration::from_millis(100)),
                }
            }

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} (PID {})", r.name, r.pid)).strong());
                if procs::critical(&r.name, r.pid) {
                    widgets::pill(ui, tr("Critical OS process"), Tone::Warning);
                }
            });
            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                for (tab, label) in [
                    (DetailTab::Overview, tr("Overview")),
                    (DetailTab::Threads, tr("Threads")),
                    (DetailTab::Network, tr("Network")),
                    (DetailTab::Windows, tr("Windows")),
                    (DetailTab::Security, tr("Security")),
                ] {
                    if ui.selectable_label(self.detail_tab == tab, label).clicked() {
                        self.detail_tab = tab;
                    }
                }
            });
            ui.separator();

            egui::ScrollArea::vertical().id_salt("proc_detail_scroll").auto_shrink([false, false]).show(ui, |ui| {
                match self.detail_tab {
                    DetailTab::Overview => {
                        let kvs = vec![
                            (tr("Description"), r.description.clone()),
                            (tr("Publisher"), r.publisher.clone()),
                            (tr("Path"), r.path.clone()),
                            (tr("Command line"), self.inspector.cmdline.clone()),
                            (tr("User"), r.user.clone()),
                            (tr("Parent PID"), r.ppid.to_string()),
                            (tr("CPU time"), format!("{:.2} s", r.cpu_time_s)),
                            (tr("Working set"), format!("{:.1} MB", r.working_set as f64 / (1024.0 * 1024.0))),
                            (tr("Private bytes"), format!("{:.1} MB", r.private as f64 / (1024.0 * 1024.0))),
                            (tr("GPU engine"), r.gpu_engine.clone()),
                        ];
                        widgets::kv_list(ui, &kvs);
                    }
                    DetailTab::Threads => {
                        ui.label(
                            RichText::new(trf("{} threads", &[&self.inspector.threads.len()]))
                                .small()
                                .color(theme::palette(ui).text_secondary),
                        );
                        for t in &self.inspector.threads {
                            let secs = t.cpu_ns as f64 / 1e9;
                            let state = if t.suspended { tr("Suspended") } else { tr("Running") };
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("TID {}", t.tid)).monospace());
                                ui.label(RichText::new(format!("{secs:.2} s")).monospace());
                                ui.label(RichText::new(state).small().weak());
                            });
                        }
                    }
                    DetailTab::Network => {
                        if self.inspector.conns.is_empty() {
                            ui.label(RichText::new(tr("No active TCP/UDP sockets for this process.")).weak());
                        } else {
                            for c in &self.inspector.conns {
                                ui.horizontal(|ui| {
                                    widgets::pill(ui, c.proto, Tone::Neutral);
                                    ui.label(RichText::new(format!("{} → {}", c.local, c.remote)).monospace().small());
                                    ui.label(RichText::new(c.state).small().weak());
                                });
                            }
                        }
                    }
                    DetailTab::Windows => {
                        let wins: Vec<&desktop::Window> = snap.windows.iter().filter(|w| w.pid == r.pid).collect();
                        if wins.is_empty() {
                            ui.label(RichText::new(tr("No visible top-level windows.")).weak());
                        } else {
                            for w in wins {
                                ui.horizontal(|ui| {
                                    ui.label(&w.title);
                                    if w.hung {
                                        widgets::pill(ui, tr("Not responding"), Tone::Danger);
                                    }
                                });
                            }
                        }
                    }
                    DetailTab::Security => {
                        let kvs = vec![
                            (tr("Publisher"), r.publisher.clone()),
                            (tr("Account"), r.user.clone()),
                            (tr("Session"), r.session.map_or_else(|| "–".into(), |s| s.to_string())),
                            (
                                tr("Anti-cheat safe"),
                                tr("Queried via kernel snapshot / limited handle only (memory never read)").into(),
                            ),
                        ];
                        widgets::kv_list(ui, &kvs);
                        ui.add_space(6.0);
                        if !r.path.is_empty() {
                            if let Some(hash) = &self.inspector.sha256 {
                                ui.label(RichText::new("SHA-256:").small().weak());
                                ui.label(RichText::new(hash).monospace().small());
                                if ui.small_button(tr("Copy SHA-256")).clicked() {
                                    ui.ctx().copy_text(hash.clone());
                                }
                            } else if ui.button(tr("Compute file SHA-256")).clicked() {
                                self.inspector.sha256 = Some(
                                    crate::update::sha256_file(std::path::Path::new(&r.path))
                                        .unwrap_or_else(|e| e.to_string()),
                                );
                            }
                        }
                    }
                }
            });
        });
    }

    fn modals(&mut self, ui: &mut Ui, _snap: &Snapshot) {
        let ctx = ui.ctx().clone();

        // 1. Confirm End Task modal (with critical process warning).
        if let Some(mut c) = self.confirm_end.take() {
            let mut keep = true;
            let modal = egui::Modal::new(egui::Id::new("cz_confirm_end")).show(&ctx, |ui| {
                ui.set_width(420.0);
                let title = if c.tree { tr("End process tree?") } else { tr("End task?") };
                ui.label(RichText::new(title).text_style(egui::TextStyle::Heading));
                ui.add_space(4.0);
                if c.critical {
                    widgets::pill(
                        ui,
                        tr("Warning: Critical system process — ending it may crash or restart the operating system."),
                        Tone::Danger,
                    );
                    ui.add_space(4.0);
                }
                for n in c.names.iter().take(8) {
                    ui.label(RichText::new(format!("• {n}")).monospace());
                }
                if c.names.len() > 8 {
                    ui.label(trf("…and {} more", &[&(c.names.len() - 8)]));
                }
                ui.add_space(6.0);
                if cfg!(unix) {
                    ui.checkbox(&mut c.force, tr("Force kill immediately (SIGKILL)"));
                }
                ui.checkbox(&mut c.elevate, tr("Run with administrator privileges"));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if widgets::primary_button(ui, tr("End task")).clicked() {
                        self.dispatch(Action::Process {
                            target: ProcessOp::End { pids: c.pids.clone(), force: c.force },
                            elevate: c.elevate,
                            identity: Vec::new(),
                        });
                        keep = false;
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        keep = false;
                    }
                });
            });
            if keep && !modal.should_close() {
                self.confirm_end = Some(c);
            }
        }

        // 2. Set Affinity modal.
        if let Some(mut a) = self.affinity.take() {
            let mut keep = true;
            let n_cpus = std::thread::available_parallelism().map_or(1, usize::from).min(64);
            let modal = egui::Modal::new(egui::Id::new("cz_affinity")).show(&ctx, |ui| {
                ui.set_width(380.0);
                ui.label(RichText::new(trf("Processor affinity — {} ({})", &[&a.name, &a.pid])).strong());
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for i in 0..n_cpus {
                            let mut bit = (a.mask >> i) & 1 == 1;
                            if ui.checkbox(&mut bit, format!("CPU {i}")).changed() {
                                if bit {
                                    a.mask |= 1u64 << i;
                                } else {
                                    a.mask &= !(1u64 << i);
                                }
                            }
                        }
                    });
                });
                ui.checkbox(&mut a.elevate, tr("Run with administrator privileges"));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if widgets::primary_button(ui, tr("Apply")).clicked() {
                        self.dispatch(Action::Process {
                            target: ProcessOp::Affinity { pid: a.pid, mask: a.mask },
                            elevate: a.elevate,
                            identity: Vec::new(),
                        });
                        keep = false;
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        keep = false;
                    }
                });
            });
            if keep && !modal.should_close() {
                self.affinity = Some(a);
            }
        }

        // 3. Analyze Wait Chain modal.
        if let Some(w) = self.wait_chain.take() {
            let mut keep = true;
            let modal = egui::Modal::new(egui::Id::new("cz_wait_chain")).show(&ctx, |ui| {
                ui.set_width(440.0);
                ui.label(RichText::new(trf("Wait chain — {} ({})", &[&w.name, &w.pid])).strong());
                ui.add_space(6.0);
                if w.lines.is_empty() {
                    ui.label(tr("Process is running normally (no blocked threads detected)."));
                } else {
                    for l in &w.lines {
                        ui.label(RichText::new(l).monospace());
                    }
                }
                ui.add_space(8.0);
                if ui.button(tr("Close")).clicked() {
                    keep = false;
                }
            });
            if keep && !modal.should_close() {
                self.wait_chain = Some(w);
            }
        }

        // 4. Run New Task modal.
        if let Some(mut rt) = self.run_task.take() {
            let mut keep = true;
            let modal = egui::Modal::new(egui::Id::new("cz_run_task")).show(&ctx, |ui| {
                ui.set_width(400.0);
                ui.label(RichText::new(tr("Create new task")).text_style(egui::TextStyle::Heading));
                ui.label(RichText::new(tr("Type the name of a program, folder, or command to open.")).weak());
                ui.add_space(6.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut rt.cmd)
                        .hint_text("cmd, powershell, explorer…")
                        .desired_width(f32::INFINITY),
                );
                resp.request_focus();
                ui.checkbox(&mut rt.admin, tr("Create this task with administrative privileges"));
                ui.add_space(8.0);
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    if (widgets::primary_button(ui, tr("Run")).clicked() || enter) && !rt.cmd.trim().is_empty() {
                        match proc_ops::run_task(&rt.cmd, rt.admin) {
                            Ok(()) => self.message = Some((tr("Task started.").into(), Tone::Success)),
                            Err(e) => self.message = Some((e, Tone::Danger)),
                        }
                        keep = false;
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        keep = false;
                    }
                });
            });
            if keep && !modal.should_close() {
                self.run_task = Some(rt);
            }
        }
    }
}

enum CtxCmd {
    End { tree: bool, force: bool },
    Suspend(u32, bool),
    Eco(u32, bool),
    Priority(u32, Priority),
    Affinity(u32, String),
    WaitChain(u32, String),
    Dump(u32),
    Virtualization(u32),
    TogglePin(String),
    Reveal(String),
    Window(u32, desktop::WindowOp),
    Inspect,
}

pub const PT_BR: &[(&str, &str)] = &[
    (
        "Protected by an anti-cheat: CpuZeeZ never reads its memory.",
        "Protegido por um anti-cheat: o CpuZeeZ nunca lê a memória dele.",
    ),
    ("System", "Sistema"),
    ("{} processes · {} threads · {} handles", "{} processos · {} threads · {} handles"),
    ("Resume", "Retomar"),
    ("Pause", "Pausar"),
    ("Pause or resume live updates (Space)", "Pausar ou retomar atualizações ao vivo (Espaço)"),
    ("High (0.5 s)", "Alta (0,5 s)"),
    ("Normal (1 s)", "Normal (1 s)"),
    ("Low (4 s)", "Baixa (4 s)"),
    ("Run new task", "Executar nova tarefa"),
    ("Processes", "Processos"),
    ("Performance", "Desempenho"),
    ("App history", "Histórico de apps"),
    ("Startup", "Inicializar"),
    ("Users", "Usuários"),
    ("Services", "Serviços"),
    ("Action completed.", "Ação concluída."),
    ("Action cancelled.", "Ação cancelada."),
    ("Action failed: {}", "Falha na ação: {}"),
    ("Per-device live performance graphs (S2a-06).", "Gráficos de desempenho em tempo real por dispositivo (S2a-06)."),
    ("Per-application resource usage history (S2a-10).", "Histórico de uso de recursos por aplicativo (S2a-10)."),
    ("Startup applications and boot impact (S2a-08).", "Aplicativos de inicialização e impacto no boot (S2a-08)."),
    (
        "Signed-in user sessions and resource usage (S2a-09).",
        "Sessões de usuários conectados e uso de recursos (S2a-09).",
    ),
    ("System services and daemons (S2a-07).", "Serviços do sistema e daemons (S2a-07)."),
    ("Grouped", "Agrupado"),
    ("Detailed", "Detalhado"),
    ("Tree", "Árvore"),
    ("Show parent → child hierarchy", "Mostrar hierarquia pai → filho"),
    ("Preset", "Predefinição"),
    ("Essential", "Essencial"),
    ("Gaming", "Jogos"),
    ("Leak hunting", "Caça a vazamentos"),
    ("Security", "Segurança"),
    ("Everything", "Tudo"),
    ("Details panel", "Painel de detalhes"),
    ("Snapshot", "Snapshot"),
    ("Compare", "Comparar"),
    ("Export", "Exportar"),
    ("Export CSV", "Exportar CSV"),
    ("Export JSON", "Exportar JSON"),
    ("End task", "Finalizar tarefa"),
    ("End {} tasks", "Finalizar {} tarefas"),
    ("Problems:", "Problemas:"),
    ("All", "Todos"),
    ("Not responding", "Não está respondendo"),
    ("Memory growing", "Memória crescendo"),
    ("Sustained high CPU", "CPU alta contínua"),
    ("Running from temp", "Executando de pasta temp"),
    ("Memory keeps growing", "A memória continua crescendo"),
    ("High CPU for over a minute", "CPU alta por mais de um minuto"),
    ("Runs from a temporary folder", "Executa a partir de uma pasta temporária"),
    ("Changes since snapshot", "Alterações desde o snapshot"),
    (
        "No processes started, exited or changed memory significantly.",
        "Nenhum processo iniciou, encerrou ou mudou de memória significativamente.",
    ),
    ("Started", "Iniciado"),
    ("Exited", "Encerrado"),
    ("Memory Δ", "Memória Δ"),
    ("Status", "Status"),
    ("Suspended", "Suspenso"),
    ("Efficiency mode", "Modo de eficiência"),
    ("60s", "60s"),
    ("Private", "Privada"),
    ("Peak Mem", "Pico Mem"),
    ("Disk", "Disco"),
    ("Top thread", "Thread mais pesada"),
    ("Threads", "Threads"),
    ("Handles", "Handles"),
    ("Page faults", "Falhas de página"),
    ("Priority", "Prioridade"),
    ("Idle", "Ociosa"),
    ("Below normal", "Abaixo do normal"),
    ("Normal", "Normal"),
    ("Above normal", "Acima do normal"),
    ("High", "Alta"),
    ("Realtime", "Tempo real"),
    ("User", "Usuário"),
    ("Session", "Sessão"),
    ("Publisher", "Fornecedor"),
    ("Path", "Caminho"),
    ("Pinned", "Fixados"),
    ("Apps", "Aplicativos"),
    ("Background processes", "Processos em segundo plano"),
    ("System processes", "Processos do sistema"),
    ("End process tree", "Finalizar árvore de processos"),
    ("Resume process", "Retomar processo"),
    ("Suspend process", "Suspender processo"),
    ("Set priority", "Definir prioridade"),
    ("Set affinity…", "Definir afinidade…"),
    ("Analyze wait chain…", "Analisar cadeia de espera…"),
    ("Create dump file", "Criar arquivo de despejo"),
    ("Toggle UAC virtualization", "Alternar virtualização UAC"),
    ("Unpin process", "Desafixar processo"),
    ("Pin to top", "Fixar no topo"),
    ("Open file location", "Abrir local do arquivo"),
    ("Window", "Janela"),
    ("Switch to", "Alternar para"),
    ("Minimize", "Minimizar"),
    ("Maximize", "Maximizar"),
    ("Inspect details", "Inspecionar detalhes"),
    ("Process details", "Detalhes do processo"),
    (
        "Select a process to inspect its threads, network sockets and security details.",
        "Selecione um processo para inspecionar suas threads, conexões de rede e detalhes de segurança.",
    ),
    ("Critical OS process", "Processo crítico do SO"),
    ("Overview", "Visão geral"),
    ("Network", "Rede"),
    ("Windows", "Janelas"),
    ("Description", "Descrição"),
    ("Command line", "Linha de comando"),
    ("Parent PID", "PID pai"),
    ("CPU time", "Tempo de CPU"),
    ("Working set", "Working set"),
    ("Private bytes", "Bytes privados"),
    ("GPU engine", "Motor de GPU"),
    ("{} threads", "{} threads"),
    ("Running", "Em execução"),
    ("No active TCP/UDP sockets for this process.", "Nenhum socket TCP/UDP ativo para este processo."),
    ("No visible top-level windows.", "Nenhuma janela principal visível."),
    ("Account", "Conta"),
    ("Anti-cheat safe", "Seguro para anti-cheat"),
    (
        "Queried via kernel snapshot / limited handle only (memory never read)",
        "Consultado apenas via snapshot do kernel / handle limitado (memória nunca lida)",
    ),
    ("Copy SHA-256", "Copiar SHA-256"),
    ("Compute file SHA-256", "Calcular SHA-256 do arquivo"),
    ("End process tree?", "Finalizar árvore de processos?"),
    ("End task?", "Finalizar tarefa?"),
    (
        "Warning: Critical system process — ending it may crash or restart the operating system.",
        "Aviso: Processo crítico do sistema — finalizá-lo pode travar ou reiniciar o sistema operacional.",
    ),
    ("…and {} more", "…e mais {}"),
    ("Force kill immediately (SIGKILL)", "Forçar encerramento imediato (SIGKILL)"),
    ("Run with administrator privileges", "Executar com privilégios de administrador"),
    ("Cancel", "Cancelar"),
    ("Processor affinity — {} ({})", "Afinidade do processador — {} ({})"),
    ("Apply", "Aplicar"),
    ("Wait chain — {} ({})", "Cadeia de espera — {} ({})"),
    (
        "Process is running normally (no blocked threads detected).",
        "O processo está em execução normal (nenhuma thread bloqueada detectada).",
    ),
    ("Close", "Fechar"),
    ("Create new task", "Criar nova tarefa"),
    (
        "Type the name of a program, folder, or command to open.",
        "Digite o nome de um programa, pasta ou comando para abrir.",
    ),
    ("Create this task with administrative privileges", "Criar esta tarefa com privilégios administrativos"),
    ("Run", "Executar"),
    ("Task started.", "Tarefa iniciada."),
    ("Window operation failed.", "Falha na operação da janela."),
];

pub const ES: &[(&str, &str)] = &[
    (
        "Protected by an anti-cheat: CpuZeeZ never reads its memory.",
        "Protegido por un anti-cheat: CpuZeeZ nunca lee su memoria.",
    ),
    ("System", "Sistema"),
    ("{} processes · {} threads · {} handles", "{} procesos · {} hilos · {} identificadores"),
    ("Resume", "Reanudar"),
    ("Pause", "Pausar"),
    ("Pause or resume live updates (Space)", "Pausar o reanudar actualizaciones en vivo (Espacio)"),
    ("High (0.5 s)", "Alta (0,5 s)"),
    ("Normal (1 s)", "Normal (1 s)"),
    ("Low (4 s)", "Baja (4 s)"),
    ("Run new task", "Ejecutar nueva tarea"),
    ("Processes", "Procesos"),
    ("Performance", "Rendimiento"),
    ("App history", "Historial de aplicaciones"),
    ("Startup", "Inicio"),
    ("Users", "Usuarios"),
    ("Services", "Servicios"),
    ("Action completed.", "Acción completada."),
    ("Action cancelled.", "Acción cancelada."),
    ("Action failed: {}", "Error en la acción: {}"),
    ("Per-device live performance graphs (S2a-06).", "Gráficos de rendimiento en vivo por dispositivo (S2a-06)."),
    ("Per-application resource usage history (S2a-10).", "Historial de uso de recursos por aplicación (S2a-10)."),
    ("Startup applications and boot impact (S2a-08).", "Aplicaciones de inicio e impacto en el arranque (S2a-08)."),
    (
        "Signed-in user sessions and resource usage (S2a-09).",
        "Sesiones de usuario conectadas y uso de recursos (S2a-09).",
    ),
    ("System services and daemons (S2a-07).", "Servicios del sistema y demonios (S2a-07)."),
    ("Grouped", "Agrupado"),
    ("Detailed", "Detallado"),
    ("Tree", "Árbol"),
    ("Show parent → child hierarchy", "Mostrar jerarquía padre → hijo"),
    ("Preset", "Preajuste"),
    ("Essential", "Esencial"),
    ("Gaming", "Juegos"),
    ("Leak hunting", "Caza de fugas"),
    ("Security", "Seguridad"),
    ("Everything", "Todo"),
    ("Details panel", "Panel de detalles"),
    ("Snapshot", "Instantánea"),
    ("Compare", "Comparar"),
    ("Export", "Exportar"),
    ("Export CSV", "Exportar CSV"),
    ("Export JSON", "Exportar JSON"),
    ("End task", "Finalizar tarea"),
    ("End {} tasks", "Finalizar {} tareas"),
    ("Problems:", "Problemas:"),
    ("All", "Todos"),
    ("Not responding", "No responde"),
    ("Memory growing", "Memoria creciendo"),
    ("Sustained high CPU", "CPU alta sostenida"),
    ("Running from temp", "Ejecutando desde temp"),
    ("Memory keeps growing", "La memoria sigue creciendo"),
    ("High CPU for over a minute", "CPU alta durante más de un minuto"),
    ("Runs from a temporary folder", "Se ejecuta desde una carpeta temporal"),
    ("Changes since snapshot", "Cambios desde la instantánea"),
    (
        "No processes started, exited or changed memory significantly.",
        "Ningún proceso se inició, terminó o cambió su memoria significativamente.",
    ),
    ("Started", "Iniciado"),
    ("Exited", "Terminado"),
    ("Memory Δ", "Memoria Δ"),
    ("Status", "Estado"),
    ("Suspended", "Suspendido"),
    ("Efficiency mode", "Modo de eficiencia"),
    ("60s", "60s"),
    ("Private", "Privada"),
    ("Peak Mem", "Pico Mem"),
    ("Disk", "Disco"),
    ("Top thread", "Hilo más pesado"),
    ("Threads", "Hilos"),
    ("Handles", "Identificadores"),
    ("Page faults", "Fallos de página"),
    ("Priority", "Prioridad"),
    ("Idle", "Inactiva"),
    ("Below normal", "Por debajo de lo normal"),
    ("Normal", "Normal"),
    ("Above normal", "Por encima de lo normal"),
    ("High", "Alta"),
    ("Realtime", "Tiempo real"),
    ("User", "Usuario"),
    ("Session", "Sesión"),
    ("Publisher", "Editor"),
    ("Path", "Ruta"),
    ("Pinned", "Fijados"),
    ("Apps", "Aplicaciones"),
    ("Background processes", "Procesos en segundo plano"),
    ("System processes", "Procesos del sistema"),
    ("End process tree", "Finalizar árbol de procesos"),
    ("Resume process", "Reanudar proceso"),
    ("Suspend process", "Suspender proceso"),
    ("Set priority", "Establecer prioridad"),
    ("Set affinity…", "Establecer afinidad…"),
    ("Analyze wait chain…", "Analizar cadena de espera…"),
    ("Create dump file", "Crear archivo de volcado"),
    ("Toggle UAC virtualization", "Alternar virtualización UAC"),
    ("Unpin process", "Desfijar proceso"),
    ("Pin to top", "Fijar arriba"),
    ("Open file location", "Abrir ubicación del archivo"),
    ("Window", "Ventana"),
    ("Switch to", "Cambiar a"),
    ("Minimize", "Minimizar"),
    ("Maximize", "Maximizar"),
    ("Inspect details", "Inspeccionar detalles"),
    ("Process details", "Detalles del proceso"),
    (
        "Select a process to inspect its threads, network sockets and security details.",
        "Seleccione un proceso para inspeccionar sus hilos, conexiones de red y detalles de seguridad.",
    ),
    ("Critical OS process", "Proceso crítico del SO"),
    ("Overview", "Resumen"),
    ("Network", "Red"),
    ("Windows", "Ventanas"),
    ("Description", "Descripción"),
    ("Command line", "Línea de comandos"),
    ("Parent PID", "PID padre"),
    ("CPU time", "Tiempo de CPU"),
    ("Working set", "Working set"),
    ("Private bytes", "Bytes privados"),
    ("GPU engine", "Motor de GPU"),
    ("{} threads", "{} hilos"),
    ("Running", "En ejecución"),
    ("No active TCP/UDP sockets for this process.", "No hay sockets TCP/UDP activos para este proceso."),
    ("No visible top-level windows.", "No hay ventanas principales visibles."),
    ("Account", "Cuenta"),
    ("Anti-cheat safe", "Seguro para anti-cheat"),
    (
        "Queried via kernel snapshot / limited handle only (memory never read)",
        "Consultado solo mediante instantánea del kernel / descriptor limitado (nunca se lee la memoria)",
    ),
    ("Copy SHA-256", "Copiar SHA-256"),
    ("Compute file SHA-256", "Calcular SHA-256 del archivo"),
    ("End process tree?", "Finalizar árbol de procesos?"),
    ("End task?", "Finalizar tarea?"),
    (
        "Warning: Critical system process — ending it may crash or restart the operating system.",
        "Advertencia: Proceso crítico del sistema — finalizarlo puede bloquear o reiniciar el sistema operativo.",
    ),
    ("…and {} more", "…y {} más"),
    ("Force kill immediately (SIGKILL)", "Forzar cierre inmediato (SIGKILL)"),
    ("Run with administrator privileges", "Ejecutar con privilegios de administrador"),
    ("Cancel", "Cancelar"),
    ("Processor affinity — {} ({})", "Afinidad del procesador — {} ({})"),
    ("Apply", "Aplicar"),
    ("Wait chain — {} ({})", "Cadena de espera — {} ({})"),
    (
        "Process is running normally (no blocked threads detected).",
        "El proceso se ejecuta normalmente (no se detectaron hilos bloqueados).",
    ),
    ("Close", "Cerrar"),
    ("Create new task", "Crear nueva tarea"),
    (
        "Type the name of a program, folder, or command to open.",
        "Escriba el nombre de un programa, carpeta o comando para abrir.",
    ),
    ("Create this task with administrative privileges", "Crear esta tarea con privilegios administrativos"),
    ("Run", "Ejecutar"),
    ("Task started.", "Tarea iniciada."),
    ("Window operation failed.", "Error en la operación de ventana."),
];

#[cfg(test)]
mod tests {
    use super::*;
    use cpuzeez_core::procs::Totals;

    fn sample_rows() -> Vec<Row> {
        vec![
            Row {
                pid: 100,
                ppid: 1,
                start_ms: 1000,
                name: "game.exe".into(),
                cpu: 42.5,
                working_set: 512 << 20,
                private: 400 << 20,
                kind: Some(Kind::App),
                not_responding: true,
                ..Default::default()
            },
            Row {
                pid: 200,
                ppid: 100,
                start_ms: 2000,
                name: "helper.exe".into(),
                cpu: 2.0,
                working_set: 64 << 20,
                private: 50 << 20,
                kind: Some(Kind::Background),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn csv_and_json_export_and_snapshot_diff() {
        let r1 = sample_rows();
        let csv = export_csv(&r1);
        assert!(csv.contains("game.exe") && csv.contains("helper.exe"));
        let json = export_json(&r1);
        assert!(json.contains("\"pid\": 100"));

        let mut r2 = r1.clone();
        r2.pop(); // helper exited
        r2[0].private += 32 << 20; // game grew by 32 MiB
        r2.push(Row { pid: 300, start_ms: 3000, name: "new.exe".into(), private: 10 << 20, ..Default::default() });
        let d = diff_snapshots(&r1, &r2);
        assert!(d.iter().any(|x| x.pid == 300 && x.kind == DiffKind::Added));
        assert!(d.iter().any(|x| x.pid == 200 && x.kind == DiffKind::Exited));
        assert!(d.iter().any(|x| x.pid == 100 && x.kind == DiffKind::MemoryChanged));
    }

    #[test]
    fn processes_table_renders_and_filters_problems() {
        use egui_kittest::{Harness, kittest::Queryable};
        let rows = sample_rows();
        let snap =
            Snapshot { totals: Totals { processes: rows.len(), threads: 12, handles: 300 }, rows, windows: Vec::new() };
        let mut h = Harness::builder().with_size(egui::vec2(1080.0, 680.0)).build_ui_state(
            move |ui, st: &mut State| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                st.page_with_snapshot(ui, &snap);
            },
            State::default(),
        );
        h.run();
        h.run();
        // Click the "Not responding (1)" problem filter pill.
        h.get_by_label("Not responding (1)").click();
        h.run();
        assert_eq!(h.state().problem_filter, Some(Problem::NotResponding));
    }
}

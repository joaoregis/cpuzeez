//! System › Services tab (S2a-07, docs/v1/task-manager.md §8):
//! Cross-platform service manager (Windows SCM, Linux systemd, macOS launchd)
//! with quick filters (Running, Stopped-Auto, Disabled, Third-party, Failed recently),
//! dependency graph & stop confirmation, protected-service lock, and undo via audit log.

use super::i18n::tr;
use super::table::{self, Cell, Column, Table};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::executor::{self, Done};
use cpuzeez_core::actions::{Action, ServiceOp};
use cpuzeez_core::services::{
    Recovery, RecoveryAction, ServiceAccount, ServiceDetails, ServiceFilter, ServiceRow, ServiceState, StartupType,
    matches_filter, transitive_dependents,
};
use eframe::egui::{self, CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Modal {
    ConfirmStop { name: String, restart: bool, dependents: Vec<String> },
}

pub struct State {
    rows: Arc<Mutex<Vec<ServiceRow>>>,
    busy_refresh: Arc<AtomicBool>,
    last_refresh: Option<Instant>,
    pub filter: ServiceFilter,
    pub table: table::State,
    pub selected_name: Option<String>,
    edit_startup: Option<(String, StartupType)>,
    edit_account: Option<(String, ServiceAccount)>,
    edit_recovery: Option<(String, Recovery)>,
    start_params: String,
    /// (service, its details) read in the background when the inspector opens it.
    details: Arc<Mutex<Option<(String, ServiceDetails)>>>,
    details_for: Option<String>,
    logs_for: Option<String>,
    logs_buf: Arc<Mutex<Vec<String>>>,
    busy_logs: Arc<AtomicBool>,
    pub undo_stack: Vec<(String, Action)>,
    pending_op: Arc<Mutex<Option<String>>>,
    status_banner: Option<(String, Tone)>,
    modal: Option<Modal>,
}

impl Default for State {
    fn default() -> Self {
        let mut t = table::State::default();
        t.sort = Some(("name".into(), true));
        Self {
            rows: Arc::new(Mutex::new(Vec::new())),
            busy_refresh: Arc::new(AtomicBool::new(false)),
            last_refresh: None,
            filter: ServiceFilter::All,
            table: t,
            selected_name: None,
            edit_startup: None,
            edit_account: None,
            edit_recovery: None,
            start_params: String::new(),
            details: Arc::new(Mutex::new(None)),
            details_for: None,
            logs_for: None,
            logs_buf: Arc::new(Mutex::new(Vec::new())),
            busy_logs: Arc::new(AtomicBool::new(false)),
            undo_stack: Vec::new(),
            pending_op: Arc::new(Mutex::new(None)),
            status_banner: None,
            modal: None,
        }
    }
}

impl State {
    pub fn show(&mut self, ui: &mut Ui) {
        let pending = self.pending_op.lock().unwrap().take();
        if let Some(msg) = pending {
            let tone = if msg.starts_with("Error:") { Tone::Danger } else { Tone::Success };
            self.status_banner = Some((msg, tone));
            self.trigger_refresh();
        }
        let should_poll = self.last_refresh.is_none_or(|t| t.elapsed() >= Duration::from_secs(6));
        if should_poll && !self.busy_refresh.load(Ordering::Relaxed) {
            self.trigger_refresh();
        }
        let snap = self.rows.lock().unwrap().clone();
        self.show_with_rows(ui, &snap);
    }

    fn trigger_refresh(&mut self) {
        if self.busy_refresh.swap(true, Ordering::SeqCst) {
            return;
        }
        self.last_refresh = Some(Instant::now());
        let rows = Arc::clone(&self.rows);
        let busy = Arc::clone(&self.busy_refresh);
        std::thread::Builder::new()
            .name("cz-services".into())
            .spawn(move || {
                let list = cpuzeez_platform::services::list();
                *rows.lock().unwrap() = list;
                busy.store(false, Ordering::SeqCst);
            })
            .ok();
    }

    pub fn show_with_rows(&mut self, ui: &mut Ui, all_rows: &[ServiceRow]) {
        let p = theme::palette(ui);

        // Toolbar: Quick filter pills + Refresh + Undo
        ui.horizontal_wrapped(|ui| {
            for f in ServiceFilter::ALL {
                let count = all_rows.iter().filter(|r| matches_filter(r, f, "")).count();
                let label = format!("{} ({count})", tr(f.label()));
                if ui.selectable_label(self.filter == f, label).clicked() {
                    self.filter = f;
                }
            }
            ui.separator();
            if ui.button(format!("{} {}", icon::ARROW_COUNTER_CLOCKWISE, tr("Refresh"))).clicked() {
                self.trigger_refresh();
            }
            if let Some((desc, _)) = self.undo_stack.last() {
                let label = format!("{} {}: {desc}", icon::ARROW_COUNTER_CLOCKWISE, tr("Undo"));
                if ui.button(label).clicked()
                    && let Some((_, act)) = self.undo_stack.pop()
                {
                    self.dispatch_action(act);
                }
            }
        });

        if let Some((msg, tone)) = self.status_banner.clone() {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                widgets::pill(ui, &msg, tone);
                if ui.small_button(icon::X).clicked() {
                    self.status_banner = None;
                }
            });
        }
        ui.add_space(6.0);

        let visible: Vec<ServiceRow> =
            all_rows.iter().filter(|r| matches_filter(r, self.filter, "")).cloned().collect();

        // Sync selected_name with table.selected if user clicked a row
        if let Some(&first_key) = self.table.selected.iter().next()
            && let Some(r) = all_rows.iter().find(|r| hash_name(&r.name) == first_key)
        {
            self.selected_name = Some(r.name.clone());
        }

        let avail_w = ui.available_width();
        let show_inspector = self.selected_name.is_some() && avail_w > 700.0;
        let table_w = if show_inspector { (avail_w - 370.0).max(340.0) } else { avail_w };

        let mut context_act: Option<(ServiceRow, &'static str)> = None;

        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(table_w, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    if visible.is_empty() {
                        widgets::empty_state(
                            ui,
                            icon::GEAR_SIX,
                            tr("No matching services"),
                            tr("Try clearing the search box or switching the quick filter."),
                        );
                    } else {
                        Table::new("sys_services_table", |r: &ServiceRow| hash_name(&r.name))
                            .col(Column::new("name", tr("Name"), 170.0, |r: &ServiceRow| {
                                if r.protected {
                                    Cell::Text(format!("{} {}", icon::LOCK, r.name))
                                } else {
                                    Cell::Text(r.name.clone())
                                }
                            }))
                            .col(Column::new("display", tr("Display name"), 220.0, |r: &ServiceRow| {
                                Cell::Text(r.display_name.clone())
                            }))
                            .col(
                                Column::new("status", tr("Status"), 95.0, |r: &ServiceRow| {
                                    Cell::Text(tr(r.state.label()).to_string())
                                })
                                .colored(move |r: &ServiceRow| match r.state {
                                    ServiceState::Running => Some(p.success),
                                    ServiceState::StartPending | ServiceState::StopPending | ServiceState::Paused => {
                                        Some(p.warning)
                                    }
                                    ServiceState::Stopped if r.failed_recently => Some(p.danger),
                                    _ => None,
                                }),
                            )
                            .col(
                                Column::new("pid", "PID", 68.0, |r: &ServiceRow| match r.pid {
                                    Some(pid) => Cell::num(pid as f64, pid.to_string()),
                                    None => Cell::num(0.0, "–"),
                                })
                                .right(),
                            )
                            .col(Column::new("startup", tr("Startup type"), 135.0, |r: &ServiceRow| {
                                Cell::Text(tr(r.startup.label()).to_string())
                            }))
                            .col(Column::new("account", tr("Log on as"), 130.0, |r: &ServiceRow| {
                                Cell::Text(if r.account.is_empty() { "–".into() } else { r.account.clone() })
                            }))
                            .col(Column::new("group", tr("Group"), 120.0, |r: &ServiceRow| {
                                Cell::Text(if r.group.is_empty() { "–".into() } else { r.group.clone() })
                            }))
                            .show(ui, &visible, &mut self.table, |ui, sel| {
                                if let Some(&first) = sel.first() {
                                    ui.label(RichText::new(&first.name).strong());
                                    ui.separator();
                                    ui.add_enabled_ui(!first.protected && first.state == ServiceState::Stopped, |ui| {
                                        if ui.button(tr("Start")).clicked() {
                                            context_act = Some(((*first).clone(), "start"));
                                            ui.close();
                                        }
                                    });
                                    ui.add_enabled_ui(!first.protected && first.state == ServiceState::Running, |ui| {
                                        if ui.button(tr("Stop")).clicked() {
                                            context_act = Some(((*first).clone(), "stop"));
                                            ui.close();
                                        }
                                        if ui.button(tr("Restart")).clicked() {
                                            context_act = Some(((*first).clone(), "restart"));
                                            ui.close();
                                        }
                                    });
                                }
                            });
                    }
                },
            );

            if show_inspector
                && let Some(sel_name) = self.selected_name.clone()
                && let Some(row) = all_rows.iter().find(|r| r.name == sel_name)
            {
                ui.add_space(8.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.render_inspector(ui, row, all_rows),
                );
            }
        });

        if let Some((row, verb)) = context_act {
            match verb {
                "start" => {
                    let name = row.name.clone();
                    self.undo_stack.push((
                        format!("Stop {name}"),
                        Action::Service { target: ServiceOp::Stop { name: name.clone() }, elevate: true },
                    ));
                    self.dispatch_action(Action::Service { target: ServiceOp::Start { name }, elevate: true });
                }
                "stop" | "restart" => {
                    let deps = transitive_dependents(all_rows, &row.name);
                    let restart = verb == "restart";
                    if deps.is_empty() {
                        let op = if restart {
                            ServiceOp::Restart { name: row.name.clone() }
                        } else {
                            ServiceOp::Stop { name: row.name.clone() }
                        };
                        self.dispatch_action(Action::Service { target: op, elevate: true });
                    } else {
                        self.modal = Some(Modal::ConfirmStop { name: row.name.clone(), restart, dependents: deps });
                    }
                }
                _ => {}
            }
        }

        self.render_modal(ui);
    }

    fn render_inspector(&mut self, ui: &mut Ui, row: &ServiceRow, all_rows: &[ServiceRow]) {
        let p = theme::palette(ui);
        Frame::default()
            .fill(p.card)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(CornerRadius::same(theme::RADIUS_CARD))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.set_width(345.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&row.name).strong().color(p.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(icon::X).clicked() {
                            self.selected_name = None;
                            self.table.selected.clear();
                        }
                    });
                });
                if !row.display_name.is_empty() {
                    ui.label(RichText::new(&row.display_name).small().color(p.text_secondary));
                }
                ui.add_space(6.0);

                ScrollArea::vertical().id_salt("svc_inspector_scroll").show(ui, |ui| {
                    if row.protected {
                        Frame::default()
                            .fill(p.warning_soft)
                            .corner_radius(CornerRadius::same(theme::RADIUS_CONTROL))
                            .inner_margin(Margin::same(8))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("{} {}", icon::LOCK, tr("Protected system service")))
                                        .strong()
                                        .color(p.warning),
                                );
                                ui.label(
                                    RichText::new(tr(
                                        "This service is required for core operating system stability and security. Stopping or disabling it is locked.",
                                    ))
                                    .small()
                                    .color(p.text),
                                );
                            });
                        ui.add_space(6.0);
                    }

                    // Control buttons
                    ui.horizontal_wrapped(|ui| {
                        ui.add_enabled_ui(!row.protected && row.state == ServiceState::Stopped, |ui| {
                            if widgets::primary_button(ui, tr("Start")).clicked() {
                                let name = row.name.clone();
                                self.undo_stack.push((
                                    format!("Stop {name}"),
                                    Action::Service { target: ServiceOp::Stop { name: name.clone() }, elevate: true },
                                ));
                                self.dispatch_action(Action::Service {
                                    target: ServiceOp::Start { name },
                                    elevate: true,
                                });
                            }
                        });
                        ui.add_enabled_ui(!row.protected && row.state == ServiceState::Running, |ui| {
                            if ui.button(tr("Stop")).clicked() {
                                let deps = transitive_dependents(all_rows, &row.name);
                                if deps.is_empty() {
                                    let name = row.name.clone();
                                    self.undo_stack.push((
                                        format!("Start {name}"),
                                        Action::Service {
                                            target: ServiceOp::Start { name: name.clone() },
                                            elevate: true,
                                        },
                                    ));
                                    self.dispatch_action(Action::Service {
                                        target: ServiceOp::Stop { name },
                                        elevate: true,
                                    });
                                } else {
                                    self.modal = Some(Modal::ConfirmStop {
                                        name: row.name.clone(),
                                        restart: false,
                                        dependents: deps,
                                    });
                                }
                            }
                            if ui.button(tr("Restart")).clicked() {
                                let deps = transitive_dependents(all_rows, &row.name);
                                if deps.is_empty() {
                                    self.dispatch_action(Action::Service {
                                        target: ServiceOp::Restart { name: row.name.clone() },
                                        elevate: true,
                                    });
                                } else {
                                    self.modal = Some(Modal::ConfirmStop {
                                        name: row.name.clone(),
                                        restart: true,
                                        dependents: deps,
                                    });
                                }
                            }
                        });
                    });

                    ui.add_space(8.0);
                    // Startup type editor
                    let cur_edit = match &self.edit_startup {
                        Some((n, st)) if n == &row.name => *st,
                        _ => row.startup,
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("Startup type")).small().color(p.text_secondary));
                        egui::ComboBox::from_id_salt("svc_startup_combo")
                            .selected_text(tr(cur_edit.label()))
                            .show_ui(ui, |ui| {
                                for st in StartupType::EDITABLE {
                                    if ui.selectable_label(cur_edit == st, tr(st.label())).clicked() {
                                        self.edit_startup = Some((row.name.clone(), st));
                                    }
                                }
                            });
                        if !row.protected
                            && cur_edit != row.startup
                            && widgets::primary_button(ui, tr("Apply")).clicked()
                        {
                            let prev = row.startup;
                            let name = row.name.clone();
                            self.undo_stack.push((
                                format!("{name} -> {}", prev.label()),
                                Action::Service {
                                    target: ServiceOp::SetStartup { name: name.clone(), startup: prev },
                                    elevate: true,
                                },
                            ));
                            self.dispatch_action(Action::Service {
                                target: ServiceOp::SetStartup { name, startup: cur_edit },
                                elevate: true,
                            });
                        }
                    });

                    self.render_config(ui, row);

                    ui.add_space(8.0);
                    if !row.description.is_empty() {
                        ui.label(RichText::new(&row.description).small().color(p.text_secondary));
                        ui.add_space(6.0);
                    }

                    let mut kvs = vec![
                        (tr("Status"), tr(row.state.label()).to_string()),
                        ("PID", row.pid.map_or_else(|| "–".into(), |p| p.to_string())),
                        (tr("Log on as"), if row.account.is_empty() { "–".into() } else { row.account.clone() }),
                        (tr("Group"), if row.group.is_empty() { "–".into() } else { row.group.clone() }),
                        (
                            tr("Third-party"),
                            if row.third_party { tr("Yes").into() } else { tr("No (OS built-in)").into() },
                        ),
                    ];
                    if !row.failure_recovery.is_empty() {
                        kvs.push((tr("Failure recovery"), row.failure_recovery.clone()));
                    }
                    if let Some(err) = &row.last_error {
                        kvs.push((tr("Last error"), err.clone()));
                    }
                    widgets::kv_list(ui, &kvs);

                    if !row.binary_path.is_empty() {
                        ui.add_space(6.0);
                        ui.label(RichText::new(tr("Binary path")).small().color(p.text_secondary));
                        ui.label(RichText::new(&row.binary_path).monospace().small());
                    }

                    // Dependency graph
                    ui.add_space(10.0);
                    ui.label(RichText::new(tr("Dependency graph")).strong().color(p.text));
                    ui.add_space(4.0);
                    ui.label(RichText::new(tr("This service depends on:")).small().color(p.text_secondary));
                    if row.dependencies.is_empty() {
                        ui.label(RichText::new(tr("None")).small().color(p.text_muted));
                    } else {
                        for dep in &row.dependencies {
                            if ui.small_button(format!("{} {dep}", icon::ARROW_ELBOW_DOWN_RIGHT)).clicked() {
                                self.selected_name = Some(dep.clone());
                                self.table.selected.clear();
                            }
                        }
                    }
                    ui.add_space(4.0);
                    let trans = transitive_dependents(all_rows, &row.name);
                    ui.label(RichText::new(tr("Stopping this service also affects:")).small().color(p.text_secondary));
                    if trans.is_empty() {
                        ui.label(RichText::new(tr("No other services depend on this")).small().color(p.text_muted));
                    } else {
                        for dep in &trans {
                            if ui.small_button(format!("{} {dep}", icon::ARROW_RIGHT)).clicked() {
                                self.selected_name = Some(dep.clone());
                                self.table.selected.clear();
                            }
                        }
                    }

                    // Service logs
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("Recent service logs")).strong().color(p.text));
                        if ui.button(tr("Load logs")).clicked()
                            && !self.busy_logs.swap(true, Ordering::SeqCst)
                        {
                            let target = row.name.clone();
                            self.logs_for = Some(target.clone());
                            let buf = Arc::clone(&self.logs_buf);
                            let busy = Arc::clone(&self.busy_logs);
                            std::thread::spawn(move || {
                                let lines = cpuzeez_platform::services::logs(&target, 15);
                                *buf.lock().unwrap() = lines;
                                busy.store(false, Ordering::SeqCst);
                            });
                        }
                    });
                    if self.logs_for.as_deref() == Some(row.name.as_str()) {
                        let lines = self.logs_buf.lock().unwrap();
                        if lines.is_empty() {
                            ui.label(RichText::new(tr("No recent log entries found.")).small().color(p.text_muted));
                        } else {
                            for l in lines.iter() {
                                ui.label(RichText::new(l).monospace().small().color(p.text_secondary));
                            }
                        }
                    }
                });
            });
    }

    /// Details read on demand: triggers, SID type, privileges, recovery.
    fn details_of(&mut self, name: &str) -> Option<ServiceDetails> {
        if self.details_for.as_deref() != Some(name) {
            self.details_for = Some(name.to_string());
            *self.details.lock().unwrap_or_else(|e| e.into_inner()) = None;
            let slot = Arc::clone(&self.details);
            let n = name.to_string();
            std::thread::spawn(move || {
                let d = cpuzeez_platform::services::details(&n);
                *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some((n, d));
            });
        }
        self.details.lock().unwrap_or_else(|e| e.into_inner()).as_ref().filter(|d| d.0 == name).map(|d| d.1.clone())
    }

    /// Start parameters, log-on account and failure recovery editors, plus
    /// the read-only triggers, SID type and privileges.
    fn render_config(&mut self, ui: &mut Ui, row: &ServiceRow) {
        let p = theme::palette(ui);
        let windows = cfg!(windows);
        let editable = !row.protected;

        if windows && row.state == ServiceState::Stopped && editable {
            ui.add_space(8.0);
            ui.label(RichText::new(tr("Start parameters")).small().color(p.text_secondary));
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.start_params).desired_width(200.0).hint_text("-v /debug"));
                if ui.button(tr("Start with parameters")).clicked() {
                    let params: Vec<String> = self.start_params.split_whitespace().map(str::to_string).collect();
                    self.dispatch_action(Action::Service {
                        target: ServiceOp::StartWithParams { name: row.name.clone(), params },
                        elevate: true,
                    });
                }
            });
        }

        if windows {
            ui.add_space(8.0);
            let current = ServiceAccount::from_name(&row.account);
            let picked = match &self.edit_account {
                Some((n, a)) if n == &row.name => Some(*a),
                _ => current,
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Log on as")).small().color(p.text_secondary));
                let shown = picked.map_or_else(|| row.account.clone(), |a| a.windows_name().to_string());
                ui.add_enabled_ui(editable, |ui| {
                    egui::ComboBox::from_id_salt("svc_account_combo").selected_text(shown).show_ui(ui, |ui| {
                        for a in ServiceAccount::ALL {
                            if ui.selectable_label(picked == Some(a), a.windows_name()).clicked() {
                                self.edit_account = Some((row.name.clone(), a));
                            }
                        }
                    });
                });
                if editable
                    && let Some(a) = picked
                    && picked != current
                    && widgets::primary_button(ui, tr("Apply")).clicked()
                {
                    if let Some(prev) = current {
                        self.undo_stack.push((
                            format!("{} -> {}", row.name, prev.windows_name()),
                            Action::Service {
                                target: ServiceOp::SetAccount { name: row.name.clone(), account: prev },
                                elevate: true,
                            },
                        ));
                    }
                    self.dispatch_action(Action::Service {
                        target: ServiceOp::SetAccount { name: row.name.clone(), account: a },
                        elevate: true,
                    });
                    self.edit_account = None;
                }
            });
            ui.label(
                RichText::new(tr("Accounts with a password are set in Windows' Services tool (services.msc)."))
                    .small()
                    .color(p.text_muted),
            );
        }

        let details = self.details_of(&row.name);
        let Some(d) = details else {
            ui.label(RichText::new(tr("Reading the service's configuration…")).small().color(p.text_muted));
            return;
        };

        if let Some(saved) = d.recovery
            && (windows || cfg!(target_os = "linux"))
        {
            ui.add_space(8.0);
            ui.label(RichText::new(tr("Recovery")).strong().color(p.text));
            let mut r = match &self.edit_recovery {
                Some((n, r)) if n == &row.name => *r,
                _ => saved,
            };
            let program = [r.first, r.second, r.subsequent].contains(&RecoveryAction::RunProgram);
            let steps: Vec<(&str, &mut RecoveryAction)> = if windows {
                vec![
                    (tr("First failure"), &mut r.first),
                    (tr("Second failure"), &mut r.second),
                    (tr("Subsequent failures"), &mut r.subsequent),
                ]
            } else {
                vec![(tr("On failure"), &mut r.first)]
            };
            egui::Grid::new(("svc_recovery", &row.name)).num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                for (i, (label, act)) in steps.into_iter().enumerate() {
                    ui.label(RichText::new(label).small().color(p.text_secondary));
                    ui.add_enabled_ui(editable && !program, |ui| {
                        egui::ComboBox::from_id_salt(("svc_recovery_step", i)).selected_text(tr(act.label())).show_ui(
                            ui,
                            |ui| {
                                for a in RecoveryAction::EDITABLE {
                                    ui.selectable_value(act, a, tr(a.label()));
                                }
                            },
                        );
                    });
                    ui.end_row();
                }
                ui.label(RichText::new(tr("Restart after (s)")).small().color(p.text_secondary));
                ui.add_enabled(editable && !program, egui::DragValue::new(&mut r.restart_delay_s).range(0..=86_400));
                ui.end_row();
                if windows {
                    ui.label(RichText::new(tr("Reset count after (days)")).small().color(p.text_secondary));
                    ui.add_enabled(editable && !program, egui::DragValue::new(&mut r.reset_days).range(0..=3650));
                    ui.end_row();
                }
            });
            if !windows {
                r.second = r.first;
                r.subsequent = r.first;
            }
            if program {
                let cmd =
                    if d.recovery_command.is_empty() { String::new() } else { format!(": {}", d.recovery_command) };
                ui.label(
                    RichText::new(format!(
                        "{}{cmd}",
                        tr("Runs a program on failure; change it in Windows' Services tool.")
                    ))
                    .small()
                    .color(p.text_muted),
                );
            }
            if r != saved {
                self.edit_recovery = Some((row.name.clone(), r));
                if editable && widgets::primary_button(ui, tr("Apply")).clicked() {
                    self.undo_stack.push((
                        format!("{} recovery", row.name),
                        Action::Service {
                            target: ServiceOp::SetRecovery { name: row.name.clone(), recovery: saved },
                            elevate: true,
                        },
                    ));
                    self.dispatch_action(Action::Service {
                        target: ServiceOp::SetRecovery { name: row.name.clone(), recovery: r },
                        elevate: true,
                    });
                    self.edit_recovery = None;
                    self.details_for = None; // read again after the change
                }
            }
        }

        let mut kvs: Vec<(&str, String)> = Vec::new();
        if !d.sid_type.is_empty() {
            kvs.push((tr("Service SID"), tr(&d.sid_type).to_string()));
        }
        kvs.push((tr("Triggers"), if d.triggers.is_empty() { tr("None").to_string() } else { d.triggers.join("\n") }));
        kvs.push((
            tr("Required privileges"),
            if d.privileges.is_empty() { tr("None").to_string() } else { d.privileges.join("\n") },
        ));
        ui.add_space(8.0);
        widgets::kv_list(ui, &kvs);
    }

    fn render_modal(&mut self, ui: &mut Ui) {
        let Some(modal) = self.modal.clone() else { return };
        let Modal::ConfirmStop { name, restart, dependents } = modal;
        let mut close = false;
        egui::Window::new(tr("Confirm service action"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ui.ctx(), |ui| {
                ui.set_max_width(420.0);
                ui.label(tr("The following dependent services will also be affected:"));
                ui.add_space(4.0);
                for d in &dependents {
                    ui.label(RichText::new(format!("{} {d}", icon::DOT_OUTLINE)).monospace());
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if widgets::primary_button(ui, tr("Proceed")).clicked() {
                        let op = if restart {
                            ServiceOp::Restart { name: name.clone() }
                        } else {
                            ServiceOp::Stop { name: name.clone() }
                        };
                        self.dispatch_action(Action::Service { target: op, elevate: true });
                        close = true;
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        close = true;
                    }
                });
            });
        if close {
            self.modal = None;
        }
    }

    fn dispatch_action(&self, action: Action) {
        let slot = Arc::clone(&self.pending_op);
        std::thread::spawn(move || {
            let msg = match executor::run(&action) {
                Done::Ok(s) if !s.is_empty() => s,
                Done::Ok(_) => action.describe(),
                Done::Cancelled => return,
                Done::Failed(e) => format!("Error: {e}"),
            };
            *slot.lock().unwrap() = Some(msg);
        });
    }
}

fn hash_name(s: &str) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub const PT_BR: &[(&str, &str)] = &[
    ("Stopped (Auto)", "Parado (Auto)"),
    ("Third-party", "Terceiros"),
    ("Failed recently", "Falhou recentemente"),
    ("Undo", "Desfazer"),
    ("Display name", "Nome de exibição"),
    ("Startup type", "Tipo de inicialização"),
    ("Log on as", "Fazer logon como"),
    ("Group", "Grupo"),
    ("Starting", "Iniciando"),
    ("Stopping", "Parando"),
    ("Paused", "Pausado"),
    ("Automatic", "Automático"),
    ("Automatic (Delayed)", "Automático (Atraso na Inicialização)"),
    ("Manual", "Manual"),
    ("Static", "Estático"),
    ("Masked", "Mascarado"),
    ("No matching services", "Nenhum serviço correspondente"),
    ("Try clearing the search box or switching the quick filter.", "Tente limpar a busca ou alternar o filtro rápido."),
    ("Protected system service", "Serviço protegido do sistema"),
    (
        "This service is required for core operating system stability and security. Stopping or disabling it is locked.",
        "Este serviço é essencial para a estabilidade e segurança do sistema operacional. Parar ou desativar está bloqueado.",
    ),
    ("Start", "Iniciar"),
    ("Stop", "Parar"),
    ("Restart", "Reiniciar"),
    ("Apply", "Aplicar"),
    ("No (OS built-in)", "Não (nativo do SO)"),
    ("Failure recovery", "Recuperação de falhas"),
    ("Last error", "Último erro"),
    ("Binary path", "Caminho do executável"),
    ("Dependency graph", "Grafo de dependências"),
    ("This service depends on:", "Este serviço depende de:"),
    ("Stopping this service also affects:", "Parar este serviço também afeta:"),
    ("No other services depend on this", "Nenhum outro serviço depende deste"),
    ("Recent service logs", "Logs recentes do serviço"),
    ("Load logs", "Carregar logs"),
    ("No recent log entries found.", "Nenhuma entrada de log recente encontrada."),
    ("Confirm service action", "Confirmar ação no serviço"),
    (
        "The following dependent services will also be affected:",
        "Os seguintes serviços dependentes também serão afetados:",
    ),
    ("Proceed", "Prosseguir"),
];

pub const ES: &[(&str, &str)] = &[
    ("Stopped (Auto)", "Detenido (Auto)"),
    ("Third-party", "De terceros"),
    ("Failed recently", "Falló recientemente"),
    ("Undo", "Deshacer"),
    ("Display name", "Nombre para mostrar"),
    ("Startup type", "Tipo de inicio"),
    ("Log on as", "Iniciar sesión como"),
    ("Group", "Grupo"),
    ("Starting", "Iniciando"),
    ("Stopping", "Deteniendo"),
    ("Paused", "En pausa"),
    ("Automatic", "Automático"),
    ("Automatic (Delayed)", "Automático (Inicio retrasado)"),
    ("Manual", "Manual"),
    ("Static", "Estático"),
    ("Masked", "Enmascarado"),
    ("No matching services", "No hay servicios coincidentes"),
    (
        "Try clearing the search box or switching the quick filter.",
        "Intente borrar el cuadro de búsqueda o cambiar el filtro rápido.",
    ),
    ("Protected system service", "Servicio del sistema protegido"),
    (
        "This service is required for core operating system stability and security. Stopping or disabling it is locked.",
        "Este servicio es necesario para la estabilidad y seguridad del sistema operativo. Detenerlo o deshabilitarlo está bloqueado.",
    ),
    ("Start", "Iniciar"),
    ("Stop", "Detener"),
    ("Restart", "Reiniciar"),
    ("Apply", "Aplicar"),
    ("No (OS built-in)", "No (integrado en el SO)"),
    ("Failure recovery", "Recuperación ante fallos"),
    ("Last error", "Último error"),
    ("Binary path", "Ruta del ejecutable"),
    ("Dependency graph", "Gráfico de dependencias"),
    ("This service depends on:", "Este servicio depende de:"),
    ("Stopping this service also affects:", "Detener este servicio también afecta a:"),
    ("No other services depend on this", "Ningún otro servicio depende de este"),
    ("Recent service logs", "Registros recientes del servicio"),
    ("Load logs", "Cargar registros"),
    ("No recent log entries found.", "No se encontraron entradas de registro recientes."),
    ("Confirm service action", "Confirmar acción del servicio"),
    (
        "The following dependent services will also be affected:",
        "Los siguientes servicios dependientes también se verán afectados:",
    ),
    ("Proceed", "Continuar"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn services_tab_filters_and_inspects_protected_service() {
        use egui_kittest::{Harness, kittest::Queryable};
        let rows = vec![
            ServiceRow {
                name: "RpcSs".into(),
                display_name: "Remote Procedure Call (RPC)".into(),
                state: ServiceState::Running,
                startup: StartupType::Automatic,
                pid: Some(888),
                protected: true,
                ..Default::default()
            },
            ServiceRow {
                name: "Spooler".into(),
                display_name: "Print Spooler".into(),
                state: ServiceState::Stopped,
                startup: StartupType::AutomaticDelayed,
                dependencies: vec!["RpcSs".into()],
                ..Default::default()
            },
        ];
        let mut h = Harness::builder().with_size(egui::vec2(1100.0, 650.0)).build_ui_state(
            move |ui, st: &mut State| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                st.show_with_rows(ui, &rows);
            },
            State { selected_name: Some("RpcSs".into()), ..State::default() },
        );
        h.run();
        h.run();
        h.get_by_label_contains("Protected system service");
        h.get_by_label_contains("Stopped (Auto) (1)").click();
        h.run();
        assert_eq!(h.state().filter, ServiceFilter::StoppedAuto);
    }
}

//! System › Users tab (S2a-09, docs/v1/task-manager.md §7):
//! Session list (Windows WTS, Linux logind, macOS utmpx/who), per-user aggregated
//! CPU / Memory / Disk / GPU usage, expandable per-user process list, and
//! Disconnect / Sign out / Send message / Manage user accounts actions.

use super::i18n::{tr, trf};
use super::table::{self, Cell, Column, Table};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::procs::Snapshot as ProcSnap;
use cpuzeez_core::procs::Row;
use cpuzeez_platform::users::{self, UserSession};
use eframe::egui::{self, CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct UserAggregate {
    pub user: String,
    pub session_id: Option<u32>,
    pub state: String,
    pub client_name: String,
    pub cpu_pct: f64,
    pub mem_bytes: u64,
    pub disk_bps: f64,
    pub gpu_pct: f64,
    pub processes: Vec<Row>,
}

#[derive(Clone, Debug)]
enum Modal {
    SendMessage { session_id: u32, user: String, title: String, body: String },
    ConfirmAction { session_id: u32, user: String, sign_out: bool },
}

pub struct State {
    sessions: Arc<Mutex<Vec<UserSession>>>,
    busy_refresh: Arc<AtomicBool>,
    last_refresh: Option<Instant>,
    pub expanded: BTreeSet<String>,
    pub selected_user: Option<String>,
    pub show_system_accounts: bool,
    proc_table: table::State,
    modal: Option<Modal>,
    status_banner: Option<(String, Tone)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(Vec::new())),
            busy_refresh: Arc::new(AtomicBool::new(false)),
            last_refresh: None,
            expanded: BTreeSet::new(),
            selected_user: None,
            show_system_accounts: false,
            proc_table: table::State::default(),
            modal: None,
            status_banner: None,
        }
    }
}

pub fn aggregate_users(sessions: &[UserSession], procs: &ProcSnap, include_system: bool) -> Vec<UserAggregate> {
    let mut map: BTreeMap<String, UserAggregate> = BTreeMap::new();
    for s in sessions {
        let key = s.user.to_ascii_lowercase();
        map.insert(
            key,
            UserAggregate {
                user: s.full_account(),
                session_id: Some(s.session_id),
                state: s.state.clone(),
                client_name: s.client_name.clone(),
                cpu_pct: 0.0,
                mem_bytes: 0,
                disk_bps: 0.0,
                gpu_pct: 0.0,
                processes: Vec::new(),
            },
        );
    }

    for r in &procs.rows {
        let raw_user = if r.user.is_empty() { "SYSTEM" } else { r.user.as_str() };
        let short_user = raw_user.rsplit('\\').next().unwrap_or(raw_user);
        if !include_system && is_service_account(short_user) && !map.contains_key(&short_user.to_ascii_lowercase()) {
            continue;
        }
        let key = short_user.to_ascii_lowercase();
        let entry = map.entry(key).or_insert_with(|| UserAggregate {
            user: raw_user.to_string(),
            session_id: r.session.filter(|&s| s > 0),
            state: if is_service_account(short_user) { "Service".into() } else { "Active".into() },
            client_name: "Console".into(),
            cpu_pct: 0.0,
            mem_bytes: 0,
            disk_bps: 0.0,
            gpu_pct: 0.0,
            processes: Vec::new(),
        });
        entry.cpu_pct += r.cpu;
        entry.mem_bytes = entry.mem_bytes.saturating_add(r.working_set);
        entry.disk_bps += r.disk_bps();
        entry.gpu_pct += r.gpu;
        entry.processes.push(r.clone());
    }

    let mut out: Vec<UserAggregate> = map.into_values().collect();
    for u in &mut out {
        u.processes.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
    }
    out.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct));
    out
}

fn is_service_account(u: &str) -> bool {
    matches!(
        u.to_ascii_uppercase().as_str(),
        "SYSTEM" | "LOCAL SERVICE" | "NETWORK SERVICE" | "DWM-1" | "DWM-2" | "UMFD-0" | "UMFD-1" | "ROOT"
    )
}

impl State {
    pub fn show(&mut self, ui: &mut Ui, procs: &ProcSnap) {
        let should_poll = self.last_refresh.is_none_or(|t| t.elapsed() >= Duration::from_secs(6));
        if should_poll && !self.busy_refresh.load(Ordering::Relaxed) {
            self.trigger_refresh();
        }
        let sess = self.sessions.lock().unwrap().clone();
        self.show_with_sessions(ui, &sess, procs);
    }

    fn trigger_refresh(&mut self) {
        if self.busy_refresh.swap(true, Ordering::SeqCst) {
            return;
        }
        self.last_refresh = Some(Instant::now());
        let sess = Arc::clone(&self.sessions);
        let busy = Arc::clone(&self.busy_refresh);
        std::thread::Builder::new()
            .name("cz-users".into())
            .spawn(move || {
                let list = users::sessions();
                *sess.lock().unwrap() = list;
                busy.store(false, Ordering::SeqCst);
            })
            .ok();
    }

    pub fn show_with_sessions(&mut self, ui: &mut Ui, sessions: &[UserSession], procs: &ProcSnap) {
        let p = theme::palette(ui);
        let users_list = aggregate_users(sessions, procs, self.show_system_accounts);

        // Top toolbar
        ui.horizontal_wrapped(|ui| {
            widgets::pill(ui, &trf("{} active user(s)", &[&users_list.len()]), Tone::Accent);
            ui.checkbox(&mut self.show_system_accounts, tr("Include system & service accounts"));
            ui.separator();
            if ui.button(format!("{} {}", icon::ARROW_COUNTER_CLOCKWISE, tr("Refresh"))).clicked() {
                self.trigger_refresh();
            }
            if ui.button(format!("{} {}", icon::USERS, tr("Manage user accounts"))).clicked() {
                let _ = users::manage_accounts();
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

        ui.add_space(8.0);

        if users_list.is_empty() {
            widgets::empty_state(
                ui,
                icon::USERS,
                tr("No user sessions found"),
                tr("Enable 'Include system & service accounts' to inspect background accounts."),
            );
            return;
        }

        ScrollArea::vertical().id_salt("sys_users_scroll").show(ui, |ui| {
            for u in &users_list {
                let is_expanded = self.expanded.contains(&u.user);
                let is_selected = self.selected_user.as_deref() == Some(u.user.as_str());
                let stroke = if is_selected { Stroke::new(1.5, p.accent) } else { Stroke::new(1.0, p.border) };
                Frame::default()
                    .fill(p.card)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(theme::RADIUS_CARD))
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let caret = if is_expanded { icon::CARET_DOWN } else { icon::CARET_RIGHT };
                            if ui.button(format!("{caret} {} ({})", u.user, u.processes.len())).clicked() {
                                if is_expanded {
                                    self.expanded.remove(&u.user);
                                } else {
                                    self.expanded.insert(u.user.clone());
                                }
                                self.selected_user = Some(u.user.clone());
                            }
                            let state_tone = if u.state == "Active" { Tone::Success } else { Tone::Neutral };
                            widgets::pill(ui, tr(&u.state), state_tone);
                            if let Some(sid) = u.session_id {
                                let client = if u.client_name.is_empty() {
                                    String::new()
                                } else {
                                    format!(" · {}", u.client_name)
                                };
                                ui.label(
                                    RichText::new(format!("{}: {sid}{client}", tr("Session")))
                                        .small()
                                        .color(p.text_secondary),
                                );
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let Some(sid) = u.session_id else { return };
                                if ui.small_button(tr("Sign out")).clicked() {
                                    self.modal = Some(Modal::ConfirmAction {
                                        session_id: sid,
                                        user: u.user.clone(),
                                        sign_out: true,
                                    });
                                }
                                if ui.small_button(tr("Disconnect")).clicked() {
                                    self.modal = Some(Modal::ConfirmAction {
                                        session_id: sid,
                                        user: u.user.clone(),
                                        sign_out: false,
                                    });
                                }
                                if ui.small_button(tr("Send message")).clicked() {
                                    self.modal = Some(Modal::SendMessage {
                                        session_id: sid,
                                        user: u.user.clone(),
                                        title: String::from("Message from Administrator"),
                                        body: String::new(),
                                    });
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 18.0;
                            let mem = format!("{:.0} MB", u.mem_bytes as f64 / 1_048_576.0);
                            let disk = format!("{:.1} MB/s", u.disk_bps / 1_048_576.0);
                            let stats = [
                                ("CPU", format!("{:.1} %", u.cpu_pct)),
                                (tr("Memory"), mem),
                                (tr("Disk"), disk),
                                ("GPU", format!("{:.0} %", u.gpu_pct)),
                            ];
                            for (k, v) in stats {
                                ui.label(RichText::new(k).small().color(p.text_muted));
                                ui.add_space(-12.0);
                                ui.label(RichText::new(v).monospace().color(p.text));
                            }
                        });

                        if is_expanded && !u.processes.is_empty() {
                            ui.add_space(6.0);
                            let row_h = ui.text_style_height(&egui::TextStyle::Body) + 8.0;
                            let h = (u.processes.len() as f32 * row_h + 3.0 * row_h).min(360.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), h),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    Table::new("sys_user_procs", |r: &Row| r.pid as u64)
                                        .col(Column::new("name", tr("Name"), 240.0, |r: &Row| {
                                            Cell::Text(r.name.clone())
                                        }))
                                        .col(
                                            Column::new("pid", "PID", 70.0, |r: &Row| {
                                                Cell::num(r.pid as f64, r.pid.to_string())
                                            })
                                            .right(),
                                        )
                                        .col(
                                            Column::new("cpu", "CPU", 80.0, |r: &Row| {
                                                Cell::num(r.cpu, format!("{:.1} %", r.cpu))
                                            })
                                            .right(),
                                        )
                                        .col(
                                            Column::new("mem", tr("Memory"), 100.0, |r: &Row| {
                                                Cell::num(
                                                    r.working_set as f64,
                                                    format!("{:.1} MB", r.working_set as f64 / 1_048_576.0),
                                                )
                                            })
                                            .right(),
                                        )
                                        .show(ui, &u.processes, &mut self.proc_table, |_, _| {});
                                },
                            );
                        }
                    });
                ui.add_space(6.0);
            }
        });

        self.render_modal(ui);
    }

    fn render_modal(&mut self, ui: &mut Ui) {
        let Some(mut modal) = self.modal.take() else { return };
        let mut keep = true;
        match &mut modal {
            Modal::SendMessage { session_id, user, title, body } => {
                egui::Window::new(tr("Send message"))
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ui.ctx(), |ui| {
                        ui.set_min_width(340.0);
                        ui.label(trf("To: {} (session {})", &[user, session_id]));
                        ui.add_space(4.0);
                        ui.label(tr("Title:"));
                        ui.text_edit_singleline(title);
                        ui.add_space(4.0);
                        ui.label(tr("Message:"));
                        ui.text_edit_multiline(body);
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if widgets::primary_button(ui, tr("Send")).clicked() {
                                match users::send_message(*session_id, title, body) {
                                    Ok(m) => self.status_banner = Some((m, Tone::Success)),
                                    Err(e) => self.status_banner = Some((e, Tone::Danger)),
                                }
                                keep = false;
                            }
                            if ui.button(tr("Cancel")).clicked() {
                                keep = false;
                            }
                        });
                    });
            }
            Modal::ConfirmAction { session_id, user, sign_out } => {
                let title_str = if *sign_out { tr("Sign out user") } else { tr("Disconnect user") };
                egui::Window::new(title_str)
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ui.ctx(), |ui| {
                        ui.set_max_width(360.0);
                        let body = if *sign_out {
                            trf(
                                "Sign out {} (session {})? Unsaved work in that session may be lost.",
                                &[user, session_id],
                            )
                        } else {
                            trf("Disconnect {} (session {})?", &[user, session_id])
                        };
                        ui.label(body);
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if widgets::primary_button(ui, title_str).clicked() {
                                let res = if *sign_out {
                                    users::sign_out(*session_id)
                                } else {
                                    users::disconnect(*session_id)
                                };
                                match res {
                                    Ok(m) => self.status_banner = Some((m, Tone::Success)),
                                    Err(e) => self.status_banner = Some((e, Tone::Danger)),
                                }
                                keep = false;
                            }
                            if ui.button(tr("Cancel")).clicked() {
                                keep = false;
                            }
                        });
                    });
            }
        }
        if keep {
            self.modal = Some(modal);
        }
    }
}

pub const PT_BR: &[(&str, &str)] = &[
    ("{} active user(s)", "{} usuário(s) ativo(s)"),
    ("Include system & service accounts", "Incluir contas do sistema e de serviço"),
    ("Manage user accounts", "Gerenciar contas de usuário"),
    ("No user sessions found", "Nenhuma sessão de usuário encontrada"),
    (
        "Enable 'Include system & service accounts' to inspect background accounts.",
        "Ative 'Incluir contas do sistema e de serviço' para inspecionar contas em segundo plano.",
    ),
    ("Active", "Ativo"),
    ("Connected", "Conectado"),
    ("Disconnected", "Desconectado"),
    ("Service", "Serviço"),
    ("Sign out", "Fazer logoff"),
    ("Disconnect", "Desconectar"),
    ("Send message", "Enviar mensagem"),
    ("To: {} (session {})", "Para: {} (sessão {})"),
    ("Title:", "Título:"),
    ("Message:", "Mensagem:"),
    ("Sign out user", "Fazer logoff do usuário"),
    ("Disconnect user", "Desconectar usuário"),
    (
        "Sign out {} (session {})? Unsaved work in that session may be lost.",
        "Fazer logoff de {} (sessão {})? Trabalhos não salvos nessa sessão podem ser perdidos.",
    ),
    ("Disconnect {} (session {})?", "Desconectar {} (sessão {})?"),
];

pub const ES: &[(&str, &str)] = &[
    ("{} active user(s)", "{} usuario(s) activo(s)"),
    ("Include system & service accounts", "Incluir cuentas del sistema y de servicio"),
    ("Manage user accounts", "Administrar cuentas de usuario"),
    ("No user sessions found", "No se encontraron sesiones de usuario"),
    (
        "Enable 'Include system & service accounts' to inspect background accounts.",
        "Habilite 'Incluir cuentas del sistema y de servicio' para inspeccionar cuentas en segundo plano.",
    ),
    ("Active", "Activo"),
    ("Connected", "Conectado"),
    ("Disconnected", "Desconectado"),
    ("Service", "Servicio"),
    ("Sign out", "Cerrar sesión"),
    ("Disconnect", "Desconectar"),
    ("Send message", "Enviar mensaje"),
    ("To: {} (session {})", "Para: {} (sesión {})"),
    ("Title:", "Título:"),
    ("Message:", "Mensaje:"),
    ("Sign out user", "Cerrar sesión del usuario"),
    ("Disconnect user", "Desconectar usuario"),
    (
        "Sign out {} (session {})? Unsaved work in that session may be lost.",
        "¿Cerrar sesión de {} (sesión {})? El trabajo no guardado en esa sesión puede perderse.",
    ),
    ("Disconnect {} (session {})?", "¿Desconectar a {} (sesión {})?"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn users_tab_aggregates_processes_and_expands_user() {
        use egui_kittest::{Harness, kittest::Queryable};
        let sessions = vec![UserSession {
            session_id: 1,
            user: "joao".into(),
            domain: "PC".into(),
            state: "Active".into(),
            client_name: "Console".into(),
        }];
        let mut procs = ProcSnap::default();
        procs.rows.push(Row {
            pid: 4242,
            name: "cpuzeez.exe".into(),
            user: "PC\\joao".into(),
            session: Some(1),
            cpu: 12.5,
            working_set: 100 * 1_048_576,
            ..Default::default()
        });
        let mut h = Harness::builder().with_size(egui::vec2(1050.0, 600.0)).build_ui_state(
            move |ui, st: &mut State| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                st.show_with_sessions(ui, &sessions, &procs);
            },
            State::default(),
        );
        h.run();
        h.run();
        h.get_by_label_contains("PC\\joao (1)").click();
        h.run();
        assert!(h.state().expanded.contains("PC\\joao"));
        h.get_by_label_contains("cpuzeez.exe");
    }
}

//! System › Startup tab (S2a-08, docs/v1/task-manager.md §6):
//! Registry Run/RunOnce + Startup folders + `StartupApproved` (with Date disabled),
//! Linux XDG autostart, macOS LaunchAgents, Last BIOS time (`FwPOSTTime` / FPDT),
//! WDI startup impact, visual boot timeline, and undo via the audit log.

use super::i18n::{tr, trf};
use super::table::{self, Cell, Column, Table};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use cpuzeez_core::actions::{Action, StartupOp};
use cpuzeez_core::startup::{BootSummary, Impact, StartupRow};
use eframe::egui::{self, CornerRadius, Frame, Margin, RichText, Sense, Stroke, Ui, vec2};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct State {
    summary: Arc<Mutex<BootSummary>>,
    busy_refresh: Arc<AtomicBool>,
    last_refresh: Option<Instant>,
    pub table: table::State,
    pub selected_id: Option<String>,
    pub show_timeline: bool,
    pub undo_stack: Vec<(String, Action)>,
    pending_op: Arc<Mutex<Option<super::system::Finished>>>,
    status_banner: Option<(String, Tone)>,
}

impl Default for State {
    fn default() -> Self {
        let mut t = table::State::default();
        t.sort = Some(("name".into(), true));
        Self {
            summary: Arc::new(Mutex::new(BootSummary::default())),
            busy_refresh: Arc::new(AtomicBool::new(false)),
            last_refresh: None,
            table: t,
            selected_id: None,
            show_timeline: false,
            undo_stack: Vec::new(),
            pending_op: Arc::new(Mutex::new(None)),
            status_banner: None,
        }
    }
}

impl State {
    pub fn show(&mut self, ui: &mut Ui) {
        let pending = self.pending_op.lock().unwrap().take();
        if let Some(f) = pending {
            if f.ok {
                self.undo_stack.extend(f.undo);
                self.status_banner = Some((f.msg, Tone::Success));
            } else {
                self.status_banner = Some((super::system::failure_text(&f.msg), Tone::Danger));
            }
            self.trigger_refresh();
        }
        let should_poll = self.last_refresh.is_none_or(|t| t.elapsed() >= Duration::from_secs(10));
        if should_poll && !self.busy_refresh.load(Ordering::Relaxed) {
            self.trigger_refresh();
        }
        let snap = self.summary.lock().unwrap().clone();
        self.show_with_summary(ui, &snap);
    }

    fn trigger_refresh(&mut self) {
        if self.busy_refresh.swap(true, Ordering::SeqCst) {
            return;
        }
        self.last_refresh = Some(Instant::now());
        let sum = Arc::clone(&self.summary);
        let busy = Arc::clone(&self.busy_refresh);
        std::thread::Builder::new()
            .name("cz-startup".into())
            .spawn(move || {
                let s = cpuzeez_platform::startup::query();
                *sum.lock().unwrap() = s;
                busy.store(false, Ordering::SeqCst);
            })
            .ok();
    }

    pub fn show_with_summary(&mut self, ui: &mut Ui, sum: &BootSummary) {
        let p = theme::palette(ui);

        // Header strip: Last BIOS time, enabled/disabled count, Boot timeline toggle, Refresh, Undo
        ui.horizontal_wrapped(|ui| {
            let bios_str = match sum.bios_time_ms {
                Some(ms) => trf("Last BIOS time: {} s", &[&format!("{:.1}", ms as f64 / 1000.0)]),
                None => format!("{}: –", tr("Last BIOS time")),
            };
            widgets::pill(ui, &bios_str, Tone::Accent);

            let enabled_n = sum.items.iter().filter(|r| r.enabled).count();
            let disabled_n = sum.items.len().saturating_sub(enabled_n);
            widgets::pill(ui, &trf("{} enabled · {} disabled", &[&enabled_n, &disabled_n]), Tone::Neutral);

            if let Some(note) = &sum.impact_source_note {
                widgets::pill(ui, note, Tone::Neutral);
            }

            ui.separator();
            if ui.selectable_label(self.show_timeline, format!("{} {}", icon::CHART_BAR, tr("Boot timeline"))).clicked()
            {
                self.show_timeline = !self.show_timeline;
            }
            if ui.button(format!("{} {}", icon::ARROW_COUNTER_CLOCKWISE, tr("Refresh"))).clicked() {
                self.trigger_refresh();
            }
            if let Some((desc, _)) = self.undo_stack.last() {
                let label = format!("{} {}: {desc}", icon::ARROW_COUNTER_CLOCKWISE, tr("Undo"));
                if ui.button(label).clicked()
                    && let Some((_, act)) = self.undo_stack.pop()
                {
                    super::system::dispatch(&self.pending_op, act, None);
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

        if self.show_timeline {
            ui.add_space(6.0);
            self.render_timeline(ui, sum);
        }

        ui.add_space(6.0);

        if let Some(&first_key) = self.table.selected.iter().next()
            && let Some(r) = sum.items.iter().find(|r| row_key(r) == first_key)
        {
            self.selected_id = Some(r.id.clone());
        }

        let mut pending_toggle: Option<StartupRow> = None;
        let mut pending_reveal: Option<String> = None;

        if sum.items.is_empty() {
            widgets::empty_state(
                ui,
                icon::ROCKET_LAUNCH,
                tr("No startup apps found"),
                tr("No registry Run entries, Startup folder items, or autostart entries were detected."),
            );
            return;
        }

        Table::new("sys_startup_table", row_key)
            .col(Column::new("name", tr("Name"), 200.0, |r: &StartupRow| Cell::Text(r.name.clone())))
            .col(Column::new("publisher", tr("Publisher"), 160.0, |r: &StartupRow| {
                Cell::Text(if r.publisher.is_empty() { "–".into() } else { r.publisher.clone() })
            }))
            .col(
                Column::new("status", tr("Status"), 95.0, |r: &StartupRow| {
                    Cell::Text(if r.enabled { tr("Enabled").into() } else { tr("Disabled").into() })
                })
                .colored(move |r: &StartupRow| if r.enabled { Some(p.success) } else { Some(p.text_muted) }),
            )
            .col(Column::new("disabled_date", tr("Date disabled"), 110.0, |r: &StartupRow| {
                Cell::Text(r.disabled_date.clone().unwrap_or_else(|| "–".into()))
            }))
            .col(
                Column::new("impact", tr("Startup impact"), 125.0, |r: &StartupRow| {
                    let ord = match r.impact {
                        Impact::High => 3.0,
                        Impact::Medium => 2.0,
                        Impact::Low => 1.0,
                        Impact::NotMeasured => 0.0,
                    };
                    Cell::num(ord, tr(r.impact.label()))
                })
                .colored(move |r: &StartupRow| match r.impact {
                    Impact::High => Some(p.danger),
                    Impact::Medium => Some(p.warning),
                    Impact::Low => Some(p.success),
                    Impact::NotMeasured => Some(p.text_muted),
                }),
            )
            .col(
                Column::new("cpu_ms", tr("CPU at startup"), 110.0, |r: &StartupRow| match r.cpu_ms {
                    Some(ms) => Cell::num(ms as f64, format!("{ms} ms")),
                    None => Cell::num(0.0, "–"),
                })
                .right(),
            )
            .col(
                Column::new("disk", tr("Disk at startup"), 110.0, |r: &StartupRow| match r.disk_bytes {
                    Some(b) if b >= 1_048_576 => Cell::num(b as f64, format!("{:.1} MB", b as f64 / 1_048_576.0)),
                    Some(b) => Cell::num(b as f64, format!("{:.0} KB", b as f64 / 1024.0)),
                    None => Cell::num(0.0, "–"),
                })
                .right(),
            )
            .col(Column::new("location", tr("Location"), 185.0, |r: &StartupRow| Cell::Text(r.source.label().into())))
            .col(Column::new("command", tr("Command line"), 280.0, |r: &StartupRow| Cell::Text(r.command.clone())))
            .show(ui, &sum.items, &mut self.table, |ui, sel| {
                if let Some(&first) = sel.first() {
                    ui.label(RichText::new(&first.name).strong());
                    ui.separator();
                    let toggle_label = if first.enabled { tr("Disable") } else { tr("Enable") };
                    if ui.button(toggle_label).clicked() {
                        pending_toggle = Some((*first).clone());
                        ui.close();
                    }
                    if !first.path.is_empty() && ui.button(tr("Open file location")).clicked() {
                        pending_reveal = Some(first.path.clone());
                        ui.close();
                    }
                }
            });

        // Bottom bar for quick toggle of the currently selected startup item
        if let Some(sel_id) = &self.selected_id
            && let Some(row) = sum.items.iter().find(|r| &r.id == sel_id)
        {
            ui.add_space(6.0);
            Frame::default()
                .fill(p.card)
                .stroke(Stroke::new(1.0, p.border))
                .corner_radius(CornerRadius::same(theme::RADIUS_CARD))
                .inner_margin(Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&row.name).strong());
                        ui.label(RichText::new(format!("({})", row.source.label())).small().color(p.text_secondary));
                        ui.label(RichText::new(&row.command).monospace().small().color(p.text_muted));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let btn_text = if row.enabled { tr("Disable") } else { tr("Enable") };
                            if widgets::primary_button(ui, btn_text).clicked() {
                                pending_toggle = Some(row.clone());
                            }
                            if !row.path.is_empty() && ui.button(tr("Open file location")).clicked() {
                                pending_reveal = Some(row.path.clone());
                            }
                        });
                    });
                });
        }

        if let Some(row) = pending_toggle {
            let new_enabled = !row.enabled;
            let revert = Action::Startup {
                target: StartupOp::SetEnabled { id: row.id.clone(), source: row.source, enabled: row.enabled },
                elevate: row.source.needs_admin(),
            };
            let undo = (format!("{} {}", if row.enabled { "Enable" } else { "Disable" }, row.name), revert);
            super::system::dispatch(
                &self.pending_op,
                Action::Startup {
                    target: StartupOp::SetEnabled { id: row.id, source: row.source, enabled: new_enabled },
                    elevate: row.source.needs_admin(),
                },
                Some(undo),
            );
        }
        if let Some(path) = pending_reveal {
            let _ = cpuzeez_platform::proc_ops::reveal(&path);
        }
    }

    fn render_timeline(&self, ui: &mut Ui, sum: &BootSummary) {
        let p = theme::palette(ui);
        widgets::card(ui, icon::CHART_BAR, tr("Boot timeline"), |ui| {
            let bios_ms = sum.bios_time_ms.unwrap_or(2_500);
            let apps_cpu_ms: u64 = sum.items.iter().filter(|r| r.enabled).filter_map(|r| r.cpu_ms).sum();
            let total_ms = (bios_ms + apps_cpu_ms.max(1_000)).max(1);

            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(5), p.sidebar);
            let w_bios = (bios_ms as f32 / total_ms as f32) * rect.width();
            let r_bios = egui::Rect::from_min_size(rect.min, vec2(w_bios, rect.height()));
            ui.painter().rect_filled(r_bios, CornerRadius::same(4), p.series[0]);
            let r_apps = egui::Rect::from_min_size(
                egui::pos2(rect.left() + w_bios, rect.top()),
                vec2((rect.width() - w_bios).max(0.0), rect.height()),
            );
            ui.painter().rect_filled(r_apps, CornerRadius::same(4), p.series[1]);

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    p.series[0],
                    format!("■ {}: {:.1} s", tr("Firmware / BIOS POST"), bios_ms as f64 / 1000.0),
                );
                ui.colored_label(
                    p.series[1],
                    format!("■ {}: {:.2} s", tr("Measured startup apps CPU"), apps_cpu_ms as f64 / 1000.0),
                );
            });
        });
    }
}

fn row_key(r: &StartupRow) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in r.id.bytes().chain(std::iter::once(r.source as u8)) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub const PT_BR: &[(&str, &str)] = &[
    ("Last BIOS time: {} s", "Último tempo de BIOS: {} s"),
    ("Last BIOS time", "Último tempo de BIOS"),
    ("{} enabled · {} disabled", "{} ativados · {} desativados"),
    ("Boot timeline", "Linha do tempo de boot"),
    ("No startup apps found", "Nenhum aplicativo de inicialização encontrado"),
    (
        "No registry Run entries, Startup folder items, or autostart entries were detected.",
        "Nenhuma entrada Run do registro, item da pasta Inicializar ou autostart foi detectado.",
    ),
    ("Date disabled", "Data de desativação"),
    ("Startup impact", "Impacto na inicialização"),
    ("High", "Alto"),
    ("Medium", "Médio"),
    ("Low", "Baixo"),
    ("Not measured", "Não medido"),
    ("CPU at startup", "CPU na inicialização"),
    ("Disk at startup", "Disco na inicialização"),
    ("Location", "Localização"),
    ("Disable", "Desativar"),
    ("Enable", "Ativar"),
    ("Firmware / BIOS POST", "POST de Firmware / BIOS"),
    ("Measured startup apps CPU", "CPU medida dos apps de inicialização"),
];

pub const ES: &[(&str, &str)] = &[
    ("Last BIOS time: {} s", "Último tiempo de BIOS: {} s"),
    ("Last BIOS time", "Último tiempo de BIOS"),
    ("{} enabled · {} disabled", "{} habilitados · {} deshabilitados"),
    ("Boot timeline", "Línea de tiempo de arranque"),
    ("No startup apps found", "No se encontraron aplicaciones de inicio"),
    (
        "No registry Run entries, Startup folder items, or autostart entries were detected.",
        "No se detectaron entradas Run del registro, elementos de la carpeta Inicio ni autostart.",
    ),
    ("Date disabled", "Fecha de deshabilitación"),
    ("Startup impact", "Impacto de inicio"),
    ("High", "Alto"),
    ("Medium", "Medio"),
    ("Low", "Bajo"),
    ("Not measured", "No medido"),
    ("CPU at startup", "CPU al iniciar"),
    ("Disk at startup", "Disco al iniciar"),
    ("Location", "Ubicación"),
    ("Disable", "Deshabilitar"),
    ("Enable", "Habilitar"),
    ("Firmware / BIOS POST", "POST de Firmware / BIOS"),
    ("Measured startup apps CPU", "CPU medida de aplicaciones de inicio"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use cpuzeez_core::startup::StartupSource;

    #[test]
    fn startup_tab_renders_bios_time_and_timeline() {
        use egui_kittest::{Harness, kittest::Queryable};
        let sum = BootSummary {
            bios_time_ms: Some(3400),
            os_boot_ms: None,
            impact_source_note: Some("WDI StartupInfo (2026-11-04)".into()),
            items: vec![StartupRow {
                id: "OneDrive".into(),
                name: "Microsoft OneDrive".into(),
                publisher: "Microsoft Corporation".into(),
                enabled: false,
                disabled_date: Some("2026-11-04".into()),
                impact: Impact::High,
                cpu_ms: Some(1250),
                disk_bytes: Some(4_500_000),
                source: StartupSource::HkcuRun,
                command: r#""C:\Program Files\Microsoft OneDrive\OneDrive.exe" /background"#.into(),
                path: r"C:\Program Files\Microsoft OneDrive\OneDrive.exe".into(),
            }],
        };
        let mut h = Harness::builder().with_size(egui::vec2(1100.0, 620.0)).build_ui_state(
            move |ui, st: &mut State| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                st.show_with_summary(ui, &sum);
            },
            State::default(),
        );
        h.run();
        h.run();
        h.get_by_label_contains("Last BIOS time: 3.4 s");
        h.get_by_label_contains("Boot timeline").click();
        h.run();
        assert!(h.state().show_timeline);
    }
}

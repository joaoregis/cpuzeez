//! Sensors page (HWiNFO-like): every telemetry series grouped by device with
//! Current / Min / Max / Avg, search, CSV logging, sampling period, and charts
//! of the checked rows. `Monitor` also owns the pieces that share the
//! telemetry sampler: the stress monitor, the tray icon and the mini monitor.

use super::i18n::tr;
use super::table::{Cell, Column, Table};
use super::theme::{self, icon};
use super::units;
use super::widgets::{self, Tone};
use super::{plot, stress, tray};
use crate::Ctx;
use crate::settings::Settings;
use crate::telemetry::{Series, Telemetry, Unit};
use eframe::egui::{self, RichText};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const PT_BR: &[(&str, &str)] = &[
    ("Some sensors aren't responding: {}", "Alguns sensores não estão respondendo: {}"),
    ("Sensors", "Sensores"),
    ("Stress test", "Teste de estresse"),
    ("Mini monitor", "Minimonitor"),
    ("Tray icon", "Ícone na bandeja"),
    ("Close to tray", "Fechar para a bandeja"),
    ("Sensor", "Sensor"),
    ("Current", "Atual"),
    ("Min", "Mín."),
    ("Max", "Máx."),
    ("Avg", "Méd."),
    ("Search…", "Pesquisar…"),
    ("Plot", "Plotar"),
    ("{} above {} for {} s (now {})", "{} acima de {} por {} s (agora {})"),
    ("{} below {} for {} s (now {})", "{} abaixo de {} por {} s (agora {})"),
    ("Stop plotting", "Parar de plotar"),
    ("Unit", "Unidade"),
    ("All", "Todos"),
    ("Temperatures", "Temperaturas"),
    ("Clocks", "Clocks"),
    ("Power", "Potência"),
    ("Voltages", "Tensões"),
    ("Fans", "Ventoinhas"),
    ("Loads", "Cargas"),
    ("Disks", "Discos"),
    ("Network", "Rede"),
    ("Export view", "Exportar visão"),
    ("Saved {}", "Salvo {}"),
    ("Reset min/max", "Zerar mín./máx."),
    ("Start CSV log", "Iniciar log CSV"),
    ("Stop CSV log", "Parar log CSV"),
    ("Open folder", "Abrir pasta"),
    ("Logging to", "Gravando em"),
    ("Sampling", "Amostragem"),
    ("Waiting for sensor data…", "Aguardando dados dos sensores…"),
    ("No settings folder", "Sem pasta de configurações"),
    ("Tray unavailable", "Bandeja indisponível"),
    ("Show CPU ZeeZ", "Mostrar CPU ZeeZ"),
    ("Close", "Fechar"),
    ("Check rows to plot them.", "Marque linhas para exibir no gráfico."),
    ("History", "Histórico"),
];
pub const ES: &[(&str, &str)] = &[
    ("Some sensors aren't responding: {}", "Algunos sensores no responden: {}"),
    ("Sensors", "Sensores"),
    ("Stress test", "Prueba de estrés"),
    ("Mini monitor", "Minimonitor"),
    ("Tray icon", "Icono en la bandeja"),
    ("Close to tray", "Cerrar a la bandeja"),
    ("Sensor", "Sensor"),
    ("Current", "Actual"),
    ("Min", "Mín."),
    ("Max", "Máx."),
    ("Avg", "Prom."),
    ("Search…", "Buscar…"),
    ("Plot", "Graficar"),
    ("{} above {} for {} s (now {})", "{} por encima de {} durante {} s (ahora {})"),
    ("{} below {} for {} s (now {})", "{} por debajo de {} durante {} s (ahora {})"),
    ("Stop plotting", "Dejar de graficar"),
    ("Unit", "Unidad"),
    ("All", "Todos"),
    ("Temperatures", "Temperaturas"),
    ("Clocks", "Relojes"),
    ("Power", "Potencia"),
    ("Voltages", "Voltajes"),
    ("Fans", "Ventiladores"),
    ("Loads", "Cargas"),
    ("Disks", "Discos"),
    ("Network", "Red"),
    ("Export view", "Exportar vista"),
    ("Saved {}", "Guardado {}"),
    ("Reset min/max", "Reiniciar mín./máx."),
    ("Start CSV log", "Iniciar registro CSV"),
    ("Stop CSV log", "Detener registro CSV"),
    ("Open folder", "Abrir carpeta"),
    ("Logging to", "Registrando en"),
    ("Sampling", "Muestreo"),
    ("Waiting for sensor data…", "Esperando datos de los sensores…"),
    ("No settings folder", "Sin carpeta de configuración"),
    ("Tray unavailable", "Bandeja no disponible"),
    ("Show CPU ZeeZ", "Mostrar CPU ZeeZ"),
    ("Close", "Cerrar"),
    ("Check rows to plot them.", "Marque filas para graficarlas."),
    ("History", "Historial"),
];

/// Temperatures within this many °C of their limit are shown as a warning.
const WARN_BAND: f64 = 10.0;
/// Heuristic limits where the device reports none (typical GPU slowdown /
/// NVMe warning composite temperatures).
pub const GPU_LIMIT: f64 = 90.0;
const STORAGE_LIMIT: f64 = 70.0;
const PERIODS_MS: [u64; 4] = [500, 1000, 2000, 5000];
const CHART_MIN_H: f32 = 120.0;
const MIN_SPAN: f64 = 60.0;
const CHART_MAX_H: f32 = 220.0;
const WINDOW_SIZE: [f32; 2] = [900.0, 760.0];

#[derive(Default)]
struct View {
    /// Filter, sort, hidden columns and collapsed groups of the table.
    table: super::table::State,
    plotted: BTreeSet<String>,
    /// The summary temperatures are plotted once data first arrives.
    defaulted: bool,
    /// Unit and Id start hidden (they're there for filters like `unit:°C`).
    defaulted_cols: bool,
    status: String,
}

pub struct Monitor {
    ctx: Arc<Ctx>,
    tel: Option<(Telemetry, Instant)>,
    pub settings: Settings,
    pub show_sensors: bool,
    pub show_stress: bool,
    pub show_mini: bool,
    alerts: Vec<String>,
    /// Sidebar layout: Sensors/Stress are pages, so `frame` only raises the
    /// `show_*` flags (tray menu) and the app navigates instead of opening windows.
    pub pages: bool,
    pub stress: stress::Stress,
    pub tray: tray::Tray,
    view: View,
    quitting: bool,
    started: bool,
    /// The main window is hidden in the tray.
    pub hidden: bool,
    /// Sampling paused from the tray menu.
    pub paused: bool,
    /// (icons, click) the tray was built with, to rebuild when they change.
    tray_shape: (usize, tray::Click),
    pub(super) mini_rt: super::mini::Runtime,
    /// A mini monitor asked for its editor (Settings opens on it).
    pub edit_mini: Option<usize>,
}

impl Monitor {
    pub fn new(ctx: Arc<Ctx>) -> Monitor {
        let mut settings = Settings::load();
        if settings.minis.is_empty() {
            // 0.3.0 / early 1.0 files: one mini monitor, open if it was.
            let open = settings.mini;
            settings.minis.push(cpuzeez_core::layout::mini::Mini { open, ..Default::default() });
        }
        Monitor {
            ctx,
            tel: None,
            show_mini: settings.minis.iter().any(|m| m.open),
            alerts: Vec::new(),
            settings,
            show_sensors: false,
            show_stress: false,
            pages: false,
            stress: stress::Stress::default(),
            tray: tray::Tray::default(),
            view: View::default(),
            quitting: false,
            started: false,
            hidden: false,
            paused: false,
            tray_shape: (0, tray::Click::App),
            mini_rt: Default::default(),
            edit_mini: None,
        }
    }

    /// Default settings and a telemetry engine over fixed readings (page tests).
    #[cfg(test)]
    pub fn fixture(ctx: Arc<Ctx>, tel: Telemetry) -> Monitor {
        let mut settings = Settings::default();
        settings.minis.push(Default::default());
        Monitor {
            ctx,
            tel: Some((tel, Instant::now())),
            show_mini: false,
            alerts: Vec::new(),
            settings,
            show_sensors: false,
            show_stress: false,
            pages: true,
            stress: stress::Stress::default(),
            tray: tray::Tray::default(),
            view: View::default(),
            quitting: false,
            started: false,
            hidden: false,
            paused: false,
            tray_shape: (0, tray::Click::App),
            mini_rt: Default::default(),
            edit_mini: None,
        }
    }

    /// Sampling period and graph history from the settings.
    fn sampling(&self) -> (Duration, Duration) {
        let s = &self.settings;
        // Paused: one read an hour keeps the engine alive without working.
        let period = if self.paused { Duration::from_secs(3600) } else { Duration::from_millis(s.sample_ms.max(100)) };
        (period, Duration::from_secs(s.graph_seconds.max(60)))
    }

    /// The shared sampler, started on first use.
    /// Local alerts firing right now (S1-G01), already worded.
    pub fn alerts(&self) -> &[String] {
        &self.alerts
    }

    /// Opens on these rows (the Overview's tiles link here).
    pub fn focus(&mut self, filter: String) {
        self.view.table.filter = filter;
    }

    pub fn ctx(&self) -> &Arc<Ctx> {
        &self.ctx
    }

    pub fn tel(&mut self) -> Telemetry {
        let (period, keep) = self.sampling();
        let ctx = self.ctx.clone();
        self.tel.get_or_insert_with(|| (Telemetry::start(ctx, period, keep), Instant::now())).0.clone()
    }

    /// Apply changed sampling settings to a sampler that is already running.
    pub fn apply_sampling(&self) {
        if let Some((tel, _)) = &self.tel {
            let (period, keep) = self.sampling();
            tel.set_period(period);
            tel.set_keep(keep);
        }
    }

    /// Show or hide every mini monitor and remember it.
    pub fn set_mini(&mut self, on: bool) {
        self.show_mini = on;
        self.set_setting(|s| {
            s.mini = on;
            if s.minis.is_empty() {
                s.minis.push(Default::default());
            }
            for m in &mut s.minis {
                m.open = on;
            }
        });
    }

    /// CPU Tjmax, else the configured warning temperature.
    pub fn tjmax(&self) -> f64 {
        crate::cpu::get(&self.ctx).info.tjmax_c.unwrap_or(self.settings.warn_temp_c)
    }

    pub fn stress_limit(&self) -> f64 {
        self.stress.limit.unwrap_or_else(|| self.tjmax())
    }

    pub fn start_stress(&mut self, threads: usize) {
        self.tel();
        self.stress.start(threads);
    }

    /// Change a setting here and on disk (re-read first so we don't clobber
    /// keys other screens changed).
    pub fn set_setting(&mut self, f: impl Fn(&mut Settings)) {
        let mut disk = Settings::load();
        f(&mut disk);
        disk.save();
        f(&mut self.settings);
    }

    pub(super) fn set_tray(&mut self, on: bool, ectx: &egui::Context) {
        let shape = (self.settings.tray_items.len().max(1), self.settings.tray_click);
        if on && self.tray.active() && shape != self.tray_shape {
            self.tray.set_enabled(false, 0, shape.1, ectx);
        }
        self.tray_shape = shape;
        self.tray.set_enabled(on, shape.0, shape.1, ectx);
        if on && !self.tray.active() {
            self.show_mini = true; // no tray host: fall back to the mini window
        }
        if on {
            self.tel();
        }
    }

    /// A CSV log is being written (● REC).
    pub fn recording(&self) -> bool {
        self.tel.as_ref().is_some_and(|(t, _)| t.csv_path().is_some())
    }

    pub fn stop_recording(&self) {
        if let Some((t, _)) = &self.tel {
            t.stop_csv();
        }
    }

    /// Close the app for real (not to the tray).
    pub fn quit(&mut self, ectx: &egui::Context) {
        self.quitting = true;
        ectx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Bring the main window back: from the tray, minimised or behind others.
    pub fn show_window(&mut self, ectx: &egui::Context) {
        self.hidden = false;
        ectx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ectx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ectx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    /// Call every frame from the app: windows, tray events, auto-stop, close-to-tray.
    pub fn frame(&mut self, ectx: &egui::Context) {
        if !self.started {
            self.started = true;
            if self.settings.tray {
                self.set_tray(true, ectx);
            }
        }
        if self.tray.active() && (self.settings.tray_items.len().max(1), self.settings.tray_click) != self.tray_shape {
            self.set_tray(true, ectx);
        }
        for a in self.tray.poll() {
            match a {
                tray::Action::Toggle if !self.hidden => {
                    ectx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    self.hidden = true;
                }
                tray::Action::Toggle | tray::Action::Sensors => {
                    self.show_sensors |= a == tray::Action::Sensors;
                    self.show_window(ectx);
                }
                tray::Action::Mini => {
                    let on = !self.show_mini;
                    self.set_mini(on);
                }
                tray::Action::Lock => {
                    let on = !self.settings.minis.iter().any(|m| m.open && m.locked);
                    self.set_setting(move |s| s.minis.iter_mut().filter(|m| m.open).for_each(|m| m.locked = on));
                }
                tray::Action::Pause => {
                    self.paused = !self.paused;
                    self.apply_sampling();
                }
                tray::Action::Speed(ms) => {
                    self.set_setting(|s| s.sample_ms = ms);
                    self.apply_sampling();
                }
                tray::Action::Quit => self.quit(ectx),
            }
        }
        if self.tray.active()
            && self.settings.close_to_tray
            && !self.quitting
            && ectx.input(|i| i.viewport().close_requested())
        {
            ectx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ectx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.hidden = true;
        }
        if let Some((tel, _)) = &self.tel {
            // Leases for what runs without its page open; current values only.
            let shown = ["cpu/", "gpu", "chipset/"];
            if self.stress.running() {
                tel.want("stress", &shown);
            }
            if self.tray.active() {
                let ws: Vec<cpuzeez_core::layout::Widget> =
                    self.settings.tray_items.iter().map(|i| i.widget.clone()).collect();
                let mut lease = super::layout_view::prefixes(&ws);
                lease.extend(shown.iter().map(|s| s.to_string()));
                tel.want("tray", &lease.iter().map(String::as_str).collect::<Vec<_>>());
            }
            if self.settings.alerts_on {
                let ws: Vec<cpuzeez_core::layout::Widget> = self
                    .settings
                    .alerts
                    .iter()
                    .filter(|r| r.enabled)
                    .map(|r| cpuzeez_core::layout::Widget::metric(cpuzeez_core::layout::Kind::Value, &r.metric, ""))
                    .collect();
                let lease = super::layout_view::prefixes(&ws);
                tel.want("alerts", &lease.iter().map(String::as_str).collect::<Vec<_>>());
            }
            let series = tel.series_with(|_| false);
            let (limit, tjmax) = (self.stress_limit(), self.tjmax());
            self.stress.tick(&series, limit);
            self.alerts =
                if self.settings.alerts_on { firing_alerts(&self.settings.alerts, &series, tel) } else { Vec::new() };
            if self.tray.active() {
                let shown = tray_pictures(&self.settings.tray_items, &series, tel, tjmax, ectx);
                let extra: Vec<String> = self.alerts.iter().map(|a| format!("{} {a}", icon::WARNING)).collect();
                let state = tray::MenuState {
                    visible: !self.hidden,
                    mini: self.show_mini,
                    paused: self.paused,
                    speed_ms: self.settings.sample_ms,
                    lock: self.show_mini.then(|| self.settings.minis.iter().any(|m| m.open && m.locked)),
                };
                self.tray.update(&shown, &extra, &state);
            }
            if self.stress.running() || self.tray.active() || self.show_mini {
                ectx.request_repaint_after(Duration::from_secs(1));
            }
        }

        let size = ectx.content_rect().size();
        let default = egui::vec2(WINDOW_SIZE[0].min(size.x - 16.0), WINDOW_SIZE[1].min(size.y - 40.0));
        if self.show_sensors && !self.pages {
            let mut open = true;
            egui::Window::new(tr("Sensors"))
                .open(&mut open)
                .default_size(default)
                .default_pos([8.0, 32.0])
                .show(ectx, |ui| page(ui, self));
            self.show_sensors = open;
        }
        if self.show_stress && !self.pages {
            let mut open = true;
            egui::Window::new(tr("Stress test")).open(&mut open).default_size(default).default_pos([8.0, 32.0]).show(
                ectx,
                |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| stress::page(ui, self, true, &[]));
                },
            );
            self.show_stress = open;
        }
        if self.show_mini {
            super::mini::show_all(self, ectx);
        }
    }
}

// ---- series helpers (shared with the stress monitor and tray) ----

/// A telemetry value in the units chosen in Settings.
/// Each tray icon's picture and tooltip line from the current values.
fn tray_pictures(
    items: &[tray::Item],
    series: &[Series],
    tel: &Telemetry,
    tjmax: f64,
    ectx: &egui::Context,
) -> Vec<tray::Shown> {
    use super::trayicon::{self, Picture, Style};
    use cpuzeez_core::layout::{Tone, temp_thresholds};
    let n = trayicon::size();
    items
        .iter()
        .map(|it| {
            let w = &it.widget;
            let s = w.metric.as_deref().and_then(|m| super::layout_view::resolve(m, series));
            let label = w.label.clone().unwrap_or_else(|| s.map_or(String::new(), |s| s.label.clone()));
            let Some(s) = s else {
                let p = Picture { style: it.style, text: None, fill: 0.0, tone: None, history: &[] };
                return tray::Shown {
                    rgba: trayicon::draw(Some(ectx), &p, n),
                    line: format!("{label} –"),
                    short: "–".into(),
                };
            };
            // Thresholds: the widget's own, else sensible ones for the unit.
            let limit = if w.metric.as_deref() == Some("@cpu_temp") { tjmax } else { GPU_LIMIT };
            let w = if w.thresholds.is_empty() {
                match s.unit {
                    Unit::Celsius => w.clone().with_thresholds(&temp_thresholds(limit)),
                    Unit::Percent => w.clone().with_thresholds(&[(90.0, Tone::Warning)]),
                    _ => w.clone(),
                }
            } else {
                w.clone()
            };
            let text = super::layout_view::value_text(&w, s);
            let number = text.split_whitespace().next().unwrap_or("").to_string();
            let fill = if s.unit == Unit::Celsius { (s.current / limit) as f32 } else { w.fill(s.current) };
            let hist: Vec<f64> = if it.style == Style::Graph {
                tel.series_with(|id| id == s.id)
                    .first()
                    .map(|x| x.points.iter().map(|p| p.1).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let p = Picture {
                style: it.style,
                text: Some(number.clone()),
                fill,
                tone: Some(w.tone(s.current)),
                history: &hist,
            };
            let degree = if s.unit == Unit::Celsius { "°" } else { "" };
            tray::Shown {
                rgba: trayicon::draw(Some(ectx), &p, n),
                line: format!("{label} {text}"),
                short: format!("{label} {number}{degree}"),
            }
        })
        .collect()
}

pub fn fmt(v: f64, u: Unit) -> String {
    super::units::get().format(v, u)
}

/// Whole-degree temperature for compact summaries.
fn fmt0(t: f64) -> String {
    super::units::temp(t, 0)
}

/// "GPU" matches "GPU · NVIDIA …".
fn in_group(s: &Series, g: &str) -> bool {
    s.group.strip_prefix(g).is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
}

/// The series of `group`/`unit` whose label or id mentions the first matching
/// `prefer` word, else the highest current value.
fn pick<'a>(series: &'a [Series], group: &str, unit: Unit, prefer: &[&str]) -> Option<&'a Series> {
    let c: Vec<&Series> = series.iter().filter(|s| s.unit == unit && in_group(s, group)).collect();
    prefer
        .iter()
        .find_map(|p| c.iter().find(|s| s.label.to_lowercase().contains(p) || s.id.ends_with(p)).copied())
        .or_else(|| c.iter().max_by(|a, b| a.current.total_cmp(&b.current)).copied())
}

pub fn cpu_temp(s: &[Series]) -> Option<&Series> {
    pick(s, "CPU", Unit::Celsius, &["package", "tctl", "tdie", "cpu"])
}
pub fn gpu_temp(s: &[Series]) -> Option<&Series> {
    pick(s, "GPU", Unit::Celsius, &["core", "edge", "gpu"])
}
pub fn chipset_temp(s: &[Series]) -> Option<&Series> {
    pick(s, "Chipset", Unit::Celsius, &[])
}
/// The sampler's all-core average clock, when it has one.
pub fn avg_clock(s: &[Series]) -> Option<&Series> {
    s.iter().find(|x| x.unit == Unit::Mhz && in_group(x, "CPU") && x.id.ends_with("/avg"))
}
pub fn package_power(s: &[Series]) -> Option<&Series> {
    pick(s, "CPU", Unit::Watt, &["package", "ppt", "cpu"])
}

/// Per-core clocks: the "effective" ones when the sampler has them, else every
/// CPU MHz series except bus/ring/uncore/memory clocks.
pub fn core_clocks(s: &[Series]) -> Vec<&Series> {
    let mhz: Vec<&Series> =
        s.iter().filter(|x| x.unit == Unit::Mhz && in_group(x, "CPU") && !x.id.ends_with("/avg")).collect();
    let eff: Vec<&Series> = mhz.iter().copied().filter(|x| x.label.to_lowercase().contains("effective")).collect();
    if !eff.is_empty() {
        return eff;
    }
    let skip = ["bus", "ring", "uncore", "mem", "fclk", "bclk"];
    mhz.into_iter().filter(|x| !skip.iter().any(|k| x.label.to_lowercase().contains(k))).collect()
}

/// (name, °C) for the summary row: CPU, GPU, Chipset.
fn summary(s: &[Series]) -> [(&'static str, Option<f64>); 3] {
    [
        ("CPU", cpu_temp(s).map(|x| x.current)),
        ("GPU", gpu_temp(s).map(|x| x.current)),
        (tr("Chipset"), chipset_temp(s).map(|x| x.current)),
    ]
}

fn limit_for(group: &str, tjmax: f64) -> Option<f64> {
    match group.split(' ').next() {
        Some("CPU") => Some(tjmax),
        Some("GPU") => Some(GPU_LIMIT),
        Some("Storage") => Some(STORAGE_LIMIT),
        _ => None,
    }
}

pub fn temp_tone(v: f64, limit: f64) -> Tone {
    if v >= limit {
        Tone::Danger
    } else if v >= limit - WARN_BAND {
        Tone::Warning
    } else {
        Tone::Neutral
    }
}

/// "CPU 71 °C · GPU 58 °C · Chipset 46 °C" as pills (only the ones present).
pub fn summary_row(ui: &mut egui::Ui, series: &[Series], tjmax: f64) {
    let limits = [tjmax, GPU_LIMIT, f64::MAX];
    ui.horizontal_wrapped(|ui| {
        for ((name, t), lim) in summary(series).into_iter().zip(limits) {
            if let Some(t) = t {
                let tone = match temp_tone(t, lim) {
                    Tone::Neutral => Tone::Accent,
                    x => x,
                };
                widgets::pill(ui, &format!("{}  {name} {}", icon::THERMOMETER, fmt0(t)), tone);
            }
        }
    });
}

fn group_icon(g: &str) -> &'static str {
    match g.split(' ').next() {
        Some("CPU") => icon::CPU,
        Some("GPU") => icon::GRAPHICS_CARD,
        Some("Mainboard") => icon::CIRCUITRY,
        Some("Chipset") => icon::CIRCUITRY,
        Some("Memory") => icon::MEMORY,
        Some("Storage") => icon::HARD_DRIVES,
        Some("Battery") => icon::BATTERY_FULL,
        _ => icon::THERMOMETER,
    }
}

// ---- the page ----

/// Sensors page: summary, toolbar, grouped table, charts of checked rows.
pub fn page(ui: &mut egui::Ui, mon: &mut Monitor) {
    let tel = mon.tel();
    tel.want("sensors", &[""]);
    let series = tel.series_with(|id| mon.view.plotted.contains(id));
    let tjmax = mon.tjmax();
    ui.ctx().request_repaint_after(Duration::from_millis(mon.settings.sample_ms.clamp(250, 1000)));
    widgets::page_header(ui, tr("Sensors"), "", |ui| toolbar_actions(ui, mon, &tel));
    // A source that panicked or hangs is isolated by the engine; say which one.
    let failed: Vec<String> = tel.failures().into_iter().map(|(name, why)| format!("{name} ({why})")).collect();
    if !failed.is_empty() {
        widgets::pill(
            ui,
            &super::i18n::trf("Some sensors aren't responding: {}", &[&failed.join(", ")]),
            widgets::Tone::Warning,
        );
        ui.add_space(theme::GAP / 2.0);
    }
    summary_row(ui, &series, tjmax);
    toolbar(ui, mon, &tel);
    if series.is_empty() {
        widgets::empty_state(ui, icon::THERMOMETER, tr("Sensors"), tr("Waiting for sensor data…"));
        return;
    }
    let v = &mut mon.view;
    if !v.defaulted {
        v.defaulted = true;
        let first = [cpu_temp(&series), gpu_temp(&series), chipset_temp(&series)];
        v.plotted.extend(first.iter().flatten().map(|s| s.id.clone()));
    }

    presets(ui, &mut v.table);
    let table_h = if v.plotted.is_empty() { ui.available_height() } else { ui.available_height() * 0.5 };
    let p = theme::palette(ui);
    let hot = move |s: &Series, x: f64| {
        let lim = (s.unit == Unit::Celsius).then(|| limit_for(&s.group, tjmax)).flatten()?;
        match temp_tone(x, lim) {
            Tone::Danger => Some(p.danger),
            Tone::Warning => Some(p.warning),
            _ => None,
        }
    };
    let plotted = v.plotted.clone();
    let num = |f: fn(&Series) -> f64| move |s: &Series| Cell::num(f(s), fmt(f(s), s.unit));
    let table = Table::new("sensors", |s: &Series| hash(&s.id))
        .col(Column::new("plot", "●", 26.0, move |s: &Series| {
            Cell::Text(if plotted.contains(&s.id) { "●" } else { "" }.into())
        }))
        .col(Column::new("sensor", tr("Sensor"), 260.0, |s: &Series| Cell::Text(s.label.clone())))
        .col(Column::new("cur", tr("Current"), 110.0, num(|s| s.current)).right().colored(move |s| hot(s, s.current)))
        .col(Column::new("min", tr("Min"), 110.0, num(|s| s.min)).right())
        .col(Column::new("max", tr("Max"), 110.0, num(|s| s.max)).right().colored(move |s| hot(s, s.max)))
        .col(Column::new("avg", tr("Avg"), 110.0, num(|s| s.avg)).right())
        .col(Column::new("unit", tr("Unit"), 60.0, |s: &Series| Cell::Text(units::get().suffix(s.unit).into())))
        .col(Column::new("id", "Id", 220.0, |s: &Series| Cell::Text(s.id.clone())))
        .grouped(|s: &Series| format!("{}  {}", group_icon(&s.group), s.group));
    for hidden in ["unit", "id"] {
        if !v.defaulted_cols {
            v.table.hidden.insert(hidden.into());
        }
    }
    v.defaulted_cols = true;
    let mut plot: Option<(Vec<String>, bool)> = None;
    let mut pin: Vec<crate::settings::Pin> = Vec::new();
    widgets::card(ui, "", "", |ui| {
        ui.allocate_ui(egui::vec2(ui.available_width(), table_h), |ui| {
            table.show(ui, &series, &mut v.table, |ui, sel| {
                let ids: Vec<String> = sel.iter().map(|s| s.id.clone()).collect();
                if ui.button(format!("{}  {}", icon::CHART_LINE, tr("Plot"))).clicked() {
                    plot = Some((ids.clone(), true));
                    ui.close();
                }
                if ui.button(tr("Stop plotting")).clicked() {
                    plot = Some((ids, false));
                    ui.close();
                }
                if ui.button(tr("Pin to Overview")).clicked() {
                    pin = sel
                        .iter()
                        .map(|s| crate::settings::Pin {
                            card: s.group.clone(),
                            label: s.label.clone(),
                            value: fmt(s.current, s.unit),
                            metric: Some(s.id.clone()),
                        })
                        .collect();
                    ui.close();
                }
                if let [s] = sel
                    && ui.button(tr("Copy value")).clicked()
                {
                    ui.ctx().copy_text(fmt(s.current, s.unit));
                    ui.close();
                }
            });
        });
    });
    if let Some((ids, on)) = plot {
        for id in ids {
            if on {
                v.plotted.insert(id);
            } else {
                v.plotted.remove(&id);
            }
        }
    }
    for p in pin {
        super::pins::request(ui.ctx(), p);
    }
    ui.add_space(theme::GAP);
    widgets::card(ui, icon::CHART_LINE, tr("History"), |ui| {
        charts(ui, &series, &v.plotted, mon.settings.graph_seconds as f64);
    });
}

/// "CPU temperature above 90 °C for 30 s (now 93 °C)" for each rule that holds.
fn firing_alerts(rules: &[cpuzeez_core::alerts::Rule], series: &[Series], tel: &Telemetry) -> Vec<String> {
    rules
        .iter()
        .filter(|r| r.enabled)
        .filter_map(|r| {
            let s = super::layout_view::resolve(&r.metric, series)?;
            cpuzeez_core::alerts::firing(r, &tel.points(&s.id)).then(|| {
                let key = if r.above { "{} above {} for {} s (now {})" } else { "{} below {} for {} s (now {})" };
                super::i18n::trf(key, &[&tr(&r.label), &fmt(r.threshold, s.unit), &r.secs, &fmt(s.current, s.unit)])
            })
        })
        .collect()
}

/// One-click filters over the table's own syntax (S1-G08); the filter box
/// shows what each one typed, so it doubles as an example.
fn presets(ui: &mut egui::Ui, st: &mut super::table::State) {
    ui.horizontal_wrapped(|ui| {
        for (label, q) in [
            ("All", ""),
            ("Temperatures", "unit:°"),
            ("Clocks", "unit:mhz"),
            ("Power", "unit:w"),
            ("Voltages", "unit:v"),
            ("Fans", "unit:rpm"),
            ("Loads", "unit:%"),
            ("Disks", "id:disk"),
            ("Network", "id:net"),
        ] {
            if ui.selectable_label(st.filter == q, tr(label)).clicked() {
                st.filter = q.to_string();
            }
        }
    });
    ui.add_space(4.0);
}

fn hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// The rows the table shows right now (filter applied) as CSV, base units.
fn export_view(mon: &Monitor, tel: &Telemetry) -> Result<PathBuf, String> {
    let series = tel.series_with(|_| false);
    let cols: Vec<Column<Series>> = vec![
        Column::new("sensor", "", 0.0, |s: &Series| Cell::Text(s.label.clone())),
        Column::new("unit", "", 0.0, |s: &Series| Cell::Text(units::get().suffix(s.unit).into())),
        Column::new("id", "", 0.0, |s: &Series| Cell::Text(s.id.clone())),
    ];
    let shown = super::table::view(&series, &cols, &mon.view.table, None);
    let q = |s: &str| {
        let s = cpuzeez_core::report::spreadsheet_safe(s);
        if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.into_owned() }
    };
    let mut out = String::from("group,sensor,id,unit,current,min,max,avg\n");
    for it in shown {
        if let super::table::Item::Row(i) = it {
            let s = &series[i];
            out += &format!(
                "{},{},{},{},{},{},{},{}\n",
                q(&s.group),
                q(&s.label),
                q(&s.id),
                s.unit.suffix(),
                s.current,
                s.min,
                s.max,
                s.avg
            );
        }
    }
    let dir = log_path()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .ok_or_else(|| tr("No settings folder").to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = cpuzeez_core::metrics::iso8601(std::time::SystemTime::now()).replace([':', '.'], "-");
    let path = dir.join(format!("sensors-view-{stamp}.csv"));
    std::fs::write(&path, out).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Header actions (right-aligned, so added right to left).
fn toolbar_actions(ui: &mut egui::Ui, mon: &mut Monitor, tel: &Telemetry) {
    if ui.button(format!("{}  {}", icon::EXPORT, tr("Export view"))).clicked() {
        mon.view.status = match export_view(mon, tel) {
            Ok(p) => super::i18n::trf("Saved {}", &[&p.display()]),
            Err(e) => e,
        };
    }
    if tel.csv_path().is_some() {
        if ui.button(format!("{}  {}", icon::STOP, tr("Stop CSV log"))).clicked() {
            tel.stop_csv();
        }
    } else if ui.button(format!("{}  {}", icon::RECORD, tr("Start CSV log"))).clicked() {
        mon.view.status = match log_path() {
            Some(p) => match tel.start_csv(&p) {
                Ok(()) => String::new(),
                Err(e) => format!("{}: {e}", p.display()),
            },
            None => tr("No settings folder").into(),
        };
    }
    if ui.button(format!("{}  {}", icon::ARROW_COUNTER_CLOCKWISE, tr("Reset min/max"))).clicked() {
        tel.reset_stats();
    }
}

fn toolbar(ui: &mut egui::Ui, mon: &mut Monitor, tel: &Telemetry) {
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Sampling"));
        let cur = mon.settings.sample_ms;
        egui::ComboBox::from_id_salt("sample-period").selected_text(format!("{} s", cur as f64 / 1000.0)).show_ui(
            ui,
            |ui| {
                for ms in PERIODS_MS {
                    if ui.selectable_label(cur == ms, format!("{} s", ms as f64 / 1000.0)).clicked() {
                        tel.set_period(Duration::from_millis(ms));
                        mon.set_setting(|s| s.sample_ms = ms);
                    }
                }
            },
        );
        if let Some(p) = tel.csv_path() {
            widgets::pill(ui, &format!("{}  {} {}", icon::RECORD, tr("Logging to"), p.display()), Tone::Danger);
            if ui.small_button(format!("{}  {}", icon::FOLDER_OPEN, tr("Open folder"))).clicked() {
                open_folder(p.parent().unwrap_or(&p));
            }
        }
    });
    if !mon.view.status.is_empty() {
        let c = theme::palette(ui).danger;
        ui.label(RichText::new(&mon.view.status).small().color(c));
    }
    ui.add_space(theme::GAP / 2.0);
}

/// One chart per unit among the checked series.
fn charts(ui: &mut egui::Ui, series: &[Series], plotted: &BTreeSet<String>, span: f64) {
    let chosen: Vec<&Series> = series.iter().filter(|s| plotted.contains(&s.id)).collect();
    if chosen.is_empty() {
        ui.label(RichText::new(tr("Check rows to plot them.")).color(theme::palette(ui).text_secondary));
        return;
    }
    let colors = theme::palette(ui).series;
    let mut units: Vec<Unit> = Vec::new();
    for s in &chosen {
        if !units.contains(&s.unit) {
            units.push(s.unit);
        }
    }
    let now = series.iter().filter_map(|s| s.points.last()).map(|p| p.0).fold(0.0, f64::max);
    // Show the history we have (at least a minute), up to the configured window.
    let oldest = chosen.iter().filter_map(|s| s.points.first()).map(|p| p.0).fold(now, f64::min);
    let span = (now - oldest).clamp(MIN_SPAN, span);
    let per = ui.available_height() / units.len() as f32 - 28.0;
    let h = per.clamp(CHART_MIN_H, CHART_MAX_H);
    egui::ScrollArea::vertical().id_salt("sensor-charts").show(ui, |ui| {
        for u in units {
            let lines: Vec<plot::Line> = chosen
                .iter()
                .filter(|s| s.unit == u)
                .enumerate()
                .map(|(i, s)| plot::Line { label: &s.label, color: colors[i % colors.len()], points: &s.points })
                .collect();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(u.suffix()).strong());
                for l in &lines {
                    ui.colored_label(l.color, format!("● {}", l.label));
                }
            });
            plot::chart(ui, &lines, now, span, h, &|v| fmt(v, u));
        }
    });
}
/// `settings::dir()/logs/cpuzeez-YYYYmmdd-HHMMSS.csv` (local time).
pub(super) fn log_path() -> Option<PathBuf> {
    let dir = crate::settings::dir()?.join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(format!("{}-{}.csv", crate::NAME, local_stamp())))
}

/// Local time as "HH:MM:SS" (the layouts' Clock widget).
pub fn local_time() -> String {
    let s = local_stamp(); // YYYYMMDD-HHMMSS
    match (s.get(9..11), s.get(11..13), s.get(13..15)) {
        (Some(h), Some(m), Some(sec)) => format!("{h}:{m}:{sec}"),
        _ => String::new(),
    }
}

fn local_stamp() -> String {
    cpuzeez_platform::os::local_stamp()
}

fn open_folder(p: &Path) {
    let prog = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(prog).arg(p).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: &str, group: &str, label: &str, unit: Unit, v: f64) -> Series {
        Series {
            id: id.into(),
            group: group.into(),
            label: label.into(),
            unit,
            points: vec![(0.0, v)],
            current: v,
            min: v,
            max: v,
            avg: v,
        }
    }

    #[test]
    fn picks_and_limits() {
        let v = vec![
            s("cpu/temp/core0", "CPU", "Core #0", Unit::Celsius, 80.0),
            s("cpu/temp/package", "CPU", "Package", Unit::Celsius, 75.0),
            s("gpu0/temp/hotspot", "GPU · Test", "Hot Spot", Unit::Celsius, 70.0),
            s("gpu0/temp/core", "GPU · Test", "GPU Core", Unit::Celsius, 60.0),
            s("chipset/temp", "Chipset", "PCH", Unit::Celsius, 45.0),
            s("cpu/clock/bus", "CPU", "Bus Speed", Unit::Mhz, 100.0),
            s("cpu/clock/core0", "CPU", "Core #0 Clock", Unit::Mhz, 4000.0),
            s("x", "CPUX", "Other", Unit::Celsius, 99.0),
        ];
        assert_eq!(cpu_temp(&v).unwrap().id, "cpu/temp/package");
        assert_eq!(gpu_temp(&v).unwrap().id, "gpu0/temp/core");
        assert_eq!(chipset_temp(&v).unwrap().current, 45.0);
        assert_eq!(core_clocks(&v).len(), 1);
        assert_eq!(temp_tone(86.0, 95.0), Tone::Warning);
        assert_eq!(temp_tone(95.0, 95.0), Tone::Danger);
        assert_eq!(temp_tone(60.0, 95.0), Tone::Neutral);
        assert_eq!(limit_for("GPU · Test", 95.0), Some(GPU_LIMIT));
        assert_eq!(fmt(20480.0, Unit::Megabytes), "20.0 GB");
        assert_eq!(local_stamp().len(), 15);
    }
}

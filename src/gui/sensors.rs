//! Sensors page (HWiNFO-like): every telemetry series grouped by device with
//! Current / Min / Max / Avg, search, CSV logging, sampling period, and charts
//! of the checked rows. `Monitor` also owns the pieces that share the
//! telemetry sampler: the stress monitor, the tray icon and the mini monitor.

use super::i18n::tr;
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use super::{plot, stress, tray};
use crate::Ctx;
use crate::settings::Settings;
use crate::telemetry::{Series, Telemetry, Unit};
use eframe::egui::{self, Color32, RichText};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const PT_BR: &[(&str, &str)] = &[
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
    ("Reset min/max", "Zerar mín./máx."),
    ("Start CSV log", "Iniciar log CSV"),
    ("Stop CSV log", "Parar log CSV"),
    ("Open folder", "Abrir pasta"),
    ("Logging to", "Gravando em"),
    ("Sampling", "Amostragem"),
    ("Waiting for sensor data…", "Aguardando dados dos sensores…"),
    ("No settings folder", "Sem pasta de configurações"),
    ("Tray unavailable", "Bandeja indisponível"),
    ("Show CpuZeeZ", "Mostrar CpuZeeZ"),
    ("Close", "Fechar"),
    ("Check rows to plot them.", "Marque linhas para exibir no gráfico."),
    ("History", "Histórico"),
];
pub const ES: &[(&str, &str)] = &[
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
    ("Reset min/max", "Reiniciar mín./máx."),
    ("Start CSV log", "Iniciar registro CSV"),
    ("Stop CSV log", "Detener registro CSV"),
    ("Open folder", "Abrir carpeta"),
    ("Logging to", "Registrando en"),
    ("Sampling", "Muestreo"),
    ("Waiting for sensor data…", "Esperando datos de los sensores…"),
    ("No settings folder", "Sin carpeta de configuración"),
    ("Tray unavailable", "Bandeja no disponible"),
    ("Show CpuZeeZ", "Mostrar CpuZeeZ"),
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
const COL_W: f32 = 72.0;
const CHART_MIN_H: f32 = 120.0;
const MIN_SPAN: f64 = 60.0;
const CHART_MAX_H: f32 = 220.0;
const WINDOW_SIZE: [f32; 2] = [900.0, 760.0];
const MINI_SIZE: [f32; 2] = [230.0, 96.0];

#[derive(Default)]
struct View {
    filter: String,
    plotted: BTreeSet<String>,
    /// The summary temperatures are plotted once data first arrives.
    defaulted: bool,
    status: String,
}

pub struct Monitor {
    ctx: Arc<Ctx>,
    tel: Option<(Telemetry, Instant)>,
    pub settings: Settings,
    pub show_sensors: bool,
    pub show_stress: bool,
    pub show_mini: bool,
    pub stress: stress::Stress,
    pub tray: tray::Tray,
    view: View,
    quitting: bool,
    started: bool,
    /// `-screenshots`: when the current capture step began.
    shot_since: Option<Instant>,
}

impl Monitor {
    pub fn new(ctx: Arc<Ctx>) -> Monitor {
        Monitor {
            ctx,
            tel: None,
            settings: Settings::load(),
            show_sensors: false,
            show_stress: false,
            show_mini: false,
            stress: stress::Stress::default(),
            tray: tray::Tray::default(),
            view: View::default(),
            quitting: false,
            started: false,
            shot_since: None,
        }
    }

    /// The shared sampler, started on first use.
    pub fn tel(&mut self) -> Telemetry {
        let s = &self.settings;
        let (period, keep) =
            (Duration::from_millis(s.sample_ms.max(100)), Duration::from_secs(s.graph_seconds.max(60)));
        let ctx = self.ctx.clone();
        self.tel.get_or_insert_with(|| (Telemetry::start(ctx, period, keep), Instant::now())).0.clone()
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
        self.tray.set_enabled(on, ectx);
        if on && !self.tray.active() {
            self.show_mini = true; // no tray host: fall back to the mini window
        }
        if on {
            self.tel();
        }
    }

    /// Tools-menu entries.
    pub fn menu(&mut self, ui: &mut egui::Ui) {
        if ui.button(tr("Sensors")).clicked() {
            self.show_sensors = true;
        }
        if ui.button(tr("Stress test")).clicked() {
            self.show_stress = true;
        }
        if ui.checkbox(&mut self.show_mini, tr("Mini monitor")).changed() && self.show_mini {
            self.tel();
        }
        let mut on = self.settings.tray;
        if ui.checkbox(&mut on, tr("Tray icon")).changed() {
            self.set_setting(|s| s.tray = on);
            self.set_tray(on, ui.ctx());
        }
        if let Some(e) = &self.tray.error {
            ui.label(RichText::new(format!("{}: {e}", tr("Tray unavailable"))).small().weak());
        }
        let mut ctt = self.settings.close_to_tray;
        if ui.add_enabled(self.settings.tray, egui::Checkbox::new(&mut ctt, tr("Close to tray"))).changed() {
            self.set_setting(|s| s.close_to_tray = ctt);
        }
    }

    /// Call every frame from the app: windows, tray events, auto-stop, close-to-tray.
    pub fn frame(&mut self, ectx: &egui::Context) {
        if !self.started {
            self.started = true;
            if self.settings.tray {
                self.set_tray(true, ectx);
            }
        }
        for a in self.tray.poll() {
            match a {
                tray::Action::Show | tray::Action::Sensors => {
                    self.show_sensors |= a == tray::Action::Sensors;
                    ectx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ectx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ectx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                tray::Action::Quit => {
                    self.quitting = true;
                    ectx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if self.tray.active()
            && self.settings.close_to_tray
            && !self.quitting
            && ectx.input(|i| i.viewport().close_requested())
        {
            ectx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ectx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if let Some((tel, _)) = &self.tel {
            let series = tel.series();
            let (limit, tjmax) = (self.stress_limit(), self.tjmax());
            self.stress.tick(&series, limit);
            let tip = summary(&series)
                .iter()
                .filter_map(|(n, t)| t.map(|t| format!("{n} {}", fmt0(t))))
                .collect::<Vec<_>>()
                .join(" · ");
            self.tray.update(cpu_temp(&series).map(|s| s.current), tjmax, &tip);
            if self.stress.running() || self.tray.active() || self.show_mini {
                ectx.request_repaint_after(Duration::from_secs(1));
            }
        }

        let size = ectx.content_rect().size();
        let default = egui::vec2(WINDOW_SIZE[0].min(size.x - 16.0), WINDOW_SIZE[1].min(size.y - 40.0));
        if self.show_sensors {
            let mut open = true;
            egui::Window::new(tr("Sensors"))
                .open(&mut open)
                .default_size(default)
                .default_pos([8.0, 32.0])
                .show(ectx, |ui| page(ui, self));
            self.show_sensors = open;
        }
        if self.show_stress {
            let mut open = true;
            egui::Window::new(tr("Stress test")).open(&mut open).default_size(default).default_pos([8.0, 32.0]).show(
                ectx,
                |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| stress::page(ui, self, true));
                },
            );
            self.show_stress = open;
        }
        if self.show_mini {
            self.mini(ectx);
        }
    }

    /// Always-on-top borderless window with the key readings (all OSes).
    fn mini(&mut self, ectx: &egui::Context) {
        let series = self.tel().series();
        let tjmax = self.tjmax();
        let builder = egui::ViewportBuilder::default()
            .with_title(format!("{} · {}", crate::DISPLAY_NAME, tr("Mini monitor")))
            .with_inner_size(MINI_SIZE)
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false);
        let keep = ectx.show_viewport_immediate(egui::ViewportId::from_hash_of("mini-monitor"), builder, |ui, _| {
            let mut keep = !ui.ctx().input(|i| i.viewport().close_requested());
            egui::CentralPanel::default().show(ui, |ui| {
                let bg = ui.interact(ui.max_rect(), ui.id().with("drag"), egui::Sense::click_and_drag());
                if bg.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                bg.context_menu(|ui| {
                    if ui.button(tr("Show CpuZeeZ")).clicked() {
                        ectx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ectx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    if ui.button(tr("Close")).clicked() {
                        keep = false;
                    }
                });
                summary_row(ui, &series, tjmax);
                let clocks = core_clocks(&series);
                let mut line = Vec::new();
                if !clocks.is_empty() {
                    let avg = clocks.iter().map(|s| s.current).sum::<f64>() / clocks.len() as f64;
                    line.push(fmt(avg, Unit::Mhz));
                }
                if let Some(p) = package_power(&series) {
                    line.push(fmt(p.current, Unit::Watt));
                }
                ui.label(RichText::new(line.join(" · ")).monospace());
            });
            keep
        });
        self.show_mini = keep;
    }

    /// `-screenshots`: step 0 = Sensors window with ≥5 s of samples, step 1 =
    /// stress monitor after ~8 s of load. True once ready to capture.
    pub fn autoshot(&mut self, step: u8, ectx: &egui::Context) -> bool {
        let since = *self.shot_since.get_or_insert_with(|| {
            ectx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(760.0, 720.0)));
            Instant::now()
        });
        // Open the window once the resize has landed so it gets the full size.
        if since.elapsed() < Duration::from_millis(600) {
            return false;
        }
        let tel_age = self.tel.as_ref().map_or(Duration::ZERO, |t| t.1.elapsed());
        if step == 0 {
            self.tel();
            self.show_sensors = true;
            return tel_age >= Duration::from_secs(5) && since.elapsed() > Duration::from_secs(1);
        }
        self.show_sensors = false;
        self.show_stress = true;
        if !self.stress.running() {
            self.start_stress(crate::os::logical_cpus());
        }
        self.stress.elapsed_secs() >= 8.0
    }

    pub fn autoshot_next(&mut self) {
        self.shot_since = None;
    }
}

// ---- series helpers (shared with the stress monitor and tray) ----

pub fn fmt(v: f64, u: Unit) -> String {
    match u {
        Unit::Celsius => format!("{v:.1} °C"),
        Unit::Volt => format!("{v:.3} V"),
        Unit::Watt => format!("{v:.1} W"),
        Unit::Ampere => format!("{v:.2} A"),
        Unit::Mhz => format!("{v:.0} MHz"),
        Unit::Rpm => format!("{v:.0} RPM"),
        Unit::Percent => format!("{v:.1} %"),
        Unit::Megabytes if v >= 10240.0 => format!("{:.1} GB", v / 1024.0),
        Unit::Megabytes => format!("{v:.0} MB"),
    }
}

/// Whole-degree temperature for compact summaries.
fn fmt0(t: f64) -> String {
    format!("{t:.0} °C")
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

/// Text color for a tone (None = default text color).
pub fn tone_color(ui: &egui::Ui, t: Tone) -> Option<Color32> {
    let p = theme::palette(ui);
    match t {
        Tone::Danger => Some(p.danger),
        Tone::Warning => Some(p.warning),
        Tone::Success => Some(p.success),
        Tone::Accent => Some(p.accent),
        Tone::Neutral => None,
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
    let series = tel.series();
    let tjmax = mon.tjmax();
    ui.ctx().request_repaint_after(Duration::from_millis(mon.settings.sample_ms.clamp(250, 1000)));
    widgets::page_header(ui, tr("Sensors"), "", |ui| toolbar_actions(ui, mon, &tel));
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

    let needle = v.filter.to_lowercase();
    let mut groups: Vec<&str> = Vec::new();
    for s in &series {
        if !groups.contains(&s.group.as_str()) {
            groups.push(&s.group);
        }
    }
    let table_h = if v.plotted.is_empty() { ui.available_height() } else { ui.available_height() * 0.5 };
    let secondary = theme::palette(ui).text_secondary;
    widgets::card(ui, "", "", |ui| {
        egui::ScrollArea::vertical().id_salt("sensor-table").max_height(table_h).auto_shrink([false, true]).show(
            ui,
            |ui| {
                for g in groups {
                    let rows: Vec<&Series> = series
                        .iter()
                        .filter(|s| s.group == g)
                        .filter(|s| {
                            needle.is_empty()
                                || s.label.to_lowercase().contains(&needle)
                                || g.to_lowercase().contains(&needle)
                        })
                        .collect();
                    if rows.is_empty() {
                        continue;
                    }
                    let head = RichText::new(format!("{}  {g}", group_icon(g))).strong();
                    egui::CollapsingHeader::new(head).id_salt(g).default_open(true).show(ui, |ui| {
                        egui::Grid::new(("sensor-grid", g)).num_columns(5).striped(true).min_col_width(COL_W).show(
                            ui,
                            |ui| {
                                for h in [tr("Sensor"), tr("Current"), tr("Min"), tr("Max"), tr("Avg")] {
                                    ui.label(RichText::new(h).small().color(secondary));
                                }
                                ui.end_row();
                                for s in rows {
                                    sensor_row(ui, s, &mut v.plotted, tjmax);
                                }
                            },
                        );
                    });
                }
            },
        );
    });
    ui.add_space(theme::GAP);
    widgets::card(ui, icon::CHART_LINE, tr("History"), |ui| {
        charts(ui, &series, &v.plotted, mon.settings.graph_seconds as f64);
    });
}

fn sensor_row(ui: &mut egui::Ui, s: &Series, plotted: &mut BTreeSet<String>, tjmax: f64) {
    let mut on = plotted.contains(&s.id);
    ui.horizontal(|ui| {
        ui.checkbox(&mut on, "");
        if ui.selectable_label(on, &s.label).clicked() {
            on = !on;
        }
    });
    if on {
        plotted.insert(s.id.clone());
    } else {
        plotted.remove(&s.id);
    }
    let lim = (s.unit == Unit::Celsius).then(|| limit_for(&s.group, tjmax)).flatten();
    for x in [s.current, s.min, s.max, s.avg] {
        let mut t = RichText::new(fmt(x, s.unit)).monospace();
        if let Some(c) = lim.and_then(|l| tone_color(ui, temp_tone(x, l))) {
            t = t.color(c);
        }
        ui.label(t);
    }
    ui.end_row();
}

/// Header actions (right-aligned, so added right to left).
fn toolbar_actions(ui: &mut egui::Ui, mon: &mut Monitor, tel: &Telemetry) {
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
        ui.add(
            egui::TextEdit::singleline(&mut mon.view.filter)
                .hint_text(format!("{}  {}", icon::MAGNIFYING_GLASS, tr("Search…")))
                .desired_width(180.0),
        );
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
fn log_path() -> Option<PathBuf> {
    let dir = crate::settings::dir()?.join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(format!("{}-{}.csv", crate::NAME, local_stamp())))
}

fn local_stamp() -> String {
    #[cfg(unix)]
    {
        // SAFETY: localtime_r only writes into the zeroed tm we pass.
        let tm = unsafe {
            let t = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&t, &mut tm);
            tm
        };
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
    #[cfg(windows)]
    {
        #[repr(C)]
        #[derive(Default)]
        struct SystemTime {
            year: u16,
            month: u16,
            dow: u16,
            day: u16,
            hour: u16,
            min: u16,
            sec: u16,
            ms: u16,
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetLocalTime(t: *mut SystemTime);
        }
        let mut t = SystemTime::default();
        // SAFETY: GetLocalTime fills the struct we own.
        unsafe { GetLocalTime(&mut t) };
        format!("{:04}{:02}{:02}-{:02}{:02}{:02}", t.year, t.month, t.day, t.hour, t.min, t.sec)
    }
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

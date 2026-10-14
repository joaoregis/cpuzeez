//! Monitored stress test: runs `bench::stress`, shows elapsed time, CPU/GPU
//! temperatures, effective clocks, package power, live charts, a throttling
//! banner, and stops itself at a temperature limit. Shown in the Bench page
//! while Stress CPU runs and usable on its own (Tools ▸ Stress test).

use super::i18n::tr;
use super::plot;
use super::sensors::{self, Monitor};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::bench::StressHandle;
use crate::telemetry::{Series, ThrottleKind, Unit};
use eframe::egui::{self, CornerRadius, Margin, RichText, Stroke};
use std::time::Instant;

pub const PT_BR: &[(&str, &str)] = &[
    ("Stress test", "Teste de estresse"),
    ("Threads", "Threads"),
    ("Start stress test", "Iniciar teste de estresse"),
    ("Stop", "Parar"),
    ("Stop at", "Parar em"),
    ("Elapsed", "Tempo decorrido"),
    ("CPU temperature", "Temperatura da CPU"),
    ("max", "máx."),
    ("Effective clock", "Clock efetivo"),
    ("min", "mín."),
    ("Package power", "Potência do pacote"),
    ("GPU temperature", "Temperatura da GPU"),
    ("Temperatures", "Temperaturas"),
    ("Clocks", "Clocks"),
    ("Power", "Potência"),
    ("Thermal throttling", "Limitação térmica"),
    ("Power limit", "Limite de potência"),
    ("Current limit", "Limite de corrente"),
    ("GPU throttling", "GPU limitando"),
    ("Stopped: temperature limit reached", "Parado: limite de temperatura atingido"),
    ("Last run", "Última execução"),
    (
        "Loads every selected thread with the widest SIMD workload.",
        "Carrega todas as threads selecionadas com a carga SIMD mais larga.",
    ),
    ("Waiting for sensor data…", "Aguardando dados dos sensores…"),
];
pub const ES: &[(&str, &str)] = &[
    ("Stress test", "Prueba de estrés"),
    ("Threads", "Hilos"),
    ("Start stress test", "Iniciar prueba de estrés"),
    ("Stop", "Detener"),
    ("Stop at", "Detener a"),
    ("Elapsed", "Tiempo transcurrido"),
    ("CPU temperature", "Temperatura de la CPU"),
    ("max", "máx."),
    ("Effective clock", "Reloj efectivo"),
    ("min", "mín."),
    ("Package power", "Potencia del paquete"),
    ("GPU temperature", "Temperatura de la GPU"),
    ("Temperatures", "Temperaturas"),
    ("Clocks", "Relojes"),
    ("Power", "Potencia"),
    ("Thermal throttling", "Limitación térmica"),
    ("Power limit", "Límite de potencia"),
    ("Current limit", "Límite de corriente"),
    ("GPU throttling", "GPU limitando"),
    ("Stopped: temperature limit reached", "Detenido: se alcanzó el límite de temperatura"),
    ("Last run", "Última ejecución"),
    (
        "Loads every selected thread with the widest SIMD workload.",
        "Carga todos los hilos seleccionados con la carga SIMD más ancha.",
    ),
    ("Waiting for sensor data…", "Esperando datos de los sensores…"),
];

const CHART_H: f32 = 90.0;
const SPARK_POINTS: usize = 60;
const TILE_MIN_W: f32 = 112.0;
const CHART_MIN_W: f32 = 260.0;
/// Charts show at least this much history while the run is young.
const MIN_SPAN: f64 = 30.0;

pub struct Stress {
    handle: Option<StressHandle>,
    started: Instant,
    pub threads: usize,
    max_temp: Option<f64>,
    pub auto_stop: bool,
    /// °C; None = Tjmax, else settings.warn_temp_c.
    pub limit: Option<f64>,
    /// Summary of the previous run.
    last: Option<String>,
}

impl Default for Stress {
    fn default() -> Self {
        Stress {
            handle: None,
            started: Instant::now(),
            threads: crate::os::logical_cpus(),
            max_temp: None,
            auto_stop: true,
            limit: None,
            last: None,
        }
    }
}

impl Stress {
    pub fn running(&self) -> bool {
        self.handle.is_some()
    }

    pub fn start(&mut self, threads: usize) {
        self.threads = threads.max(1);
        self.handle = Some(crate::bench::stress(self.threads));
        self.started = Instant::now();
        self.max_temp = None;
        self.last = None;
    }

    pub fn stop(&mut self, reason: Option<&str>) {
        if let Some(h) = self.handle.take() {
            h.stop();
            let max = self.max_temp.map(|t| format!(", {} {}", tr("max"), sensors::fmt(t, Unit::Celsius)));
            let why = reason.map(|r| format!(" — {r}")).unwrap_or_default();
            self.last = Some(format!("{}: {}{}{why}", tr("Last run"), elapsed(self.started), max.unwrap_or_default()));
        }
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    /// Every frame: track the max temperature and auto-stop at the limit.
    pub fn tick(&mut self, series: &[Series], limit: f64) {
        if !self.running() {
            return;
        }
        let Some(t) = sensors::cpu_temp(series).map(|s| s.current) else { return };
        self.max_temp = Some(self.max_temp.map_or(t, |m| m.max(t)));
        if self.auto_stop && t >= limit {
            self.stop(Some(tr("Stopped: temperature limit reached")));
        }
    }
}

fn elapsed(since: Instant) -> String {
    let s = since.elapsed().as_secs();
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

/// Full-width warning strip.
fn banner(ui: &mut egui::Ui, tone: Tone, title: &str, detail: &str) {
    let p = theme::palette(ui);
    let (bg, fg) = if tone == Tone::Danger { (p.danger_soft, p.danger) } else { (p.warning_soft, p.warning) };
    egui::Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, fg))
        .corner_radius(CornerRadius::same(theme::RADIUS_CARD))
        .inner_margin(Margin::same(theme::PAD_CARD))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let head = RichText::new(format!("{}  {title}", icon::WARNING)).text_style(egui::TextStyle::Heading);
            ui.label(head.color(fg));
            if !detail.is_empty() {
                ui.label(RichText::new(detail).color(p.text));
            }
        });
    ui.add_space(theme::GAP);
}

fn spark(s: Option<&Series>) -> Vec<f64> {
    s.map(|s| s.points.iter().rev().take(SPARK_POINTS).rev().map(|p| p.1).collect()).unwrap_or_default()
}

/// Average across cores per sample (the sampler records every series each pass).
fn avg_spark(clocks: &[&Series]) -> Vec<f64> {
    let n = clocks.iter().map(|s| s.points.len()).min().unwrap_or(0).min(SPARK_POINTS);
    (0..n)
        .map(|i| clocks.iter().map(|s| s.points[s.points.len() - n + i].1).sum::<f64>() / clocks.len() as f64)
        .collect()
}

/// The stress page. `controls`: header with thread count + start/stop
/// (standalone use; the Bench page has its own start button).
pub fn page(ui: &mut egui::Ui, mon: &mut Monitor, controls: bool) {
    let limit = mon.stress_limit();
    let running = mon.stress.running();
    if controls {
        let sub = tr("Loads every selected thread with the widest SIMD workload.");
        widgets::page_header(ui, tr("Stress test"), sub, |ui| {
            if running {
                if ui.button(format!("{}  {}", icon::STOP, tr("Stop"))).clicked() {
                    mon.stress.stop(None);
                }
            } else if widgets::primary_button(ui, &format!("{}  {}", icon::PLAY, tr("Start stress test"))).clicked() {
                let n = mon.stress.threads;
                mon.start_stress(n);
            }
            let max = crate::os::logical_cpus();
            ui.add_enabled(!running, egui::DragValue::new(&mut mon.stress.threads).range(1..=max));
            ui.label(tr("Threads"));
        });
    }
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut mon.stress.auto_stop, tr("Stop at"));
        let mut l = limit;
        if ui.add(egui::DragValue::new(&mut l).range(50.0..=115.0).suffix(" °C")).changed() {
            mon.stress.limit = Some(l);
        }
        if let Some(last) = &mon.stress.last {
            ui.label(RichText::new(last).color(theme::palette(ui).text_secondary));
        }
    });
    ui.add_space(theme::GAP / 2.0);
    if !mon.stress.running() {
        return;
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    let tel = mon.tel();
    for t in tel.throttling() {
        let (what, tone) = match t.kind {
            ThrottleKind::Thermal => (tr("Thermal throttling"), Tone::Danger),
            ThrottleKind::Power => (tr("Power limit"), Tone::Warning),
            ThrottleKind::Current => (tr("Current limit"), Tone::Warning),
            ThrottleKind::Gpu => (tr("GPU throttling"), Tone::Warning),
        };
        banner(ui, tone, &format!("{what} · {}", t.device), &t.detail);
    }
    let series = tel.series();
    let tjmax = mon.tjmax();
    let st = &mon.stress;
    let cpu_t = sensors::cpu_temp(&series);
    let gpu_t = sensors::gpu_temp(&series);
    let clocks = sensors::core_clocks(&series);
    let power = sensors::package_power(&series);
    let colors = theme::palette(ui).series;
    let dash = || "–".to_string();

    let cur: Vec<f64> = clocks.iter().map(|s| s.current).collect();
    // Values stay short (big metric font); the secondary figure goes in the label.
    let (clock_text, clock_label) = if cur.is_empty() {
        (dash(), tr("Effective clock").to_string())
    } else {
        let avg = cur.iter().sum::<f64>() / cur.len() as f64;
        let min = cur.iter().copied().fold(f64::MAX, f64::min);
        (sensors::fmt(avg, Unit::Mhz), format!("{} · {} {min:.0}", tr("Effective clock"), tr("min")))
    };
    let cpu_text = cpu_t.map_or_else(dash, |s| format!("{:.0} °C", s.current));
    let cpu_label = match st.max_temp {
        Some(m) => format!("{} · {} {m:.0} °C", tr("CPU temperature"), tr("max")),
        None => tr("CPU temperature").to_string(),
    };
    let gpu_text = gpu_t.map_or_else(dash, |s| format!("{:.0} °C", s.current));
    let power_text = power.map_or_else(dash, |s| sensors::fmt(s.current, Unit::Watt));
    let tone = |s: Option<&Series>, lim: f64| s.map_or(Tone::Neutral, |s| sensors::temp_tone(s.current, lim));
    let tiles: Vec<(&str, String, Vec<f64>, egui::Color32, Tone)> = vec![
        (tr("Elapsed"), elapsed(st.started), Vec::new(), colors[5], Tone::Neutral),
        (&cpu_label, cpu_text, spark(cpu_t), colors[0], tone(cpu_t, tjmax)),
        (
            &clock_label,
            clock_text,
            sensors::avg_clock(&series).map_or_else(|| avg_spark(&clocks), |a| spark(Some(a))),
            colors[3],
            Tone::Neutral,
        ),
        (tr("Package power"), power_text, spark(power), colors[4], Tone::Neutral),
        (tr("GPU temperature"), gpu_text, spark(gpu_t), colors[1], tone(gpu_t, sensors::GPU_LIMIT)),
    ];
    widgets::columns(ui, 5, TILE_MIN_W, tiles.len(), |ui, i| {
        let (l, v, sp, col, tone) = &tiles[i];
        widgets::metric(ui, l, v, sp, *col, *tone);
    });
    if series.is_empty() {
        widgets::empty_state(ui, icon::THERMOMETER, tr("Stress test"), tr("Waiting for sensor data…"));
        return;
    }
    let now = series.iter().filter_map(|s| s.points.last()).map(|p| p.0).fold(0.0, f64::max);
    let span = st.elapsed_secs().clamp(MIN_SPAN, mon.settings.graph_seconds as f64);
    let temps: Vec<&Series> = cpu_t.into_iter().chain(gpu_t).collect();
    let charts: Vec<(&str, &str, Vec<&Series>, Unit)> = [
        (icon::THERMOMETER, tr("Temperatures"), temps, Unit::Celsius),
        (icon::GAUGE, tr("Clocks"), sensors::avg_clock(&series).map_or(clocks, |a| vec![a]), Unit::Mhz),
        (icon::LIGHTNING, tr("Power"), power.into_iter().collect(), Unit::Watt),
    ]
    .into_iter()
    .filter(|c| !c.2.is_empty())
    .collect();
    widgets::columns(ui, 3, CHART_MIN_W, charts.len(), |ui, i| {
        let (ic, title, list, unit) = &charts[i];
        widgets::card(ui, ic, title, |ui| {
            let lines: Vec<plot::Line> = list
                .iter()
                .enumerate()
                .map(|(i, s)| plot::Line { label: &s.label, color: colors[i % colors.len()], points: &s.points })
                .collect();
            plot::chart(ui, &lines, now, span, CHART_H, &|v| sensors::fmt(v, *unit));
        });
    });
}

//! Monitored stress test (S1-K07): CPU (light / heavy / mixed), a user-mode
//! memory test and GPU stress, alone or combined (PSU and cooling). Shows
//! elapsed time, CPU/GPU temperatures, effective clocks, package power, live
//! charts and a throttling banner, and stops itself at the CPU or GPU
//! temperature limit or on the first memory error. Shown in the Bench page
//! while Stress CPU runs and usable on its own (Tools ▸ Stress test).

use super::i18n::{tr, trf};
use super::plot;
use super::sensors::{self, Monitor};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::bench::{Load, StressHandle};
use crate::benchkit::Job;
use crate::gui::widgets::Control;
use crate::memtest::MemTest;
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
    ("CPU", "CPU"),
    ("Memory", "Memória"),
    ("GPU", "GPU"),
    ("Light", "Leve"),
    ("Heavy", "Pesada"),
    ("Mixed", "Mista"),
    (
        "Light: scalar code. Heavy: the widest SIMD unit (most heat). Mixed: switches every 50 ms to catch unstable voltage changes.",
        "Leve: código escalar. Pesada: a unidade SIMD mais larga (mais calor). Mista: alterna a cada 50 ms para pegar variações de tensão instáveis.",
    ),
    ("Memory test", "Teste de memória"),
    ("{} rounds · {} errors", "{} rodadas · {} erros"),
    (
        "Not a replacement for MemTest86 or TestMem5: it only reaches the memory the OS gives this app.",
        "Não substitui o MemTest86 ou o TestMem5: só alcança a memória que o sistema entrega a este app.",
    ),
    ("Stopped: GPU temperature limit reached", "Parado: limite de temperatura da GPU atingido"),
    ("Stopped: memory error found", "Parado: erro de memória encontrado"),
    ("Stopped: {}", "Parado: {}"),
    ("First error: {}", "Primeiro erro: {}"),
    ("Choose at least one part to stress.", "Escolha pelo menos uma parte para estressar."),
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
    ("CPU", "CPU"),
    ("Memory", "Memoria"),
    ("GPU", "GPU"),
    ("Light", "Ligera"),
    ("Heavy", "Pesada"),
    ("Mixed", "Mixta"),
    (
        "Light: scalar code. Heavy: the widest SIMD unit (most heat). Mixed: switches every 50 ms to catch unstable voltage changes.",
        "Ligera: código escalar. Pesada: la unidad SIMD más ancha (más calor). Mixta: alterna cada 50 ms para detectar cambios de tensión inestables.",
    ),
    ("Memory test", "Prueba de memoria"),
    ("{} rounds · {} errors", "{} rondas · {} errores"),
    (
        "Not a replacement for MemTest86 or TestMem5: it only reaches the memory the OS gives this app.",
        "No sustituye a MemTest86 ni a TestMem5: solo alcanza la memoria que el sistema da a esta app.",
    ),
    ("Stopped: GPU temperature limit reached", "Detenido: se alcanzó el límite de temperatura de la GPU"),
    ("Stopped: memory error found", "Detenido: se encontró un error de memoria"),
    ("Stopped: {}", "Detenido: {}"),
    ("First error: {}", "Primer error: {}"),
    ("Choose at least one part to stress.", "Elija al menos una parte para estresar."),
];

const CHART_H: f32 = 90.0;
const SPARK_POINTS: usize = 60;
const TILE_MIN_W: f32 = 112.0;
const CHART_MIN_W: f32 = 260.0;
/// Charts show at least this much history while the run is young.
const MIN_SPAN: f64 = 30.0;

pub struct Stress {
    handle: Option<StressHandle>,
    mem: Option<MemTest>,
    gpu: Option<Job<()>>,
    started: Instant,
    pub threads: usize,
    /// What to stress: CPU (with its load), memory (GB, 0 = off), GPU.
    pub cpu_on: bool,
    pub load: Load,
    pub mem_gb: u64,
    pub gpu_on: bool,
    pub gpu_pick: usize,
    max_gpu: Option<f64>,
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
            mem: None,
            gpu: None,
            started: Instant::now(),
            threads: crate::os::logical_cpus(),
            cpu_on: true,
            load: Load::Heavy,
            mem_gb: 0,
            gpu_on: false,
            gpu_pick: 0,
            max_gpu: None,
            max_temp: None,
            auto_stop: true,
            limit: None,
            last: None,
        }
    }
}

impl Stress {
    pub fn running(&self) -> bool {
        self.handle.is_some() || self.mem.is_some() || self.gpu.is_some()
    }

    /// Starts every selected part; the CPU part on `threads` threads.
    pub fn start(&mut self, threads: usize) {
        self.threads = threads.max(1);
        if self.cpu_on {
            self.handle = Some(crate::bench::stress(self.threads, self.load));
        }
        if self.mem_gb > 0 {
            // One tester per 2 logical CPUs: enough to saturate the memory
            // controller while leaving the CPU part room.
            self.mem = Some(MemTest::start(self.mem_gb << 30, (crate::os::logical_cpus() / 2).max(1)));
        }
        if self.gpu_on {
            let pick = self.gpu_pick;
            self.gpu = Some(Job::spawn(move |ctl| crate::gpubench::stress(pick, ctl)));
        }
        self.started = Instant::now();
        self.max_temp = None;
        self.max_gpu = None;
        self.last = None;
    }

    pub fn stop(&mut self, reason: Option<&str>) {
        if !self.running() {
            return;
        }
        if let Some(h) = self.handle.take() {
            h.stop();
        }
        let mem = self.mem.take().map(|m| {
            let r = format!(", {}", trf("{} rounds · {} errors", &[&m.rounds(), &m.errors()]));
            m.stop();
            r
        });
        if let Some(g) = self.gpu.take() {
            g.cancel();
            g.wait_quietly();
        }
        let max = self.max_temp.map(|t| format!(", {} {}", tr("max"), sensors::fmt(t, Unit::Celsius)));
        let why = reason.map(|r| format!(" — {r}")).unwrap_or_default();
        self.last = Some(format!(
            "{}: {}{}{}{why}",
            tr("Last run"),
            elapsed(self.started),
            max.unwrap_or_default(),
            mem.unwrap_or_default()
        ));
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }

    /// Every frame: track the max temperatures and stop at the CPU limit, the
    /// GPU limit, the first memory error or a GPU failure.
    pub fn tick(&mut self, series: &[Series], limit: f64) {
        if !self.running() {
            return;
        }
        if let Some(m) = &self.mem
            && (m.errors() > 0 || !m.running())
        {
            let why = if m.errors() > 0 {
                tr("Stopped: memory error found").to_string()
            } else {
                trf("Stopped: {}", &[&m.note().unwrap_or_default()])
            };
            return self.stop(Some(&why));
        }
        if let Some(r) = self.gpu.as_ref().and_then(|g| g.take()) {
            self.gpu = None;
            if let Err(e) = r {
                return self.stop(Some(&trf("Stopped: {}", &[&e])));
            }
        }
        if let Some(t) = sensors::gpu_temp(series).map(|s| s.current)
            && self.gpu.is_some()
        {
            self.max_gpu = Some(self.max_gpu.map_or(t, |m| m.max(t)));
            if self.auto_stop && t >= sensors::GPU_LIMIT {
                return self.stop(Some(tr("Stopped: GPU temperature limit reached")));
            }
        }
        let Some(t) = sensors::cpu_temp(series).map(|s| s.current) else { return };
        self.max_temp = Some(self.max_temp.map_or(t, |m| m.max(t)));
        if self.auto_stop && t >= limit {
            self.stop(Some(tr("Stopped: temperature limit reached")));
        }
    }

    /// The memory test's live line, while it runs.
    fn mem_line(&self) -> Option<(String, Option<String>)> {
        let m = self.mem.as_ref()?;
        Some((trf("{} rounds · {} errors", &[&m.rounds(), &m.errors()]), m.note()))
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
pub fn page(ui: &mut egui::Ui, mon: &mut Monitor, controls: bool, gpus: &[String]) {
    let limit = mon.stress_limit();
    let running = mon.stress.running();
    if controls {
        let sub = tr("Loads every selected thread with the widest SIMD workload.");
        widgets::page_header(ui, tr("Stress test"), sub, |ui| {
            if running {
                if ui.button(format!("{}  {}", icon::STOP, tr("Stop"))).clicked() {
                    mon.stress.stop(None);
                }
            } else {
                let any = mon.stress.cpu_on || mon.stress.mem_gb > 0 || (mon.stress.gpu_on && !gpus.is_empty());
                let b = widgets::primary_button(ui, &format!("{}  {}", icon::PLAY, tr("Start stress test")));
                if b.clicked() && any {
                    let n = mon.stress.threads;
                    mon.start_stress(n);
                }
                if !any {
                    ui.label(
                        RichText::new(tr("Choose at least one part to stress.")).color(theme::palette(ui).warning),
                    );
                }
            }
        });
        parts(ui, mon, gpus, running);
    }
    ui.horizontal_wrapped(|ui| {
        let c = ui.checkbox(&mut mon.stress.auto_stop, tr("Stop at")).id;
        let mut l = limit;
        if ui.add(egui::DragValue::new(&mut l).range(50.0..=115.0).suffix(" °C")).labelled_by(c).changed() {
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
    if let Some((line, note)) = mon.stress.mem_line() {
        let p = theme::palette(ui);
        widgets::card(ui, icon::MEMORY, tr("Memory test"), |ui| {
            ui.label(RichText::new(line).strong());
            if let Some(n) = note {
                ui.label(RichText::new(trf("First error: {}", &[&n])).color(p.danger));
            }
            ui.label(
                RichText::new(tr(
                    "Not a replacement for MemTest86 or TestMem5: it only reaches the memory the OS gives this app.",
                ))
                .small()
                .color(p.text_muted),
            );
        });
        ui.add_space(theme::GAP / 2.0);
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
    tel.want("stress", &["cpu/", "gpu"]);
    let series = tel.series_with(|id| id.starts_with("cpu/") || id.starts_with("gpu"));
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
    let cpu_text = cpu_t.map_or_else(dash, |s| super::units::temp(s.current, 0));
    let cpu_label = match st.max_temp {
        Some(m) => format!("{} · {} {}", tr("CPU temperature"), tr("max"), super::units::temp(m, 0)),
        None => tr("CPU temperature").to_string(),
    };
    let gpu_text = gpu_t.map_or_else(dash, |s| super::units::temp(s.current, 0));
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

/// What to stress: CPU (threads, load), memory (GB) and GPU (adapter).
fn parts(ui: &mut egui::Ui, mon: &mut Monitor, gpus: &[String], running: bool) {
    let st = &mut mon.stress;
    let p = theme::palette(ui);
    ui.add_enabled_ui(!running, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut st.cpu_on, tr("CPU"));
            let max = crate::os::logical_cpus();
            let d = ui.add(egui::DragValue::new(&mut st.threads).range(1..=max));
            d.name_by(ui.label(tr("Threads")).id);
            for (l, name) in [(Load::Light, "Light"), (Load::Heavy, "Heavy"), (Load::Mixed, "Mixed")] {
                ui.selectable_value(&mut st.load, l, tr(name));
            }
        });
        ui.label(
            RichText::new(tr("Light: scalar code. Heavy: the widest SIMD unit (most heat). Mixed: switches every 50 ms to catch unstable voltage changes."))
                .small()
                .color(p.text_muted),
        );
        ui.horizontal_wrapped(|ui| {
            let mut on = st.mem_gb > 0;
            if ui.checkbox(&mut on, tr("Memory")).changed() {
                st.mem_gb = if on { 4 } else { 0 };
            }
            if on {
                for gb in [1u64, 4, 8, 16] {
                    ui.selectable_value(&mut st.mem_gb, gb, format!("{gb} GB"));
                }
            }
        });
        if !gpus.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut st.gpu_on, tr("GPU"));
                if st.gpu_on {
                    super::gpubench_view::adapter_picker(ui, "stress-gpu", gpus, &mut st.gpu_pick, true);
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::Series;

    fn temp(id: &str, group: &str, v: f64) -> Series {
        Series {
            id: id.into(),
            group: group.into(),
            label: id.into(),
            unit: Unit::Celsius,
            points: vec![(0.0, v)],
            current: v,
            min: v,
            max: v,
            avg: v,
        }
    }

    #[test]
    fn auto_stop_limits() {
        let _heavy = crate::heavy_test();
        let mut s = Stress { cpu_on: true, load: Load::Light, ..Default::default() };
        s.start(1);
        s.tick(&[temp("cpu/temp/package", "CPU", 70.0)], 95.0);
        assert!(s.running(), "under the limit keeps going");
        s.tick(&[temp("cpu/temp/package", "CPU", 96.0)], 95.0);
        assert!(!s.running(), "the CPU limit stops it");
        assert!(s.last.as_deref().is_some_and(|l| l.contains("temperature limit")), "{:?}", s.last);

        let mut s = Stress { cpu_on: true, load: Load::Light, auto_stop: false, ..Default::default() };
        s.start(1);
        s.tick(&[temp("cpu/temp/package", "CPU", 120.0)], 95.0);
        assert!(s.running(), "auto-stop off");
        s.stop(None);

        // A memory test that can't allocate stops the run and says why.
        let mut s = Stress { cpu_on: false, mem_gb: 1 << 20, ..Default::default() }; // 1 PiB
        s.start(1);
        let t0 = Instant::now();
        while s.running() && t0.elapsed().as_secs() < 10 {
            s.tick(&[], 95.0);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(s.last.as_deref().is_some_and(|l| l.contains("not enough free memory")), "{:?}", s.last);
    }
}

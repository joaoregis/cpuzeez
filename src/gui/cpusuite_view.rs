//! Benchmarks › Processor › CPU suite (S1-K02): the full suite, thread
//! scaling, P × E cores (hybrids) and the sustained run.

use super::benchkit_view::{self as kit, Saved};
use super::i18n::tr;
use super::theme::{self, icon};
use super::tools::{Series, line_chart};
use super::widgets;
use crate::Ctx;
use crate::benchkit::{self, Job, Sub};
use crate::cpusuite;
use eframe::egui::{self, RichText};

pub const PT_BR: &[(&str, &str)] = &[
    ("CPU suite", "Suíte de CPU"),
    ("Suite", "Suíte"),
    ("Thread scaling", "Escala por threads"),
    ("P × E cores", "Núcleos P × E"),
    ("Sustained", "Sustentado"),
    ("Run", "Executar"),
    ("Integer", "Inteiros"),
    ("Floating point", "Ponto flutuante"),
    ("SIMD GEMM", "GEMM SIMD"),
    ("AES-128 (hardware)", "AES-128 (hardware)"),
    ("AES-128 (software)", "AES-128 (software)"),
    ("SHA-256", "SHA-256"),
    ("Ray tracer", "Ray tracer"),
    ("Branchy sort", "Ordenação com desvios"),
    ("First minute", "Primeiro minuto"),
    ("Last minute", "Último minuto"),
    ("Drop", "Queda"),
    ("{} min", "{} min"),
    (
        "Eight workloads, each on one core and then on every thread: compression, n-body physics, matrix math with the widest SIMD unit, AES with and without the CPU's AES instructions, SHA-256, a ray tracer and sorting.",
        "Oito cargas, cada uma em um núcleo e depois em todas as threads: compressão, física n-corpos, matrizes com a unidade SIMD mais larga, AES com e sem as instruções AES da CPU, SHA-256, um ray tracer e ordenação.",
    ),
    (
        "How the integer score grows from 1 thread to all of them. Flat steps show where SMT or a second CCD adds little.",
        "Como a pontuação de inteiros cresce de 1 thread até todas. Degraus planos mostram onde SMT ou um segundo CCD acrescenta pouco.",
    ),
    (
        "Every test on one core of each type, so P-cores and E-cores can be compared.",
        "Cada teste em um núcleo de cada tipo, para comparar núcleos P e E.",
    ),
    ("This processor has one core type.", "Este processador tem um tipo de núcleo só."),
    (
        "Matrix math on every thread for the chosen time. A drop between the first and the last minute means the CPU throttles (heat or power limits).",
        "Matrizes em todas as threads pelo tempo escolhido. Uma queda entre o primeiro e o último minuto indica que a CPU reduz o clock (calor ou limite de energia).",
    ),
];

pub const ES: &[(&str, &str)] = &[
    ("CPU suite", "Suite de CPU"),
    ("Suite", "Suite"),
    ("Thread scaling", "Escalado por hilos"),
    ("P × E cores", "Núcleos P × E"),
    ("Sustained", "Sostenida"),
    ("Run", "Ejecutar"),
    ("Integer", "Enteros"),
    ("Floating point", "Coma flotante"),
    ("SIMD GEMM", "GEMM SIMD"),
    ("AES-128 (hardware)", "AES-128 (hardware)"),
    ("AES-128 (software)", "AES-128 (software)"),
    ("SHA-256", "SHA-256"),
    ("Ray tracer", "Trazador de rayos"),
    ("Branchy sort", "Ordenación con saltos"),
    ("First minute", "Primer minuto"),
    ("Last minute", "Último minuto"),
    ("Drop", "Caída"),
    ("{} min", "{} min"),
    (
        "Eight workloads, each on one core and then on every thread: compression, n-body physics, matrix math with the widest SIMD unit, AES with and without the CPU's AES instructions, SHA-256, a ray tracer and sorting.",
        "Ocho cargas, cada una en un núcleo y luego en todos los hilos: compresión, física de n cuerpos, matrices con la unidad SIMD más ancha, AES con y sin las instrucciones AES de la CPU, SHA-256, un trazador de rayos y ordenación.",
    ),
    (
        "How the integer score grows from 1 thread to all of them. Flat steps show where SMT or a second CCD adds little.",
        "Cómo crece la puntuación de enteros de 1 hilo a todos. Los escalones planos muestran dónde SMT o un segundo CCD aportan poco.",
    ),
    (
        "Every test on one core of each type, so P-cores and E-cores can be compared.",
        "Cada prueba en un núcleo de cada tipo, para comparar núcleos P y E.",
    ),
    ("This processor has one core type.", "Este procesador tiene un solo tipo de núcleo."),
    (
        "Matrix math on every thread for the chosen time. A drop between the first and the last minute means the CPU throttles (heat or power limits).",
        "Matrices en todos los hilos durante el tiempo elegido. Una caída entre el primer y el último minuto indica que la CPU baja el reloj (calor o límite de potencia).",
    ),
];

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Suite,
    Scaling,
    Hybrid,
    Sustained,
}

type Out = (Vec<Sub>, Vec<(f64, f64)>);

pub struct State {
    mode: Mode,
    runs: usize,
    minutes: f64,
    job: Option<Job<Out>>,
    started: Option<(Mode, benchkit::Conditions)>,
    result: Option<Saved>,
    error: Option<String>,
    label: String,
    status: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            mode: Mode::Suite,
            runs: 1,
            minutes: 10.0,
            job: None,
            started: None,
            result: None,
            error: None,
            label: String::new(),
            status: String::new(),
        }
    }
}

/// First logical CPU of each core type, on hybrids.
fn core_types(ctx: &Ctx) -> Vec<(String, usize)> {
    let c = crate::cpu::get(ctx);
    let mut v: Vec<(String, usize)> = Vec::new();
    for l in &c.logical {
        if let Some(k) = &l.kind
            && !v.iter().any(|x| &x.0 == k)
        {
            v.push((k.clone(), l.cpu));
        }
    }
    if v.len() > 1 { v } else { Vec::new() }
}

impl State {
    pub fn page(&mut self, ui: &mut egui::Ui, ctx: &Ctx, start_temp: Option<f64>) {
        let p = theme::palette(ui);
        widgets::card(ui, icon::CPU, tr("CPU suite"), |ui| {
            let idle = self.job.is_none();
            ui.add_enabled_ui(idle, |ui| {
                widgets::segmented(
                    ui,
                    &mut self.mode,
                    &[
                        (Mode::Suite, "", tr("Suite")),
                        (Mode::Scaling, "", tr("Thread scaling")),
                        (Mode::Hybrid, "", tr("P × E cores")),
                        (Mode::Sustained, "", tr("Sustained")),
                    ],
                );
            });
            let types = core_types(ctx);
            let about = match self.mode {
                Mode::Suite => {
                    "Eight workloads, each on one core and then on every thread: compression, n-body physics, matrix math with the widest SIMD unit, AES with and without the CPU's AES instructions, SHA-256, a ray tracer and sorting."
                }
                Mode::Scaling => {
                    "How the integer score grows from 1 thread to all of them. Flat steps show where SMT or a second CCD adds little."
                }
                Mode::Hybrid if types.is_empty() => "This processor has one core type.",
                Mode::Hybrid => "Every test on one core of each type, so P-cores and E-cores can be compared.",
                Mode::Sustained => {
                    "Matrix math on every thread for the chosen time. A drop between the first and the last minute means the CPU throttles (heat or power limits)."
                }
            };
            ui.label(RichText::new(tr(about)).color(p.text_secondary));
            ui.add_space(4.0);
            ui.add_enabled_ui(idle, |ui| {
                ui.horizontal(|ui| match self.mode {
                    Mode::Suite => kit::runs_picker(ui, &mut self.runs),
                    Mode::Sustained => {
                        for m in [1.0, 10.0, 30.0] {
                            ui.selectable_value(&mut self.minutes, m, super::i18n::trf("{} min", &[&m]));
                        }
                    }
                    _ => {}
                });
            });
            if let Some(r) = kit::poll(ui, &mut self.job) {
                let (mode, cond) = self.started.take().expect("started with the job");
                match r {
                    Ok((subs, series)) => self.result = Some(saved(mode, cond, subs, series)),
                    Err(e) => self.error = Some(e),
                }
            }
            if self.job.is_none()
                && !(self.mode == Mode::Hybrid && types.is_empty())
                && widgets::primary_button(ui, &format!("{}  {}", icon::PLAY, tr("Run"))).clicked()
            {
                self.start(start_temp, types);
            }
            if let Some(e) = &self.error {
                ui.label(RichText::new(super::i18n::trf("Stopped: {}", &[e])).color(p.danger));
            }
            ui.label(
                RichText::new(tr("1000 points = the reference machine (Ryzen 9 7900X). The total is the geometric mean of the sub-tests."))
                    .small()
                    .color(p.text_muted),
            );
        });
        ui.add_space(theme::GAP);
        if let Some(r) = &self.result {
            let title = if r.unit == "points" { format!("{}: {:.0}", tr("Score"), r.score) } else { String::new() };
            widgets::card(ui, "", &title, |ui| {
                kit::conditions(ui, &r.conditions);
                if r.series.len() > 1 {
                    let pts = r.series.clone();
                    let s = [Series { name: String::new(), color: p.series[0], points: pts }];
                    let unit = r.subs.first().map_or(String::new(), |s| s.unit.clone());
                    let scaling = r.kind == "CPU scaling";
                    let x = move |x: f64| if scaling { format!("{x:.0}") } else { format!("{x:.0} s") };
                    line_chart(ui, &s, false, &x, &move |y| format!("{} {unit}", kit::num(y)));
                }
                kit::table(ui, "cpusuite", &r.subs);
            });
            ui.add_space(theme::GAP);
            kit::save(ui, ctx, r, &mut self.label, &mut self.status);
        }
    }

    fn start(&mut self, start_temp: Option<f64>, types: Vec<(String, usize)>) {
        let cpus: Vec<usize> = (0..crate::os::logical_cpus()).collect();
        let (mode, runs, minutes) = (self.mode, self.runs, self.minutes);
        self.result = None;
        self.error = None;
        self.status.clear();
        self.started = Some((mode, benchkit::conditions(start_temp, if mode == Mode::Suite { runs } else { 1 })));
        self.job = Some(Job::spawn(move |ctl| match mode {
            Mode::Suite => {
                benchkit::repeat(ctl, runs, |c, i| cpusuite::suite(&cpus, c, i, runs)).map(|s| (s, Vec::new()))
            }
            Mode::Scaling => cpusuite::scaling(&cpus, ctl).map(|s| {
                let pts = s.iter().filter_map(|x| Some((x.name.split(' ').next()?.parse().ok()?, x.value))).collect();
                (s, pts)
            }),
            Mode::Hybrid => cpusuite::p_vs_e(&types, ctl).map(|s| (s, Vec::new())),
            Mode::Sustained => cpusuite::sustained(&cpus, minutes, 10.0, ctl),
        }));
    }
}

fn saved(mode: Mode, conditions: benchkit::Conditions, subs: Vec<Sub>, series: Vec<(f64, f64)>) -> Saved {
    let threads = crate::os::logical_cpus();
    let variant = cpusuite::simd().to_string();
    let (kind, score, unit) = match mode {
        Mode::Suite => ("CPU suite", benchkit::total(&subs).unwrap_or(0.0), "points"),
        Mode::Scaling => ("CPU scaling", subs.last().map_or(0.0, |s| s.value), "MB/s"),
        Mode::Hybrid => ("CPU P × E", subs.first().map_or(0.0, |s| s.value), "MB/s"),
        Mode::Sustained => ("CPU sustained", subs.get(2).map_or(0.0, |s| s.value), "%"),
    };
    Saved { kind, score, unit, variant, threads, subs, conditions, series }
}

//! Bottleneck detector (S2c, docs/v1/bottleneck-detector.md): what limits
//! the frame rate, measured rather than guessed. Pure, so every rule is
//! tested on windows of data and on recorded sessions without a GPU or a
//! game.
//!
//! One [`Window`] is one second of a game. [`classify`] runs the rules in
//! priority order (limiter, GPU, CPU, throttling, memory, disk, background
//! load) and returns the first that fires as the verdict, with a confidence
//! and numeric evidence, and the others as notes. [`analyze`] turns many
//! windows into the session result: time per class, a timeline, the
//! strongest evidence and recommendations that each carry their evidence.
//!
//! Every user-visible text is a `key` (an English template with `{}`
//! placeholders) and its arguments; the GUI translates the key, the HTML
//! report fills it in English with [`Evidence::render`].

use crate::frames::{self, Frame};
use crate::session::{Band, HwLog};

// ------------------------------------------------------------------ thresholds

/// Every number the rules use, in one place so anyone can audit why the app
/// reached a conclusion (docs/v1/bottleneck-detector.md §3).
#[derive(Debug, Clone, PartialEq)]
pub struct Thresholds {
    /// GPU Busy ≥ this share of the frame time: the GPU worked the whole frame.
    pub gpu_busy_bound: f64,
    /// … and ≥ this much for *high* confidence.
    pub gpu_busy_certain: f64,
    /// GPU Busy below this share: the GPU has headroom.
    pub gpu_busy_headroom: f64,
    /// Without GPU Busy: GPU utilization ≥ this % is "probably GPU-bound" …
    pub gpu_util_bound: f64,
    /// … and below this % the GPU has headroom.
    pub gpu_util_headroom: f64,
    /// The game's busiest thread, % of one core, that counts as saturated.
    pub thread_bound: f64,
    /// CPU Busy share of the frame time that counts as saturated (PresentMon).
    pub cpu_busy_bound: f64,
    /// Frame-time variation (stdev / mean) under this is "very regular".
    pub steady_cv: f64,
    /// An FPS within this share of a refresh rate or a cap matches it.
    pub match_tolerance: f64,
    /// Round numbers games and drivers cap at.
    pub caps: &'static [f64],
    /// VRAM used, % of the card's, that is "nearly full".
    pub vram_full: f64,
    /// RAM used, %, that is "nearly full".
    pub ram_full: f64,
    /// Hard page faults per second (pages read in) that mean real paging.
    pub paging: f64,
    /// The game reading this many MB/s while it stutters.
    pub disk_read_mbps: f64,
    /// Another process using this % of the whole CPU.
    pub background_cpu: f64,
    /// Windows needed before a verdict is trusted.
    pub min_frames: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            gpu_busy_bound: 0.92,
            gpu_busy_certain: 0.95,
            gpu_busy_headroom: 0.80,
            gpu_util_bound: 97.0,
            gpu_util_headroom: 85.0,
            thread_bound: 90.0,
            cpu_busy_bound: 0.90,
            steady_cv: 0.03,
            match_tolerance: 0.015,
            caps: &[
                30.0, 40.0, 45.0, 48.0, 50.0, 60.0, 72.0, 75.0, 90.0, 100.0, 120.0, 144.0, 165.0, 170.0, 180.0, 240.0,
                360.0,
            ],
            vram_full: 95.0,
            ram_full: 92.0,
            paging: 200.0,
            disk_read_mbps: 150.0,
            background_cpu: 12.0,
            min_frames: 10,
        }
    }
}

// ------------------------------------------------------------------ model

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Class {
    Gpu,
    Cpu,
    Limiter,
    Thermal,
    Power,
    Vram,
    Ram,
    Disk,
    Background,
    Inconclusive,
}

impl Class {
    pub const ALL: [Class; 10] = [
        Class::Gpu,
        Class::Cpu,
        Class::Limiter,
        Class::Thermal,
        Class::Power,
        Class::Vram,
        Class::Ram,
        Class::Disk,
        Class::Background,
        Class::Inconclusive,
    ];

    /// The short name used in the stacked bar and the timeline legend.
    pub fn label(self) -> &'static str {
        match self {
            Class::Gpu => "GPU",
            Class::Cpu => "CPU",
            Class::Limiter => "Limiter",
            Class::Thermal => "Thermal",
            Class::Power => "Power limit",
            Class::Vram => "VRAM",
            Class::Ram => "RAM",
            Class::Disk => "Disk",
            Class::Background => "Background load",
            Class::Inconclusive => "Inconclusive",
        }
    }

    /// The verdict headline.
    pub fn headline(self) -> &'static str {
        match self {
            Class::Gpu => "GPU at its limit",
            Class::Cpu => "CPU at its limit",
            Class::Limiter => "Held back on purpose",
            Class::Thermal => "Thermal throttling",
            Class::Power => "Power limit",
            Class::Vram => "Video memory is full",
            Class::Ram => "The system is paging",
            Class::Disk => "Waiting on the disk",
            Class::Background => "Another program is competing",
            Class::Inconclusive => "Nothing at its limit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LimiterKind {
    /// Frame time equals 1/refresh: V-Sync, or VRR at its ceiling.
    VSync,
    /// FPS steady at a round number or a Reflex-style cap.
    FpsCap,
    /// The game isn't the window in front.
    Menu,
    /// Presented through the desktop compositor with nothing else limiting.
    Compositor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn label(self) -> &'static str {
        match self {
            Confidence::Low => "Low confidence",
            Confidence::Medium => "Medium confidence",
            Confidence::High => "High confidence",
        }
    }
}

/// A sentence with numbers: an English template (`{}` placeholders, filled in
/// order) and its arguments. The GUI translates `key`.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub key: &'static str,
    pub args: Vec<String>,
}

impl Evidence {
    fn new(key: &'static str, args: &[String]) -> Evidence {
        Evidence { key, args: args.to_vec() }
    }

    /// The sentence in English.
    pub fn render(&self) -> String {
        fill(self.key, &self.args)
    }
}

/// `{}` placeholders filled in order (the same convention as the GUI's `trf`).
pub fn fill(template: &str, args: &[String]) -> String {
    let mut it = args.iter();
    let mut out = String::new();
    let mut rest = template;
    while let Some(i) = rest.find("{}") {
        out += &rest[..i];
        out += it.next().map_or("", String::as_str);
        rest = &rest[i + 2..];
    }
    out + rest
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub class: Class,
    pub confidence: Confidence,
    pub limiter: Option<LimiterKind>,
    pub evidence: Vec<Evidence>,
}

/// One second of a game.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Window {
    pub frames: usize,
    pub fps: f64,
    pub median_ms: f64,
    /// Frame-time variation: stdev / mean.
    pub ms_cv: f64,
    pub stutters: usize,
    /// GPU Busy / CPU Busy as a share of the frame time (PresentMon 2.x).
    pub gpu_busy_frac: Option<f64>,
    pub cpu_busy_frac: Option<f64>,
    pub present_mode: String,
    pub refresh_hz: Option<f64>,
    /// The game's window is the one in front.
    pub focused: Option<bool>,
    /// GPU utilization, %.
    pub gpu_util: Option<f64>,
    pub cpu_total: Option<f64>,
    pub busiest_core: Option<f64>,
    pub busiest_core_id: Option<usize>,
    /// The game's busiest thread, % of one core.
    pub game_thread: Option<f64>,
    pub vram_used_pct: Option<f64>,
    pub ram_used_pct: Option<f64>,
    /// Hard page faults (pages read in) per second.
    pub pages_in: Option<f64>,
    pub game_read_mbps: Option<f64>,
    pub cpu_thermal: bool,
    pub cpu_power: bool,
    pub gpu_thermal: bool,
    pub gpu_power: bool,
    pub cpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
    /// The heaviest other process: name, % of the whole CPU.
    pub background: Option<(String, f64)>,
}

impl Window {
    /// Frame statistics of one second of `frames` (the rest stays unset).
    pub fn from_frames(frames: &[Frame]) -> Window {
        let ms: Vec<f64> = frames.iter().map(|f| f.ms).filter(|m| *m > 0.0).collect();
        if ms.len() < 2 {
            return Window { frames: ms.len(), ..Default::default() };
        }
        let total: f64 = ms.iter().sum();
        let mean = total / ms.len() as f64;
        let var = ms.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / ms.len() as f64;
        let mut sorted = ms.clone();
        sorted.sort_by(f64::total_cmp);
        let median = frames::percentile(&sorted, 50.0);
        let share = |sel: fn(&Frame) -> Option<f64>| {
            let v: Vec<f64> = frames.iter().filter_map(sel).collect();
            (!v.is_empty()).then(|| (v.iter().sum::<f64>() / v.len() as f64 / mean).clamp(0.0, 2.0))
        };
        Window {
            frames: ms.len(),
            fps: ms.len() as f64 / (total / 1000.0),
            median_ms: median,
            ms_cv: var.sqrt() / mean,
            stutters: ms.iter().filter(|&&m| frames::is_stutter(m, median)).count(),
            gpu_busy_frac: share(|f| f.gpu_busy_ms),
            cpu_busy_frac: share(|f| f.cpu_busy_ms),
            present_mode: frames.last().map(|f| f.present_mode.clone()).unwrap_or_default(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub primary: Finding,
    /// Other rules that fired in the same window (a thermal flag under a GPU-bound game, say).
    pub also: Vec<Finding>,
}

// ------------------------------------------------------------------ rules

fn pct(x: f64) -> String {
    format!("{:.0}", x * 100.0)
}

fn n0(x: f64) -> String {
    format!("{x:.0}")
}

/// Does the GPU have headroom? `Some(true)` yes, `Some(false)` no, None: unknown.
fn gpu_headroom(w: &Window, t: &Thresholds) -> Option<bool> {
    match (w.gpu_busy_frac, w.gpu_util) {
        (Some(b), _) => Some(b < t.gpu_busy_headroom),
        (None, Some(u)) => Some(u < t.gpu_util_headroom),
        _ => None,
    }
}

fn gpu_saturated(w: &Window, t: &Thresholds) -> bool {
    match (w.gpu_busy_frac, w.gpu_util) {
        (Some(b), _) => b >= t.gpu_busy_bound,
        (None, Some(u)) => u >= t.gpu_util_bound,
        _ => false,
    }
}

fn cpu_saturated(w: &Window, t: &Thresholds) -> bool {
    w.game_thread.is_some_and(|x| x >= t.thread_bound) || w.cpu_busy_frac.is_some_and(|x| x >= t.cpu_busy_bound)
}

/// Limiters (§2.3): a game held back on purpose, with neither side at its limit.
pub fn rule_limiter(w: &Window, t: &Thresholds) -> Option<Finding> {
    if w.frames < t.min_frames {
        return None;
    }
    let at = |hz: f64| (w.fps - hz).abs() <= hz * t.match_tolerance;
    let finding =
        |kind, confidence, evidence| Some(Finding { class: Class::Limiter, confidence, limiter: Some(kind), evidence });
    if w.focused == Some(false) {
        return finding(
            LimiterKind::Menu,
            Confidence::Medium,
            vec![Evidence::new("The game's window isn't in front: {} FPS", &[n0(w.fps)])],
        );
    }
    if gpu_saturated(w, t) || cpu_saturated(w, t) {
        return None; // a limit, not a limiter
    }
    let steady = w.ms_cv <= t.steady_cv;
    if steady && let Some(hz) = w.refresh_hz {
        if at(hz) || at(hz / 2.0) {
            let shown = if at(hz) { hz } else { hz / 2.0 };
            return finding(
                LimiterKind::VSync,
                Confidence::High,
                vec![Evidence::new(
                    "Frame time is very regular ({} % variation) and the frame rate ({} FPS) equals the {} Hz refresh rate: V-Sync, or VRR at its ceiling",
                    &[format!("{:.1}", w.ms_cv * 100.0), n0(w.fps), n0(shown)],
                )],
            );
        }
        // Reflex / low-latency modes cap a few FPS under the refresh rate.
        if at(hz - 3.0) {
            return finding(
                LimiterKind::FpsCap,
                Confidence::High,
                vec![Evidence::new(
                    "Steady at {} FPS, 3 under the {} Hz refresh rate: a low-latency cap (Reflex, Anti-Lag)",
                    &[n0(w.fps), n0(hz)],
                )],
            );
        }
    }
    if steady && let Some(cap) = t.caps.iter().copied().find(|c| at(*c)) {
        return finding(
            LimiterKind::FpsCap,
            Confidence::Medium,
            vec![Evidence::new(
                "Steady at {} FPS with room to spare: an FPS cap (in the game, the driver or RTSS)",
                &[n0(cap)],
            )],
        );
    }
    // Nothing at its limit and presented through the compositor: it may be in the way.
    if w.present_mode.to_ascii_lowercase().starts_with("composed") && gpu_headroom(w, t) == Some(true) {
        return finding(
            LimiterKind::Compositor,
            Confidence::Low,
            vec![Evidence::new(
                "Presented through the desktop compositor ({}) with the GPU not at its limit",
                std::slice::from_ref(&w.present_mode),
            )],
        );
    }
    None
}

/// The GPU worked the whole frame (§2.1).
pub fn rule_gpu(w: &Window, t: &Thresholds) -> Option<Finding> {
    if w.frames < t.min_frames {
        return None;
    }
    if let Some(b) = w.gpu_busy_frac {
        if b < t.gpu_busy_bound {
            return None;
        }
        let confidence = if b >= t.gpu_busy_certain { Confidence::High } else { Confidence::Medium };
        return Some(Finding {
            class: Class::Gpu,
            confidence,
            limiter: None,
            evidence: vec![Evidence::new("GPU Busy was {} % of the frame time", &[pct(b)])],
        });
    }
    // Without GPU Busy (Linux, macOS, no administrator): utilization, less sure.
    let u = w.gpu_util.filter(|u| *u >= t.gpu_util_bound)?;
    Some(Finding {
        class: Class::Gpu,
        confidence: Confidence::Low,
        limiter: None,
        evidence: vec![Evidence::new(
            "GPU utilization is {} % (GPU Busy isn't available, so this is an estimate)",
            &[n0(u)],
        )],
    })
}

/// One or two threads saturated while the GPU waits (§2.2).
pub fn rule_cpu(w: &Window, t: &Thresholds) -> Option<Finding> {
    if w.frames < t.min_frames || !cpu_saturated(w, t) {
        return None;
    }
    let headroom = gpu_headroom(w, t);
    if headroom == Some(false) {
        return None; // the GPU is the limit
    }
    let mut ev = Vec::new();
    if let Some(th) = w.game_thread {
        let core = w.busiest_core_id.map_or(String::new(), |c| format!(" (core {c})"));
        ev.push(Evidence::new("The game's busiest thread is at {} % of one core{}", &[n0(th), core]));
    }
    if let Some(c) = w.cpu_busy_frac {
        ev.push(Evidence::new("CPU Busy was {} % of the frame time", &[pct(c)]));
    }
    match (w.gpu_busy_frac, w.gpu_util) {
        (Some(b), _) => ev.push(Evidence::new("GPU Busy was only {} % of the frame time", &[pct(b)])),
        (None, Some(u)) => ev.push(Evidence::new("GPU utilization is only {} %", &[n0(u)])),
        _ => {}
    }
    let thread = w.game_thread.is_some_and(|x| x >= t.thread_bound);
    let confidence = match (headroom, thread, w.cpu_busy_frac.is_some_and(|x| x >= t.cpu_busy_bound)) {
        (Some(true), true, _) | (Some(true), _, true) if w.gpu_busy_frac.is_some() => Confidence::High,
        (Some(true), _, _) => Confidence::Medium,
        _ => Confidence::Low, // no GPU reading at all: can't say the GPU has room
    };
    Some(Finding { class: Class::Cpu, confidence, limiter: None, evidence: ev })
}

/// A throttling flag set (§2.4).
pub fn rule_throttling(w: &Window, _t: &Thresholds) -> Vec<Finding> {
    let mut out = Vec::new();
    let temp = |t: Option<f64>| t.map_or(String::new(), |t| format!(", {t:.0} °C"));
    if w.cpu_thermal {
        out.push(Finding {
            class: Class::Thermal,
            confidence: Confidence::Medium,
            limiter: None,
            evidence: vec![Evidence::new("The CPU's thermal throttle is on{}", &[temp(w.cpu_temp)])],
        });
    }
    if w.gpu_thermal {
        out.push(Finding {
            class: Class::Thermal,
            confidence: Confidence::Medium,
            limiter: None,
            evidence: vec![Evidence::new("The GPU's thermal throttle is on{}", &[temp(w.gpu_temp)])],
        });
    }
    if w.cpu_power {
        out.push(Finding {
            class: Class::Power,
            confidence: Confidence::Medium,
            limiter: None,
            evidence: vec![Evidence::new("The CPU is at its power or current limit", &[])],
        });
    }
    if w.gpu_power {
        out.push(Finding {
            class: Class::Power,
            confidence: Confidence::Medium,
            limiter: None,
            evidence: vec![Evidence::new("The GPU is at its power cap", &[])],
        });
    }
    out
}

/// Video memory full while the game stutters.
pub fn rule_vram(w: &Window, t: &Thresholds) -> Option<Finding> {
    let used = w.vram_used_pct.filter(|u| *u >= t.vram_full)?;
    (w.stutters > 0).then(|| Finding {
        class: Class::Vram,
        confidence: Confidence::Medium,
        limiter: None,
        evidence: vec![Evidence::new(
            "The game uses {} % of the video memory and {} frame(s) stuttered",
            &[n0(used), w.stutters.to_string()],
        )],
    })
}

/// RAM nearly full and the system reading from disk to resolve faults, with stutters.
pub fn rule_ram(w: &Window, t: &Thresholds) -> Option<Finding> {
    let used = w.ram_used_pct.filter(|u| *u >= t.ram_full)?;
    let faults = w.pages_in.filter(|p| *p >= t.paging)?;
    (w.stutters > 0).then(|| Finding {
        class: Class::Ram,
        confidence: Confidence::Medium,
        limiter: None,
        evidence: vec![Evidence::new(
            "RAM is {} % used and the system read {} pages/s from disk to resolve faults",
            &[n0(used), n0(faults)],
        )],
    })
}

/// The game reading a lot from disk while it stutters (streaming, shader compilation).
pub fn rule_disk(w: &Window, t: &Thresholds) -> Option<Finding> {
    let r = w.game_read_mbps.filter(|r| *r >= t.disk_read_mbps)?;
    (w.stutters > 0).then(|| Finding {
        class: Class::Disk,
        confidence: Confidence::Medium,
        limiter: None,
        evidence: vec![Evidence::new(
            "The game read {} MB/s from disk while {} frame(s) stuttered",
            &[n0(r), w.stutters.to_string()],
        )],
    })
}

/// Another program using the CPU.
pub fn rule_background(w: &Window, t: &Thresholds) -> Option<Finding> {
    let (name, cpu) = w.background.as_ref().filter(|(_, c)| *c >= t.background_cpu)?;
    Some(Finding {
        class: Class::Background,
        confidence: Confidence::Medium,
        limiter: None,
        evidence: vec![Evidence::new("{} is using {} % of the CPU", &[name.clone(), n0(*cpu)])],
    })
}

/// Run every rule in priority order; the first finding is the verdict.
pub fn classify(w: &Window, t: &Thresholds) -> Verdict {
    let mut all: Vec<Finding> = Vec::new();
    all.extend(rule_limiter(w, t));
    all.extend(rule_gpu(w, t));
    all.extend(rule_cpu(w, t));
    all.extend(rule_throttling(w, t));
    all.extend(rule_vram(w, t));
    all.extend(rule_ram(w, t));
    all.extend(rule_disk(w, t));
    all.extend(rule_background(w, t));
    if all.is_empty() {
        let evidence = if w.frames < t.min_frames {
            vec![Evidence::new("Too few frames in this second to tell", &[])]
        } else {
            vec![Evidence::new(
                "Neither the GPU nor the CPU is at its limit and nothing is holding the frame rate back",
                &[],
            )]
        };
        return Verdict {
            primary: Finding { class: Class::Inconclusive, confidence: Confidence::Low, limiter: None, evidence },
            also: Vec::new(),
        };
    }
    let primary = all.remove(0);
    Verdict { primary, also: all }
}

/// The current state from the last few verdicts: the commonest class wins
/// (ties go to the latest), so one odd second doesn't flip the headline.
pub fn smooth(recent: &[Verdict]) -> Option<&Verdict> {
    let tail = &recent[recent.len().saturating_sub(5)..];
    let mut best: Option<(Class, usize)> = None;
    for v in tail {
        let n = tail.iter().filter(|x| x.primary.class == v.primary.class).count();
        if best.is_none_or(|(_, b)| n >= b) {
            best = Some((v.primary.class, n));
        }
    }
    let class = best?.0;
    tail.iter().rev().find(|v| v.primary.class == class)
}

// ------------------------------------------------------------------ session

#[derive(Debug, Clone, PartialEq)]
pub struct Recommendation {
    pub key: &'static str,
    pub args: Vec<String>,
    /// What was measured to say it. A recommendation without one can't exist.
    pub because: Evidence,
}

impl Recommendation {
    pub fn render(&self) -> String {
        fill(self.key, &self.args)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Analysis {
    pub seconds: usize,
    /// Share of the seconds per class, biggest first.
    pub share: Vec<(Class, f64)>,
    /// One class per second.
    pub timeline: Vec<Class>,
    /// The strongest evidence: one line per leading class.
    pub evidence: Vec<Evidence>,
    pub recommendations: Vec<Recommendation>,
    /// Seconds judged without GPU Busy (lower confidence).
    pub estimated_seconds: usize,
    pub lowest_confidence: Option<Confidence>,
}

fn avg(v: impl Iterator<Item = f64>) -> Option<f64> {
    let (s, n) = v.fold((0.0, 0usize), |a, x| (a.0 + x, a.1 + 1));
    (n > 0).then(|| s / n as f64)
}

/// Classify every window and sum it up.
pub fn analyze(windows: &[Window], t: &Thresholds) -> Analysis {
    let verdicts: Vec<Verdict> = windows.iter().map(|w| classify(w, t)).collect();
    analyze_verdicts(windows, &verdicts, t)
}

pub fn analyze_verdicts(windows: &[Window], verdicts: &[Verdict], _t: &Thresholds) -> Analysis {
    let seconds = windows.len();
    if seconds == 0 {
        return Analysis::default();
    }
    let timeline: Vec<Class> = verdicts.iter().map(|v| v.primary.class).collect();
    let count = |c: Class| timeline.iter().filter(|x| **x == c).count();
    let mut share: Vec<(Class, f64)> =
        Class::ALL.iter().map(|c| (*c, count(*c) as f64 / seconds as f64)).filter(|s| s.1 > 0.0).collect();
    share.sort_by(|a, b| b.1.total_cmp(&a.1));
    let sel = |c: Class| windows.iter().zip(verdicts).filter(move |(_, v)| v.primary.class == c).map(|(w, _)| w);
    // Seconds a flag was set, whether or not it was the verdict.
    let flagged = |f: fn(&Window) -> bool| windows.iter().filter(|w| f(w)).count();
    let s_of = |n: usize| format!("{n} of {seconds}");

    let mut evidence = Vec::new();
    for (class, _) in share.iter().take(3) {
        let ev = match class {
            Class::Gpu => avg(sel(Class::Gpu).filter_map(|w| w.gpu_busy_frac))
                .map(|b| {
                    Evidence::new(
                        "GPU Busy averaged {} % of the frame time, in {} s",
                        &[pct(b), s_of(count(Class::Gpu))],
                    )
                })
                .or_else(|| {
                    avg(sel(Class::Gpu).filter_map(|w| w.gpu_util)).map(|u| {
                        Evidence::new(
                            "GPU utilization averaged {} % (an estimate), in {} s",
                            &[n0(u), s_of(count(Class::Gpu))],
                        )
                    })
                }),
            Class::Cpu => avg(sel(Class::Cpu).filter_map(|w| w.game_thread))
                .map(|th| {
                    Evidence::new(
                        "The game's busiest thread averaged {} % of one core, in {} s",
                        &[n0(th), s_of(count(Class::Cpu))],
                    )
                })
                .or_else(|| {
                    avg(sel(Class::Cpu).filter_map(|w| w.cpu_busy_frac)).map(|c| {
                        Evidence::new(
                            "CPU Busy averaged {} % of the frame time, in {} s",
                            &[pct(c), s_of(count(Class::Cpu))],
                        )
                    })
                }),
            Class::Limiter => avg(sel(Class::Limiter).map(|w| w.fps)).map(|fps| {
                Evidence::new("Frame rate held at about {} FPS, in {} s", &[n0(fps), s_of(count(Class::Limiter))])
            }),
            Class::Thermal => Some(Evidence::new(
                "A thermal throttle flag was set for {} s",
                &[s_of(flagged(|w| w.cpu_thermal || w.gpu_thermal))],
            )),
            Class::Power => Some(Evidence::new(
                "A power limit flag was set for {} s",
                &[s_of(flagged(|w| w.cpu_power || w.gpu_power))],
            )),
            Class::Vram => avg(sel(Class::Vram).filter_map(|w| w.vram_used_pct)).map(|u| {
                Evidence::new(
                    "Video memory was {} % used while frames stuttered, in {} s",
                    &[n0(u), s_of(count(Class::Vram))],
                )
            }),
            Class::Ram => {
                Some(Evidence::new("The system was paging while frames stuttered, in {} s", &[s_of(count(Class::Ram))]))
            }
            Class::Disk => avg(sel(Class::Disk).filter_map(|w| w.game_read_mbps)).map(|r| {
                Evidence::new(
                    "The game read {} MB/s from disk while frames stuttered, in {} s",
                    &[n0(r), s_of(count(Class::Disk))],
                )
            }),
            Class::Background => sel(Class::Background).filter_map(|w| w.background.as_ref()).next().map(|(n, _)| {
                Evidence::new("{} competed for the CPU in {} s", &[n.clone(), s_of(count(Class::Background))])
            }),
            Class::Inconclusive => {
                Some(Evidence::new("Nothing was at its limit in {} s", &[s_of(count(Class::Inconclusive))]))
            }
        };
        evidence.extend(ev);
    }

    let estimated_seconds = windows.iter().filter(|w| w.gpu_busy_frac.is_none() && w.frames >= 1).count();
    let lowest = verdicts.iter().map(|v| v.primary.confidence).min();
    let mut a = Analysis {
        seconds,
        share,
        timeline,
        evidence,
        recommendations: Vec::new(),
        estimated_seconds,
        lowest_confidence: lowest,
    };
    a.recommendations = recommend(windows, verdicts);
    a
}

/// Advice that always carries the measurement behind it. Nothing generic:
/// each branch needs numbers from this session to exist.
pub fn recommend(windows: &[Window], verdicts: &[Verdict]) -> Vec<Recommendation> {
    let n = windows.len();
    if n == 0 {
        return Vec::new();
    }
    let share = |c: Class| verdicts.iter().filter(|v| v.primary.class == c).count() as f64 / n as f64;
    let sel = |c: Class| windows.iter().zip(verdicts).filter(move |(_, v)| v.primary.class == c).map(|(w, _)| w);
    let secs = |c: Class| format!("{} of {n}", verdicts.iter().filter(|v| v.primary.class == c).count());
    let rec = |key, args: Vec<String>, because| Recommendation { key, args, because };
    let mut out = Vec::new();

    if share(Class::Gpu) >= 0.4 {
        let busy = avg(sel(Class::Gpu).filter_map(|w| w.gpu_busy_frac));
        let because = match busy {
            Some(b) => Evidence::new("GPU Busy averaged {} % of the frame time, in {} s", &[pct(b), secs(Class::Gpu)]),
            None => Evidence::new("The GPU was the limit in {} s", &[secs(Class::Gpu)]),
        };
        out.push(rec(
            "Lower the graphics settings or the resolution, or turn on upscaling (DLSS, FSR or XeSS).",
            vec![],
            because,
        ));
        if let Some(th) = avg(sel(Class::Gpu).filter_map(|w| w.game_thread)).filter(|th| *th < 70.0) {
            out.push(rec(
                "The CPU has room to spare: CPU-heavy settings (view distance, crowd density) cost no FPS here.",
                vec![],
                Evidence::new(
                    "The game's busiest thread averaged {} % of one core while the GPU was the limit",
                    &[n0(th)],
                ),
            ));
        }
    }
    if share(Class::Cpu) >= 0.4 {
        let because = match (
            avg(sel(Class::Cpu).filter_map(|w| w.gpu_busy_frac)),
            avg(sel(Class::Cpu).filter_map(|w| w.game_thread)),
        ) {
            (Some(b), Some(th)) => Evidence::new(
                "GPU Busy averaged {} % of the frame time while the game's busiest thread was at {} %",
                &[pct(b), n0(th)],
            ),
            (Some(b), None) => Evidence::new("GPU Busy averaged only {} % of the frame time", &[pct(b)]),
            (None, Some(th)) => Evidence::new(
                "The game's busiest thread averaged {} % of one core, in {} s",
                &[n0(th), secs(Class::Cpu)],
            ),
            (None, None) => Evidence::new("The CPU was the limit in {} s", &[secs(Class::Cpu)]),
        };
        out.push(rec(
            "You can raise the resolution or the graphics quality almost for free: the GPU has room to spare.",
            vec![],
            because.clone(),
        ));
        out.push(rec("Check the Build health card on the Overview: a memory profile left off, the power plan and the display's refresh rate all cost CPU-bound frames.", vec![], because));
    }
    if let Some((name, cpu, s)) = windows
        .iter()
        .filter_map(|w| w.background.as_ref())
        .fold(std::collections::BTreeMap::<&str, (f64, usize)>::new(), |mut m, (n, c)| {
            let e = m.entry(n.as_str()).or_default();
            e.0 += c;
            e.1 += 1;
            m
        })
        .into_iter()
        .filter(|(_, (_, s))| *s >= 5)
        .map(|(n, (c, s))| (n.to_string(), c / s as f64, s))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|(_, c, _)| *c >= 12.0)
    {
        out.push(rec(
            "Close {} while you play: it was using CPU that the game needed.",
            vec![name.clone()],
            Evidence::new("{} used {} % of the CPU in {} s", &[name, n0(cpu), format!("{s} of {n}")]),
        ));
    }
    let thermal = windows.iter().filter(|w| w.cpu_thermal || w.gpu_thermal).count();
    if thermal >= 5 {
        let which = if windows.iter().any(|w| w.cpu_thermal) { "CPU" } else { "GPU" };
        let temp =
            windows.iter().filter_map(|w| if which == "CPU" { w.cpu_temp } else { w.gpu_temp }).fold(0.0, f64::max);
        let temp = if temp > 0.0 { format!(" and reached {temp:.0} °C") } else { String::new() };
        out.push(rec(
            "Check the {} cooler, the thermal paste and the case airflow: the clock dropped to hold the temperature.",
            vec![which.into()],
            Evidence::new("The {} throttled for {} s{}", &[which.into(), format!("{thermal} of {n}"), temp]),
        ));
    }
    let power = windows.iter().filter(|w| w.cpu_power || w.gpu_power).count();
    if power >= 5 {
        out.push(rec(
            "A power limit held the clocks down: raise it in the BIOS or the GPU's tool if the cooling allows.",
            vec![],
            Evidence::new("A power limit flag was set for {} s", &[format!("{power} of {n}")]),
        ));
    }
    if share(Class::Vram) >= 0.1 {
        let used = avg(sel(Class::Vram).filter_map(|w| w.vram_used_pct)).unwrap_or(0.0);
        out.push(rec(
            "Lower the texture quality: the video memory is full and the game spills into system memory.",
            vec![],
            Evidence::new("Video memory was {} % used while frames stuttered, in {} s", &[n0(used), secs(Class::Vram)]),
        ));
    }
    if share(Class::Ram) >= 0.1 {
        out.push(rec(
            "Close other programs or add memory: Windows was reading from disk to keep up.",
            vec![],
            Evidence::new("The system was paging while frames stuttered, in {} s", &[secs(Class::Ram)]),
        ));
    }
    if share(Class::Disk) >= 0.1 {
        let r = avg(sel(Class::Disk).filter_map(|w| w.game_read_mbps)).unwrap_or(0.0);
        out.push(rec(
            "Install the game on an SSD. Stutters while the game loads assets or compiles shaders usually fade after the first minutes.",
            vec![],
            Evidence::new("The game read {} MB/s from disk while frames stuttered, in {} s", &[n0(r), secs(Class::Disk)]),
        ));
    }
    if let Some(w) = sel(Class::Limiter).next() {
        let kind = verdicts.iter().find(|v| v.primary.class == Class::Limiter).and_then(|v| v.primary.limiter);
        let fps = avg(sel(Class::Limiter).map(|w| w.fps)).unwrap_or(w.fps);
        let because = Evidence::new("Frame rate held at about {} FPS, in {} s", &[n0(fps), secs(Class::Limiter)]);
        match kind {
            Some(LimiterKind::VSync) => out.push(rec("Frames are locked to the refresh rate: turn V-Sync off to see what the PC can do (VRR keeps it smooth).", vec![], because)),
            Some(LimiterKind::FpsCap) => out.push(rec("An FPS cap is holding the game at about {} FPS: raise or remove it (in the game, the driver, RTSS) to see what the PC can do.", vec![n0(fps)], because)),
            Some(LimiterKind::Menu) => out.push(rec("The game wasn't in front for part of this session: measure while you play.", vec![], because)),
            _ => {}
        }
    }
    out
}

/// Every translatable template in this module, for the GUI's translation test.
pub fn keys() -> Vec<&'static str> {
    let t = Thresholds::default();
    let mut v: Vec<&'static str> = Vec::new();
    let mut w = Window { frames: 30, fps: 60.0, ..Default::default() };
    // Run a spread of windows so every rule emits its text.
    let cases: Vec<Window> = vec![
        Window { focused: Some(false), ..w.clone() },
        Window { ms_cv: 0.01, refresh_hz: Some(60.0), ..w.clone() },
        Window { ms_cv: 0.01, fps: 141.0, refresh_hz: Some(144.0), ..w.clone() },
        Window { ms_cv: 0.01, fps: 120.0, ..w.clone() },
        Window { present_mode: "Composed: Flip".into(), gpu_busy_frac: Some(0.3), ..w.clone() },
        Window { gpu_busy_frac: Some(0.97), ..w.clone() },
        Window { gpu_util: Some(99.0), ..w.clone() },
        Window {
            game_thread: Some(99.0),
            cpu_busy_frac: Some(0.95),
            gpu_busy_frac: Some(0.4),
            busiest_core_id: Some(3),
            ..w.clone()
        },
        Window { game_thread: Some(99.0), gpu_util: Some(40.0), ..w.clone() },
        Window { cpu_thermal: true, gpu_thermal: true, cpu_power: true, gpu_power: true, ..w.clone() },
        Window {
            stutters: 2,
            vram_used_pct: Some(99.0),
            ram_used_pct: Some(95.0),
            pages_in: Some(900.0),
            game_read_mbps: Some(500.0),
            ..w.clone()
        },
        Window { background: Some(("x.exe".into(), 30.0)), ..w.clone() },
        Window { frames: 1, ..Default::default() },
        Window { fps: 77.0, ms_cv: 0.2, ..w.clone() },
    ];
    for c in &cases {
        let verdict = classify(c, &t);
        for f in std::iter::once(&verdict.primary).chain(&verdict.also) {
            v.extend(f.evidence.iter().map(|e| e.key));
        }
    }
    w.gpu_busy_frac = Some(0.97);
    // Session texts: analyze and recommend a spread of sessions.
    let mut sessions: Vec<Vec<Window>> = Vec::new();
    let mk = |f: &dyn Fn(&mut Window)| {
        (0..20)
            .map(|_| {
                let mut x = w.clone();
                f(&mut x);
                x
            })
            .collect::<Vec<_>>()
    };
    sessions.push(mk(&|_| {}));
    sessions.push(mk(&|x| {
        x.game_thread = Some(40.0);
    }));
    sessions.push(mk(&|x| {
        x.gpu_busy_frac = Some(0.3);
        x.game_thread = Some(99.0);
        x.background = Some(("a.exe".into(), 25.0));
    }));
    sessions.push(mk(&|x| {
        x.gpu_busy_frac = None;
        x.gpu_util = Some(99.0);
    }));
    sessions.push(mk(&|x| {
        x.gpu_busy_frac = Some(0.5);
        x.cpu_thermal = true;
        x.cpu_power = true;
        x.cpu_temp = Some(95.0);
    }));
    sessions.push(mk(&|x| {
        x.gpu_busy_frac = Some(0.5);
        x.stutters = 3;
        x.vram_used_pct = Some(99.0);
    }));
    sessions.push(mk(&|x| {
        x.gpu_busy_frac = Some(0.5);
        x.stutters = 3;
        x.ram_used_pct = Some(99.0);
        x.pages_in = Some(900.0);
    }));
    sessions.push(mk(&|x| {
        x.gpu_busy_frac = Some(0.5);
        x.stutters = 3;
        x.game_read_mbps = Some(500.0);
    }));
    sessions.push(mk(&|x| {
        x.gpu_busy_frac = Some(0.5);
        x.ms_cv = 0.01;
        x.fps = 60.0;
        x.refresh_hz = Some(60.0);
    }));
    sessions.push(mk(&|x| {
        x.gpu_busy_frac = Some(0.5);
        x.ms_cv = 0.01;
        x.fps = 120.0;
    }));
    sessions.push(mk(&|x| {
        x.gpu_busy_frac = Some(0.5);
        x.focused = Some(false);
    }));
    for s in &sessions {
        let a = analyze(s, &t);
        v.extend(a.evidence.iter().map(|e| e.key));
        for r in &a.recommendations {
            v.push(r.key);
            v.push(r.because.key);
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

// ------------------------------------------------------------------ recorded sessions

/// Windows of a recorded session: one per second of `frames`, with the
/// hardware log's readings of that second and the throttling spans.
pub fn windows_from_session(frames: &[Frame], hw: &HwLog, refresh_hz: Option<f64>, throttling: &[Band]) -> Vec<Window> {
    let secs = frames.last().map_or(0, |f| f.t_s.floor() as usize + 1);
    let mut by_sec: Vec<Vec<Frame>> = vec![Vec::new(); secs];
    for f in frames {
        if let Some(b) = by_sec.get_mut(f.t_s.max(0.0).floor() as usize) {
            b.push(f.clone());
        }
    }
    let at = |id: &str, s: usize| -> Option<f64> {
        let v: Vec<f64> = hw.series(id).into_iter().filter(|(t, _)| t.floor() as usize == s).map(|p| p.1).collect();
        (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
    };
    by_sec
        .iter()
        .enumerate()
        .filter(|(_, b)| !b.is_empty())
        .map(|(s, b)| {
            let mut w = Window::from_frames(b);
            w.refresh_hz = refresh_hz;
            w.gpu_util = at("gpu0/load/core", s);
            w.cpu_total = at("cpu/load/total", s);
            w.busiest_core = at("cpu/load/max", s);
            w.ram_used_pct = at("mem/load/used", s);
            w.pages_in = at("mem/rate/pages_in", s);
            w.cpu_temp = at("cpu/temp/package", s);
            w.gpu_temp = at("gpu0/temp/core", s);
            let on = |kinds: &[&str]| {
                throttling
                    .iter()
                    .any(|t| t.from_s <= s as f64 + 1.0 && t.to_s >= s as f64 && kinds.contains(&t.kind.as_str()))
            };
            w.cpu_thermal = on(&["cpu-thermal"]);
            w.cpu_power = on(&["cpu-power"]);
            w.gpu_thermal = on(&["gpu-thermal"]);
            w.gpu_power = on(&["gpu-power"]);
            w
        })
        .collect()
}

/// The report section for a session, as English HTML (the offline report).
pub fn html_section(a: &Analysis) -> String {
    if a.seconds == 0 {
        return String::new();
    }
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let mut s = String::from("<h2>What limited the frame rate</h2><table>");
    for (c, f) in &a.share {
        s += &format!("<tr><th>{}</th><td>{:.0} %</td></tr>", c.label(), f * 100.0);
    }
    s += "</table><ul>";
    for e in &a.evidence {
        s += &format!("<li>{}</li>", esc(&e.render()));
    }
    s += "</ul>";
    if !a.recommendations.is_empty() {
        s += "<h3>Recommendations</h3><ul>";
        for r in &a.recommendations {
            s += &format!("<li>{} <small>({})</small></li>", esc(&r.render()), esc(&r.because.render()));
        }
        s += "</ul>";
    }
    if a.estimated_seconds > 0 {
        s += &format!(
            "<p><small>{} s were judged without GPU Busy, so those are estimates.</small></p>",
            a.estimated_seconds
        );
    }
    s
}

/// Which throttle flags a GPU's NVML reason names set: (thermal, power).
pub fn gpu_throttle_flags(reasons: &[&str]) -> (bool, bool) {
    let has = |needle: &str| reasons.iter().any(|r| r.to_ascii_lowercase().contains(needle));
    (has("thermal"), has("power"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Window {
        Window { frames: 120, fps: 120.0, median_ms: 8.3, ms_cv: 0.2, ..Default::default() }
    }

    fn class(w: &Window) -> Class {
        classify(w, &Thresholds::default()).primary.class
    }

    #[test]
    fn gpu_busy_decides_gpu_bound_with_graded_confidence() {
        let w = Window { gpu_busy_frac: Some(0.97), ..base() };
        let v = classify(&w, &Thresholds::default());
        assert_eq!((v.primary.class, v.primary.confidence), (Class::Gpu, Confidence::High));
        assert!(v.primary.evidence[0].render().contains("97 %"));
        let v = classify(&Window { gpu_busy_frac: Some(0.93), ..base() }, &Thresholds::default());
        assert_eq!(v.primary.confidence, Confidence::Medium);
        assert_eq!(class(&Window { gpu_busy_frac: Some(0.7), ..base() }), Class::Inconclusive);
    }

    #[test]
    fn cpu_bound_needs_a_saturated_thread_and_gpu_headroom() {
        let cpu = Window { gpu_busy_frac: Some(0.41), game_thread: Some(98.0), busiest_core_id: Some(4), ..base() };
        let v = classify(&cpu, &Thresholds::default());
        assert_eq!((v.primary.class, v.primary.confidence), (Class::Cpu, Confidence::High));
        let text: Vec<String> = v.primary.evidence.iter().map(Evidence::render).collect();
        assert!(text[0].contains("98 %") && text[0].contains("core 4"), "{text:?}");
        assert!(text.iter().any(|t| t.contains("41 %")));
        // The same hot thread with a saturated GPU is the GPU's limit, not the CPU's.
        assert_eq!(class(&Window { gpu_busy_frac: Some(0.97), game_thread: Some(98.0), ..base() }), Class::Gpu);
        // PresentMon's CPU Busy alone (no thread data) still says so, less surely.
        let v =
            classify(&Window { gpu_busy_frac: Some(0.4), cpu_busy_frac: Some(0.95), ..base() }, &Thresholds::default());
        assert_eq!(v.primary.class, Class::Cpu);
        // A low CPU total is not a reason to rule it out: the thread decides.
        assert_eq!(
            class(&Window { gpu_busy_frac: Some(0.4), game_thread: Some(95.0), cpu_total: Some(7.0), ..base() }),
            Class::Cpu
        );
    }

    #[test]
    fn limiters_are_told_from_limits() {
        let steady = Window { ms_cv: 0.01, ..base() };
        let v = classify(
            &Window { fps: 60.0, refresh_hz: Some(60.0), gpu_busy_frac: Some(0.4), ..steady.clone() },
            &Thresholds::default(),
        );
        assert_eq!(
            (v.primary.class, v.primary.limiter, v.primary.confidence),
            (Class::Limiter, Some(LimiterKind::VSync), Confidence::High)
        );
        // 30 FPS on a 60 Hz display: V-Sync's second step.
        assert_eq!(
            classify(&Window { fps: 30.0, refresh_hz: Some(60.0), ..steady.clone() }, &Thresholds::default())
                .primary
                .limiter,
            Some(LimiterKind::VSync)
        );
        // Reflex-style cap 3 under the refresh rate.
        let v = classify(&Window { fps: 141.0, refresh_hz: Some(144.0), ..steady.clone() }, &Thresholds::default());
        assert_eq!(v.primary.limiter, Some(LimiterKind::FpsCap));
        // A round cap with no refresh rate known.
        assert_eq!(
            classify(&Window { fps: 120.2, ..steady.clone() }, &Thresholds::default()).primary.limiter,
            Some(LimiterKind::FpsCap)
        );
        // The same steady 60 with the GPU pinned is the GPU's limit.
        assert_eq!(
            class(&Window { fps: 60.0, refresh_hz: Some(60.0), gpu_busy_frac: Some(0.98), ..steady.clone() }),
            Class::Gpu
        );
        // Not in front: a menu or an alt-tab.
        assert_eq!(
            classify(&Window { focused: Some(false), ..base() }, &Thresholds::default()).primary.limiter,
            Some(LimiterKind::Menu)
        );
        // An irregular 77 FPS is not a cap.
        assert_eq!(class(&Window { fps: 77.0, ms_cv: 0.2, ..base() }), Class::Inconclusive);
        // Composed with nothing at its limit: low-confidence compositor note.
        let v = classify(
            &Window { gpu_busy_frac: Some(0.3), present_mode: "Composed: Flip".into(), ..base() },
            &Thresholds::default(),
        );
        assert_eq!((v.primary.limiter, v.primary.confidence), (Some(LimiterKind::Compositor), Confidence::Low));
    }

    #[test]
    fn without_gpu_busy_the_fallback_is_less_sure() {
        let v = classify(&Window { gpu_util: Some(99.0), ..base() }, &Thresholds::default());
        assert_eq!((v.primary.class, v.primary.confidence), (Class::Gpu, Confidence::Low));
        assert!(v.primary.evidence[0].render().contains("estimate"));
        let v = classify(&Window { gpu_util: Some(40.0), game_thread: Some(97.0), ..base() }, &Thresholds::default());
        assert_eq!((v.primary.class, v.primary.confidence), (Class::Cpu, Confidence::Medium));
        // No GPU reading at all: a hot thread is only a guess.
        let v = classify(&Window { game_thread: Some(97.0), ..base() }, &Thresholds::default());
        assert_eq!((v.primary.class, v.primary.confidence), (Class::Cpu, Confidence::Low));
        // A hot thread with a busy GPU is not CPU-bound.
        assert_eq!(class(&Window { gpu_util: Some(92.0), game_thread: Some(97.0), ..base() }), Class::Inconclusive);
    }

    #[test]
    fn memory_disk_and_background_need_stutters_and_numbers() {
        let s = Window { stutters: 3, ..base() };
        assert_eq!(class(&Window { vram_used_pct: Some(98.0), ..s.clone() }), Class::Vram);
        assert_eq!(
            class(&Window { vram_used_pct: Some(98.0), ..base() }),
            Class::Inconclusive,
            "full VRAM without stutters is fine"
        );
        assert_eq!(class(&Window { ram_used_pct: Some(95.0), pages_in: Some(800.0), ..s.clone() }), Class::Ram);
        assert_eq!(
            class(&Window { ram_used_pct: Some(95.0), pages_in: Some(10.0), ..s.clone() }),
            Class::Inconclusive,
            "full RAM without paging"
        );
        assert_eq!(class(&Window { game_read_mbps: Some(400.0), ..s }), Class::Disk);
        let v =
            classify(&Window { background: Some(("antivirus.exe".into(), 30.0)), ..base() }, &Thresholds::default());
        assert_eq!(v.primary.class, Class::Background);
        assert!(v.primary.evidence[0].render().contains("antivirus.exe"));
    }

    #[test]
    fn throttling_is_a_note_under_a_limit_and_the_verdict_alone() {
        let v = classify(
            &Window { gpu_busy_frac: Some(0.97), cpu_thermal: true, cpu_temp: Some(96.0), ..base() },
            &Thresholds::default(),
        );
        assert_eq!(v.primary.class, Class::Gpu);
        assert_eq!(v.also[0].class, Class::Thermal);
        assert!(v.also[0].evidence[0].render().contains("96 °C"));
        assert_eq!(class(&Window { cpu_thermal: true, ..base() }), Class::Thermal);
        assert_eq!(class(&Window { gpu_power: true, ..base() }), Class::Power);
        assert_eq!(gpu_throttle_flags(&["Power", "HW thermal"]), (true, true));
        assert_eq!(gpu_throttle_flags(&[]), (false, false));
    }

    #[test]
    fn too_few_frames_say_so() {
        let v = classify(&Window { frames: 3, gpu_busy_frac: Some(0.99), ..base() }, &Thresholds::default());
        assert_eq!(v.primary.class, Class::Inconclusive);
        assert!(v.primary.evidence[0].render().contains("Too few"));
    }

    #[test]
    fn the_headline_does_not_flip_on_one_odd_second() {
        let gpu = classify(&Window { gpu_busy_frac: Some(0.97), ..base() }, &Thresholds::default());
        let odd = classify(&base(), &Thresholds::default());
        let recent = vec![gpu.clone(), gpu.clone(), odd.clone(), gpu.clone(), gpu.clone()];
        assert_eq!(smooth(&recent).unwrap().primary.class, Class::Gpu);
        assert!(smooth(&[]).is_none());
        assert_eq!(smooth(&[gpu, odd]).unwrap().primary.class, Class::Inconclusive, "a tie goes to the latest");
    }

    fn session(n: usize, f: impl Fn(usize) -> Window) -> Vec<Window> {
        (0..n).map(f).collect()
    }

    #[test]
    fn a_session_sums_up_time_per_class() {
        let w = session(60, |i| {
            if i < 45 {
                Window { gpu_busy_frac: Some(0.97), game_thread: Some(55.0), ..base() }
            } else {
                Window { gpu_busy_frac: Some(0.4), game_thread: Some(98.0), ..base() }
            }
        });
        let a = analyze(&w, &Thresholds::default());
        assert_eq!(a.seconds, 60);
        assert_eq!((a.share[0].0, (a.share[0].1 * 100.0).round() as i32), (Class::Gpu, 75));
        assert_eq!(a.share[1].0, Class::Cpu);
        assert_eq!(&a.timeline[..2], &[Class::Gpu, Class::Gpu]);
        assert_eq!(a.timeline[59], Class::Cpu);
        let ev: Vec<String> = a.evidence.iter().map(Evidence::render).collect();
        assert!(ev[0].contains("97 %") && ev[0].contains("45 of 60"), "{ev:?}");
        assert_eq!(a.estimated_seconds, 0);
        assert_eq!(a.lowest_confidence, Some(Confidence::High));
    }

    #[test]
    fn every_recommendation_carries_its_evidence() {
        let t = Thresholds::default();
        // The generator in keys() runs every branch; here the contents are checked.
        let gpu =
            analyze(&session(30, |_| Window { gpu_busy_frac: Some(0.97), game_thread: Some(40.0), ..base() }), &t);
        let r: Vec<String> = gpu.recommendations.iter().map(Recommendation::render).collect();
        assert!(r.iter().any(|x| x.contains("upscaling")) && r.iter().any(|x| x.contains("CPU has room")), "{r:?}");
        assert!(
            gpu.recommendations.iter().all(|r| !r.because.render().is_empty() && !r.because.render().contains("{}"))
        );
        let cpu = analyze(
            &session(30, |_| Window {
                gpu_busy_frac: Some(0.4),
                game_thread: Some(98.0),
                background: Some(("chrome.exe".into(), 20.0)),
                ..base()
            }),
            &t,
        );
        let r: Vec<String> = cpu.recommendations.iter().map(Recommendation::render).collect();
        assert!(r.iter().any(|x| x.contains("raise the resolution")), "{r:?}");
        assert!(r.iter().any(|x| x.contains("Close chrome.exe")), "named from the measured process: {r:?}");
        let hot = analyze(
            &session(30, |_| Window { gpu_busy_frac: Some(0.5), cpu_thermal: true, cpu_temp: Some(95.0), ..base() }),
            &t,
        );
        assert!(
            hot.recommendations
                .iter()
                .any(|r| r.render().contains("CPU cooler") && r.because.render().contains("95 °C"))
        );
        // Nothing wrong: nothing to recommend (no generic tips).
        let fine = analyze(&session(30, |_| base()), &t);
        assert!(fine.recommendations.is_empty(), "{:?}", fine.recommendations);
        // A different story gets different advice, never the generic list.
        let cap = analyze(&session(30, |_| Window { fps: 120.0, ms_cv: 0.01, gpu_busy_frac: Some(0.5), ..base() }), &t);
        assert!(cap.recommendations[0].render().contains("120 FPS"));
    }

    #[test]
    fn estimates_are_counted_and_shown() {
        let a = analyze(&session(10, |_| Window { gpu_util: Some(99.0), ..base() }), &Thresholds::default());
        assert_eq!((a.estimated_seconds, a.lowest_confidence), (10, Some(Confidence::Low)));
        assert!(html_section(&a).contains("without GPU Busy"));
    }

    #[test]
    fn keys_cover_every_text_and_fill_in_order() {
        let k = keys();
        assert!(k.len() > 30, "{} keys", k.len());
        assert!(k.iter().all(|k| !k.is_empty()));
        assert_eq!(fill("{} of {} s", &["3".into(), "10".into()]), "3 of 10 s");
        assert_eq!(fill("no holes", &[]), "no holes");
        for c in Class::ALL {
            assert!(!c.label().is_empty() && !c.headline().is_empty());
        }
    }

    /// A real capture: PresentMon 2.6.0 on a Ryzen 9 7900X + RTX 4060 while
    /// Deadlock ran (DX11, borderless, no cap). It starts in a loading
    /// screen (41 FPS; CPU Busy 99.5 % of the frame, GPU Busy 25 %), passes a
    /// transition (GPU Busy 81 %, uneven frames) and ends in play at ~195 FPS
    /// with GPU Busy at 100 % of the frame. The detector must say exactly
    /// that: the CPU's limit while loading, the GPU's once playing, and never
    /// a limiter.
    #[test]
    fn a_real_presentmon_capture_tells_cpu_bound_loading_from_gpu_bound_play() {
        let csv = include_str!("../tests/fixtures/presentmon-2.6-deadlock.csv");
        let frames = frames::parse_presentmon(csv).unwrap();
        assert!(frames.len() >= 300 && frames.iter().all(|f| f.app == "deadlock.exe"));
        let w = windows_from_session(&frames, &HwLog::default(), Some(165.0), &[]);
        assert_eq!(w.len(), 3, "{:?}", w.iter().map(|x| (x.frames, x.fps as i32)).collect::<Vec<_>>());
        let t = Thresholds::default();
        let v: Vec<Verdict> = w.iter().map(|x| classify(x, &t)).collect();
        assert_eq!(
            (v[0].primary.class, v[0].primary.confidence),
            (Class::Cpu, Confidence::High),
            "loading: {:?}",
            w[0]
        );
        assert_eq!((v[2].primary.class, v[2].primary.confidence), (Class::Gpu, Confidence::High), "{:?}", w[2]);
        assert!(v.iter().all(|x| x.primary.class != Class::Limiter));
        assert!(v[2].primary.evidence[0].render().contains("GPU Busy was"));
        let a = analyze_verdicts(&w, &v, &t);
        assert_eq!(a.timeline.last(), Some(&Class::Gpu));
    }

    /// Windows from a recorded session: frames grouped per second, the
    /// hardware log's readings joined, throttling spans applied.
    #[test]
    fn recorded_session_windows() {
        let frames: Vec<Frame> = (0..600)
            .map(|i| Frame {
                t_s: i as f64 / 100.0,
                ms: 10.0,
                gpu_busy_ms: Some(9.8),
                cpu_busy_ms: Some(4.0),
                present_mode: "Hardware: Independent Flip".into(),
                ..Default::default()
            })
            .collect();
        let hw = crate::session::parse_hw_csv(
            "time,gpu0/load/core,cpu/temp/package\n# units,%,°C\n2026-11-05T20:00:00.000Z,99,80\n2026-11-05T20:00:03.000Z,98,96\n",
        );
        let band = Band { from_s: 3.0, to_s: 6.0, label: "CPU".into(), kind: "cpu-thermal".into() };
        let w = windows_from_session(&frames, &hw, Some(144.0), &[band]);
        assert_eq!(w.len(), 6);
        assert!((w[0].fps - 100.0).abs() < 0.5 && (w[0].gpu_busy_frac.unwrap() - 0.98).abs() < 1e-9);
        assert_eq!(w[0].gpu_util, Some(99.0));
        assert_eq!(w[3].cpu_temp, Some(96.0));
        assert!(!w[1].cpu_thermal && w[4].cpu_thermal);
        assert!(w.iter().all(|x| x.refresh_hz == Some(144.0)));
        let a = analyze(&w, &Thresholds::default());
        assert_eq!(a.share[0].0, Class::Gpu);
    }
}

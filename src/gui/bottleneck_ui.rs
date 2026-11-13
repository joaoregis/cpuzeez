//! ZeeZ Live › Bottleneck (S2c, docs/v1/bottleneck-detector.md §5): what
//! limits the frame rate right now, why, and over a whole session.
//!
//! The rules and the thresholds are `cpuzeez_core::bottleneck`. This file
//! builds one [`Window`] of data per second from the frame hub, the telemetry
//! engine and the process sampler, keeps the latest verdicts, and draws the
//! "Now" card, the live readings (each with the threshold where its rule
//! fires), the session analysis and the overlay pill.

use super::i18n::{tr, trf};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::Ctx;
use crate::procs::Sampler;
use cpuzeez_core::bottleneck::{self as bn, Analysis, Class, Confidence, Evidence, Thresholds, Verdict, Window};
use cpuzeez_core::frames;
use eframe::egui::{self, Color32, RichText, Sense, Ui, pos2, vec2};
use std::collections::VecDeque;
use std::fmt::Display;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Seconds of windows kept for the smoothed headline and the meters.
const KEEP: usize = 600;

/// The display refresh rate when every display runs the same one (else the
/// detector doesn't guess which monitor the game is on). Read in the
/// background.
pub fn display_hz(ctx: &Arc<Ctx>) -> Option<f64> {
    static HZ: super::cache::Cached<Option<f64>> = super::cache::Cached::new(Duration::from_secs(10));
    HZ.get(ctx, |_| {
        let hz: Vec<u32> = cpuzeez_platform::display::modes().iter().map(|m| m.hz).filter(|h| *h > 1).collect();
        let first = *hz.first()?;
        hz.iter().all(|h| *h == first).then_some(first as f64)
    })
    .and_then(|a| *a)
}

/// No display is queried for a recorded capture: its refresh rate isn't known.
fn display_hz_hint(_: &[frames::Frame]) -> Option<f64> {
    None
}

/// The class's colour. Never the only cue: a label always goes with it.
pub fn class_color(c: Class) -> Color32 {
    match c {
        Class::Gpu => Color32::from_rgb(139, 92, 246),
        Class::Cpu => Color32::from_rgb(59, 130, 246),
        Class::Limiter => Color32::from_rgb(245, 158, 11),
        Class::Thermal => Color32::from_rgb(239, 68, 68),
        Class::Power => Color32::from_rgb(249, 115, 22),
        Class::Vram => Color32::from_rgb(236, 72, 153),
        Class::Ram => Color32::from_rgb(20, 184, 166),
        Class::Disk => Color32::from_rgb(132, 204, 22),
        Class::Background => Color32::from_rgb(148, 163, 184),
        Class::Inconclusive => Color32::from_rgb(100, 100, 110),
    }
}

/// A plain-language sentence for each verdict.
fn explain(c: Class) -> &'static str {
    match c {
        Class::Gpu => tr(
            "The graphics card is working the whole time. Lowering the resolution or graphics settings raises the frame rate; a faster CPU would not.",
        ),
        Class::Cpu => tr(
            "One or two CPU threads are saturated while the graphics card waits for them. Raising the resolution or graphics settings costs almost nothing; a faster graphics card would not help.",
        ),
        Class::Limiter => tr(
            "The frame rate is being held at a level on purpose (V-Sync, a frame cap, or the game isn't in front), not by the hardware.",
        ),
        Class::Thermal => tr("A chip is hot enough that it slows itself down to cool off."),
        Class::Power => tr("A power or current limit is holding the clocks down."),
        Class::Vram => tr("The video memory is full, so the game has to move data around while it runs."),
        Class::Ram => tr("The system is short of memory and is reading from disk to keep up."),
        Class::Disk => tr("The game is waiting on the disk while it loads assets or compiles shaders."),
        Class::Background => tr("Another program is using the CPU the game needs."),
        Class::Inconclusive => {
            tr("Neither the graphics card nor the CPU is at its limit, and nothing is holding the frame rate back.")
        }
    }
}

fn headline(v: &Verdict, w: &Window) -> String {
    if v.primary.class == Class::Cpu
        && let Some(core) = w.busiest_core_id
    {
        return trf("CPU at its limit (core {})", &[&core]);
    }
    if let Some(k) = v.primary.limiter {
        use bn::LimiterKind::*;
        return tr(match k {
            VSync => "Frame rate locked to the refresh rate",
            FpsCap => "Frame rate capped",
            Menu => "The game isn't in front",
            Compositor => "The desktop compositor may be in the way",
        })
        .to_string();
    }
    tr(v.primary.class.headline()).to_string()
}

/// An evidence sentence, translated.
pub fn evidence_text(e: &Evidence) -> String {
    let args: Vec<&dyn Display> = e.args.iter().map(|a| a as &dyn Display).collect();
    trf(e.key, &args)
}

/// The smoothed state shown on the card and the pill.
#[derive(Debug, Clone)]
pub struct Now {
    pub verdict: Verdict,
    pub window: Window,
    pub app: String,
    pub fps: f64,
    pub low1: f64,
}

struct Run {
    start: Instant,
    length: Option<Duration>,
    windows: Vec<Window>,
    verdicts: Vec<Verdict>,
}

pub struct Detector {
    windows: VecDeque<Window>,
    verdicts: VecDeque<Verdict>,
    last: Option<Instant>,
    pub now: Option<Now>,
    /// The newest second's readings, even when no game is presenting.
    pub latest: Window,
    run: Option<Run>,
    pub result: Option<Analysis>,
    /// Analysis length in seconds (None: until stopped).
    pub length: Option<u64>,
    pub thresholds: Thresholds,
    tjmax: f64,
}

impl Default for Detector {
    fn default() -> Self {
        Detector {
            windows: VecDeque::new(),
            verdicts: VecDeque::new(),
            last: None,
            now: None,
            latest: Window::default(),
            run: None,
            result: None,
            length: Some(60),
            thresholds: Thresholds::default(),
            tjmax: 95.0,
        }
    }
}

impl Detector {
    /// Show a verdict (tests).
    pub fn preview(now: Option<Now>, result: Option<Analysis>) -> Detector {
        Detector {
            latest: now.as_ref().map(|n| n.window.clone()).unwrap_or_default(),
            now,
            result,
            ..Default::default()
        }
    }

    /// A recorded capture shown as if it were live: its last second as the
    /// verdict and the whole capture as the session result (screenshots).
    pub fn from_capture(csv: &str, app: &str) -> Detector {
        let frames = frames::parse_presentmon(csv).unwrap_or_default();
        let t = Thresholds::default();
        let windows = bn::windows_from_session(&frames, &Default::default(), display_hz_hint(&frames), &[]);
        let verdicts: Vec<Verdict> = windows.iter().map(|w| bn::classify(w, &t)).collect();
        let result = Some(bn::analyze_verdicts(&windows, &verdicts, &t));
        let now = windows.last().zip(verdicts.last()).map(|(w, v)| {
            let s = frames::summarize(&frames, Default::default());
            Now { verdict: v.clone(), window: w.clone(), app: app.to_string(), fps: w.fps, low1: s.low1_fps }
        });
        let mut d = Detector::preview(now, result);
        d.windows = windows.into();
        d.verdicts = verdicts.into();
        d
    }

    /// Every frame: build the second's window when due. `active`: the tab is
    /// open, an analysis runs or the overlay shows the pill.
    pub fn tick(&mut self, mon: &mut super::sensors::Monitor, sampler: &Sampler, active: bool) {
        if !active && self.run.is_none() {
            return;
        }
        sampler.want();
        let tel = mon.tel();
        tel.want("bottleneck", &["cpu/load", "cpu/temp", "gpu", "mem/load", "mem/rate", "frames"]);
        if self.last.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) {
            return;
        }
        self.last = Some(Instant::now());
        self.tjmax = mon.tjmax();

        let live = crate::live_frames::frame_age().is_some_and(|a| a < Duration::from_secs(2));
        let frames = if live { crate::live_frames::recent(1.0) } else { Vec::new() };
        let mut w = Window::from_frames(&frames);
        w.refresh_hz = display_hz(mon.ctx());
        let series = tel.series_with(|_| false);
        let cur = |id: &str| series.iter().find(|s| s.id == id).map(|s| s.current);
        w.gpu_util = cur("gpu0/load/core");
        w.cpu_total = cur("cpu/load/total");
        w.busiest_core = cur("cpu/load/max");
        w.busiest_core_id = series
            .iter()
            .filter_map(|s| Some((s.id.strip_prefix("cpu/load/cpu")?.parse::<usize>().ok()?, s.current)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|c| c.0);
        w.ram_used_pct = cur("mem/load/used");
        w.pages_in = cur("mem/rate/pages_in");
        w.cpu_temp = super::sensors::cpu_temp(&series).map(|s| s.current);
        w.gpu_temp = super::sensors::gpu_temp(&series).map(|s| s.current);
        for th in tel.throttling() {
            use crate::telemetry::ThrottleKind::*;
            match th.kind {
                Thermal => w.cpu_thermal = true,
                Power | Current => w.cpu_power = true,
                Gpu => w.gpu_thermal = true,
            }
        }
        if let Some(reasons) = tel.gpu_live().first().and_then(|l| l.throttle.clone()) {
            let (thermal, power) = bn::gpu_throttle_flags(&reasons);
            w.gpu_thermal |= thermal;
            w.gpu_power |= power;
        }
        let target = crate::live_frames::target();
        let snap = sampler.snapshot();
        let me = std::process::id();
        if let Some((pid, _)) = &target {
            if let Some(r) = snap.rows.iter().find(|r| r.pid == *pid) {
                w.game_thread = r.top_thread.map(|t| t.1);
                w.game_read_mbps = Some(r.disk_read_bps / 1_048_576.0);
                let total_mb = crate::gpu::adapters(mon.ctx()).first().and_then(|a| a.mem_size_mb).filter(|m| *m > 0);
                if let (Some(total), true) = (total_mb, r.vram > 0) {
                    w.vram_used_pct = Some(r.vram as f64 / 1_048_576.0 / total as f64 * 100.0);
                }
            }
            w.focused = cpuzeez_platform::desktop::foreground_pid().map(|f| f == *pid);
            #[cfg(feature = "soak")]
            if crate::soak::enabled() {
                w.focused = Some(true); // the synthetic game has no window
            }
        }
        let target_pid = target.as_ref().map(|t| t.0);
        w.background = snap
            .rows
            .iter()
            .filter(|r| Some(r.pid) != target_pid && r.pid != me && r.pid > 4)
            .filter(|r| !["cpuzeez.exe", "PresentMon.exe", "System Idle Process", "System"].contains(&r.name.as_str()))
            .max_by(|a, b| a.cpu.total_cmp(&b.cpu))
            .filter(|r| r.cpu > 0.0)
            .map(|r| (r.name.clone(), r.cpu));
        self.latest = w.clone();

        if frames.is_empty() {
            self.now = None;
            return;
        }
        let v = bn::classify(&w, &self.thresholds);
        self.verdicts.push_back(v.clone());
        self.windows.push_back(w.clone());
        while self.windows.len() > KEEP {
            self.windows.pop_front();
            self.verdicts.pop_front();
        }
        let recent: Vec<Verdict> = self.verdicts.iter().cloned().collect();
        if let Some(smoothed) = bn::smooth(&recent) {
            let s = frames::summary(&crate::live_frames::recent_ms(30.0), Default::default());
            self.now = Some(Now {
                verdict: smoothed.clone(),
                window: w.clone(),
                app: target.map(|t| t.1).unwrap_or_default(),
                fps: if w.fps > 0.0 { w.fps } else { s.avg_fps },
                low1: s.low1_fps,
            });
        }
        if let Some(run) = &mut self.run {
            run.windows.push(w);
            run.verdicts.push(v);
            if run.length.is_some_and(|l| run.start.elapsed() >= l) {
                self.finish();
            }
        }
    }

    pub fn start(&mut self) {
        self.result = None;
        self.run = Some(Run {
            start: Instant::now(),
            length: self.length.map(Duration::from_secs),
            windows: Vec::new(),
            verdicts: Vec::new(),
        });
    }

    pub fn finish(&mut self) {
        if let Some(run) = self.run.take() {
            self.result = Some(bn::analyze_verdicts(&run.windows, &run.verdicts, &self.thresholds));
        }
    }

    /// (class, confidence) for the overlay pill; nothing when there's nothing to say.
    pub fn pill(&self) -> Option<(Class, Confidence)> {
        let n = self.now.as_ref()?;
        (n.verdict.primary.class != Class::Inconclusive)
            .then_some((n.verdict.primary.class, n.verdict.primary.confidence))
    }

    pub fn page(&mut self, ui: &mut Ui, access: &mut super::access_ui::Access) {
        super::live::State::frame_source(ui, access);
        ui.add_space(6.0);
        self.now_card(ui);
        self.readings_card(ui);
        self.analysis_card(ui);
    }

    fn now_card(&self, ui: &mut Ui) {
        let p = theme::palette(ui);
        widgets::card(ui, icon::GAUGE, tr("What limits your FPS"), |ui| {
            let Some(n) = &self.now else {
                ui.label(RichText::new(tr("No game is presenting frames yet.")).size(20.0).strong().color(p.text));
                ui.label(
                    RichText::new(tr("Start a game (borderless or windowed works best). The verdict appears within a few seconds of its first frames."))
                        .color(p.text_secondary),
                );
                return;
            };
            let v = &n.verdict;
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(icon::CIRCLE).color(class_color(v.primary.class)).size(22.0));
                ui.label(RichText::new(headline(v, &n.window)).size(26.0).strong().color(p.text));
            });
            ui.label(
                RichText::new(trf(
                    "{} · {} FPS · 1% low {}",
                    &[&n.app, &format!("{:.0}", n.fps), &format!("{:.0}", n.low1)],
                ))
                .color(p.text_secondary),
            );
            ui.add_space(4.0);
            ui.label(RichText::new(explain(v.primary.class)).color(p.text));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                let tone = match v.primary.confidence {
                    Confidence::High => Tone::Success,
                    Confidence::Medium => Tone::Neutral,
                    Confidence::Low => Tone::Warning,
                };
                widgets::pill(ui, tr(v.primary.confidence.label()), tone);
                if n.window.gpu_busy_frac.is_none() && n.window.frames > 0 {
                    widgets::pill(ui, tr("Estimated without GPU Busy: lower confidence"), Tone::Warning);
                }
                for f in &v.also {
                    widgets::pill(ui, &format!("{} {}", icon::INFO, tr(f.class.label())), Tone::Neutral);
                }
            });
            egui::CollapsingHeader::new(RichText::new(tr("Why?")).strong()).default_open(false).show(ui, |ui| {
                for f in std::iter::once(&v.primary).chain(&v.also) {
                    for e in &f.evidence {
                        ui.label(
                            RichText::new(format!("{} {}", icon::DOT_OUTLINE, evidence_text(e)))
                                .color(p.text_secondary),
                        );
                    }
                }
            });
        });
    }

    fn readings_card(&self, ui: &mut Ui) {
        let w = &self.latest;
        let t = &self.thresholds;
        widgets::card(ui, icon::CHART_BAR, tr("Live readings"), |ui| {
            ui.label(
                RichText::new(tr("The mark on each bar is the level where its rule fires."))
                    .small()
                    .color(theme::palette(ui).text_muted),
            );
            let rows: [(&str, Option<f64>, f64, Option<f64>, &str); 9] = [
                ("GPU Busy", w.gpu_busy_frac.map(|b| b * 100.0), 100.0, Some(t.gpu_busy_bound * 100.0), "%"),
                ("GPU load", w.gpu_util, 100.0, Some(t.gpu_util_bound), "%"),
                ("CPU load (all cores)", w.cpu_total, 100.0, None, "%"),
                ("Busiest core", w.busiest_core, 100.0, Some(t.thread_bound), "%"),
                ("The game's busiest thread", w.game_thread, 100.0, Some(t.thread_bound), "%"),
                ("Video memory used by the game", w.vram_used_pct, 100.0, Some(t.vram_full), "%"),
                ("RAM", w.ram_used_pct, 100.0, Some(t.ram_full), "%"),
                ("CPU temperature", w.cpu_temp, 110.0, Some(self.tjmax - 10.0), "°C"),
                ("GPU temperature", w.gpu_temp, 110.0, Some(83.0), "°C"),
            ];
            for (label, value, max, mark, unit) in rows {
                meter(ui, tr(label), value, max, mark, unit);
            }
            // The game's frame time over the last minute.
            let ft: Vec<(f64, f64)> = self.windows.iter().enumerate().map(|(i, x)| (i as f64, x.median_ms)).collect();
            if ft.len() > 3 {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(tr("Frame time, one point per second (median)"))
                        .small()
                        .color(theme::palette(ui).text_secondary),
                );
                spark(ui, &ft, 60.0);
            }
        });
    }

    fn analysis_card(&mut self, ui: &mut Ui) {
        let p = theme::palette(ui);
        widgets::card(ui, icon::TIMER, tr("Session analysis"), |ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(run) = &self.run {
                    let secs = run.start.elapsed().as_secs();
                    let of = run.length.map_or(String::new(), |l| format!(" / {} s", l.as_secs()));
                    widgets::pill(ui, &format!("{} {secs} s{of}", icon::CIRCLE), Tone::Danger);
                    if ui.button(format!("{} {}", icon::STOP, tr("Stop and analyse"))).clicked() {
                        self.finish();
                    }
                    ui.ctx().request_repaint_after(Duration::from_millis(500));
                } else {
                    for (label, secs) in
                        [("1 min", Some(60)), ("3 min", Some(180)), ("5 min", Some(300)), ("Until I stop", None)]
                    {
                        if ui.selectable_label(self.length == secs, tr(label)).clicked() {
                            self.length = secs;
                        }
                    }
                    if widgets::primary_button(ui, &format!("{} {}", icon::PLAY, tr("Analyse"))).clicked() {
                        self.start();
                    }
                }
            });
            ui.label(
                RichText::new(tr("Play normally while it runs. Nothing is recorded to disk."))
                    .small()
                    .color(p.text_muted),
            );
            if let Some(a) = &self.result {
                ui.add_space(6.0);
                draw_analysis(ui, a);
            }
        });
    }
}

/// A labelled bar with the rule's threshold marked on it.
fn meter(ui: &mut Ui, label: &str, value: Option<f64>, max: f64, mark: Option<f64>, unit: &str) {
    let p = theme::palette(ui);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(vec2(210.0, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(210.0);
            ui.label(RichText::new(label).color(p.text_secondary));
        });
        let (rect, resp) = ui.allocate_exact_size(vec2((ui.available_width() - 80.0).max(80.0), 14.0), Sense::hover());
        let painter = ui.painter_at(rect.expand(2.0));
        painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
        let over = value.zip(mark).is_some_and(|(v, m)| v >= m);
        resp.widget_info(|| {
            let v = value.map_or_else(|| "–".to_string(), |v| format!("{v:.0} {unit}"));
            let at = mark.map_or(String::new(), |m| {
                format!(", {}", trf("The rule fires at {} {}", &[&format!("{m:.0}"), &unit]))
            });
            egui::WidgetInfo::labeled(egui::Role::ProgressIndicator, true, format!("{label}: {v}{at}"))
        });
        if let Some(v) = value {
            let w = (v / max).clamp(0.0, 1.0) as f32 * rect.width();
            let color = if over { p.warning } else { p.accent };
            painter.rect_filled(egui::Rect::from_min_size(rect.min, vec2(w, rect.height())), 4.0, color);
        }
        if let Some(m) = mark {
            let x = rect.left() + (m / max).clamp(0.0, 1.0) as f32 * rect.width();
            painter.vline(x, rect.y_range().expand(2.0), egui::Stroke::new(2.0, p.text));
        }
        let text = value.map_or_else(|| "–".to_string(), |v| format!("{v:.0} {unit}"));
        // The warning colour is never the only cue: the text gets an icon too.
        let shown = if over { format!("{} {text}", icon::WARNING) } else { text };
        // Proportional: the monospace font has no icon glyphs.
        ui.label(RichText::new(shown).color(if over { p.warning } else { p.text }));
        if let Some(m) = mark {
            resp.on_hover_text(trf("The rule fires at {} {}", &[&format!("{m:.0}"), &unit]));
        }
    });
}

fn spark(ui: &mut Ui, pts: &[(f64, f64)], h: f32) {
    let p = theme::palette(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, ui.visuals().extreme_bg_color);
    let (x0, x1) = (pts[0].0, pts[pts.len() - 1].0);
    let y1 = pts.iter().map(|p| p.1).fold(1.0, f64::max) * 1.1;
    if x1 <= x0 {
        return;
    }
    let v: Vec<egui::Pos2> = pts
        .iter()
        .map(|q| {
            pos2(
                rect.left() + ((q.0 - x0) / (x1 - x0)) as f32 * rect.width(),
                rect.bottom() - (q.1 / y1) as f32 * (rect.height() - 4.0) - 2.0,
            )
        })
        .collect();
    painter.add(egui::Shape::line(v, egui::Stroke::new(1.5, p.accent)));
}

/// The session result: the stacked bar, the timeline, the evidence and the advice.
pub fn draw_analysis(ui: &mut Ui, a: &Analysis) {
    let p = theme::palette(ui);
    if a.seconds == 0 {
        ui.label(RichText::new(tr("No frames were measured: was a game presenting?")).color(p.text_muted));
        return;
    }
    // Stacked bar: time per class.
    let (rect, bar) = ui.allocate_exact_size(vec2(ui.available_width(), 22.0), Sense::hover());
    bar.widget_info(|| {
        let shares: Vec<String> =
            a.share.iter().map(|(c, f)| format!("{} {:.0} %", tr(c.label()), f * 100.0)).collect();
        egui::WidgetInfo::labeled(
            egui::Role::ProgressIndicator,
            true,
            format!("{}: {}", tr("Time per cause"), shares.join(", ")),
        )
    });
    let painter = ui.painter_at(rect);
    let mut x = rect.left();
    for (c, f) in &a.share {
        let w = (*f as f32 * rect.width()).max(1.0);
        painter.rect_filled(
            egui::Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height())),
            0.0,
            class_color(*c),
        );
        x += w;
    }
    ui.horizontal_wrapped(|ui| {
        for (c, f) in &a.share {
            ui.label(RichText::new(icon::SQUARE).color(class_color(*c)));
            ui.label(format!("{} {:.0} %", tr(c.label()), f * 100.0));
            ui.add_space(8.0);
        }
    });
    // Timeline: one stripe per second.
    ui.add_space(4.0);
    ui.label(RichText::new(tr("Second by second")).small().color(p.text_secondary));
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 16.0), Sense::hover());
    resp.widget_info(|| {
        // One stripe per second; the longest unbroken stretch says more than a colour strip.
        let mut best = (0usize, None);
        let mut run = (0usize, None);
        for c in &a.timeline {
            run = if run.1 == Some(*c) { (run.0 + 1, run.1) } else { (1, Some(*c)) };
            if run.0 > best.0 {
                best = run;
            }
        }
        let longest =
            best.1.map_or(String::new(), |c| trf("Longest stretch: {} for {} s.", &[&tr(c.label()), &best.0]));
        egui::WidgetInfo::labeled(
            egui::Role::Unknown,
            true,
            format!("{}: {} s. {longest}", tr("Second by second"), a.timeline.len()),
        )
    });
    let painter = ui.painter_at(rect);
    let w = rect.width() / a.timeline.len() as f32;
    for (i, c) in a.timeline.iter().enumerate() {
        painter.rect_filled(
            egui::Rect::from_min_size(pos2(rect.left() + i as f32 * w, rect.top()), vec2(w.max(1.0), rect.height())),
            0.0,
            class_color(*c),
        );
    }
    if let Some(h) = resp.hover_pos() {
        let i = (((h.x - rect.left()) / w) as usize).min(a.timeline.len() - 1);
        resp.on_hover_text(format!("{} s · {}", i, tr(a.timeline[i].label())));
    }
    if a.estimated_seconds > 0 {
        ui.label(
            RichText::new(trf("{} s were judged without GPU Busy: those are estimates.", &[&a.estimated_seconds]))
                .small()
                .color(p.warning),
        );
    }
    ui.add_space(6.0);
    ui.label(RichText::new(tr("Strongest evidence")).strong());
    for e in &a.evidence {
        ui.label(RichText::new(format!("{} {}", icon::DOT_OUTLINE, evidence_text(e))).color(p.text_secondary));
    }
    if !a.recommendations.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new(tr("What you can do")).strong());
        for r in &a.recommendations {
            let args: Vec<&dyn Display> = r.args.iter().map(|x| x as &dyn Display).collect();
            ui.label(RichText::new(format!("{} {}", icon::LIGHTBULB, trf(r.key, &args))).color(p.text));
            ui.label(RichText::new(format!("    {}", evidence_text(&r.because))).small().color(p.text_muted));
        }
    }
}

pub const PT_BR: &[(&str, &str)] = &[
    ("A power limit flag was set for {} s", "Um limite de energia ficou ativo por {} s"),
    (
        "A power limit held the clocks down: raise it in the BIOS or the GPU's tool if the cooling allows.",
        "Um limite de energia segurou os clocks: aumente-o na BIOS ou na ferramenta da GPU, se o resfriamento permitir.",
    ),
    (
        "An FPS cap is holding the game at about {} FPS: raise or remove it (in the game, the driver, RTSS) to see what the PC can do.",
        "Um limite de FPS está segurando o jogo em cerca de {} FPS: aumente ou remova (no jogo, no driver, no RTSS) para ver o que o PC consegue.",
    ),
    ("CPU Busy was {} % of the frame time", "A CPU ficou ocupada {} % do tempo do quadro"),
    (
        "Check the Build health card on the Overview: a memory profile left off, the power plan and the display's refresh rate all cost CPU-bound frames.",
        "Veja o cartão Saúde da montagem na Visão geral: perfil de memória desligado, plano de energia e taxa de atualização do monitor custam quadros quando a CPU limita.",
    ),
    (
        "Check the {} cooler, the thermal paste and the case airflow: the clock dropped to hold the temperature.",
        "Verifique o cooler da {}, a pasta térmica e o fluxo de ar do gabinete: o clock caiu para segurar a temperatura.",
    ),
    (
        "Close {} while you play: it was using CPU that the game needed.",
        "Feche o {} enquanto joga: ele estava usando CPU de que o jogo precisava.",
    ),
    ("Frame rate held at about {} FPS, in {} s", "Taxa de quadros mantida em cerca de {} FPS, em {} s"),
    (
        "Frame time is very regular ({} % variation) and the frame rate ({} FPS) equals the {} Hz refresh rate: V-Sync, or VRR at its ceiling",
        "O tempo de quadro é muito regular (variação de {} %) e a taxa ({} FPS) é igual à atualização de {} Hz do monitor: V-Sync, ou VRR no teto",
    ),
    (
        "Frames are locked to the refresh rate: turn V-Sync off to see what the PC can do (VRR keeps it smooth).",
        "Os quadros estão travados na taxa de atualização: desligue o V-Sync para ver o que o PC consegue (o VRR mantém a fluidez).",
    ),
    (
        "GPU Busy averaged {} % of the frame time while the game's busiest thread was at {} %",
        "O GPU Busy ficou em média {} % do tempo do quadro enquanto a thread mais ocupada do jogo estava em {} %",
    ),
    ("GPU Busy averaged {} % of the frame time, in {} s", "O GPU Busy ficou em média {} % do tempo do quadro, em {} s"),
    ("GPU Busy was only {} % of the frame time", "O GPU Busy foi de apenas {} % do tempo do quadro"),
    ("GPU Busy was {} % of the frame time", "O GPU Busy foi {} % do tempo do quadro"),
    (
        "GPU utilization averaged {} % (an estimate), in {} s",
        "A utilização da GPU ficou em média {} % (estimativa), em {} s",
    ),
    ("GPU utilization is only {} %", "A utilização da GPU é de apenas {} %"),
    (
        "GPU utilization is {} % (GPU Busy isn't available, so this is an estimate)",
        "A utilização da GPU é {} % (o GPU Busy não está disponível, então é uma estimativa)",
    ),
    (
        "Lower the graphics settings or the resolution, or turn on upscaling (DLSS, FSR or XeSS).",
        "Reduza os gráficos ou a resolução, ou ative o upscaling (DLSS, FSR ou XeSS).",
    ),
    (
        "Neither the GPU nor the CPU is at its limit and nothing is holding the frame rate back",
        "Nem a GPU nem a CPU estão no limite e nada está segurando a taxa de quadros",
    ),
    (
        "RAM is {} % used and the system read {} pages/s from disk to resolve faults",
        "A RAM está {} % usada e o sistema leu {} páginas/s do disco para resolver falhas",
    ),
    (
        "Steady at {} FPS with room to spare: an FPS cap (in the game, the driver or RTSS)",
        "Estável em {} FPS com folga: um limite de FPS (no jogo, no driver ou no RTSS)",
    ),
    (
        "Steady at {} FPS, 3 under the {} Hz refresh rate: a low-latency cap (Reflex, Anti-Lag)",
        "Estável em {} FPS, 3 abaixo dos {} Hz do monitor: um limite de baixa latência (Reflex, Anti-Lag)",
    ),
    (
        "The CPU has room to spare: CPU-heavy settings (view distance, crowd density) cost no FPS here.",
        "A CPU tem folga: opções pesadas para a CPU (distância de visão, densidade de multidão) não custam FPS aqui.",
    ),
    ("The CPU is at its power or current limit", "A CPU está no limite de energia ou de corrente"),
    ("The CPU's thermal throttle is on{}", "A redução térmica da CPU está ativa{}"),
    ("The GPU is at its power cap", "A GPU está no limite de energia"),
    ("The GPU was the limit in {} s", "A GPU foi o limite em {} s"),
    ("The GPU's thermal throttle is on{}", "A redução térmica da GPU está ativa{}"),
    (
        "The game read {} MB/s from disk while {} frame(s) stuttered",
        "O jogo leu {} MB/s do disco enquanto {} quadro(s) engasgaram",
    ),
    (
        "The game uses {} % of the video memory and {} frame(s) stuttered",
        "O jogo usa {} % da memória de vídeo e {} quadro(s) engasgaram",
    ),
    (
        "The game wasn't in front for part of this session: measure while you play.",
        "O jogo não estava em primeiro plano durante parte desta sessão: meça enquanto joga.",
    ),
    (
        "The game's busiest thread averaged {} % of one core while the GPU was the limit",
        "A thread mais ocupada do jogo ficou em média {} % de um núcleo enquanto a GPU era o limite",
    ),
    (
        "The game's busiest thread averaged {} % of one core, in {} s",
        "A thread mais ocupada do jogo ficou em média {} % de um núcleo, em {} s",
    ),
    ("The game's busiest thread is at {} % of one core{}", "A thread mais ocupada do jogo está em {} % de um núcleo{}"),
    ("The game's window isn't in front: {} FPS", "A janela do jogo não está em primeiro plano: {} FPS"),
    ("The {} throttled for {} s{}", "A {} reduziu o clock por {} s{}"),
    ("Too few frames in this second to tell", "Poucos quadros neste segundo para concluir"),
    (
        "You can raise the resolution or the graphics quality almost for free: the GPU has room to spare.",
        "Você pode aumentar a resolução ou a qualidade gráfica quase de graça: a GPU tem folga.",
    ),
    ("{} is using {} % of the CPU", "{} está usando {} % da CPU"),
    ("{} used {} % of the CPU in {} s", "{} usou {} % da CPU em {} s"),
    (
        "Presented through the desktop compositor ({}) with the GPU not at its limit",
        "Apresentado pelo compositor da área de trabalho ({}) com a GPU fora do limite",
    ),
    ("A thermal throttle flag was set for {} s", "Uma redução térmica ficou ativa por {} s"),
    ("Nothing was at its limit in {} s", "Nada estava no limite em {} s"),
    (
        "Video memory was {} % used while frames stuttered, in {} s",
        "A memória de vídeo estava {} % usada enquanto os quadros engasgavam, em {} s",
    ),
    (
        "The system was paging while frames stuttered, in {} s",
        "O sistema estava paginando enquanto os quadros engasgavam, em {} s",
    ),
    (
        "The game read {} MB/s from disk while frames stuttered, in {} s",
        "O jogo leu {} MB/s do disco enquanto os quadros engasgavam, em {} s",
    ),
    ("{} competed for the CPU in {} s", "{} disputou a CPU em {} s"),
    (
        "Lower the texture quality: the video memory is full and the game spills into system memory.",
        "Reduza a qualidade das texturas: a memória de vídeo está cheia e o jogo transborda para a memória do sistema.",
    ),
    (
        "Close other programs or add memory: Windows was reading from disk to keep up.",
        "Feche outros programas ou adicione memória: o Windows estava lendo do disco para acompanhar.",
    ),
    (
        "Install the game on an SSD. Stutters while the game loads assets or compiles shaders usually fade after the first minutes.",
        "Instale o jogo num SSD. Engasgos enquanto o jogo carrega recursos ou compila shaders costumam sumir depois dos primeiros minutos.",
    ),
    ("The game wasn't in front", "O jogo não estava em primeiro plano"),
    ("GPU at its limit", "GPU no limite"),
    ("CPU at its limit", "CPU no limite"),
    ("Limiter", "Limitador"),
    ("Held back on purpose", "Segurado de propósito"),
    ("Thermal", "Térmico"),
    ("VRAM", "VRAM"),
    ("Video memory is full", "A memória de vídeo está cheia"),
    ("The system is paging", "O sistema está paginando"),
    ("Waiting on the disk", "Esperando o disco"),
    ("Background load", "Carga em segundo plano"),
    ("Another program is competing", "Outro programa está competindo"),
    ("Inconclusive", "Inconclusivo"),
    ("Nothing at its limit", "Nada no limite"),
    ("Low confidence", "Confiança baixa"),
    ("Medium confidence", "Confiança média"),
    ("High confidence", "Confiança alta"),
    (
        "The graphics card is working the whole time. Lowering the resolution or graphics settings raises the frame rate; a faster CPU would not.",
        "A placa de vídeo trabalha o tempo todo. Reduzir a resolução ou os gráficos aumenta a taxa de quadros; uma CPU mais rápida não ajudaria.",
    ),
    (
        "One or two CPU threads are saturated while the graphics card waits for them. Raising the resolution or graphics settings costs almost nothing; a faster graphics card would not help.",
        "Uma ou duas threads da CPU estão saturadas enquanto a placa de vídeo espera por elas. Aumentar a resolução ou os gráficos quase não custa; uma placa mais rápida não ajudaria.",
    ),
    (
        "The frame rate is being held at a level on purpose (V-Sync, a frame cap, or the game isn't in front), not by the hardware.",
        "A taxa de quadros está sendo mantida num nível de propósito (V-Sync, um limite de quadros, ou o jogo não está em primeiro plano), não pelo hardware.",
    ),
    (
        "A chip is hot enough that it slows itself down to cool off.",
        "Um chip está quente a ponto de reduzir a própria velocidade para esfriar.",
    ),
    (
        "A power or current limit is holding the clocks down.",
        "Um limite de energia ou de corrente está segurando os clocks.",
    ),
    (
        "The video memory is full, so the game has to move data around while it runs.",
        "A memória de vídeo está cheia, então o jogo precisa mover dados enquanto roda.",
    ),
    (
        "The system is short of memory and is reading from disk to keep up.",
        "O sistema está com pouca memória e lê do disco para acompanhar.",
    ),
    (
        "The game is waiting on the disk while it loads assets or compiles shaders.",
        "O jogo espera pelo disco enquanto carrega recursos ou compila shaders.",
    ),
    ("Another program is using the CPU the game needs.", "Outro programa está usando a CPU de que o jogo precisa."),
    (
        "Neither the graphics card nor the CPU is at its limit, and nothing is holding the frame rate back.",
        "Nem a placa de vídeo nem a CPU estão no limite, e nada está segurando a taxa de quadros.",
    ),
    ("What limits your FPS", "O que limita o seu FPS"),
    ("No game is presenting frames yet.", "Nenhum jogo está apresentando quadros ainda."),
    (
        "Start a game (borderless or windowed works best). The verdict appears within a few seconds of its first frames.",
        "Abra um jogo (sem bordas ou em janela funciona melhor). O veredito aparece poucos segundos depois dos primeiros quadros.",
    ),
    ("Estimated without GPU Busy: lower confidence", "Estimado sem o GPU Busy: confiança menor"),
    ("Why?", "Por quê?"),
    ("Live readings", "Leituras ao vivo"),
    (
        "The mark on each bar is the level where its rule fires.",
        "A marca em cada barra é o nível em que a regra dispara.",
    ),
    ("Frame time, one point per second (median)", "Tempo de quadro, um ponto por segundo (mediana)"),
    ("Session analysis", "Análise de sessão"),
    ("Stop and analyse", "Parar e analisar"),
    ("Analyse", "Analisar"),
    (
        "Play normally while it runs. Nothing is recorded to disk.",
        "Jogue normalmente enquanto roda. Nada é gravado em disco.",
    ),
    ("No frames were measured: was a game presenting?", "Nenhum quadro foi medido: havia um jogo apresentando?"),
    ("Second by second", "Segundo a segundo"),
    ("Time per cause", "Tempo por causa"),
    ("Longest stretch: {} for {} s.", "Trecho mais longo: {} por {} s."),
    ("Strongest evidence", "Evidências mais fortes"),
    ("What you can do", "O que você pode fazer"),
    ("CPU at its limit (core {})", "CPU no limite (núcleo {})"),
    ("{} · {} FPS · 1% low {}", "{} · {} FPS · 1% mínimo {}"),
    ("The rule fires at {} {}", "A regra dispara em {} {}"),
    (
        "{} s were judged without GPU Busy: those are estimates.",
        "{} s foram avaliados sem o GPU Busy: são estimativas.",
    ),
    ("Bottleneck", "Gargalo"),
    ("What limited the frame rate", "O que limitou a taxa de quadros"),
    (
        "Show what limits the frame rate (GPU, CPU…) on the overlay",
        "Mostrar no overlay o que limita a taxa de quadros (GPU, CPU…)",
    ),
    ("What limits my FPS?", "O que limita o meu FPS?"),
    ("Open the bottleneck detector in ZeeZ Live", "Abrir o detector de gargalo na ZeeZ Live"),
    ("Frame rate locked to the refresh rate", "Taxa de quadros travada na atualização do monitor"),
    ("Frame rate capped", "Taxa de quadros limitada"),
    ("The game isn't in front", "O jogo não está em primeiro plano"),
    ("The desktop compositor may be in the way", "O compositor da área de trabalho pode estar atrapalhando"),
    ("CPU load (all cores)", "Carga da CPU (todos os núcleos)"),
    ("The game's busiest thread", "Thread mais ocupada do jogo"),
    ("Video memory used by the game", "Memória de vídeo usada pelo jogo"),
    ("1 min", "1 min"),
    ("3 min", "3 min"),
    ("5 min", "5 min"),
    ("Until I stop", "Até eu parar"),
];

pub const ES: &[(&str, &str)] = &[
    ("A power limit flag was set for {} s", "Un límite de potencia estuvo activo durante {} s"),
    (
        "A power limit held the clocks down: raise it in the BIOS or the GPU's tool if the cooling allows.",
        "Un límite de potencia frenó los relojes: súbelo en la BIOS o en la herramienta de la GPU si la refrigeración lo permite.",
    ),
    (
        "An FPS cap is holding the game at about {} FPS: raise or remove it (in the game, the driver, RTSS) to see what the PC can do.",
        "Un límite de FPS mantiene el juego en unos {} FPS: súbelo o quítalo (en el juego, el controlador o RTSS) para ver qué puede hacer el PC.",
    ),
    ("CPU Busy was {} % of the frame time", "La CPU estuvo ocupada el {} % del tiempo del fotograma"),
    (
        "Check the Build health card on the Overview: a memory profile left off, the power plan and the display's refresh rate all cost CPU-bound frames.",
        "Mira la tarjeta Salud del equipo en el Resumen: un perfil de memoria apagado, el plan de energía y la frecuencia del monitor cuestan fotogramas cuando limita la CPU.",
    ),
    (
        "Check the {} cooler, the thermal paste and the case airflow: the clock dropped to hold the temperature.",
        "Revisa el disipador de la {}, la pasta térmica y el flujo de aire de la caja: el reloj bajó para sostener la temperatura.",
    ),
    (
        "Close {} while you play: it was using CPU that the game needed.",
        "Cierra {} mientras juegas: usaba CPU que el juego necesitaba.",
    ),
    ("Frame rate held at about {} FPS, in {} s", "Tasa de fotogramas mantenida en unos {} FPS, en {} s"),
    (
        "Frame time is very regular ({} % variation) and the frame rate ({} FPS) equals the {} Hz refresh rate: V-Sync, or VRR at its ceiling",
        "El tiempo de fotograma es muy regular (variación del {} %) y la tasa ({} FPS) iguala los {} Hz del monitor: V-Sync, o VRR en su techo",
    ),
    (
        "Frames are locked to the refresh rate: turn V-Sync off to see what the PC can do (VRR keeps it smooth).",
        "Los fotogramas están fijados a la frecuencia del monitor: apaga el V-Sync para ver qué puede hacer el PC (VRR mantiene la fluidez).",
    ),
    (
        "GPU Busy averaged {} % of the frame time while the game's busiest thread was at {} %",
        "GPU Busy promedió el {} % del tiempo del fotograma mientras el hilo más ocupado del juego estaba al {} %",
    ),
    (
        "GPU Busy averaged {} % of the frame time, in {} s",
        "GPU Busy promedió el {} % del tiempo del fotograma, en {} s",
    ),
    ("GPU Busy was only {} % of the frame time", "GPU Busy fue solo el {} % del tiempo del fotograma"),
    ("GPU Busy was {} % of the frame time", "GPU Busy fue el {} % del tiempo del fotograma"),
    (
        "GPU utilization averaged {} % (an estimate), in {} s",
        "La utilización de la GPU promedió el {} % (estimación), en {} s",
    ),
    ("GPU utilization is only {} %", "La utilización de la GPU es solo del {} %"),
    (
        "GPU utilization is {} % (GPU Busy isn't available, so this is an estimate)",
        "La utilización de la GPU es del {} % (GPU Busy no está disponible, así que es una estimación)",
    ),
    (
        "Lower the graphics settings or the resolution, or turn on upscaling (DLSS, FSR or XeSS).",
        "Baja los gráficos o la resolución, o activa el escalado (DLSS, FSR o XeSS).",
    ),
    (
        "Neither the GPU nor the CPU is at its limit and nothing is holding the frame rate back",
        "Ni la GPU ni la CPU están al límite y nada frena la tasa de fotogramas",
    ),
    (
        "RAM is {} % used and the system read {} pages/s from disk to resolve faults",
        "La RAM está al {} % y el sistema leyó {} páginas/s del disco para resolver fallos",
    ),
    (
        "Steady at {} FPS with room to spare: an FPS cap (in the game, the driver or RTSS)",
        "Estable en {} FPS con margen: un límite de FPS (en el juego, el controlador o RTSS)",
    ),
    (
        "Steady at {} FPS, 3 under the {} Hz refresh rate: a low-latency cap (Reflex, Anti-Lag)",
        "Estable en {} FPS, 3 por debajo de los {} Hz del monitor: un límite de baja latencia (Reflex, Anti-Lag)",
    ),
    (
        "The CPU has room to spare: CPU-heavy settings (view distance, crowd density) cost no FPS here.",
        "La CPU tiene margen: las opciones pesadas para la CPU (distancia de visión, densidad de multitudes) no cuestan FPS aquí.",
    ),
    ("The CPU is at its power or current limit", "La CPU está en su límite de potencia o de corriente"),
    ("The CPU's thermal throttle is on{}", "La reducción térmica de la CPU está activa{}"),
    ("The GPU is at its power cap", "La GPU está en su límite de potencia"),
    ("The GPU was the limit in {} s", "La GPU fue el límite en {} s"),
    ("The GPU's thermal throttle is on{}", "La reducción térmica de la GPU está activa{}"),
    (
        "The game read {} MB/s from disk while {} frame(s) stuttered",
        "El juego leyó {} MB/s del disco mientras {} fotograma(s) se atascaron",
    ),
    (
        "The game uses {} % of the video memory and {} frame(s) stuttered",
        "El juego usa el {} % de la memoria de vídeo y {} fotograma(s) se atascaron",
    ),
    (
        "The game wasn't in front for part of this session: measure while you play.",
        "El juego no estaba en primer plano durante parte de esta sesión: mide mientras juegas.",
    ),
    (
        "The game's busiest thread averaged {} % of one core while the GPU was the limit",
        "El hilo más ocupado del juego promedió el {} % de un núcleo mientras la GPU era el límite",
    ),
    (
        "The game's busiest thread averaged {} % of one core, in {} s",
        "El hilo más ocupado del juego promedió el {} % de un núcleo, en {} s",
    ),
    ("The game's busiest thread is at {} % of one core{}", "El hilo más ocupado del juego está al {} % de un núcleo{}"),
    ("The game's window isn't in front: {} FPS", "La ventana del juego no está en primer plano: {} FPS"),
    ("The {} throttled for {} s{}", "La {} redujo su reloj durante {} s{}"),
    ("Too few frames in this second to tell", "Pocos fotogramas en este segundo para concluir"),
    (
        "You can raise the resolution or the graphics quality almost for free: the GPU has room to spare.",
        "Puedes subir la resolución o la calidad gráfica casi gratis: la GPU tiene margen.",
    ),
    ("{} is using {} % of the CPU", "{} está usando el {} % de la CPU"),
    ("{} used {} % of the CPU in {} s", "{} usó el {} % de la CPU en {} s"),
    (
        "Presented through the desktop compositor ({}) with the GPU not at its limit",
        "Presentado por el compositor del escritorio ({}) con la GPU fuera de su límite",
    ),
    ("A thermal throttle flag was set for {} s", "Una reducción térmica estuvo activa durante {} s"),
    ("Nothing was at its limit in {} s", "Nada estaba al límite en {} s"),
    (
        "Video memory was {} % used while frames stuttered, in {} s",
        "La memoria de vídeo estaba al {} % mientras los fotogramas se atascaban, en {} s",
    ),
    (
        "The system was paging while frames stuttered, in {} s",
        "El sistema estaba paginando mientras los fotogramas se atascaban, en {} s",
    ),
    (
        "The game read {} MB/s from disk while frames stuttered, in {} s",
        "El juego leyó {} MB/s del disco mientras los fotogramas se atascaban, en {} s",
    ),
    ("{} competed for the CPU in {} s", "{} compitió por la CPU en {} s"),
    (
        "Lower the texture quality: the video memory is full and the game spills into system memory.",
        "Baja la calidad de las texturas: la memoria de vídeo está llena y el juego se desborda a la memoria del sistema.",
    ),
    (
        "Close other programs or add memory: Windows was reading from disk to keep up.",
        "Cierra otros programas o añade memoria: Windows leía del disco para seguir el ritmo.",
    ),
    (
        "Install the game on an SSD. Stutters while the game loads assets or compiles shaders usually fade after the first minutes.",
        "Instala el juego en un SSD. Los tirones mientras el juego carga recursos o compila shaders suelen desaparecer tras los primeros minutos.",
    ),
    ("The game wasn't in front", "El juego no estaba en primer plano"),
    ("GPU at its limit", "GPU al límite"),
    ("CPU at its limit", "CPU al límite"),
    ("Limiter", "Limitador"),
    ("Held back on purpose", "Frenado a propósito"),
    ("Thermal", "Térmico"),
    ("VRAM", "VRAM"),
    ("Video memory is full", "La memoria de vídeo está llena"),
    ("The system is paging", "El sistema está paginando"),
    ("Waiting on the disk", "Esperando al disco"),
    ("Background load", "Carga en segundo plano"),
    ("Another program is competing", "Otro programa está compitiendo"),
    ("Inconclusive", "No concluyente"),
    ("Nothing at its limit", "Nada al límite"),
    ("Low confidence", "Confianza baja"),
    ("Medium confidence", "Confianza media"),
    ("High confidence", "Confianza alta"),
    (
        "The graphics card is working the whole time. Lowering the resolution or graphics settings raises the frame rate; a faster CPU would not.",
        "La tarjeta gráfica trabaja todo el tiempo. Bajar la resolución o los gráficos sube la tasa de fotogramas; una CPU más rápida no ayudaría.",
    ),
    (
        "One or two CPU threads are saturated while the graphics card waits for them. Raising the resolution or graphics settings costs almost nothing; a faster graphics card would not help.",
        "Uno o dos hilos de la CPU están saturados mientras la tarjeta gráfica los espera. Subir la resolución o los gráficos casi no cuesta; una tarjeta más rápida no ayudaría.",
    ),
    (
        "The frame rate is being held at a level on purpose (V-Sync, a frame cap, or the game isn't in front), not by the hardware.",
        "La tasa de fotogramas se mantiene a propósito en un nivel (V-Sync, un límite de fotogramas, o el juego no está en primer plano), no por el hardware.",
    ),
    (
        "A chip is hot enough that it slows itself down to cool off.",
        "Un chip está tan caliente que reduce su propia velocidad para enfriarse.",
    ),
    ("A power or current limit is holding the clocks down.", "Un límite de potencia o de corriente frena los relojes."),
    (
        "The video memory is full, so the game has to move data around while it runs.",
        "La memoria de vídeo está llena, así que el juego tiene que mover datos mientras se ejecuta.",
    ),
    (
        "The system is short of memory and is reading from disk to keep up.",
        "El sistema está corto de memoria y lee del disco para seguir el ritmo.",
    ),
    (
        "The game is waiting on the disk while it loads assets or compiles shaders.",
        "El juego espera al disco mientras carga recursos o compila shaders.",
    ),
    ("Another program is using the CPU the game needs.", "Otro programa usa la CPU que el juego necesita."),
    (
        "Neither the graphics card nor the CPU is at its limit, and nothing is holding the frame rate back.",
        "Ni la tarjeta gráfica ni la CPU están al límite, y nada frena la tasa de fotogramas.",
    ),
    ("What limits your FPS", "Qué limita tus FPS"),
    ("No game is presenting frames yet.", "Ningún juego está presentando fotogramas todavía."),
    (
        "Start a game (borderless or windowed works best). The verdict appears within a few seconds of its first frames.",
        "Abre un juego (sin bordes o en ventana funciona mejor). El veredicto aparece unos segundos después de los primeros fotogramas.",
    ),
    ("Estimated without GPU Busy: lower confidence", "Estimado sin GPU Busy: menor confianza"),
    ("Why?", "¿Por qué?"),
    ("Live readings", "Lecturas en vivo"),
    (
        "The mark on each bar is the level where its rule fires.",
        "La marca de cada barra es el nivel en que se activa su regla.",
    ),
    ("Frame time, one point per second (median)", "Tiempo de fotograma, un punto por segundo (mediana)"),
    ("Session analysis", "Análisis de sesión"),
    ("Stop and analyse", "Detener y analizar"),
    ("Analyse", "Analizar"),
    (
        "Play normally while it runs. Nothing is recorded to disk.",
        "Juega con normalidad mientras se ejecuta. No se graba nada en disco.",
    ),
    ("No frames were measured: was a game presenting?", "No se midió ningún fotograma: ¿había un juego presentando?"),
    ("Second by second", "Segundo a segundo"),
    ("Time per cause", "Tiempo por causa"),
    ("Longest stretch: {} for {} s.", "Tramo más largo: {} durante {} s."),
    ("Strongest evidence", "Evidencias más fuertes"),
    ("What you can do", "Qué puedes hacer"),
    ("CPU at its limit (core {})", "CPU al límite (núcleo {})"),
    ("{} · {} FPS · 1% low {}", "{} · {} FPS · 1% mínimo {}"),
    ("The rule fires at {} {}", "La regla se activa en {} {}"),
    ("{} s were judged without GPU Busy: those are estimates.", "{} s se evaluaron sin GPU Busy: son estimaciones."),
    ("Bottleneck", "Cuello de botella"),
    ("What limited the frame rate", "Qué limitó la tasa de fotogramas"),
    (
        "Show what limits the frame rate (GPU, CPU…) on the overlay",
        "Mostrar en el overlay qué limita la tasa de fotogramas (GPU, CPU…)",
    ),
    ("What limits my FPS?", "¿Qué limita mis FPS?"),
    ("Open the bottleneck detector in ZeeZ Live", "Abrir el detector de cuello de botella en ZeeZ Live"),
    ("Frame rate locked to the refresh rate", "Tasa de fotogramas fijada a la frecuencia del monitor"),
    ("Frame rate capped", "Tasa de fotogramas limitada"),
    ("The game isn't in front", "El juego no está en primer plano"),
    ("The desktop compositor may be in the way", "El compositor del escritorio puede estar estorbando"),
    ("CPU load (all cores)", "Carga de la CPU (todos los núcleos)"),
    ("The game's busiest thread", "Hilo más ocupado del juego"),
    ("Video memory used by the game", "Memoria de vídeo usada por el juego"),
    ("1 min", "1 min"),
    ("3 min", "3 min"),
    ("5 min", "5 min"),
    ("Until I stop", "Hasta que pare"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    fn show(d: Detector) -> Harness<'static, Detector> {
        let mut h = Harness::builder().with_size(egui::vec2(1000.0, 1200.0)).build_ui_state(
            |ui, d: &mut Detector| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                let mut access = super::super::access_ui::Access::default();
                d.page(ui, &mut access);
            },
            d,
        );
        h.run();
        h.run();
        h
    }

    fn now(w: Window) -> Now {
        let v = bn::classify(&w, &Thresholds::default());
        Now { verdict: v, window: w.clone(), app: "cs2.exe".into(), fps: 597.0, low1: 154.0 }
    }

    #[test]
    fn the_card_says_who_limits_and_why() {
        let w = Window {
            frames: 120,
            fps: 597.0,
            ms_cv: 0.2,
            gpu_busy_frac: Some(0.41),
            game_thread: Some(98.0),
            busiest_core_id: Some(4),
            ..Default::default()
        };
        let h = show(Detector::preview(Some(now(w)), None));
        h.get_by_label("CPU at its limit (core 4)");
        h.get_by_label("cs2.exe · 597 FPS · 1% low 154");
        h.get_by_label("High confidence");
        h.get_by_label_contains("One or two CPU threads are saturated");
        h.get_by_label("Why?");
    }

    #[test]
    fn without_a_game_it_says_so_and_without_gpu_busy_it_says_that() {
        let h = show(Detector::default());
        h.get_by_label("No game is presenting frames yet.");
        let w = Window { frames: 120, fps: 140.0, ms_cv: 0.2, gpu_util: Some(99.0), ..Default::default() };
        let h = show(Detector::preview(Some(now(w)), None));
        h.get_by_label("GPU at its limit");
        h.get_by_label("Low confidence");
        h.get_by_label("Estimated without GPU Busy: lower confidence");
    }

    #[test]
    fn a_session_result_lists_shares_evidence_and_advice() {
        let w = Window {
            frames: 120,
            fps: 140.0,
            ms_cv: 0.2,
            gpu_busy_frac: Some(0.97),
            game_thread: Some(40.0),
            ..Default::default()
        };
        let a = bn::analyze(&vec![w.clone(); 30], &Thresholds::default());
        let h = show(Detector::preview(Some(now(w)), Some(a)));
        // The share is in the list and, now, in the bar's accessible name.
        assert!(h.get_all_by_label_contains("GPU 100 %").count() >= 2);
        h.get_by_label_contains("Time per cause: GPU 100 %");
        assert!(
            h.get_all_by_label_contains("GPU Busy averaged 97 % of the frame time").count() >= 2,
            "as evidence and as the basis of the advice"
        );
        h.get_by_label_contains("turn on upscaling");
        h.get_by_label("Strongest evidence");
    }

    #[test]
    fn the_pill_is_for_real_limits_only() {
        let gpu = Window { frames: 120, fps: 100.0, ms_cv: 0.2, gpu_busy_frac: Some(0.97), ..Default::default() };
        assert_eq!(Detector::preview(Some(now(gpu)), None).pill().map(|p| p.0), Some(Class::Gpu));
        let nothing = Window { frames: 120, fps: 77.0, ms_cv: 0.2, ..Default::default() };
        assert_eq!(Detector::preview(Some(now(nothing)), None).pill(), None);
        assert_eq!(Detector::default().pill(), None);
    }
}

//! ZeeZ Live (S2b, docs/v1/zeez-live.md): the sidebar entry with its Live,
//! Overlay and Sessions tabs, and the overlay window itself.
//!
//! The overlay is CpuZeeZ's own window above the game: transparent, always on
//! top, never focused and click-through outside edit mode. Nothing is
//! injected into the game (P4). It shows a Widget Engine layout, the same
//! kind the mini monitors use.

use super::i18n::tr;
use super::sensors::Monitor;
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use cpuzeez_core::layout::presets;
use cpuzeez_core::live::{self as model, Anchor, HotkeyAction};
use cpuzeez_core::session::{self, Band, Marker, Mode, Session};
use cpuzeez_platform::overlay as os_overlay;
use eframe::egui::{self, RichText, Ui};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// The overlay window's title: how `platform::overlay` finds it.
pub const OVERLAY_TITLE: &str = "CpuZeeZ ZeeZ Live overlay";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    #[default]
    Live,
    Overlay,
    Sessions,
}

#[derive(Default)]
pub struct State {
    pub tab: Tab,
    /// The overlay takes the mouse (drag it), until Esc or "Done".
    pub edit_mode: bool,
    /// First use: 0 style, 1 hotkey, 2 try it.
    first_step: u8,
    customize: bool,
    import: String,
    styled: Option<(os_overlay::Style, Instant)>,
    hotkeys: super::hotkeys::Hotkeys,
    /// Capture and marker presses, for the recorder (S2b-09).
    pub capture_requests: Vec<HotkeyAction>,
    capture: Option<Capture>,
    /// Timed capture length, seconds.
    timed_secs: u32,
    /// The capture just written, opened in Sessions.
    pub last_session: Option<PathBuf>,
    capture_error: Option<String>,
    pub sessions: super::live_sessions::Sessions,
    /// Profile games running at the last check, and when it was.
    running: Vec<String>,
    checked: Option<Instant>,
    /// The overlay was shown, or the capture started, by a profile.
    auto_shown: Option<String>,
    auto_capture: Option<String>,
}

/// A capture in progress (S2b-09).
struct Capture {
    dir: PathBuf,
    mode: Mode,
    wall: SystemTime,
    /// The countdown before a timed capture (zeez-live §6).
    begins: Instant,
    started: Option<Instant>,
    length: Option<Duration>,
    game: (u32, String),
    markers: Vec<Marker>,
    throttling: Vec<Band>,
    open_band: Option<(f64, String)>,
}

impl Capture {
    fn elapsed(&self) -> f64 {
        self.started.map_or(0.0, |s| s.elapsed().as_secs_f64())
    }
}

/// Where captures go: `sessions/` next to the settings.
pub fn sessions_dir() -> Option<PathBuf> {
    crate::settings::dir().map(|d| d.join("sessions"))
}

fn timer(s: f64) -> String {
    let s = s.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Every frame: the hotkeys, and the overlay when it's shown. `live`: false
/// in the screenshot run, which must not grab the user's shortcuts.
pub fn overlay(mon: &mut Monitor, st: &mut State, ectx: &egui::Context, live: bool) {
    if live {
        st.hotkeys.sync(&mon.settings.overlay.hotkeys);
        for a in st.hotkeys.poll() {
            st.hotkey(a, mon);
        }
    }
    for a in std::mem::take(&mut st.capture_requests) {
        match a {
            HotkeyAction::Capture if st.capture.is_some() => st.stop_capture(mon),
            HotkeyAction::Capture => st.start_capture(mon, Mode::Manual),
            _ => st.marker(),
        }
    }
    st.tick_capture(mon);
    if live {
        st.automation(mon);
    }
    let o = mon.settings.overlay.clone();
    if !o.shown {
        st.styled = None;
        return;
    }
    let rec = st.capture.as_ref().map(|c| (c.started.is_some(), c.elapsed(), c.begins));
    let tel = mon.tel();
    // A game's profile can bring its own layout and place.
    let target = crate::live_frames::target().map(|t| t.1);
    let (layout, anchor, offset) = o.effective(target.as_deref());
    let lease = super::layout_view::prefixes(&layout.widgets);
    tel.want("overlay", &lease.iter().map(String::as_str).collect::<Vec<_>>());
    let series = tel.series_with(|_| false);
    let edit = st.edit_mode;
    let builder = egui::ViewportBuilder::default()
        .with_title(OVERLAY_TITLE)
        .with_always_on_top()
        .with_decorations(false)
        .with_resizable(false)
        .with_transparent(true)
        .with_taskbar(false)
        .with_active(false)
        .with_mouse_passthrough(!edit);
    let mut moved: Option<([f32; 2], [f32; 2], [f32; 2])> = None;
    let mut end_edit = false;
    ectx.show_viewport_immediate(egui::ViewportId::from_hash_of("zeez-live-overlay"), builder, |ui, _| {
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            // ● REC is always on the overlay while capturing (zeez-live §7).
            let rec_rect = rec.map(|(on, t, begins)| {
                let p = theme::palette(ui);
                let text = if on {
                    format!("{} REC {}", icon::CIRCLE, timer(t))
                } else {
                    format!("{} REC {}…", icon::CIRCLE, begins.saturating_duration_since(Instant::now()).as_secs() + 1)
                };
                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(170))
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| ui.label(RichText::new(text).strong().color(p.danger)))
                    .response
                    .rect
            });
            let mut drawn = super::layout_view::show(ui, &layout, &series, &|id| tel.points(id)).rect;
            if let Some(r) = rec_rect {
                drawn = drawn.union(r);
            }
            if edit {
                let p = theme::palette(ui);
                ui.painter().rect_stroke(
                    drawn.expand(1.0),
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(2.0, p.accent),
                    egui::StrokeKind::Inside,
                );
                let bg = ui.interact(ui.max_rect(), ui.id().with("drag"), egui::Sense::click_and_drag());
                if bg.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    end_edit = true;
                }
            }
            let want = drawn.size() + egui::vec2(2.0, 2.0);
            if (ui.ctx().content_rect().size() - want).length() > 2.0 {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(want));
            }
            let (outer, screen, down) =
                ui.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size, i.pointer.any_down()));
            if let (Some(r), Some(scr)) = (outer, screen) {
                let (size, scr) = ([r.width(), r.height()], [scr.x, scr.y]);
                if edit {
                    if !down {
                        moved = Some(([r.min.x, r.min.y], size, scr));
                    }
                } else {
                    // ponytail: placed on the monitor the window is on, from its
                    // origin, so the primary display; per-monitor origins need winit.
                    let pos = model::place(anchor, offset, size, scr);
                    if (pos[0] - r.min.x).abs() + (pos[1] - r.min.y).abs() > 1.0 {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos.into()));
                    }
                }
            }
        });
    });
    // Window styles: when they change, and every 2 s (a game going
    // fullscreen can push other topmost windows down).
    let want = os_overlay::Style { click_through: !edit, no_activate: true, exclude_from_capture: o.hide_from_capture };
    if st.styled.is_none_or(|(s, t)| s != want || t.elapsed() > Duration::from_secs(2))
        && (os_overlay::apply(OVERLAY_TITLE, want) || !cfg!(windows))
    {
        st.styled = Some((want, Instant::now()));
    }
    if let Some((pos, size, scr)) = moved {
        let anchor = model::nearest_anchor(pos, size, scr);
        let offset = model::offset_for(anchor, pos, size, scr).map(|v| v.round());
        if anchor != o.anchor || offset != o.offset {
            mon.set_setting(move |s| {
                s.overlay.anchor = anchor;
                s.overlay.offset = offset;
            });
        }
    }
    if end_edit {
        st.edit_mode = false;
    }
    ectx.request_repaint_after(Duration::from_secs_f32(1.0 / o.text_hz.clamp(1.0, 30.0)));
}

/// The ● LIVE indicator, wherever the overlay's state shows.
pub fn live_pill(ui: &mut Ui, shown: bool) {
    if shown {
        widgets::pill(ui, &format!("{} {}", icon::CIRCLE, tr("LIVE")), Tone::Success);
    }
}

impl State {
    /// What the frame source is doing, and what the user can do about it.
    fn frame_source(&mut self, ui: &mut Ui, access: &mut super::access_ui::Access) {
        use crate::live_frames::Status;
        let p = theme::palette(ui);
        let (tone, text) = match crate::live_frames::status() {
            Status::Running => (Tone::Success, tr("Frame timing on (PresentMon)").to_string()),
            Status::Off | Status::Starting => (Tone::Neutral, tr("Starting frame timing…").to_string()),
            Status::NeedsAdmin => {
                (Tone::Warning, tr("Frame timing needs CpuZeeZ to run as administrator.").to_string())
            }
            Status::Missing => {
                (Tone::Warning, tr("The PresentMon sidecar isn't next to CpuZeeZ: reinstall to get FPS.").to_string())
            }
            Status::BadChecksum => (
                Tone::Danger,
                tr("PresentMon next to CpuZeeZ isn't the version we ship (SHA-256 differs), so it was not run.")
                    .to_string(),
            ),
            Status::Failed(e) => (Tone::Danger, super::i18n::trf("Frame timing stopped: {}", &[&e])),
        };
        let needs_admin = crate::live_frames::status() == Status::NeedsAdmin;
        ui.horizontal_wrapped(|ui| {
            if cfg!(windows) {
                widgets::pill(ui, &text, tone);
            } else if cfg!(target_os = "linux") {
                widgets::pill(ui, tr("FPS on Linux comes from MangoHud's logs (below)."), Tone::Neutral);
            } else {
                widgets::pill(ui, tr("FPS on macOS is experimental (Metal HUD)."), Tone::Neutral);
            }
            if needs_admin {
                if access.restarting() {
                    ui.spinner();
                } else if widgets::primary_button(
                    ui,
                    &format!("{} {}", icon::SHIELD_CHECK, tr("Restart as administrator")),
                )
                .clicked()
                {
                    access.restart_elevated();
                }
            }
        });
        ui.label(
            RichText::new(tr("Frames are read from the graphics kernel's events: nothing is injected into the game."))
                .small()
                .color(p.text_muted),
        );
    }

    pub fn capturing(&self) -> bool {
        self.capture.as_ref().is_some_and(|c| c.started.is_some())
    }

    /// Every 2 s while a profile automates: which of its games run, and
    /// show/hide, capture and safe mode as they start and stop.
    fn automation(&mut self, mon: &mut Monitor) {
        let profiles: Vec<model::Profile> =
            mon.settings.overlay.profiles.iter().filter(|p| p.automates()).cloned().collect();
        if profiles.is_empty() {
            if !self.running.is_empty() {
                self.running.clear();
                cpuzeez_platform::hw::pause(false);
            }
            return;
        }
        if self.checked.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
            return;
        }
        self.checked = Some(Instant::now());
        // One kernel snapshot without threads: a few ms every 2 s.
        let names: Vec<String> = cpuzeez_platform::procs::snapshot(false)
            .into_iter()
            .map(|p| p.name)
            .filter(|n| model::profile_for(&profiles, n).is_some())
            .collect();
        let (started, stopped) = model::transitions(&self.running, &names);
        for exe in started {
            let Some(p) = model::profile_for(&profiles, exe) else { continue };
            if p.auto_show && !mon.settings.overlay.shown {
                mon.set_setting(|s| s.overlay.shown = true);
                self.auto_shown = Some(exe.to_string());
            }
            if p.auto_capture && self.capture.is_none() {
                self.start_capture_of(mon, Mode::Automatic, Some((0, exe.to_string())));
                self.auto_capture = Some(exe.to_string());
            }
        }
        for exe in stopped {
            if self.auto_shown.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(&exe)) {
                mon.set_setting(|s| s.overlay.shown = false);
                self.auto_shown = None;
            }
            if self.auto_capture.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(&exe)) {
                self.stop_capture(mon);
                self.auto_capture = None;
            }
        }
        // Safe mode holds while any safe-mode game runs.
        let safe = names.iter().any(|n| model::profile_for(&profiles, n).is_some_and(|p| p.safe_mode));
        cpuzeez_platform::hw::pause(safe);
        self.running = names;
    }

    /// Start a capture: manual now, timed after a 3 s countdown.
    fn start_capture(&mut self, mon: &mut Monitor, mode: Mode) {
        self.start_capture_of(mon, mode, None);
    }

    /// `game`: what an automatic capture is of (else the measured app).
    fn start_capture_of(&mut self, mon: &mut Monitor, mode: Mode, game: Option<(u32, String)>) {
        if self.capture.is_some() {
            return;
        }
        let Some(root) = sessions_dir() else {
            self.capture_error = Some(tr("No data folder to save the session in.").into());
            return;
        };
        let wall = SystemTime::now();
        let iso = cpuzeez_core::metrics::iso8601(wall);
        let stamp = format!("{}_{}{}", &iso[..10], &iso[11..13], &iso[14..16]);
        let game = game.or_else(crate::live_frames::target).unwrap_or((0, "desktop".into()));
        let mut dir = root.join(session::folder_name(&stamp, &game.1));
        let mut n = 2;
        while dir.exists() {
            dir = root.join(format!("{}-{n}", session::folder_name(&stamp, &game.1)));
            n += 1;
        }
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.capture_error = Some(e.to_string());
            return;
        }
        let countdown = if mode == Mode::Timed { 3 } else { 0 };
        let length = (mode == Mode::Timed).then(|| Duration::from_secs(self.timed_secs.max(1) as u64));
        self.capture_error = None;
        self.capture = Some(Capture {
            dir,
            mode,
            wall,
            begins: Instant::now() + Duration::from_secs(countdown),
            started: None,
            length,
            game,
            markers: Vec::new(),
            throttling: Vec::new(),
            open_band: None,
        });
        self.tick_capture(mon);
    }

    fn marker(&mut self) {
        if let Some(c) = self.capture.as_mut().filter(|c| c.started.is_some()) {
            let n = c.markers.len() + 1;
            c.markers.push(Marker { t_s: c.elapsed(), label: format!("Marker {n}") });
        }
    }

    /// Every frame while capturing: start after the countdown, keep the
    /// metrics sampled, record throttling, stop a timed capture.
    fn tick_capture(&mut self, mon: &mut Monitor) {
        let Some(c) = self.capture.as_mut() else { return };
        let tel = mon.tel();
        tel.want("capture", &["cpu", "gpu", "mem", "frames"]);
        if c.started.is_none() {
            if Instant::now() < c.begins {
                return;
            }
            if let Err(e) = tel.start_capture(&c.dir.join("hw.csv")) {
                self.capture_error = Some(e.to_string());
                self.capture = None;
                return;
            }
            crate::live_frames::begin_capture();
            c.started = Some(Instant::now());
        }
        let t = c.elapsed();
        let thr = tel.throttling();
        match (&c.open_band, thr.first()) {
            (None, Some(th)) => c.open_band = Some((t, format!("{}: {}", th.device, th.detail))),
            (Some((from, label)), None) => {
                c.throttling.push(Band { from_s: *from, to_s: t, label: label.clone() });
                c.open_band = None;
            }
            _ => {}
        }
        if c.length.is_some_and(|l| t >= l.as_secs_f64()) {
            self.stop_capture(mon);
        }
    }

    /// Stop and write `frames.csv` and `session.json` (on a thread: a long
    /// capture is millions of lines).
    fn stop_capture(&mut self, mon: &mut Monitor) {
        let Some(mut c) = self.capture.take() else { return };
        let tel = mon.tel();
        tel.stop_capture();
        if c.started.is_none() {
            let _ = std::fs::remove_dir(&c.dir); // cancelled during the countdown
            return;
        }
        let frames = crate::live_frames::end_capture();
        let duration_s = c.elapsed();
        if let Some((from, label)) = c.open_band.take() {
            c.throttling.push(Band { from_s: from, to_s: duration_s, label });
        }
        let ctx = mon.ctx().clone();
        let cpu = crate::cpu::get(&ctx).name.clone();
        let gpu = crate::gpu::adapters(&ctx).first().cloned();
        let s = Session {
            version: session::VERSION,
            game: c.game.1.clone(),
            pid: c.game.0,
            started: cpuzeez_core::metrics::iso8601(c.wall),
            duration_s,
            mode: c.mode,
            source: if cfg!(windows) {
                format!("PresentMon {}", crate::executor::pin("PRESENTMON_VERSION"))
            } else {
                "MangoHud".into()
            },
            system: session::System {
                cpu,
                gpu: gpu.as_ref().map(|g| g.name.clone()).unwrap_or_default(),
                gpu_driver: gpu.and_then(|g| g.driver_version).unwrap_or_default(),
                os: std::env::consts::OS.into(),
                ..Default::default()
            },
            markers: c.markers,
            throttling: c.throttling,
            summary: cpuzeez_core::frames::summarize(&frames, Default::default()),
            convention: Default::default(),
            note: String::new(),
        };
        let dir = c.dir.clone();
        std::thread::spawn(move || write_session(&dir, &frames, &s));
        self.last_session = Some(c.dir);
        self.tab = Tab::Sessions;
    }

    fn capture_card(&mut self, ui: &mut Ui, mon: &mut Monitor) {
        let p = theme::palette(ui);
        if self.timed_secs == 0 {
            self.timed_secs = 60; // the usual benchmark pass
        }
        widgets::card(ui, icon::RECORD, tr("Capture"), |ui| {
            ui.horizontal_wrapped(|ui| match &self.capture {
                Some(c) => {
                    let t = c.elapsed();
                    let text = if c.started.is_some() {
                        format!("{} REC {}", icon::CIRCLE, timer(t))
                    } else {
                        tr("Starting…").into()
                    };
                    widgets::pill(ui, &text, Tone::Danger);
                    if ui.button(format!("{} {}", icon::STOP, tr("Stop"))).clicked() {
                        self.stop_capture(mon);
                    }
                    if ui.button(format!("{} {}", icon::FLAG, tr("Marker"))).clicked() {
                        self.marker();
                    }
                }
                None => {
                    if widgets::primary_button(ui, &format!("{} {}", icon::RECORD, tr("Start capture"))).clicked() {
                        self.start_capture(mon, Mode::Manual);
                    }
                    ui.separator();
                    ui.add(egui::DragValue::new(&mut self.timed_secs).range(5..=3600).suffix(" s"));
                    if ui.button(format!("{} {}", icon::TIMER, tr("Timed capture"))).clicked() {
                        self.start_capture(mon, Mode::Timed);
                    }
                }
            });
            if let Some(e) = &self.capture_error {
                ui.label(RichText::new(super::i18n::trf("Capture failed: {}", &[e])).color(p.danger));
            }
            let keys = mon.settings.overlay.keys(HotkeyAction::Capture).to_string();
            ui.label(
                RichText::new(super::i18n::trf(
                    "Timed captures count down 3 s first. {} starts or stops a capture from the game. Nothing is recorded unless you start it.",
                    &[&keys],
                ))
                .small()
                .color(p.text_muted),
            );
        });
    }

    fn hotkey(&mut self, a: HotkeyAction, mon: &mut Monitor) {
        match a {
            HotkeyAction::ToggleOverlay => {
                let on = !mon.settings.overlay.shown;
                mon.set_setting(move |s| s.overlay.shown = on);
                if !on {
                    self.edit_mode = false;
                }
            }
            HotkeyAction::CycleStyle => {
                let styles = presets::live::all(mon.tjmax());
                let cur = mon.settings.overlay.style.clone();
                let i = styles.iter().position(|l| Some(&l.name) == cur.as_ref()).map_or(0, |i| (i + 1) % styles.len());
                let next = styles[i].clone();
                mon.set_setting(move |s| {
                    s.overlay.style = Some(next.name.clone());
                    s.overlay.layout = next.clone();
                });
            }
            HotkeyAction::EditMode => {
                self.edit_mode = !self.edit_mode;
                if self.edit_mode && !mon.settings.overlay.shown {
                    mon.set_setting(|s| s.overlay.shown = true);
                }
            }
            HotkeyAction::Capture | HotkeyAction::Marker => self.capture_requests.push(a),
        }
    }

    fn hotkeys_card(&mut self, ui: &mut Ui, o: &mut model::Overlay) {
        let p = theme::palette(ui);
        widgets::card(ui, icon::KEYBOARD, tr("Hotkeys"), |ui| {
            egui::Grid::new("live-hotkeys").num_columns(3).spacing([12.0, 4.0]).show(ui, |ui| {
                for (a, keys) in o.hotkeys.iter_mut() {
                    ui.label(tr(a.label()));
                    ui.add(egui::TextEdit::singleline(keys).desired_width(140.0).hint_text(tr("off")));
                    if let Some((_, why)) = self.hotkeys.problems.iter().find(|x| x.0 == *a) {
                        ui.label(RichText::new(format!("{} {why}", icon::WARNING)).small().color(p.warning));
                    } else if ui.small_button(tr("Default")).clicked() {
                        *keys = a.default_keys().to_string();
                    }
                    ui.end_row();
                }
            });
            ui.label(
                RichText::new(tr("Empty turns a shortcut off. Example: Ctrl+Alt+F9.")).small().color(p.text_muted),
            );
            if cfg!(target_os = "linux") {
                ui.label(
                    RichText::new(tr(
                        "On Wayland, shortcuts work only while CpuZeeZ has focus; use the tray menu instead.",
                    ))
                    .small()
                    .color(p.text_muted),
                );
            }
        });
    }

    pub fn page(&mut self, ui: &mut Ui, mon: &mut Monitor, access: &mut super::access_ui::Access) {
        let shown = mon.settings.overlay.shown;
        widgets::page_header(ui, "ZeeZ Live", tr("Overlay, capture and reports while you play or work."), |ui| {
            let label = if shown { tr("Hide overlay") } else { tr("Show overlay") };
            if widgets::primary_button(ui, &format!("{} {label}", icon::MONITOR)).clicked() {
                mon.set_setting(move |s| s.overlay.shown = !shown);
            }
            live_pill(ui, shown);
        });
        widgets::segmented(
            ui,
            &mut self.tab,
            &[
                (Tab::Live, icon::PULSE, tr("Live")),
                (Tab::Overlay, icon::STACK, tr("Overlay")),
                (Tab::Sessions, icon::FILM_STRIP, tr("Sessions")),
            ],
        );
        ui.add_space(8.0);
        match self.tab {
            Tab::Live => self.live_tab(ui, mon, access),
            Tab::Overlay => {
                if mon.settings.overlay.first_use_done {
                    self.overlay_tab(ui, mon);
                } else {
                    self.first_use(ui, mon);
                }
            }
            Tab::Sessions => {
                if let Some(dir) = self.last_session.take() {
                    self.sessions.focus(dir);
                }
                self.sessions.show(ui);
            }
        }
    }

    fn live_tab(&mut self, ui: &mut Ui, mon: &mut Monitor, access: &mut super::access_ui::Access) {
        let p = theme::palette(ui);
        let tel = mon.tel();
        // Reading frames/* is what runs the frame source.
        tel.want("live-frames", &["frames"]);
        let series = tel.series_with(|id| id == "frames/ms");
        widgets::card(ui, icon::GAME_CONTROLLER, tr("Game"), |ui| {
            self.frame_source(ui, access);
            let apps = crate::live_frames::apps();
            let target = crate::live_frames::target();
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Measuring")).color(p.text_secondary));
                let shown =
                    target.as_ref().map_or_else(|| tr("nothing yet").to_string(), |t| format!("{} ({})", t.1, t.0));
                egui::ComboBox::from_id_salt("live-target").selected_text(shown).show_ui(ui, |ui| {
                    if ui.selectable_label(false, tr("Automatic: the app in front")).clicked() {
                        crate::live_frames::pin_target(None);
                    }
                    for a in &apps {
                        if ui
                            .selectable_label(
                                target.as_ref().is_some_and(|t| t.0 == a.pid),
                                format!("{} ({}) · {:.0} FPS", a.name, a.pid, a.fps),
                            )
                            .clicked()
                        {
                            crate::live_frames::pin_target(Some(a.pid));
                        }
                    }
                });
            });
            if cpuzeez_core::frames::exclusive_fullscreen(&crate::live_frames::present_mode()) {
                widgets::pill(
                    ui,
                    &format!(
                        "{} {}",
                        icon::WARNING,
                        tr(
                            "This game is in exclusive fullscreen: switch it to borderless window to see the overlay. Capture still works."
                        )
                    ),
                    Tone::Warning,
                );
            }
            let v = |id: &str| series.iter().find(|s| s.id == id).map(|s| s.current);
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                for (id, label, tip) in [
                    ("frames/fps", "FPS", "Frames per second over the last second."),
                    ("frames/low1", "1% low", "How smooth the worst moments feel: the 99th-percentile frame time as FPS, last 30 s. Higher is better."),
                    ("frames/low01", "0.1% low", "The rare worst hitches: the 99.9th-percentile frame time as FPS, last 30 s."),
                    ("frames/gpu_busy", "GPU Busy", "How long the GPU actually worked on a frame. Close to the frame time: the GPU limits the frame rate."),
                ] {
                    let text = v(id).map_or_else(|| "–".to_string(), |x| if id == "frames/gpu_busy" { format!("{x:.1} ms") } else { format!("{x:.0}") });
                    widgets::card(ui, "", tr(label), |ui| {
                        ui.label(RichText::new(text).size(26.0).strong().color(p.text)).on_hover_text(tr(tip));
                    });
                }
            });
            if let Some(ms) = series.iter().find(|s| s.id == "frames/ms") {
                let now = ms.points.last().map_or(0.0, |p| p.0);
                ui.label(RichText::new(tr("Frame time")).small().color(p.text_secondary));
                super::plot::chart(
                    ui,
                    &[super::plot::Line { label: tr("Frame time"), color: p.accent, points: &ms.points }],
                    now,
                    60.0,
                    120.0,
                    &|v| format!("{v:.1} ms"),
                );
            }
        });
        self.capture_card(ui, mon);
        let o = mon.settings.overlay.clone();
        widgets::card(ui, icon::MONITOR, tr("Overlay"), |ui| {
            ui.label(
                RichText::new(if o.shown {
                    tr("The overlay is showing. It never takes focus or clicks; hide it here or with its hotkey.")
                } else {
                    tr("The overlay is hidden.")
                })
                .color(theme::palette(ui).text_secondary),
            );
            let tel = mon.tel();
            let lease = super::layout_view::prefixes(&o.layout.widgets);
            tel.want("live-tab", &lease.iter().map(String::as_str).collect::<Vec<_>>());
            let series = tel.series_with(|_| false);
            ui.add_space(6.0);
            super::layout_view::show(ui, &o.layout, &series, &|id| tel.points(id));
        });
    }

    fn first_use(&mut self, ui: &mut Ui, mon: &mut Monitor) {
        let p = theme::palette(ui);
        let tjmax = mon.tjmax();
        let steps = [tr("Pick a style"), tr("Confirm the hotkey"), tr("Try it now")];
        ui.horizontal(|ui| {
            for (i, s) in steps.iter().enumerate() {
                let tone = if i as u8 == self.first_step { Tone::Accent } else { Tone::Neutral };
                widgets::pill(ui, &format!("{}. {s}", i + 1), tone);
            }
        });
        ui.add_space(8.0);
        match self.first_step {
            0 => {
                let current = mon.settings.overlay.style.clone();
                for l in presets::live::all(tjmax) {
                    let on = current.as_deref() == Some(l.name.as_str());
                    let title = tr(&l.name).to_string();
                    widgets::card(ui, if on { icon::CHECK_CIRCLE } else { icon::CIRCLE }, &title, |ui| {
                        let tel = mon.tel();
                        let lease = super::layout_view::prefixes(&l.widgets);
                        tel.want("live-styles", &lease.iter().map(String::as_str).collect::<Vec<_>>());
                        let series = tel.series_with(|_| false);
                        super::layout_view::show(ui, &l, &series, &|id| tel.points(id));
                        if ui.button(tr("Use this style")).clicked() {
                            let name = l.name.clone();
                            mon.set_setting(move |s| {
                                s.overlay.layout = l.clone();
                                s.overlay.style = Some(name.clone());
                            });
                            self.first_step = 1;
                        }
                    });
                }
            }
            1 => {
                ui.label(RichText::new(tr("Show or hide the overlay with this shortcut, from any app:")).color(p.text));
                let keys = mon.settings.overlay.keys(HotkeyAction::ToggleOverlay).to_string();
                ui.label(RichText::new(keys).monospace().strong().size(18.0));
                if let Some((_, why)) = self.hotkeys.problems.iter().find(|x| x.0 == HotkeyAction::ToggleOverlay) {
                    ui.label(RichText::new(format!("{} {why}", icon::WARNING)).color(p.warning));
                }
                ui.horizontal(|ui| {
                    if ui.button(tr("Back")).clicked() {
                        self.first_step = 0;
                    }
                    if widgets::primary_button(ui, tr("OK")).clicked() {
                        self.first_step = 2;
                    }
                });
            }
            _ => {
                ui.label(
                    RichText::new(tr(
                        "The overlay appears over your desktop now, so you see it before starting a game.",
                    ))
                    .color(p.text),
                );
                ui.horizontal(|ui| {
                    if ui.button(tr("Back")).clicked() {
                        self.first_step = 1;
                    }
                    if widgets::primary_button(ui, tr("Try it now")).clicked() {
                        mon.set_setting(|s| {
                            s.overlay.shown = true;
                            s.overlay.first_use_done = true;
                        });
                    }
                });
            }
        }
    }

    fn overlay_tab(&mut self, ui: &mut Ui, mon: &mut Monitor) {
        let p = theme::palette(ui);
        let tjmax = mon.tjmax();
        let mut o = mon.settings.overlay.clone();
        widgets::card(ui, icon::STACK, tr("Style"), |ui| {
            ui.horizontal_wrapped(|ui| {
                for l in presets::live::all(tjmax) {
                    if ui.selectable_label(o.style.as_deref() == Some(l.name.as_str()), tr(&l.name)).clicked() {
                        o.style = Some(l.name.clone());
                        o.layout = l;
                    }
                }
                if ui.selectable_label(self.customize, format!("{} {}", icon::PENCIL_SIMPLE, tr("Customize"))).clicked()
                {
                    self.customize = !self.customize;
                }
            });
            if self.customize {
                let before = o.layout.clone();
                egui::Frame::group(ui.style())
                    .show(ui, |ui| super::layout_editor::editor(ui, "overlay", &mut o.layout, &mut self.import, tjmax));
                if o.layout != before {
                    o.style = None;
                }
            }
        });
        widgets::card(ui, icon::ARROWS_OUT_CARDINAL, tr("Position"), |ui| {
            egui::Grid::new("overlay-anchors").spacing([4.0, 4.0]).show(ui, |ui| {
                for (i, a) in Anchor::ALL.into_iter().enumerate() {
                    if ui.selectable_label(o.anchor == a, tr(a.label())).clicked() {
                        o.anchor = a;
                        o.offset = [0.0, 0.0];
                    }
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Offset"));
                ui.add(egui::DragValue::new(&mut o.offset[0]).range(-4000.0..=4000.0).suffix(" px"));
                ui.add(egui::DragValue::new(&mut o.offset[1]).range(-4000.0..=4000.0).suffix(" px"));
            });
            ui.horizontal(|ui| {
                let label = if self.edit_mode { tr("Done") } else { tr("Move with the mouse") };
                if ui.add_enabled(o.shown, egui::Button::new(format!("{} {label}", icon::HAND_GRABBING))).clicked() {
                    self.edit_mode = !self.edit_mode;
                }
                if self.edit_mode {
                    ui.label(
                        RichText::new(tr("Drag the overlay, then press Esc or Done.")).small().color(p.text_muted),
                    );
                }
            });
        });
        self.hotkeys_card(ui, &mut o);
        profiles_card(ui, &mut o);
        widgets::card(ui, icon::GEAR_SIX, tr("Behaviour"), |ui| {
            if cfg!(windows) {
                ui.checkbox(&mut o.hide_from_capture, tr("Hide from screenshots, recordings and streams"));
            }
            ui.horizontal(|ui| {
                ui.label(tr("Text refresh"));
                ui.add(egui::DragValue::new(&mut o.text_hz).range(1.0..=30.0).suffix(" Hz"));
                ui.label(tr("Graph refresh"));
                ui.add(egui::DragValue::new(&mut o.graph_hz).range(1.0..=60.0).suffix(" Hz"));
            });
            for note in limits() {
                ui.label(RichText::new(note).small().color(p.text_muted));
            }
        });
        if o != mon.settings.overlay {
            mon.set_setting(move |s| s.overlay = o.clone());
        }
    }
}

/// `frames.csv` (PresentMon columns, opens in CapFrameX) and `session.json`.
pub fn write_session(dir: &std::path::Path, frames: &[cpuzeez_core::frames::Frame], s: &Session) {
    use std::io::Write;
    if let Ok(f) = std::fs::File::create(dir.join("frames.csv")) {
        let mut w = std::io::BufWriter::new(f);
        let _ = writeln!(w, "{}", cpuzeez_core::frames::FRAMES_CSV_HEADER);
        for fr in frames {
            let _ = writeln!(w, "{}", cpuzeez_core::frames::frame_csv_line(fr));
        }
    }
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(dir.join("session.json"), json);
    }
}

/// Per-game profiles (zeez-live §5.4, S2b-11) and anti-cheat safe mode (§7, S2b-12).
fn profiles_card(ui: &mut Ui, o: &mut model::Overlay) {
    let p = theme::palette(ui);
    widgets::card(ui, icon::GAME_CONTROLLER, tr("Per-game profiles"), |ui| {
        ui.label(RichText::new(tr("A game's own style and place, and what happens when it starts. Everything here is off until you turn it on.")).small().color(p.text_secondary));
        let mut remove = None;
        let (layout, anchor, offset) = (o.layout.clone(), o.anchor, o.offset);
        for (i, pr) in o.profiles.iter_mut().enumerate() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut pr.exe).desired_width(180.0).hint_text("game.exe"));
                    let own = pr.layout.is_some();
                    let label = if own { tr("Own style: update to the current one") } else { tr("Use the current style and place") };
                    if ui.button(label).clicked() {
                        pr.layout = Some(layout.clone());
                        pr.anchor = Some(anchor);
                        pr.offset = Some(offset);
                    }
                    if own && ui.button(tr("Use the overlay's")).clicked() {
                        pr.layout = None;
                        pr.anchor = None;
                        pr.offset = None;
                    }
                    if ui.small_button(icon::TRASH).clicked() {
                        remove = Some(i);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut pr.auto_show, tr("Show the overlay when it starts"));
                    ui.checkbox(&mut pr.auto_capture, tr("Capture while it runs"));
                    ui.checkbox(&mut pr.safe_mode, tr("Anti-cheat safe mode"))
                        .on_hover_text(tr("While this game runs, CpuZeeZ reads nothing through the PawnIO driver: temperatures and clocks that need it go blank, GPU and OS readings stay. The driver stays loaded; if an anti-cheat refuses the driver itself, uninstall PawnIO."));
                });
                if pr.auto_capture {
                    ui.label(RichText::new(tr("Records every session of this game; ● REC shows while it does.")).small().color(p.warning));
                }
            });
        }
        if let Some(i) = remove {
            o.profiles.remove(i);
        }
        ui.horizontal(|ui| {
            if ui.button(format!("{} {}", icon::PLUS, tr("Add a game"))).clicked() {
                o.profiles.push(model::Profile::default());
            }
            if let Some((_, game)) = crate::live_frames::target()
                && model::profile_for(&o.profiles, &game).is_none()
                && ui.button(format!("{} {}", icon::PLUS, super::i18n::trf("Add {}", &[&game]))).clicked()
            {
                o.profiles.push(model::Profile { exe: game, ..Default::default() });
            }
        });
    });
}

/// What the overlay can't do on this OS (zeez-live §8), said up front.
fn limits() -> Vec<&'static str> {
    let mut v = Vec::new();
    if cfg!(windows) {
        v.push(tr(
            "Games in true exclusive fullscreen draw over every window: switch them to borderless to see the overlay.",
        ));
    }
    if cfg!(target_os = "linux") {
        v.push(tr("On Wayland, apps can't place themselves above others: there the overlay may stay behind games."));
    }
    if cfg!(target_os = "macos") {
        v.push(tr("Over fullscreen apps (their own Space) the overlay isn't shown yet."));
    }
    v
}

pub const PT_BR: &[(&str, &str)] = &[
    ("Per-game profiles", "Perfis por jogo"),
    (
        "A game's own style and place, and what happens when it starts. Everything here is off until you turn it on.",
        "O estilo e a posição de cada jogo, e o que acontece quando ele abre. Tudo aqui fica desligado até você ligar.",
    ),
    ("Own style: update to the current one", "Estilo próprio: atualizar para o atual"),
    ("Use the current style and place", "Usar o estilo e a posição atuais"),
    ("Use the overlay's", "Usar o do overlay"),
    ("Show the overlay when it starts", "Mostrar o overlay quando ele abrir"),
    ("Capture while it runs", "Capturar enquanto ele roda"),
    ("Anti-cheat safe mode", "Modo seguro para anti-cheat"),
    (
        "While this game runs, CpuZeeZ reads nothing through the PawnIO driver: temperatures and clocks that need it go blank, GPU and OS readings stay. The driver stays loaded; if an anti-cheat refuses the driver itself, uninstall PawnIO.",
        "Enquanto este jogo roda, o CpuZeeZ não lê nada pelo driver PawnIO: temperaturas e clocks que dependem dele ficam em branco; leituras da GPU e do sistema continuam. O driver continua carregado; se um anti-cheat recusar o próprio driver, desinstale o PawnIO.",
    ),
    (
        "Records every session of this game; ● REC shows while it does.",
        "Grava todas as sessões deste jogo; ● REC aparece enquanto grava.",
    ),
    ("Add a game", "Adicionar um jogo"),
    ("Add {}", "Adicionar {}"),
    ("No data folder to save the session in.", "Sem pasta de dados para salvar a sessão."),
    ("Capture", "Captura"),
    ("Starting…", "Iniciando…"),
    ("Stop", "Parar"),
    ("Marker", "Marcador"),
    ("Start capture", "Iniciar captura"),
    ("Timed capture", "Captura cronometrada"),
    ("Capture failed: {}", "A captura falhou: {}"),
    (
        "Timed captures count down 3 s first. {} starts or stops a capture from the game. Nothing is recorded unless you start it.",
        "Capturas cronometradas contam 3 s antes. {} inicia ou para a captura de dentro do jogo. Nada é gravado sem você iniciar.",
    ),
    ("Game", "Jogo"),
    ("Measuring", "Medindo"),
    ("nothing yet", "nada ainda"),
    ("Automatic: the app in front", "Automático: o app em primeiro plano"),
    (
        "This game is in exclusive fullscreen: switch it to borderless window to see the overlay. Capture still works.",
        "Este jogo está em tela cheia exclusiva: mude para janela sem bordas para ver o overlay. A captura continua funcionando.",
    ),
    ("Frames per second over the last second.", "Quadros por segundo no último segundo."),
    (
        "How smooth the worst moments feel: the 99th-percentile frame time as FPS, last 30 s. Higher is better.",
        "Quão suaves são os piores momentos: o percentil 99 do tempo de quadro em FPS, últimos 30 s. Maior é melhor.",
    ),
    (
        "The rare worst hitches: the 99.9th-percentile frame time as FPS, last 30 s.",
        "Os raros piores engasgos: o percentil 99,9 do tempo de quadro em FPS, últimos 30 s.",
    ),
    (
        "How long the GPU actually worked on a frame. Close to the frame time: the GPU limits the frame rate.",
        "Quanto tempo a GPU realmente trabalhou num quadro. Perto do tempo de quadro: a GPU limita o FPS.",
    ),
    ("Frame timing on (PresentMon)", "Medição de quadros ligada (PresentMon)"),
    ("Starting frame timing…", "Iniciando a medição de quadros…"),
    ("Frame timing needs CpuZeeZ to run as administrator.", "A medição de quadros exige o CpuZeeZ como administrador."),
    (
        "The PresentMon sidecar isn't next to CpuZeeZ: reinstall to get FPS.",
        "O PresentMon não está junto do CpuZeeZ: reinstale para ter FPS.",
    ),
    (
        "PresentMon next to CpuZeeZ isn't the version we ship (SHA-256 differs), so it was not run.",
        "O PresentMon junto do CpuZeeZ não é a versão que distribuímos (SHA-256 diferente), então não foi executado.",
    ),
    ("Frame timing stopped: {}", "A medição de quadros parou: {}"),
    ("FPS on Linux comes from MangoHud's logs (below).", "No Linux, o FPS vem dos logs do MangoHud (abaixo)."),
    ("FPS on macOS is experimental (Metal HUD).", "No macOS, o FPS é experimental (Metal HUD)."),
    ("Restart as administrator", "Reiniciar como administrador"),
    (
        "Frames are read from the graphics kernel's events: nothing is injected into the game.",
        "Os quadros vêm dos eventos do kernel gráfico: nada é injetado no jogo.",
    ),
    ("Same shortcut as another action", "Mesmo atalho de outra ação"),
    ("Taken by another app", "Usado por outro app"),
    ("Not a shortcut CpuZeeZ understands", "Atalho não reconhecido pelo CpuZeeZ"),
    ("Hotkeys", "Atalhos"),
    ("off", "desligado"),
    ("Default", "Padrão"),
    ("Empty turns a shortcut off. Example: Ctrl+Alt+F9.", "Vazio desliga o atalho. Exemplo: Ctrl+Alt+F9."),
    (
        "On Wayland, shortcuts work only while CpuZeeZ has focus; use the tray menu instead.",
        "No Wayland, os atalhos só funcionam com o CpuZeeZ em foco; use o menu da bandeja.",
    ),
    ("Show/hide overlay", "Mostrar/ocultar overlay"),
    ("Next style", "Próximo estilo"),
    ("Start/stop capture", "Iniciar/parar captura"),
    ("Marker during capture", "Marcador durante a captura"),
    ("Overlay edit mode", "Modo de edição do overlay"),
    ("Minimal", "Mínimo"),
    ("Benchmark", "Benchmark"),
    ("Overclocker", "Overclocker"),
    ("Frames", "Quadros"),
    ("Frame time", "Tempo de quadro"),
    ("Busiest core", "Núcleo mais ocupado"),
    ("GPU Busy", "GPU ocupada"),
    ("1% low", "1% mínimo"),
    ("0.1% low", "0,1% mínimo"),
    ("GPU clock", "Clock da GPU"),
    ("VID", "VID"),
    ("GPU power", "Energia da GPU"),
    ("RAM", "RAM"),
    ("ZeeZ Live", "ZeeZ Live"),
    (
        "Overlay, capture and reports while you play or work.",
        "Overlay, captura e relatórios enquanto você joga ou trabalha.",
    ),
    ("Show overlay", "Mostrar overlay"),
    ("Hide overlay", "Ocultar overlay"),
    ("LIVE", "AO VIVO"),
    ("Live", "Ao vivo"),
    ("Overlay", "Overlay"),
    ("Sessions", "Sessões"),
    ("No sessions yet", "Nenhuma sessão ainda"),
    (
        "Captured sessions and their reports will be listed here.",
        "As sessões capturadas e seus relatórios aparecem aqui.",
    ),
    (
        "The overlay is showing. It never takes focus or clicks; hide it here or with its hotkey.",
        "O overlay está visível. Ele nunca pega o foco nem os cliques; oculte aqui ou pelo atalho.",
    ),
    ("The overlay is hidden.", "O overlay está oculto."),
    ("Pick a style", "Escolha um estilo"),
    ("Confirm the hotkey", "Confirme o atalho"),
    ("Try it now", "Experimente agora"),
    ("Use this style", "Usar este estilo"),
    (
        "Show or hide the overlay with this shortcut, from any app:",
        "Mostre ou oculte o overlay com este atalho, de qualquer app:",
    ),
    ("Back", "Voltar"),
    ("OK", "OK"),
    (
        "The overlay appears over your desktop now, so you see it before starting a game.",
        "O overlay aparece agora sobre a área de trabalho, para você vê-lo antes de abrir um jogo.",
    ),
    ("Style", "Estilo"),
    ("Customize", "Personalizar"),
    ("Position", "Posição"),
    ("Top left", "Superior esquerdo"),
    ("Top", "Superior"),
    ("Top right", "Superior direito"),
    ("Left", "Esquerda"),
    ("Center", "Centro"),
    ("Right", "Direita"),
    ("Bottom left", "Inferior esquerdo"),
    ("Bottom", "Inferior"),
    ("Bottom right", "Inferior direito"),
    ("Offset", "Deslocamento"),
    ("Move with the mouse", "Mover com o mouse"),
    ("Drag the overlay, then press Esc or Done.", "Arraste o overlay e depois pressione Esc ou Concluir."),
    ("Behaviour", "Comportamento"),
    ("Hide from screenshots, recordings and streams", "Ocultar de capturas de tela, gravações e transmissões"),
    ("Text refresh", "Atualização do texto"),
    ("Graph refresh", "Atualização dos gráficos"),
    (
        "Games in true exclusive fullscreen draw over every window: switch them to borderless to see the overlay.",
        "Jogos em tela cheia exclusiva de verdade desenham sobre qualquer janela: mude para sem bordas para ver o overlay.",
    ),
    (
        "On Wayland, apps can't place themselves above others: there the overlay may stay behind games.",
        "No Wayland, apps não podem se colocar acima de outros: lá o overlay pode ficar atrás dos jogos.",
    ),
    (
        "Over fullscreen apps (their own Space) the overlay isn't shown yet.",
        "Sobre apps em tela cheia (no próprio Space) o overlay ainda não aparece.",
    ),
];

pub const ES: &[(&str, &str)] = &[
    ("Per-game profiles", "Perfiles por juego"),
    (
        "A game's own style and place, and what happens when it starts. Everything here is off until you turn it on.",
        "El estilo y la posición de cada juego, y qué pasa cuando se abre. Todo aquí está apagado hasta que lo actives.",
    ),
    ("Own style: update to the current one", "Estilo propio: actualizar al actual"),
    ("Use the current style and place", "Usar el estilo y la posición actuales"),
    ("Use the overlay's", "Usar el del overlay"),
    ("Show the overlay when it starts", "Mostrar el overlay cuando se abra"),
    ("Capture while it runs", "Capturar mientras se ejecuta"),
    ("Anti-cheat safe mode", "Modo seguro para anti-trampas"),
    (
        "While this game runs, CpuZeeZ reads nothing through the PawnIO driver: temperatures and clocks that need it go blank, GPU and OS readings stay. The driver stays loaded; if an anti-cheat refuses the driver itself, uninstall PawnIO.",
        "Mientras este juego se ejecuta, CpuZeeZ no lee nada a través del controlador PawnIO: las temperaturas y relojes que lo necesitan quedan en blanco; las lecturas de la GPU y del sistema siguen. El controlador sigue cargado; si un anti-trampas rechaza el controlador, desinstala PawnIO.",
    ),
    (
        "Records every session of this game; ● REC shows while it does.",
        "Graba cada sesión de este juego; ● REC se muestra mientras graba.",
    ),
    ("Add a game", "Añadir un juego"),
    ("Add {}", "Añadir {}"),
    ("No data folder to save the session in.", "No hay carpeta de datos para guardar la sesión."),
    ("Capture", "Captura"),
    ("Starting…", "Iniciando…"),
    ("Stop", "Detener"),
    ("Marker", "Marcador"),
    ("Start capture", "Iniciar captura"),
    ("Timed capture", "Captura cronometrada"),
    ("Capture failed: {}", "La captura falló: {}"),
    (
        "Timed captures count down 3 s first. {} starts or stops a capture from the game. Nothing is recorded unless you start it.",
        "Las capturas cronometradas cuentan 3 s antes. {} inicia o detiene la captura desde el juego. No se graba nada si no lo inicias.",
    ),
    ("Game", "Juego"),
    ("Measuring", "Midiendo"),
    ("nothing yet", "nada aún"),
    ("Automatic: the app in front", "Automático: la app en primer plano"),
    (
        "This game is in exclusive fullscreen: switch it to borderless window to see the overlay. Capture still works.",
        "Este juego está en pantalla completa exclusiva: cámbialo a ventana sin bordes para ver el overlay. La captura sigue funcionando.",
    ),
    ("Frames per second over the last second.", "Fotogramas por segundo en el último segundo."),
    (
        "How smooth the worst moments feel: the 99th-percentile frame time as FPS, last 30 s. Higher is better.",
        "Qué tan fluidos son los peores momentos: el percentil 99 del tiempo de fotograma en FPS, últimos 30 s. Más alto es mejor.",
    ),
    (
        "The rare worst hitches: the 99.9th-percentile frame time as FPS, last 30 s.",
        "Los raros peores tirones: el percentil 99,9 del tiempo de fotograma en FPS, últimos 30 s.",
    ),
    (
        "How long the GPU actually worked on a frame. Close to the frame time: the GPU limits the frame rate.",
        "Cuánto tiempo trabajó realmente la GPU en un fotograma. Cerca del tiempo de fotograma: la GPU limita los FPS.",
    ),
    ("Frame timing on (PresentMon)", "Medición de fotogramas activa (PresentMon)"),
    ("Starting frame timing…", "Iniciando la medición de fotogramas…"),
    (
        "Frame timing needs CpuZeeZ to run as administrator.",
        "La medición de fotogramas requiere CpuZeeZ como administrador.",
    ),
    (
        "The PresentMon sidecar isn't next to CpuZeeZ: reinstall to get FPS.",
        "PresentMon no está junto a CpuZeeZ: reinstala para tener FPS.",
    ),
    (
        "PresentMon next to CpuZeeZ isn't the version we ship (SHA-256 differs), so it was not run.",
        "El PresentMon junto a CpuZeeZ no es la versión que distribuimos (SHA-256 distinto), así que no se ejecutó.",
    ),
    ("Frame timing stopped: {}", "La medición de fotogramas se detuvo: {}"),
    (
        "FPS on Linux comes from MangoHud's logs (below).",
        "En Linux, los FPS vienen de los registros de MangoHud (abajo).",
    ),
    ("FPS on macOS is experimental (Metal HUD).", "En macOS, los FPS son experimentales (Metal HUD)."),
    ("Restart as administrator", "Reiniciar como administrador"),
    (
        "Frames are read from the graphics kernel's events: nothing is injected into the game.",
        "Los fotogramas vienen de los eventos del kernel gráfico: nada se inyecta en el juego.",
    ),
    ("Same shortcut as another action", "Mismo atajo que otra acción"),
    ("Taken by another app", "Usado por otra app"),
    ("Not a shortcut CpuZeeZ understands", "Atajo no reconocido por CpuZeeZ"),
    ("Hotkeys", "Atajos"),
    ("off", "desactivado"),
    ("Default", "Predeterminado"),
    ("Empty turns a shortcut off. Example: Ctrl+Alt+F9.", "Vacío desactiva el atajo. Ejemplo: Ctrl+Alt+F9."),
    (
        "On Wayland, shortcuts work only while CpuZeeZ has focus; use the tray menu instead.",
        "En Wayland, los atajos solo funcionan con CpuZeeZ en foco; usa el menú de la bandeja.",
    ),
    ("Show/hide overlay", "Mostrar/ocultar overlay"),
    ("Next style", "Siguiente estilo"),
    ("Start/stop capture", "Iniciar/detener captura"),
    ("Marker during capture", "Marcador durante la captura"),
    ("Overlay edit mode", "Modo de edición del overlay"),
    ("Minimal", "Mínimo"),
    ("Benchmark", "Benchmark"),
    ("Overclocker", "Overclocker"),
    ("Frames", "Fotogramas"),
    ("Frame time", "Tiempo de fotograma"),
    ("Busiest core", "Núcleo más ocupado"),
    ("GPU Busy", "GPU ocupada"),
    ("1% low", "1% mínimo"),
    ("0.1% low", "0,1% mínimo"),
    ("GPU clock", "Reloj de GPU"),
    ("VID", "VID"),
    ("GPU power", "Potencia de GPU"),
    ("RAM", "RAM"),
    ("ZeeZ Live", "ZeeZ Live"),
    ("Overlay, capture and reports while you play or work.", "Overlay, captura e informes mientras juegas o trabajas."),
    ("Show overlay", "Mostrar overlay"),
    ("Hide overlay", "Ocultar overlay"),
    ("LIVE", "EN VIVO"),
    ("Live", "En vivo"),
    ("Overlay", "Overlay"),
    ("Sessions", "Sesiones"),
    ("No sessions yet", "Aún no hay sesiones"),
    (
        "Captured sessions and their reports will be listed here.",
        "Las sesiones capturadas y sus informes aparecen aquí.",
    ),
    (
        "The overlay is showing. It never takes focus or clicks; hide it here or with its hotkey.",
        "El overlay está visible. Nunca toma el foco ni los clics; ocúltalo aquí o con su atajo.",
    ),
    ("The overlay is hidden.", "El overlay está oculto."),
    ("Pick a style", "Elige un estilo"),
    ("Confirm the hotkey", "Confirma el atajo"),
    ("Try it now", "Pruébalo ahora"),
    ("Use this style", "Usar este estilo"),
    (
        "Show or hide the overlay with this shortcut, from any app:",
        "Muestra u oculta el overlay con este atajo, desde cualquier app:",
    ),
    ("Back", "Atrás"),
    ("OK", "Aceptar"),
    (
        "The overlay appears over your desktop now, so you see it before starting a game.",
        "El overlay aparece ahora sobre el escritorio, para que lo veas antes de abrir un juego.",
    ),
    ("Style", "Estilo"),
    ("Customize", "Personalizar"),
    ("Position", "Posición"),
    ("Top left", "Arriba a la izquierda"),
    ("Top", "Arriba"),
    ("Top right", "Arriba a la derecha"),
    ("Left", "Izquierda"),
    ("Center", "Centro"),
    ("Right", "Derecha"),
    ("Bottom left", "Abajo a la izquierda"),
    ("Bottom", "Abajo"),
    ("Bottom right", "Abajo a la derecha"),
    ("Offset", "Desplazamiento"),
    ("Move with the mouse", "Mover con el ratón"),
    ("Drag the overlay, then press Esc or Done.", "Arrastra el overlay y luego pulsa Esc o Listo."),
    ("Behaviour", "Comportamiento"),
    ("Hide from screenshots, recordings and streams", "Ocultar de capturas, grabaciones y transmisiones"),
    ("Text refresh", "Actualización del texto"),
    ("Graph refresh", "Actualización de gráficos"),
    (
        "Games in true exclusive fullscreen draw over every window: switch them to borderless to see the overlay.",
        "Los juegos en pantalla completa exclusiva real dibujan sobre cualquier ventana: cámbialos a sin bordes para ver el overlay.",
    ),
    (
        "On Wayland, apps can't place themselves above others: there the overlay may stay behind games.",
        "En Wayland, las apps no pueden colocarse sobre otras: ahí el overlay puede quedar detrás de los juegos.",
    ),
    (
        "Over fullscreen apps (their own Space) the overlay isn't shown yet.",
        "Sobre apps a pantalla completa (en su propio Space) el overlay aún no se muestra.",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// What a capture writes reads back: frames through the PresentMon parser
    /// (so CapFrameX reads it too), the session through serde.
    #[test]
    fn a_written_session_reads_back() {
        let dir = std::env::temp_dir().join(format!("cz-session-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let frames: Vec<cpuzeez_core::frames::Frame> = (0..100)
            .map(|i| cpuzeez_core::frames::Frame {
                t_s: i as f64 * 0.01,
                ms: 10.0,
                app: "game.exe".into(),
                pid: 7,
                gpu_busy_ms: Some(8.0),
                ..Default::default()
            })
            .collect();
        let s = Session {
            game: "game.exe".into(),
            summary: cpuzeez_core::frames::summarize(&frames, Default::default()),
            markers: vec![Marker { t_s: 0.5, label: "Marker 1".into() }],
            ..Default::default()
        };
        write_session(&dir, &frames, &s);
        let back =
            cpuzeez_core::frames::parse_presentmon(&std::fs::read_to_string(dir.join("frames.csv")).unwrap()).unwrap();
        assert_eq!(back.len(), 100);
        assert_eq!((back[99].ms, back[99].gpu_busy_ms, back[99].pid), (10.0, Some(8.0), 7));
        let js: Session = serde_json::from_str(&std::fs::read_to_string(dir.join("session.json")).unwrap()).unwrap();
        assert_eq!(js, s);
        assert!((js.summary.avg_fps - 100.0).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(dir);
    }
}

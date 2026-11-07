//! Modern layout: a sidebar with pages (Overview, Processor, Mainboard, Memory,
//! Graphics, Devices, Sensors, Benchmarks, Tools, Settings, About) built from
//! the design system in `theme`/`widgets`. Every page reads the same report
//! tree as `-txt`. F5 BMP, F6 clipboard, F7 .cvf (our JSON). UI text goes
//! through `i18n::tr` (English keys; tables at the end of each file).
//!
//! No `unsafe` here (S0-E06): OS calls live in `cpuzeez_platform`.
#![forbid(unsafe_code)]

#[cfg(test)]
mod a11y;
mod access_ui;
mod bench_tab;
mod bmp;
mod bottleneck_ui;
pub mod detail;
mod devices;
pub mod i18n;
pub mod overview;
mod pages;
mod palette;
pub mod pins;
mod screens;
mod settings_ui;
mod shots;
#[cfg(feature = "soak")]
mod soak;
mod update_ui;
// ---- monitoring (sensors / stress / tray) ----
#[cfg(test)]
mod bench_views_test;
mod benchkit_view;
mod cache;
mod cpusuite_view;
mod diskbench_view;
mod drivehealth_view;
mod gpubench_view;
mod hotkeys;
#[cfg(all(unix, not(target_os = "macos")))]
mod hotkeys_portal;
mod latency_view;
mod layout_editor;
pub mod layout_view;
pub mod live;
mod live_sessions;
mod memplus_view;
mod mini;
mod plot;
pub mod sensors;
pub mod stress;
pub mod system;
pub mod system_history;
pub mod system_perf;
pub mod system_services;
pub mod system_startup;
pub mod system_users;
pub mod table;
mod tray;
mod trayicon;
pub mod units;
mod welcome;
// ---- end monitoring ----
pub mod theme;
mod tools;
pub mod widgets;

use crate::Ctx;
use crate::report::{self, Node, Section};
use crate::update::{Installer, Release, Stage};
use eframe::egui::{self, RichText};
use i18n::{tr, trf};
use pages::{mainboard_tab, memory_tab};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use theme::icon;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Page {
    Overview,
    System,
    Live,
    Processor,
    Mainboard,
    Memory,
    Graphics,
    Devices,
    Sensors,
    Benchmarks,
    Tools,
    Settings,
    About,
}

/// (page, icon, English name: translated when shown, used as-is for file names).
const NAV: [(Page, &str, &str); 11] = [
    (Page::Overview, icon::SQUARES_FOUR, "Overview"),
    (Page::System, icon::LIST_BULLETS, "System"),
    (Page::Live, icon::PULSE, "ZeeZ Live"),
    (Page::Processor, icon::CPU, "Processor"),
    (Page::Mainboard, icon::CIRCUITRY, "Mainboard"),
    (Page::Memory, icon::MEMORY, "Memory"),
    (Page::Graphics, icon::GRAPHICS_CARD, "Graphics"),
    (Page::Devices, icon::HARD_DRIVES, "Devices"),
    (Page::Sensors, icon::THERMOMETER, "Sensors"),
    (Page::Benchmarks, icon::GAUGE, "Benchmarks"),
    (Page::Tools, icon::TOOLBOX, "Tools"),
];
const NAV_BOTTOM: [(Page, &str, &str); 2] =
    [(Page::Settings, icon::GEAR_SIX, "Settings"), (Page::About, icon::INFO, "About")];

fn page_name(p: Page) -> &'static str {
    NAV.iter().chain(NAV_BOTTOM.iter()).find(|n| n.0 == p).map_or("Overview", |n| n.2)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BenchView {
    Cpu,
    Memory,
    Latency,
    Gpu,
    Storage,
    Stress,
    History,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ToolView {
    Compare,
    Clocks,
    Timers,
    Reports,
    Drives,
}

enum Shot {
    Clipboard,
    Bmp,
    Png(std::path::PathBuf),
}

/// One `-screenshots` capture.
#[derive(Clone, Copy)]
enum Step {
    Page(Page),
    Bench(BenchView),
    Tool(ToolView),
    System(system::SysTab),
    /// ZeeZ Live › Bottleneck, on a recorded capture.
    Bottleneck,
    /// The About page in Brazilian Portuguese, to show the translations.
    AboutPt,
}

const SHOTS: [(&str, Step); 25] = [
    ("overview", Step::Page(Page::Overview)),
    ("processor", Step::Page(Page::Processor)),
    ("mainboard", Step::Page(Page::Mainboard)),
    ("memory", Step::Page(Page::Memory)),
    ("graphics", Step::Page(Page::Graphics)),
    ("devices", Step::Page(Page::Devices)),
    ("sensors", Step::Page(Page::Sensors)),
    ("bench", Step::Bench(BenchView::Cpu)),
    ("membench", Step::Bench(BenchView::Memory)),
    ("stress", Step::Bench(BenchView::Stress)),
    ("history", Step::Bench(BenchView::History)),
    ("compare", Step::Tool(ToolView::Compare)),
    ("clocks", Step::Tool(ToolView::Clocks)),
    ("system-processes", Step::System(system::SysTab::Processes)),
    ("system-performance", Step::System(system::SysTab::Performance)),
    ("system-history", Step::System(system::SysTab::AppHistory)),
    ("system-startup", Step::System(system::SysTab::Startup)),
    ("system-users", Step::System(system::SysTab::Users)),
    ("system-services", Step::System(system::SysTab::Services)),
    ("live", Step::Page(Page::Live)),
    ("bottleneck", Step::Bottleneck),
    ("settings", Step::Page(Page::Settings)),
    ("about", Step::Page(Page::About)),
    ("about-pt", Step::AboutPt),
    ("overview-light", Step::Page(Page::Overview)),
];

/// `-screenshots=DIR`: visit every tab and tool window, save a PNG of each,
/// then quit. Used for the README and by CI on every OS.
struct AutoShots {
    dir: std::path::PathBuf,
    step: usize,
    wait: u32,
    started: std::time::Instant,
    /// Step whose one-time setup already ran, and when.
    prepared: Option<usize>,
    since: std::time::Instant,
}

struct App {
    ctx: Arc<Ctx>,
    report: Arc<Mutex<Option<Vec<Section>>>>,
    /// Capability banners and "Restart as administrator".
    access: access_ui::Access,
    /// The first-run privacy screen, until the user continues.
    welcome: Option<welcome::Welcome>,
    page: Page,
    /// Reload history.json when the History view is opened.
    history_dirty: bool,
    /// Mini monitor whose editor is open in Settings, and its JSON import box.
    mini_edit: Option<usize>,
    mini_import: String,
    bench_view: BenchView,
    tool_view: ToolView,
    core: usize,
    slot: usize,
    gpu: usize,
    status: String,
    shot: Option<Shot>,
    auto: Option<AutoShots>,
    bench: bench_tab::State,
    devices: devices::State,
    history: tools::History,
    compare: tools::Compare,
    /// Newer release found by the background check (None = none / not yet).
    update: Arc<Mutex<Option<Release>>>,
    update_dismissed: bool,
    installer: Option<Installer>,
    logo: Option<egui::TextureHandle>,
    /// About > Check for updates: Some(result text) once finished.
    update_manual: Arc<Mutex<Option<String>>>,
    /// Last window title sent (● REC, MHz while minimized).
    title: String,
    /// Sensors page, stress monitor, tray and mini monitor (gui/sensors.rs).
    /// Its `settings` is the app's one copy of settings.json.
    mon: sensors::Monitor,
    /// Ctrl+K search (S1-U02).
    palette: palette::Palette,
    overview: overview::State,
    /// Processor › Cores table (sort, columns).
    cores_table: table::State,
    pcie_table: table::State,
    latency: latency_view::State,
    cpusuite: cpusuite_view::State,
    memplus: memplus_view::State,
    drivehealth: drivehealth_view::State,
    gpubench: gpubench_view::State,
    diskbench: diskbench_view::State,
    system: system::State,
    live: live::State,
    #[cfg(feature = "soak")]
    soak: Option<soak::Tour>,
}

pub fn run(ctx: Ctx, shots: Option<std::path::PathBuf>) {
    #[cfg(feature = "soak")]
    crate::soak::start();
    let ctx = Arc::new(ctx);
    let report = Arc::new(Mutex::new(None));
    {
        let (c, r) = (ctx.clone(), report.clone());
        std::thread::spawn(move || *r.lock().unwrap() = Some(crate::collect(&c)));
    }
    // One copy of the settings (the monitor's), edited by the Settings page.
    // Screenshots are reproducible: default settings, English.
    let mut mon = sensors::Monitor::new(ctx.clone());
    mon.pages = true;
    if shots.is_some() {
        mon.settings = Default::default();
        mon.show_mini = false;
    }
    i18n::set(if shots.is_some() { i18n::Lang::En } else { settings_ui::lang_of(&mon.settings) });

    let always_top = mon.settings.always_on_top;
    let start_page =
        NAV.iter().find(|n| n.2.eq_ignore_ascii_case(&mon.settings.default_start_page)).map_or(Page::Overview, |n| n.0);
    // OpenGL (glow) by default: it uses far less memory than wgpu (ADR 0013).
    // CZ_RENDERER=wgpu forces wgpu, e.g. to work around a broken GL driver.
    let opts = |renderer| {
        let mut vp = egui::ViewportBuilder::default()
            .with_title(crate::DISPLAY_NAME)
            .with_icon(app_icon())
            .with_inner_size([1120.0, 760.0])
            .with_min_inner_size([860.0, 560.0]);
        if always_top {
            vp = vp.with_always_on_top();
        }
        eframe::NativeOptions { viewport: vp, renderer, ..Default::default() }
    };
    let theme = mon.settings.theme;
    let app = App {
        ctx,
        report,
        access: Default::default(),
        welcome: (shots.is_none() && mon.settings.privacy_seen < welcome::PRIVACY_VERSION).then(Default::default),
        page: if shots.is_some() { Page::Overview } else { start_page },
        history_dirty: true,
        mini_edit: None,
        mini_import: String::new(),
        bench_view: BenchView::Cpu,
        tool_view: ToolView::Compare,
        core: 0,
        slot: 0,
        gpu: 0,
        status: String::new(),
        shot: None,
        auto: shots.map(|dir| {
            let _ = std::fs::create_dir_all(&dir);
            MASK_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
            let now = std::time::Instant::now();
            AutoShots { dir, step: 0, wait: 0, started: now, prepared: None, since: now }
        }),
        bench: bench_tab::State::default(),
        devices: devices::State::default(),
        history: tools::History::default(),
        compare: tools::Compare::default(),
        update: Arc::new(Mutex::new(None)),
        update_dismissed: false,
        installer: None,
        logo: None,
        update_manual: Arc::new(Mutex::new(None)),
        title: String::new(),
        mon,
        palette: Default::default(),
        overview: Default::default(),
        cores_table: Default::default(),
        pcie_table: Default::default(),
        latency: Default::default(),
        cpusuite: Default::default(),
        memplus: Default::default(),
        drivehealth: Default::default(),
        gpubench: Default::default(),
        diskbench: Default::default(),
        system: Default::default(),
        live: Default::default(),
        #[cfg(feature = "soak")]
        soak: None,
    };
    if app.auto.is_none() {
        crate::data::expire_logs(app.mon.settings.log_retention_days);
        if app.welcome.is_none() {
            app.check_updates_if_allowed();
        }
    }
    let font_size = app.ctx.cfg.text_font_size;
    // Shared so a second attempt can still hand it over: the renderer fails
    // before eframe asks for the app.
    let app = std::rc::Rc::new(std::cell::RefCell::new(Some(app)));
    let run = |renderer| {
        let app = app.clone();
        eframe::run_native(
            crate::DISPLAY_NAME,
            opts(renderer),
            Box::new(move |cc| {
                // Theme text styles + Phosphor icons.
                theme::install(&cc.egui_ctx);
                cc.egui_ctx.set_theme(settings_ui::theme_pref(theme));
                if font_size > 0.0 {
                    cc.egui_ctx.global_style_mut(|s| {
                        for f in s.text_styles.values_mut() {
                            f.size = font_size;
                        }
                    });
                }
                let app = app.borrow_mut().take().ok_or("the app was already started")?;
                Ok(Box::new(app))
            }),
        )
    };
    let wgpu_only = std::env::var("CZ_RENDERER").as_deref() == Ok("wgpu");
    // eframe panics (rather than erring) on some GL failures.
    let attempt = |renderer| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(renderer)))
            .unwrap_or_else(|_| Err(eframe::Error::AppCreation("the renderer panicked".into())))
    };
    let mut r = attempt(if wgpu_only { eframe::Renderer::Wgpu } else { eframe::Renderer::Glow });
    if let Err(e) = &r
        && !wgpu_only
        && app.borrow().is_some()
    {
        // VMs, Remote Desktop and machines without a GPU driver have no usable
        // OpenGL; wgpu gets DX12 (with WARP software rendering), Vulkan or Metal.
        crate::applog::warn(&format!("OpenGL renderer failed ({e}); retrying with wgpu"));
        r = attempt(eframe::Renderer::Wgpu);
    }
    if let Err(e) = r {
        crate::applog::warn(&format!("GUI failed to start: {e}"));
        eprintln!("GUI failed to start ({e}); use -console or -txt=FILE");
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // The ETW session behind the Network column outlives the process otherwise.
        cpuzeez_platform::proc_net::stop();
        crate::live_frames::stop();
        self.system.history.flush();
    }

    /// Transparent, so the mini monitors' rounded, see-through cards show the
    /// desktop behind them (the main window's panels cover it).
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        units::set(self.mon.settings.units);
        let ectx = ui.ctx().clone();
        self.take_pins(&ectx);
        ectx.request_repaint_after(Duration::from_millis(500));
        #[cfg(feature = "soak")]
        self.soak_tour(&ectx);
        if self.auto.is_none() {
            self.system.background(&self.mon.settings);
        }
        live::overlay(
            &mut self.mon,
            &mut self.live,
            &ectx,
            self.auto.is_none(),
            &self.system.sampler,
            self.page == Page::Live,
        );
        self.hotkeys(&ectx);
        self.window_title(&ectx);
        self.access.frame(&ectx);
        if std::mem::take(&mut self.access.restarted) {
            self.mon.quit(&ectx); // the elevated copy is up
        }

        if let Some(w) = &mut self.welcome {
            let allowed = self.ctx.cfg.check_updates;
            let p = theme::palette(ui);
            let done = egui::CentralPanel::default()
                .frame(egui::Frame::default().fill(p.bg).inner_margin(egui::Margin::symmetric(48, 32)))
                .show(ui, |ui| welcome::page(ui, w, allowed))
                .inner;
            if let Some(updates) = done {
                self.mon.set_setting(|s| {
                    s.privacy_seen = welcome::PRIVACY_VERSION;
                    s.update_check = Some(updates);
                });
                self.welcome = None;
                self.check_updates_if_allowed();
            }
            return;
        }
        let report = self.report.lock().unwrap().clone();
        // Tray menu / monitor shortcuts navigate instead of opening windows.
        if std::mem::take(&mut self.mon.show_sensors) {
            self.page = Page::Sensors;
        }
        if std::mem::take(&mut self.mon.show_stress) {
            self.page = Page::Benchmarks;
            self.bench_view = BenchView::Stress;
        }
        let p = theme::palette(ui);
        egui::Panel::left("nav")
            .exact_size(theme::SIDEBAR_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::default().fill(p.sidebar).inner_margin(egui::Margin::symmetric(10, 14)))
            .show(ui, |ui| self.sidebar(ui));
        self.update_banner(ui);
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(p.bg).inner_margin(egui::Margin::symmetric(24, 18)))
            .show(ui, |ui| {
                let Some(r) = report.as_deref() else {
                    ui.centered_and_justified(|ui| {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(tr("Reading hardware…"));
                        });
                    });
                    return;
                };
                // The capability banner reflects this machine; CI screenshots would show the runner's.
                let caps = if self.auto.is_some() { &[][..] } else { access_ui::page_caps(page_name(self.page)) };
                let t0 = std::time::Instant::now();
                if matches!(self.page, Page::Sensors | Page::System) {
                    // Full-height dashboard with its own scrolling table and charts.
                    self.access.banner(ui, &self.ctx, page_name(self.page), caps);
                    self.page_body(ui, r);
                } else {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).id_salt(page_name(self.page)).show(
                        ui,
                        |ui| {
                            self.access.banner(ui, &self.ctx, page_name(self.page), caps);
                            self.page_body(ui, r)
                        },
                    );
                }
                frame_log(page_name(self.page), "page", t0);
            });
        let t0 = std::time::Instant::now();
        self.mon.frame(&ectx);
        frame_log("all", "monitor", t0);
        if let Some(i) = self.mon.edit_mini.take() {
            self.page = Page::Settings;
            self.mini_edit = Some(i);
        }
        self.take_screenshot(&ectx);
        self.auto_shots(&ectx, report.as_deref());
    }
}

// ---- helpers ----

/// `CZ_FRAME_LOG=1`: frames whose part took longer than 4 ms, on stderr
/// (a 60 Hz frame has 16 ms for everything).
fn frame_log(page: &str, part: &str, t0: std::time::Instant) {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    if *ON.get_or_init(|| std::env::var_os("CZ_FRAME_LOG").is_some()) && ms > 4.0 {
        eprintln!("frame {page} {part} {ms:.1} ms");
    }
}

fn value(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).monospace()).wrap());
}

/// Set by `-screenshots`: published images shouldn't carry the machine's
/// serial numbers, MAC addresses or IP addresses.
static MASK_PRIVATE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn shown<'a>(key: &str, v: &'a str) -> &'a str {
    if !MASK_PRIVATE.load(std::sync::atomic::Ordering::Relaxed) || v.is_empty() {
        return v;
    }
    if report::is_private(key) { "••••••••" } else { v }
}

/// A card of label/value rows (labels translated, private values masked),
/// keeping the rows the page's detail level shows; nothing at all if none.
fn boxed(ui: &mut egui::Ui, title: &str, rows: &[(&str, String)]) {
    use cpuzeez_core::detail::{level_of, shows};
    let level = detail::get(ui.ctx());
    let rows: Vec<(&str, String)> =
        rows.iter().filter(|(k, _)| shows(level, level_of(k))).map(|(k, v)| (tr(k), shown(k, v).to_string())).collect();
    if rows.is_empty() {
        return;
    }
    widgets::card(ui, "", tr(title), |ui| widgets::kv_list(ui, &rows));
    ui.add_space(theme::GAP);
}

/// Expert: thread 0's raw CPUID leaves and MSRs, from the report's thread dumps.
fn cpuid_dump(ui: &mut egui::Ui, r: &[Section]) {
    let Some(t0) = sec(r, "Thread dumps").and_then(|s| report::groups(&s.nodes, "CPU Thread").into_iter().next())
    else {
        return;
    };
    for (group, title) in [("CPUID", "CPUID leaves (thread 0)"), ("MSR", "MSRs (thread 0)")] {
        let Some((_, rows)) = report::groups(t0.1, group).into_iter().next() else { continue };
        heading(ui, tr(title));
        widgets::card(ui, "", "", |ui| {
            for (k, v) in report::fields(rows) {
                ui.label(RichText::new(format!("{k}  {}", v.replace('\t', "  "))).monospace());
            }
        });
        ui.add_space(theme::GAP);
    }
}

/// Section label between cards.
fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).text_style(egui::TextStyle::Heading).color(theme::palette(ui).text));
    ui.add_space(4.0);
}

fn sec<'a>(r: &'a [Section], t: &str) -> Option<&'a Section> {
    report::find_section(r, t)
}
fn sget(r: &[Section], s: &str, k: &str) -> String {
    sec(r, s).and_then(|x| x.get(k)).unwrap_or_default().to_string()
}
fn sgi(r: &[Section], s: &str, g: &str, k: &str) -> String {
    sec(r, s).and_then(|x| x.get_in(g, k)).unwrap_or_default().to_string()
}

impl App {
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let p = theme::palette(ui);
        let logo = self.logo_texture(ui);
        ui.horizontal(|ui| {
            ui.image((logo, egui::vec2(28.0, 28.0)));
            ui.label(RichText::new(crate::DISPLAY_NAME).font(egui::FontId::new(17.0, theme::semibold())).color(p.text));
        });
        ui.add_space(14.0);
        ui.spacing_mut().item_spacing.y = 2.0;
        for (pg, ic, name) in NAV {
            if widgets::sidebar_item(ui, ic, tr(name), self.page == pg).clicked() {
                self.page = pg;
            }
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.label(RichText::new(format!("v{}", crate::VERSION)).small().color(p.text_muted));
            if self.mon.recording() {
                let rec =
                    widgets::pill(ui, "● REC", widgets::Tone::Danger).on_hover_text(tr("Recording · click to stop"));
                if rec.interact(egui::Sense::click()).clicked() {
                    self.mon.stop_recording();
                }
            }
            if !self.status.is_empty() {
                ui.add(egui::Label::new(RichText::new(&self.status).small().color(p.text_secondary)).wrap());
            }
            ui.add_space(6.0);
            for (pg, ic, name) in NAV_BOTTOM.iter().rev() {
                if widgets::sidebar_item(ui, ic, tr(name), self.page == *pg).clicked() {
                    self.page = *pg;
                }
            }
        });
    }

    fn logo_texture(&mut self, ui: &egui::Ui) -> egui::TextureId {
        self.logo
            .get_or_insert_with(|| {
                let icon = decode_png(include_bytes!("../../assets/icon-128.png")).unwrap_or_default();
                let img =
                    egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
                ui.ctx().load_texture("logo", img, egui::TextureOptions::LINEAR)
            })
            .id()
    }

    /// Pins asked for by any row last frame (S1-U03) go into the settings.
    fn take_pins(&mut self, ectx: &egui::Context) {
        let mut changed = false;
        for p in pins::take_requests(ectx) {
            let s = &mut self.mon.settings.pins;
            if !s.iter().any(|q| q.card == p.card && q.label == p.label && q.metric == p.metric) {
                s.push(p);
                changed = true;
            }
        }
        if changed && self.auto.is_none() {
            self.mon.settings.save();
        }
        pins::publish(ectx, &self.mon.settings.pins);
    }

    /// A page header with the detail switch; the chosen level is remembered per page.
    fn info_header(&mut self, ui: &mut egui::Ui, title: &str, sub: &str) {
        let key = format!("{:?}", self.page);
        let mut level = self.mon.settings.detail.get(&key).copied().unwrap_or_default();
        let mut changed = false;
        widgets::page_header(ui, title, sub, |ui| changed = detail::switch(ui, &mut level));
        if changed {
            self.mon.settings.detail.insert(key, level);
            if self.auto.is_none() {
                self.mon.settings.save();
            }
        }
        detail::set(ui.ctx(), level);
    }

    fn page_body(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let ctx = self.ctx.clone();
        let c = crate::cpu::get(&ctx);
        // Pages without the switch draw everything (as at Expert); boxed() reads this.
        detail::set(ui.ctx(), cpuzeez_core::detail::Level::Expert);
        match self.page {
            Page::Overview => {
                let navs = overview::page(ui, &self.ctx, r, &mut self.mon, &mut self.overview);
                for n in navs {
                    match n {
                        overview::Nav::Page(p) => self.page = p,
                        overview::Nav::Sensors(filter) => {
                            self.page = Page::Sensors;
                            self.mon.focus(filter);
                        }
                        overview::Nav::Act(a) => self.run_palette(palette::Target::Act(a)),
                        overview::Nav::Bottleneck => {
                            self.page = Page::Live;
                            self.live.tab = live::Tab::Bottleneck;
                        }
                        overview::Nav::Save if self.auto.is_none() => self.mon.settings.save(),
                        overview::Nav::Save => {}
                    }
                }
            }
            Page::System => self.system.page(ui, &self.ctx, &mut self.mon),
            Page::Live => {
                self.live.page(ui, &mut self.mon, &mut self.access);
                if let Some(path) = self.live.sessions.png_request.take() {
                    self.shot = Some(Shot::Png(path));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                }
            }
            Page::Processor => {
                let sub = [c.info.codename.clone(), c.info.package.clone(), c.info.technology.clone()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · ");
                self.info_header(ui, &c.name, &sub);
                self.cpu_tab(ui, r);
                heading(ui, tr("Caches"));
                self.caches_tab(ui);
                self.processor_more(ui);
                if detail::get(ui.ctx()) == cpuzeez_core::detail::Level::Expert {
                    cpuid_dump(ui, r);
                }
            }
            Page::Mainboard => {
                let board =
                    format!("{} {}", sgi(r, "DMI", "DMI Baseboard", "vendor"), sgi(r, "DMI", "DMI Baseboard", "model"));
                self.info_header(ui, tr("Mainboard"), board.trim());
                mainboard_tab(ui, r);
                self.mainboard_more(ui, r);
            }
            Page::Memory => {
                let k = |key: &str| sget(r, "Chipset", key);
                let sub = [k("Memory Type"), k("Memory Size"), k("Channels")]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · ");
                self.info_header(ui, tr("Memory"), &sub);
                memory_tab(ui, r);
                self.memory_more(ui, r);
                heading(ui, "SPD");
                self.spd_tab(ui, r);
            }
            Page::Graphics => {
                self.info_header(ui, tr("Graphics"), "");
                self.graphics_tab(ui, r);
            }
            Page::Devices => self.devices.page(ui, &self.ctx),
            Page::Sensors => sensors::page(ui, &mut self.mon),
            Page::Benchmarks => {
                widgets::page_header(
                    ui,
                    tr("Benchmarks"),
                    tr("Our own workloads: scores aren't comparable with CPU-Z's"),
                    |_| {},
                );
                widgets::segmented(
                    ui,
                    &mut self.bench_view,
                    &[
                        (BenchView::Cpu, icon::CPU, tr("Processor")),
                        (BenchView::Memory, icon::MEMORY, tr("Memory")),
                        (BenchView::Latency, icon::GRID_FOUR, tr("Latency")),
                        (BenchView::Gpu, icon::GRAPHICS_CARD, tr("Graphics")),
                        (BenchView::Storage, icon::HARD_DRIVES, tr("Storage")),
                        (BenchView::Stress, icon::FLAME, tr("Stress test")),
                        (BenchView::History, icon::CLOCK_COUNTER_CLOCKWISE, tr("History")),
                    ],
                );
                match self.bench_view {
                    BenchView::Cpu => {
                        self.bench.cpu_page(ui, &self.ctx, &mut self.mon);
                        ui.add_space(theme::GAP);
                        let t0 = benchkit_view::cpu_temp(&mut self.mon);
                        self.cpusuite.page(ui, &self.ctx, t0);
                    }
                    BenchView::Memory => {
                        let t0 = benchkit_view::cpu_temp(&mut self.mon);
                        self.bench.memory_page(ui, &self.ctx, t0);
                        ui.add_space(theme::GAP);
                        self.memplus.page(ui, &self.ctx, t0);
                    }
                    BenchView::Latency => {
                        let t0 = benchkit_view::cpu_temp(&mut self.mon);
                        self.latency.page(ui, &self.ctx, t0)
                    }
                    BenchView::Gpu => {
                        let t0 = benchkit_view::cpu_temp(&mut self.mon);
                        self.gpubench.page(ui, &self.ctx, t0)
                    }
                    BenchView::Storage => {
                        let t0 = benchkit_view::cpu_temp(&mut self.mon);
                        self.diskbench.page(ui, &self.ctx, t0)
                    }
                    BenchView::Stress => {
                        let gpus = cache::gpu_adapters(&self.ctx).map(|g| g.to_vec()).unwrap_or_default();
                        stress::page(ui, &mut self.mon, true, &gpus)
                    }
                    BenchView::History => {
                        if self.history_dirty {
                            self.history.reload();
                            self.history_dirty = false;
                        }
                        self.history.page(ui)
                    }
                }
                if self.bench_view != BenchView::History {
                    self.history_dirty = true;
                }
            }
            Page::Tools => self.tools_page(ui, r),
            Page::Settings => {
                let s = &self.mon.settings;
                let (tray, sampling) = (s.tray, (s.sample_ms, s.graph_seconds));
                let persist = self.auto.is_none();
                settings_ui::page(ui, &mut self.mon.settings, self.ctx.cfg.check_updates, persist, Some(r));
                if self.mon.settings.tray != tray {
                    let on = self.mon.settings.tray;
                    self.mon.set_tray(on, ui.ctx());
                }
                if (self.mon.settings.sample_ms, self.mon.settings.graph_seconds) != sampling {
                    self.mon.apply_sampling();
                }
                ui.add_space(theme::GAP);
                let tjmax = self.mon.tjmax();
                let mut minis = self.mon.settings.minis.clone();
                widgets::card(ui, icon::MONITOR, tr("Mini monitors"), |ui| {
                    mini::settings_card(ui, &mut minis, &mut self.mini_edit, &mut self.mini_import, tjmax);
                });
                if minis != self.mon.settings.minis {
                    let on = minis.iter().any(|m| m.open);
                    if persist {
                        self.mon.set_setting(move |s| s.minis = minis.clone());
                    } else {
                        self.mon.settings.minis = minis;
                    }
                    self.mon.show_mini = on;
                    if on {
                        self.mon.tel();
                    }
                }
            }
            Page::About => self.about_tab(ui, r),
        }
    }

    fn hotkeys(&mut self, ectx: &egui::Context) {
        let (f5, f6, f7) =
            ectx.input(|i| (i.key_pressed(egui::Key::F5), i.key_pressed(egui::Key::F6), i.key_pressed(egui::Key::F7)));
        if f5 || f6 {
            self.shot = Some(if f6 { Shot::Clipboard } else { Shot::Bmp });
            ectx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        if f7 {
            let r = self.report.lock().unwrap().clone();
            self.save(&r, "cvf");
        }
        if ectx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) && self.welcome.is_none() {
            let r = self.report.lock().unwrap().clone().unwrap_or_default();
            let pages: Vec<(Page, &str)> = NAV.iter().chain(NAV_BOTTOM.iter()).map(|n| (n.0, n.2)).collect();
            self.palette.open(&r, &pages);
        }
        if let Some(t) = self.palette.show(ectx) {
            self.run_palette(t);
        }
    }

    /// What the command palette picked (S1-U02).
    fn run_palette(&mut self, t: palette::Target) {
        use palette::{Act, Target};
        match t {
            Target::Page(p) => self.page = p,
            Target::Act(Act::SaveTxt | Act::SaveHtml) => {
                let r = self.report.lock().unwrap().clone();
                self.save(&r, if t == Target::Act(Act::SaveTxt) { "txt" } else { "html" });
            }
            Target::Act(Act::ToggleCsv) => {
                let tel = self.mon.tel();
                if tel.csv_path().is_some() {
                    tel.stop_csv();
                } else if let Some(p) = sensors::log_path()
                    && let Err(e) = tel.start_csv(&p)
                {
                    self.status = format!("{}: {e}", p.display());
                }
            }
            Target::Act(Act::ToggleMini) => {
                let on = !self.mon.show_mini;
                self.mon.set_mini(on);
            }
            Target::Act(Act::ResetMinMax) => self.mon.tel().reset_stats(),
            Target::Act(Act::Stress) => self.mon.show_stress = true,
        }
    }

    /// The update check, only when the user opted in and cpuzeez.ini allows it (P1).
    fn check_updates_if_allowed(&self) {
        if crate::update::startup_check_allowed(self.ctx.cfg.check_updates, self.mon.settings.update_check) {
            let slot = self.update.clone();
            let channel = self.mon.settings.channel();
            std::thread::spawn(move || *slot.lock().unwrap() = crate::update::check(channel));
        }
    }

    /// Window title: ● REC while recording, and the core clock while
    /// minimized (as CPU-Z does). Sent only when it changes.
    fn window_title(&mut self, ectx: &egui::Context) {
        let mut t = String::new();
        if self.mon.recording() || self.live.capturing() {
            t += "● REC · ";
        }
        if ectx.input(|i| i.viewport().minimized.unwrap_or(false)) {
            let tel = self.mon.tel();
            tel.want("title", &["cpu/clock"]);
            if let Some(m) = crate::telemetry::core_mhz(&tel.snapshot(), 0) {
                t += &format!("{m:.0} MHz - ");
            }
        }
        t += crate::DISPLAY_NAME;
        if t != self.title {
            ectx.send_viewport_cmd(egui::ViewportCommand::Title(t.clone()));
            self.title = t;
        }
    }

    fn save(&mut self, r: &Option<Vec<Section>>, ext: &str) {
        let Some(r) = r else { return };
        let body = match ext {
            "html" => report::to_html(r),
            "cvf" => report::to_json(r),
            _ => report::to_txt(r),
        };
        let name = format!("{}.{ext}", hostname());
        self.status = match std::fs::write(&name, body) {
            Ok(()) => {
                trf("Saved {}", &[&std::fs::canonicalize(&name).map_or(name.clone(), |p| p.display().to_string())])
            }
            Err(e) => format!("{name}: {e}"),
        };
    }
}

fn no_data(ui: &egui::Ui, what: &str, ctx: &Ctx) -> RichText {
    let why = if ctx.hw.name().starts_with("none") {
        "this needs privileged hardware access (see README: PawnIO on Windows, root on Linux)"
    } else {
        "nothing was detected"
    };
    RichText::new(trf("No {} information: {}.", &[&tr(what), &tr(why)])).color(theme::palette(ui).text_muted)
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .or_else(|| crate::os::read_trim("/etc/hostname"))
        .or_else(|| crate::os::cmd("hostname", &[]).map(|s| s.trim().to_string()))
        .unwrap_or_else(|| crate::NAME.into())
}

/// Window/taskbar icon (the same artwork as assets/icon.svg).
fn app_icon() -> egui::IconData {
    decode_png(include_bytes!("../../assets/icon-256.png")).unwrap_or_default()
}

/// RGBA PNG -> pixels (our icons are always 8-bit RGBA).
fn decode_png(png: &[u8]) -> Option<egui::IconData> {
    let mut r = png::Decoder::new(std::io::Cursor::new(png)).read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    (info.color_type == png::ColorType::Rgba).then_some(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}

// ---------------------------------------------------------------- strings

/// This file's UI strings (see i18n.rs). CPU-Z's established technical
/// labels (Core Speed, Bus Speed, SPD, CAS#, tRCD…) stay in English, as in
/// CPU-Z's own Portuguese/Spanish builds and local reviews.
pub const PT_BR: &[(&str, &str)] = &[
    ("Recording · click to stop", "Gravando · clique para parar"),
    // tabs, menus
    ("CPU", "CPU"),
    ("Caches", "Caches"),
    ("Mainboard", "Placa-mãe"),
    ("Memory", "Memória"),
    ("SPD", "SPD"),
    ("Graphics", "Gráficos"),
    ("Devices", "Dispositivos"),
    ("Bench", "Bench"),
    ("About", "Sobre"),
    ("Tools", "Ferramentas"),
    ("Clocks", "Clocks"),
    ("Timers", "Timers"),
    ("Settings", "Configurações"),
    ("Benchmark history", "Histórico de benchmarks"),
    ("Compare reports", "Comparar relatórios"),
    ("Save Report as .TXT", "Salvar relatório como .TXT"),
    ("Save Report as .HTML", "Salvar relatório como .HTML"),
    ("Save .cvf (JSON)", "Salvar .cvf (JSON)"),
    ("Reading hardware…", "Lendo o hardware…"),
    // CPU
    ("Processor", "Processador"),
    ("Name", "Nome"),
    ("Code Name", "Codinome"),
    ("Max TDP", "TDP máx."),
    ("Package", "Encapsulamento"),
    ("Technology", "Tecnologia"),
    ("Specification", "Especificação"),
    ("Family / Model / Stepping", "Família / Modelo / Stepping"),
    ("Ext. Family / Model", "Família / Modelo est."),
    ("Revision", "Revisão"),
    ("Instructions", "Instruções"),
    ("Microcode", "Microcódigo"),
    ("Clocks (Core #{})", "Clocks (Núcleo #{})"),
    ("Core Speed", "Core Speed"),
    ("Multiplier", "Multiplicador"),
    ("Bus Speed", "Bus Speed"),
    ("Base", "Base"),
    ("Cache", "Cache"),
    ("L1 Data", "L1 Dados"),
    ("L1 Inst.", "L1 Instr."),
    ("Level 2", "Nível 2"),
    ("Level 3", "Nível 3"),
    ("Selection", "Seleção"),
    ("Core #{}", "Núcleo #{}"),
    ("Cores {}   Threads {}", "Núcleos {}   Threads {}"),
    ("{}: {} cores, {} threads", "{}: {} núcleos, {} threads"),
    // Caches
    ("L1 D-Cache", "Cache L1 de dados"),
    ("L1 I-Cache", "Cache L1 de instruções"),
    ("Level {}", "Nível {}"),
    ("fully associative", "totalmente associativa"),
    ("{}-way set associative", "associativa de {} vias"),
    ("{}, {}-byte line size", "{}, linha de {} bytes"),
    ("{} thread(s)", "{} thread(s)"),
    ("Size", "Tamanho"),
    ("Descriptor", "Descritor"),
    ("Shared by", "Compartilhada por"),
    ("Core set", "Conjunto de núcleos"),
    // SPD
    ("Memory Slot Selection", "Seleção do slot de memória"),
    ("Slot #{}", "Slot #{}"),
    ("Module", "Módulo"),
    ("Timings Table", "Tabela de timings"),
    ("JEDEC #{}", "JEDEC #{}"),
    ("Frequency", "Frequência"),
    ("CAS# Latency", "CAS# Latency"),
    ("RAS# to CAS#", "RAS# to CAS#"),
    ("RAS# Precharge", "RAS# Precharge"),
    ("tRAS", "tRAS"),
    ("tRC", "tRC"),
    ("Command Rate", "Command Rate"),
    ("Voltage", "Tensão"),
    ("SMBus address", "Endereço SMBus"),
    ("Memory type", "Tipo de memória"),
    ("Module format", "Formato do módulo"),
    ("Module Manufacturer(ID)", "Fabricante do módulo (ID)"),
    ("SDRAM Manufacturer (ID)", "Fabricante da SDRAM (ID)"),
    ("Max bandwidth", "Largura de banda máx."),
    ("Max JEDEC", "JEDEC máx."),
    ("Channels", "Canais"),
    ("Part number", "Part number"),
    ("Serial number", "Número de série"),
    ("Manufacturing date", "Data de fabricação"),
    ("Nominal Voltage", "Tensão nominal"),
    ("Temperature Limit", "Limite de temperatura"),
    ("Clock Driver (CKD)", "Clock Driver (CKD)"),
    ("Clock Driver Manufacturer", "Fabricante do Clock Driver"),
    // Graphics
    ("Display Device Selection", "Seleção do dispositivo de vídeo"),
    ("GPU", "GPU"),
    ("Board Manufacturer", "Fabricante da placa"),
    ("Codename", "Codinome"),
    ("Core family", "Família do núcleo"),
    ("Power Limit", "Limite de potência"),
    ("Thermal Limit", "Limite térmico"),
    ("Cores", "Núcleos"),
    ("ROP Units", "Unidades ROP"),
    ("TM Units", "Unidades TM"),
    ("RayTracing Cores", "Núcleos de Ray Tracing"),
    ("Tensor Cores", "Tensor Cores"),
    ("Memory vendor", "Fabricante da memória"),
    ("Memory size", "Tamanho da memória"),
    ("Memory bus width", "Largura do barramento de memória"),
    ("Current Link Width", "Largura do link atual"),
    ("Current Link Speed", "Velocidade do link atual"),
    ("Max Link Width", "Largura máx. do link"),
    ("Max Link Speed", "Velocidade máx. do link"),
    ("PCI device", "Dispositivo PCI"),
    ("Root device", "Dispositivo raiz"),
    ("Core clock", "Clock do núcleo"),
    ("Shader clock", "Clock dos shaders"),
    ("Memory clock", "Clock da memória"),
    // About
    (
        "Clean-room, open-source reimplementation of CPU-Z's feature set.",
        "Reimplementação clean-room e de código aberto dos recursos do CPU-Z.",
    ),
    ("Hardware access: {}", "Acesso ao hardware: {}"),
    ("Save Report (.TXT)", "Salvar relatório (.TXT)"),
    ("Save Report (.HTML)", "Salvar relatório (.HTML)"),
    ("Project page", "Página do projeto"),
    ("Third-party components", "Componentes de terceiros"),
    ("PawnIO driver installer {}, by namazso (pawnio.eu)", "Instalador do driver PawnIO {}, de namazso (pawnio.eu)"),
    ("PawnIO modules {}, LGPL-2.1-or-later", "Módulos do PawnIO {}, LGPL-2.1-or-later"),
    (
        "Rust crates under MIT, Apache-2.0 and compatible licenses.",
        "Crates Rust sob MIT, Apache-2.0 e licenças compatíveis.",
    ),
    ("Full list and licenses", "Lista completa e licenças"),
    ("Check for updates", "Procurar atualizações"),
    ("Checking...", "Verificando..."),
    ("{} {} is available.", "{} {} está disponível."),
    ("You have the latest version ({}).", "Você tem a versão mais recente ({})."),
    (
        "Not affiliated with CPUID. CPU-Z is a trademark of CPUID.",
        "Sem vínculo com a CPUID. CPU-Z é marca registrada da CPUID.",
    ),
    ("Windows Version", "Versão do Windows"),
    ("Linux Version", "Versão do Linux"),
    ("Windows Installation Date", "Data de instalação do Windows"),
    ("Installation Date", "Data de instalação"),
    ("DirectX Version", "Versão do DirectX"),
    ("OpenGL Version", "Versão do OpenGL"),
    ("Vulkan Version", "Versão do Vulkan"),
    ("Metal Version", "Versão do Metal"),
    // update banner
    ("{} {} is available", "{} {} está disponível"),
    ("(you have {})", "(você tem a {})"),
    ("Download & install", "Baixar e instalar"),
    ("What's new", "Novidades"),
    ("Downloading… {} / {} MB", "Baixando… {} / {} MB"),
    ("Verifying checksum…", "Verificando o checksum…"),
    ("Starting the installer…", "Iniciando o instalador…"),
    ("Installed. Restart CpuZeeZ to use the new version.", "Instalado. Reinicie o CpuZeeZ para usar a nova versão."),
    (
        "The installer was opened: follow its steps, then restart CpuZeeZ.",
        "O instalador foi aberto: siga os passos e depois reinicie o CpuZeeZ.",
    ),
    ("Downloaded and verified. Opened {}", "Baixado e verificado. Pasta aberta: {}"),
    ("Update failed: {}", "Falha na atualização: {}"),
    ("Retry", "Tentar de novo"),
    ("Open download page", "Abrir página de download"),
    ("Dismiss", "Dispensar"),
    // windows, status
    ("source: {}", "fonte: {}"),
    ("hot spot", "hot spot"),
    ("throttling", "throttling"),
    ("No timer information.", "Nenhuma informação de timers."),
    ("Tab copied to clipboard", "Aba copiada para a área de transferência"),
    ("Saved {}", "Salvo {}"),
    // Mainboard
    ("Motherboard", "Placa-mãe"),
    ("Manufacturer", "Fabricante"),
    ("Model", "Modelo"),
    ("Bus Specs.", "Especificação do barramento"),
    ("Chipset", "Chipset"),
    ("Southbridge", "Southbridge"),
    ("LPCIO", "LPCIO"),
    ("BIOS", "BIOS"),
    ("Brand", "Marca"),
    ("Version", "Versão"),
    ("Date", "Data"),
    ("UEFI", "UEFI"),
    ("Graphic Interface", "Interface gráfica"),
    ("Bus", "Barramento"),
    // Memory
    ("General", "Geral"),
    ("Type", "Tipo"),
    ("Channel #", "Canais"),
    ("Uncore Frequency", "Frequência do uncore"),
    ("Timings", "Timings"),
    ("DRAM Frequency", "Frequência da DRAM"),
    ("CAS# Latency (CL)", "CAS# Latency (CL)"),
    ("RAS# to CAS# Delay (tRCD)", "RAS# to CAS# Delay (tRCD)"),
    ("RAS# Precharge (tRP)", "RAS# Precharge (tRP)"),
    ("Cycle Time (tRAS)", "Cycle Time (tRAS)"),
    ("Bank Cycle Time (tRC)", "Bank Cycle Time (tRC)"),
    ("Command Rate (CR)", "Command Rate (CR)"),
    ("Row Refresh Cycle Time (tRFC)", "Row Refresh Cycle Time (tRFC)"),
    (
        "macOS does not expose live DRAM timings. Dashes indicate unavailable fields.",
        "O macOS não expõe os timings atuais da DRAM. Traços indicam campos indisponíveis.",
    ),
    // no data
    ("No {} information: {}.", "Sem informações de {}: {}."),
    (
        "this needs privileged hardware access (see README: PawnIO on Windows, root on Linux)",
        "requer acesso privilegiado ao hardware (veja o README: PawnIO no Windows, root no Linux)",
    ),
    ("nothing was detected", "nada foi detectado"),
    ("graphics", "gráficos"),
    (
        "A small always-on-top window with CPU, GPU and chipset temperatures.",
        "Uma janela pequena, sempre visível, com as temperaturas de CPU, GPU e chipset.",
    ),
    ("Benchmarks", "Benchmarks"),
    (
        "Our own workloads: scores aren't comparable with CPU-Z's",
        "Cargas de trabalho próprias: os scores não são comparáveis com os do CPU-Z",
    ),
    ("Reports", "Relatórios"),
    (
        "Same format as CPU-Z's -txt / -html reports, or our JSON.",
        "Mesmo formato dos relatórios -txt / -html do CPU-Z, ou o nosso JSON.",
    ),
    ("Save .HTML", "Salvar .HTML"),
    ("Save .TXT", "Salvar .TXT"),
    ("Save a report", "Salvar um relatório"),
    ("Show the mini monitor", "Mostrar o mini-monitor"),
];

pub const ES: &[(&str, &str)] = &[
    ("Recording · click to stop", "Grabando · haz clic para detener"),
    // tabs, menus
    ("CPU", "CPU"),
    ("Caches", "Cachés"),
    ("Mainboard", "Placa base"),
    ("Memory", "Memoria"),
    ("SPD", "SPD"),
    ("Graphics", "Gráficos"),
    ("Devices", "Dispositivos"),
    ("Bench", "Bench"),
    ("About", "Acerca de"),
    ("Tools", "Herramientas"),
    ("Clocks", "Relojes"),
    ("Timers", "Temporizadores"),
    ("Settings", "Configuración"),
    ("Benchmark history", "Historial de pruebas"),
    ("Compare reports", "Comparar informes"),
    ("Save Report as .TXT", "Guardar informe como .TXT"),
    ("Save Report as .HTML", "Guardar informe como .HTML"),
    ("Save .cvf (JSON)", "Guardar .cvf (JSON)"),
    ("Reading hardware…", "Leyendo el hardware…"),
    // CPU
    ("Processor", "Procesador"),
    ("Name", "Nombre"),
    ("Code Name", "Nombre en clave"),
    ("Max TDP", "TDP máx."),
    ("Package", "Encapsulado"),
    ("Technology", "Tecnología"),
    ("Specification", "Especificación"),
    ("Family / Model / Stepping", "Familia / Modelo / Stepping"),
    ("Ext. Family / Model", "Familia / Modelo ext."),
    ("Revision", "Revisión"),
    ("Instructions", "Instrucciones"),
    ("Microcode", "Microcódigo"),
    ("Clocks (Core #{})", "Relojes (Núcleo #{})"),
    ("Core Speed", "Core Speed"),
    ("Multiplier", "Multiplicador"),
    ("Bus Speed", "Bus Speed"),
    ("Base", "Base"),
    ("Cache", "Caché"),
    ("L1 Data", "L1 Datos"),
    ("L1 Inst.", "L1 Instr."),
    ("Level 2", "Nivel 2"),
    ("Level 3", "Nivel 3"),
    ("Selection", "Selección"),
    ("Core #{}", "Núcleo #{}"),
    ("Cores {}   Threads {}", "Núcleos {}   Hilos {}"),
    ("{}: {} cores, {} threads", "{}: {} núcleos, {} hilos"),
    // Caches
    ("L1 D-Cache", "Caché L1 de datos"),
    ("L1 I-Cache", "Caché L1 de instrucciones"),
    ("Level {}", "Nivel {}"),
    ("fully associative", "totalmente asociativa"),
    ("{}-way set associative", "asociativa de {} vías"),
    ("{}, {}-byte line size", "{}, línea de {} bytes"),
    ("{} thread(s)", "{} hilo(s)"),
    ("Size", "Tamaño"),
    ("Descriptor", "Descriptor"),
    ("Shared by", "Compartida por"),
    ("Core set", "Conjunto de núcleos"),
    // SPD
    ("Memory Slot Selection", "Selección de ranura de memoria"),
    ("Slot #{}", "Ranura #{}"),
    ("Module", "Módulo"),
    ("Timings Table", "Tabla de timings"),
    ("JEDEC #{}", "JEDEC #{}"),
    ("Frequency", "Frecuencia"),
    ("CAS# Latency", "CAS# Latency"),
    ("RAS# to CAS#", "RAS# to CAS#"),
    ("RAS# Precharge", "RAS# Precharge"),
    ("tRAS", "tRAS"),
    ("tRC", "tRC"),
    ("Command Rate", "Command Rate"),
    ("Voltage", "Voltaje"),
    ("SMBus address", "Dirección SMBus"),
    ("Memory type", "Tipo de memoria"),
    ("Module format", "Formato del módulo"),
    ("Module Manufacturer(ID)", "Fabricante del módulo (ID)"),
    ("SDRAM Manufacturer (ID)", "Fabricante de la SDRAM (ID)"),
    ("Max bandwidth", "Ancho de banda máx."),
    ("Max JEDEC", "JEDEC máx."),
    ("Channels", "Canales"),
    ("Part number", "Número de pieza"),
    ("Serial number", "Número de serie"),
    ("Manufacturing date", "Fecha de fabricación"),
    ("Nominal Voltage", "Voltaje nominal"),
    ("Temperature Limit", "Límite de temperatura"),
    ("Clock Driver (CKD)", "Clock Driver (CKD)"),
    ("Clock Driver Manufacturer", "Fabricante del Clock Driver"),
    // Graphics
    ("Display Device Selection", "Selección del dispositivo de pantalla"),
    ("GPU", "GPU"),
    ("Board Manufacturer", "Fabricante de la tarjeta"),
    ("Codename", "Nombre en clave"),
    ("Core family", "Familia del núcleo"),
    ("Power Limit", "Límite de potencia"),
    ("Thermal Limit", "Límite térmico"),
    ("Cores", "Núcleos"),
    ("ROP Units", "Unidades ROP"),
    ("TM Units", "Unidades TM"),
    ("RayTracing Cores", "Núcleos de trazado de rayos"),
    ("Tensor Cores", "Tensor Cores"),
    ("Memory vendor", "Fabricante de la memoria"),
    ("Memory size", "Tamaño de la memoria"),
    ("Memory bus width", "Ancho del bus de memoria"),
    ("Current Link Width", "Ancho de enlace actual"),
    ("Current Link Speed", "Velocidad de enlace actual"),
    ("Max Link Width", "Ancho de enlace máx."),
    ("Max Link Speed", "Velocidad de enlace máx."),
    ("PCI device", "Dispositivo PCI"),
    ("Root device", "Dispositivo raíz"),
    ("Core clock", "Reloj del núcleo"),
    ("Shader clock", "Reloj de shaders"),
    ("Memory clock", "Reloj de memoria"),
    // About
    (
        "Clean-room, open-source reimplementation of CPU-Z's feature set.",
        "Reimplementación clean-room y de código abierto de las funciones de CPU-Z.",
    ),
    ("Hardware access: {}", "Acceso al hardware: {}"),
    ("Save Report (.TXT)", "Guardar informe (.TXT)"),
    ("Save Report (.HTML)", "Guardar informe (.HTML)"),
    ("Project page", "Página del proyecto"),
    ("Third-party components", "Componentes de terceros"),
    (
        "PawnIO driver installer {}, by namazso (pawnio.eu)",
        "Instalador del controlador PawnIO {}, de namazso (pawnio.eu)",
    ),
    ("PawnIO modules {}, LGPL-2.1-or-later", "Módulos de PawnIO {}, LGPL-2.1-or-later"),
    (
        "Rust crates under MIT, Apache-2.0 and compatible licenses.",
        "Crates de Rust bajo MIT, Apache-2.0 y licencias compatibles.",
    ),
    ("Full list and licenses", "Lista completa y licencias"),
    ("Check for updates", "Buscar actualizaciones"),
    ("Checking...", "Comprobando..."),
    ("{} {} is available.", "{} {} está disponible."),
    ("You have the latest version ({}).", "Tiene la versión más reciente ({})."),
    (
        "Not affiliated with CPUID. CPU-Z is a trademark of CPUID.",
        "Sin relación con CPUID. CPU-Z es una marca registrada de CPUID.",
    ),
    ("Windows Version", "Versión de Windows"),
    ("Linux Version", "Versión de Linux"),
    ("Windows Installation Date", "Fecha de instalación de Windows"),
    ("Installation Date", "Fecha de instalación"),
    ("DirectX Version", "Versión de DirectX"),
    ("OpenGL Version", "Versión de OpenGL"),
    ("Vulkan Version", "Versión de Vulkan"),
    ("Metal Version", "Versión de Metal"),
    // update banner
    ("{} {} is available", "{} {} está disponible"),
    ("(you have {})", "(tiene la {})"),
    ("Download & install", "Descargar e instalar"),
    ("What's new", "Novedades"),
    ("Downloading… {} / {} MB", "Descargando… {} / {} MB"),
    ("Verifying checksum…", "Verificando la suma de comprobación…"),
    ("Starting the installer…", "Iniciando el instalador…"),
    ("Installed. Restart CpuZeeZ to use the new version.", "Instalado. Reinicie CpuZeeZ para usar la nueva versión."),
    (
        "The installer was opened: follow its steps, then restart CpuZeeZ.",
        "Se abrió el instalador: siga sus pasos y luego reinicie CpuZeeZ.",
    ),
    ("Downloaded and verified. Opened {}", "Descargado y verificado. Carpeta abierta: {}"),
    ("Update failed: {}", "Error al actualizar: {}"),
    ("Retry", "Reintentar"),
    ("Open download page", "Abrir página de descarga"),
    ("Dismiss", "Descartar"),
    // windows, status
    ("source: {}", "fuente: {}"),
    ("hot spot", "punto caliente"),
    ("throttling", "limitando"),
    ("No timer information.", "No hay información de temporizadores."),
    ("Tab copied to clipboard", "Pestaña copiada al portapapeles"),
    ("Saved {}", "Guardado {}"),
    // Mainboard
    ("Motherboard", "Placa base"),
    ("Manufacturer", "Fabricante"),
    ("Model", "Modelo"),
    ("Bus Specs.", "Especificación del bus"),
    ("Chipset", "Chipset"),
    ("Southbridge", "Southbridge"),
    ("LPCIO", "LPCIO"),
    ("BIOS", "BIOS"),
    ("Brand", "Marca"),
    ("Version", "Versión"),
    ("Date", "Fecha"),
    ("UEFI", "UEFI"),
    ("Graphic Interface", "Interfaz gráfica"),
    ("Bus", "Bus"),
    // Memory
    ("General", "General"),
    ("Type", "Tipo"),
    ("Channel #", "Canales"),
    ("Uncore Frequency", "Frecuencia del uncore"),
    ("Timings", "Timings"),
    ("DRAM Frequency", "Frecuencia de la DRAM"),
    ("CAS# Latency (CL)", "CAS# Latency (CL)"),
    ("RAS# to CAS# Delay (tRCD)", "RAS# to CAS# Delay (tRCD)"),
    ("RAS# Precharge (tRP)", "RAS# Precharge (tRP)"),
    ("Cycle Time (tRAS)", "Cycle Time (tRAS)"),
    ("Bank Cycle Time (tRC)", "Bank Cycle Time (tRC)"),
    ("Command Rate (CR)", "Command Rate (CR)"),
    ("Row Refresh Cycle Time (tRFC)", "Row Refresh Cycle Time (tRFC)"),
    (
        "macOS does not expose live DRAM timings. Dashes indicate unavailable fields.",
        "macOS no expone los timings actuales de la DRAM. Los guiones indican campos no disponibles.",
    ),
    // no data
    ("No {} information: {}.", "Sin información de {}: {}."),
    (
        "this needs privileged hardware access (see README: PawnIO on Windows, root on Linux)",
        "requiere acceso privilegiado al hardware (vea el README: PawnIO en Windows, root en Linux)",
    ),
    ("nothing was detected", "no se detectó nada"),
    ("graphics", "gráficos"),
    (
        "A small always-on-top window with CPU, GPU and chipset temperatures.",
        "Una ventana pequeña, siempre visible, con las temperaturas de CPU, GPU y chipset.",
    ),
    ("Benchmarks", "Pruebas de rendimiento"),
    (
        "Our own workloads: scores aren't comparable with CPU-Z's",
        "Cargas de trabajo propias: las puntuaciones no son comparables con las de CPU-Z",
    ),
    ("Reports", "Informes"),
    (
        "Same format as CPU-Z's -txt / -html reports, or our JSON.",
        "Mismo formato que los informes -txt / -html de CPU-Z, o nuestro JSON.",
    ),
    ("Save .HTML", "Guardar .HTML"),
    ("Save .TXT", "Guardar .TXT"),
    ("Save a report", "Guardar un informe"),
    ("Show the mini monitor", "Mostrar el minimonitor"),
];

#[cfg(test)]
mod tests {
    #[test]
    fn screenshot_masking() {
        use std::sync::atomic::Ordering;
        assert_eq!(super::shown("MAC", "aa:bb"), "aa:bb");
        super::MASK_PRIVATE.store(true, Ordering::Relaxed);
        for k in ["MAC", "IPv4", "IPv6", "Gateway", "DNS", "SSID", "Serial number", "UUID"] {
            assert_eq!(super::shown(k, "x"), "••••••••", "{k}");
        }
        assert_eq!(super::shown("Machine", "x"), "x");
        assert_eq!(super::shown("Name", "x"), "x");
        super::MASK_PRIVATE.store(false, Ordering::Relaxed);
    }

    /// S1-U01: a card keeps the rows its page's level shows, and vanishes when none.
    #[test]
    fn detail_levels_filter_cards() {
        use cpuzeez_core::detail::Level;
        use eframe::egui;
        use egui_kittest::{Harness, kittest::Queryable};
        let draw = |level: Level| {
            let mut h = Harness::builder().with_size(egui::vec2(600.0, 400.0)).build_ui(move |ui| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                super::detail::set(ui.ctx(), level);
                super::boxed(ui, "Processor", &[("Name", "Ryzen".into()), ("Microcode", "0xA601209".into())]);
                super::boxed(ui, "Only advanced", &[("Bus Speed", "100 MHz".into())]);
            });
            h.run();
            h
        };
        let basic = draw(Level::Basic);
        assert!(basic.query_by_label("Ryzen").is_some());
        assert!(basic.query_by_label("0xA601209").is_none(), "Microcode is Advanced");
        assert!(basic.query_by_label("Only advanced").is_none(), "an empty card isn't drawn");
        let advanced = draw(Level::Advanced);
        assert!(advanced.query_by_label("0xA601209").is_some() && advanced.query_by_label("100 MHz").is_some());
    }

    /// S1-U03: right-click a row, "Pin to Overview" asks the app to pin it under its card.
    #[test]
    fn a_row_pins_under_its_card() {
        use eframe::egui;
        use egui_kittest::{Harness, kittest::Queryable};
        let mut h = Harness::builder().with_size(egui::vec2(600.0, 400.0)).build_ui_state(
            |ui, got: &mut Vec<crate::settings::Pin>| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                super::detail::set(ui.ctx(), cpuzeez_core::detail::Level::Expert);
                super::boxed(ui, "Processor", &[("Name", "Ryzen".into())]);
                got.extend(super::pins::take_requests(ui.ctx()));
            },
            Vec::new(),
        );
        h.run();
        h.get_by_label("Ryzen").click_secondary();
        h.run();
        h.run();
        h.get_by_label("Pin to Overview").click();
        h.run();
        let p = h.state().first().cloned().expect("a pin request");
        assert_eq!(
            (p.card.as_str(), p.label.as_str(), p.value.as_str(), p.metric),
            ("Processor", "Name", "Ryzen", None)
        );
    }
}

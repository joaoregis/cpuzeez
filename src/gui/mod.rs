//! Modern layout: a sidebar with pages (Overview, Processor, Mainboard, Memory,
//! Graphics, Devices, Sensors, Benchmarks, Tools, Settings, About) built from
//! the design system in `theme`/`widgets`. Every page reads the same report
//! tree as `-txt`. F5 BMP, F6 clipboard, F7 .cvf (our JSON). UI text goes
//! through `i18n::tr` (English keys; tables at the end of each file).

mod bench_tab;
mod bmp;
mod devices;
pub mod i18n;
pub mod overview;
mod pages;
mod settings_ui;
mod shots;
mod update_ui;
// ---- monitoring (sensors / stress / tray) ----
mod plot;
pub mod sensors;
pub mod stress;
mod tray;
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
const NAV: [(Page, &str, &str); 9] = [
    (Page::Overview, icon::SQUARES_FOUR, "Overview"),
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
    Stress,
    History,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ToolView {
    Compare,
    Clocks,
    Timers,
    Reports,
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
    /// The About page in Brazilian Portuguese, to show the translations.
    AboutPt,
}

const SHOTS: [(&str, Step); 17] = [
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

#[derive(Default)]
struct Live {
    /// MHz per logical CPU (index = logical CPU), refreshed in the background.
    mhz: Vec<Option<f64>>,
    source: &'static str,
    gpus: Vec<crate::gpu::Live>,
}

struct App {
    ctx: Arc<Ctx>,
    report: Arc<Mutex<Option<Vec<Section>>>>,
    live: Arc<Mutex<Live>>,
    page: Page,
    /// Reload history.json when the History view is opened.
    history_dirty: bool,
    bench_view: BenchView,
    tool_view: ToolView,
    core: usize,
    slot: usize,
    gpu: usize,
    show_clocks: Arc<Mutex<bool>>,
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
    title_minimized: bool,
    /// Sensors page, stress monitor, tray and mini monitor (gui/sensors.rs).
    /// Its `settings` is the app's one copy of settings.json.
    mon: sensors::Monitor,
}

pub fn run(ctx: Ctx, shots: Option<std::path::PathBuf>) {
    let ctx = Arc::new(ctx);
    let report = Arc::new(Mutex::new(None));
    {
        let (c, r) = (ctx.clone(), report.clone());
        std::thread::spawn(move || *r.lock().unwrap() = Some(crate::collect(&c)));
    }
    let live = Arc::new(Mutex::new(Live::default()));
    let show_clocks = Arc::new(Mutex::new(false));
    spawn_sampler(ctx.clone(), live.clone(), show_clocks.clone());

    // One copy of the settings (the monitor's), edited by the Settings page.
    // Screenshots are reproducible: default settings, English.
    let mut mon = sensors::Monitor::new(ctx.clone());
    mon.pages = true;
    if shots.is_some() {
        mon.settings = Default::default();
    }
    i18n::set(if shots.is_some() { i18n::Lang::En } else { settings_ui::lang_of(&mon.settings) });

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(crate::DISPLAY_NAME)
            .with_icon(app_icon())
            .with_inner_size([1120.0, 760.0])
            .with_min_inner_size([860.0, 560.0]),
        ..Default::default()
    };
    let theme = mon.settings.theme;
    let app = App {
        ctx,
        report,
        live,
        page: Page::Overview,
        history_dirty: true,
        bench_view: BenchView::Cpu,
        tool_view: ToolView::Compare,
        core: 0,
        slot: 0,
        gpu: 0,
        show_clocks,
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
        title_minimized: false,
        mon,
    };
    if app.ctx.cfg.check_updates && app.auto.is_none() {
        let slot = app.update.clone();
        std::thread::spawn(move || *slot.lock().unwrap() = crate::update::check());
    }
    let font_size = app.ctx.cfg.text_font_size;
    if let Err(e) = eframe::run_native(
        crate::DISPLAY_NAME,
        opts,
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
            Ok(Box::new(app))
        }),
    ) {
        eprintln!("GUI failed to start ({e}); use -console or -txt=FILE");
    }
}

/// Core 0 every second; every logical CPU while the Clocks window is open.
fn spawn_sampler(ctx: Arc<Ctx>, live: Arc<Mutex<Live>>, all: Arc<Mutex<bool>>) {
    std::thread::spawn(move || {
        let n = crate::os::logical_cpus();
        live.lock().unwrap().mhz = vec![None; n];
        loop {
            let every = *all.lock().unwrap();
            let targets: Vec<usize> = if every { (0..n).collect() } else { vec![0] };
            for cpu in targets {
                if let Some((m, src)) = crate::cpu::clocks::core_mhz(&ctx, cpu) {
                    let mut l = live.lock().unwrap();
                    l.mhz[cpu] = Some(m);
                    l.source = src.label();
                }
            }
            if every {
                let g = crate::gpu::live(&ctx);
                live.lock().unwrap().gpus = g;
            }
            std::thread::sleep(Duration::from_millis(1000));
        }
    });
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ectx = ui.ctx().clone();
        ectx.request_repaint_after(Duration::from_millis(500));
        self.hotkeys(&ectx);
        self.minimized_title(&ectx);

        let report = self.report.lock().unwrap().clone();
        // Tray menu / monitor shortcuts navigate instead of opening windows.
        if std::mem::take(&mut self.mon.show_sensors) {
            self.page = Page::Sensors;
        }
        if std::mem::take(&mut self.mon.show_stress) {
            self.page = Page::Benchmarks;
            self.bench_view = BenchView::Stress;
        }
        *self.show_clocks.lock().unwrap() = self.page == Page::Tools && self.tool_view == ToolView::Clocks;
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
                if self.page == Page::Sensors {
                    // Full-height dashboard with its own scrolling table and charts.
                    self.page_body(ui, r);
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .id_salt(page_name(self.page))
                        .show(ui, |ui| self.page_body(ui, r));
                }
            });
        self.mon.frame(&ectx);
        self.take_screenshot(&ectx);
        self.auto_shots(&ectx, report.as_deref());
    }
}

// ---- helpers ----

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
    let k = key.to_ascii_lowercase();
    let first = k.split_whitespace().next().unwrap_or_default();
    let private = k.contains("serial")
        || key.contains("UUID")
        || ["mac", "ipv4", "ipv6", "ip", "gateway", "dns", "ssid"].contains(&first);
    if private { "••••••••" } else { v }
}

/// A card of label/value rows (labels translated, private values masked).
fn boxed(ui: &mut egui::Ui, title: &str, rows: &[(&str, String)]) {
    let rows: Vec<(&str, String)> = rows.iter().map(|(k, v)| (tr(k), shown(k, v).to_string())).collect();
    widgets::card(ui, "", tr(title), |ui| widgets::kv_list(ui, &rows));
    ui.add_space(theme::GAP);
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

    fn page_body(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let c = crate::cpu::get(&self.ctx);
        let core_mhz = self.live.lock().unwrap().mhz.first().copied().flatten();
        match self.page {
            Page::Overview => {
                let tel = self.mon.tel();
                overview::page(ui, &self.ctx, r, &tel, core_mhz);
            }
            Page::Processor => {
                let sub = [c.info.codename.clone(), c.info.package.clone(), c.info.technology.clone()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" · ");
                widgets::page_header(ui, &c.name, &sub, |_| {});
                self.cpu_tab(ui, r);
                heading(ui, tr("Caches"));
                self.caches_tab(ui);
            }
            Page::Mainboard => {
                let board =
                    format!("{} {}", sgi(r, "DMI", "DMI Baseboard", "vendor"), sgi(r, "DMI", "DMI Baseboard", "model"));
                widgets::page_header(ui, tr("Mainboard"), board.trim(), |_| {});
                mainboard_tab(ui, r);
            }
            Page::Memory => {
                let k = |key: &str| sget(r, "Chipset", key);
                let sub = [k("Memory Type"), k("Memory Size"), k("Channels")]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · ");
                widgets::page_header(ui, tr("Memory"), &sub, |_| {});
                memory_tab(ui, r);
                heading(ui, "SPD");
                self.spd_tab(ui, r);
            }
            Page::Graphics => {
                widgets::page_header(ui, tr("Graphics"), "", |_| {});
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
                        (BenchView::Stress, icon::FLAME, tr("Stress test")),
                        (BenchView::History, icon::CLOCK_COUNTER_CLOCKWISE, tr("History")),
                    ],
                );
                match self.bench_view {
                    BenchView::Cpu => self.bench.cpu_page(ui, &self.ctx, &mut self.mon),
                    BenchView::Memory => self.bench.memory_page(ui, &self.ctx),
                    BenchView::Stress => stress::page(ui, &mut self.mon, true),
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
                let tray = self.mon.settings.tray;
                let persist = self.auto.is_none();
                settings_ui::page(ui, &mut self.mon.settings, self.ctx.cfg.check_updates, persist);
                if self.mon.settings.tray != tray {
                    let on = self.mon.settings.tray;
                    self.mon.set_tray(on, ui.ctx());
                }
                ui.add_space(theme::GAP);
                widgets::card(ui, icon::MONITOR, tr("Mini monitor"), |ui| {
                    ui.label(
                        RichText::new(tr("A small always-on-top window with CPU, GPU and chipset temperatures."))
                            .color(theme::palette(ui).text_secondary),
                    );
                    if ui.checkbox(&mut self.mon.show_mini, tr("Show the mini monitor")).changed() && self.mon.show_mini
                    {
                        self.mon.tel();
                    }
                });
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
    }

    fn minimized_title(&mut self, ectx: &egui::Context) {
        // CPU-Z shows the core clock in the title while iconized.
        let min = ectx.input(|i| i.viewport().minimized.unwrap_or(false));
        if min {
            let m = self.live.lock().unwrap().mhz.first().copied().flatten();
            if let Some(m) = m {
                ectx.send_viewport_cmd(egui::ViewportCommand::Title(format!("{m:.0} MHz - {}", crate::DISPLAY_NAME)));
            }
            self.title_minimized = true;
        } else if self.title_minimized {
            ectx.send_viewport_cmd(egui::ViewportCommand::Title(crate::DISPLAY_NAME.into()));
            self.title_minimized = false;
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
}

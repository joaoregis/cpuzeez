//! CPU-Z style window: CPU, Caches, Mainboard, Memory, SPD, Graphics, Devices,
//! Bench, About tabs; Tools ▸ Clocks / Timers / Settings / Benchmark history /
//! Compare reports / Save report; F5 BMP, F6 clipboard, F7 .cvf (our JSON).
//! Every tab reads the same report tree as `-txt`. UI text goes through
//! `i18n::tr` (English keys; tables at the end of each file).

mod bench_tab;
mod bmp;
mod devices;
pub mod i18n;
pub mod overview;
mod settings_ui;
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
use std::sync::{Arc, Mutex};
use std::time::Duration;
use theme::icon;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Cpu,
    Caches,
    Mainboard,
    Memory,
    Spd,
    Graphics,
    Devices,
    Bench,
    About,
}

/// English names: translated when shown, used as-is for screenshot/BMP file names.
const TABS: [(Tab, &str); 9] = [
    (Tab::Cpu, "CPU"),
    (Tab::Caches, "Caches"),
    (Tab::Mainboard, "Mainboard"),
    (Tab::Memory, "Memory"),
    (Tab::Spd, "SPD"),
    (Tab::Graphics, "Graphics"),
    (Tab::Devices, "Devices"),
    (Tab::Bench, "Bench"),
    (Tab::About, "About"),
];

enum Shot {
    Clipboard,
    Bmp,
    Png(std::path::PathBuf),
}

/// One `-screenshots` capture.
#[derive(Clone, Copy)]
enum Step {
    Tab(Tab),
    Clocks,
    /// gui/sensors.rs captures: 0 = Sensors window, 1 = stress monitor.
    Monitor(u8),
    Settings,
    History,
    Compare,
    MemBench,
    /// The About tab in Brazilian Portuguese, to show the translations.
    AboutPt,
}

const SHOTS: [(&str, Step); 17] = [
    ("cpu", Step::Tab(Tab::Cpu)),
    ("caches", Step::Tab(Tab::Caches)),
    ("mainboard", Step::Tab(Tab::Mainboard)),
    ("memory", Step::Tab(Tab::Memory)),
    ("spd", Step::Tab(Tab::Spd)),
    ("graphics", Step::Tab(Tab::Graphics)),
    ("devices", Step::Tab(Tab::Devices)),
    ("bench", Step::Tab(Tab::Bench)),
    ("about", Step::Tab(Tab::About)),
    ("clocks", Step::Clocks),
    ("sensors", Step::Monitor(0)),
    ("stress", Step::Monitor(1)),
    ("settings", Step::Settings),
    ("history", Step::History),
    ("compare", Step::Compare),
    ("membench", Step::MemBench),
    ("about-pt", Step::AboutPt),
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
    tab: Tab,
    core: usize,
    slot: usize,
    gpu: usize,
    show_clocks: Arc<Mutex<bool>>,
    show_timers: bool,
    show_settings: bool,
    show_history: bool,
    show_compare: bool,
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
    if shots.is_some() {
        mon.settings = Default::default();
    }
    i18n::set(if shots.is_some() { i18n::Lang::En } else { settings_ui::lang_of(&mon.settings) });

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(crate::DISPLAY_NAME)
            .with_icon(app_icon())
            .with_inner_size([520.0, 560.0])
            .with_min_inner_size([460.0, 440.0]),
        ..Default::default()
    };
    let theme = mon.settings.theme;
    let app = App {
        ctx,
        report,
        live,
        tab: Tab::Cpu,
        core: 0,
        slot: 0,
        gpu: 0,
        show_clocks,
        show_timers: false,
        show_settings: false,
        show_history: false,
        show_compare: false,
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
        self.update_banner(ui);
        egui::Panel::top("tabs").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (t, name) in TABS {
                    ui.selectable_value(&mut self.tab, t, tr(name));
                }
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} {}", crate::DISPLAY_NAME, crate::VERSION)).weak());
                if ui.button(tr("Sensors")).clicked() {
                    self.mon.show_sensors = true;
                }
                ui.menu_button(tr("Tools"), |ui| self.tools_menu(ui, &report));
                if ui.button(icon::GEAR_SIX).on_hover_text(tr("Settings")).clicked() {
                    self.show_settings = !self.show_settings;
                }
                ui.label(RichText::new(&self.status).small());
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let Some(r) = report.as_deref() else {
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr("Reading hardware…"));
                    });
                });
                return;
            };
            egui::ScrollArea::vertical().show(ui, |ui| match self.tab {
                Tab::Cpu => self.cpu_tab(ui, r),
                Tab::Caches => self.caches_tab(ui),
                Tab::Mainboard => mainboard_tab(ui, r),
                Tab::Memory => memory_tab(ui, r),
                Tab::Spd => self.spd_tab(ui, r),
                Tab::Graphics => self.graphics_tab(ui, r),
                Tab::Devices => self.devices.page(ui, &self.ctx),
                Tab::Bench => self.bench.ui(ui, &self.ctx, &mut self.mon),
                Tab::About => self.about_tab(ui, r),
            });
        });
        self.clocks_window(&ectx);
        self.timers_window(&ectx, report.as_deref());
        self.tool_windows(&ectx, report.as_deref());
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

/// A titled box of label/value rows (labels translated, private values masked).
fn boxed(ui: &mut egui::Ui, title: &str, rows: &[(&str, String)]) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(tr(title)).strong());
        // Fixed label column; values get the rest and wrap instead of widening the window.
        let label_w = 170.0_f32.min(ui.available_width() * 0.45);
        let value_w = (ui.available_width() - label_w - 16.0).max(80.0);
        egui::Grid::new(title)
            .num_columns(2)
            .spacing([12.0, 3.0])
            .striped(true)
            .min_col_width(label_w.min(value_w))
            .max_col_width(value_w.max(label_w))
            .show(ui, |ui| {
                for (k, v) in rows {
                    ui.label(tr(k));
                    value(ui, if v.is_empty() { "-" } else { shown(k, v) });
                    ui.end_row();
                }
            });
    });
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

/// The update notice: version, one-click install with progress, errors, notes.
fn update_bar(ui: &mut egui::Ui, rel: &Release, installer: &mut Option<Installer>, dismissed: &mut bool) {
    let p = theme::palette(ui);
    let stage = installer.as_ref().map(Installer::stage);
    if matches!(stage, Some(Stage::Quit)) {
        // msiexec is running: leave so it can replace our files.
        std::process::exit(0);
    }
    if matches!(stage, Some(Stage::Downloading { .. } | Stage::Verifying | Stage::Installing)) {
        ui.ctx().request_repaint_after(Duration::from_millis(150));
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(icon::ARROW_CIRCLE_UP).color(p.accent));
        ui.label(RichText::new(trf("{} {} is available", &[&crate::DISPLAY_NAME, &rel.version])).strong());
        ui.label(RichText::new(trf("(you have {})", &[&crate::VERSION])).color(p.text_secondary));
        let start = |installer: &mut Option<Installer>| *installer = Some(Installer::start(rel));
        match &stage {
            None => {
                if rel.asset.is_some()
                    && widgets::primary_button(ui, &format!("{} {}", icon::DOWNLOAD_SIMPLE, tr("Download & install")))
                        .clicked()
                {
                    start(installer);
                }
            }
            Some(Stage::Downloading { done, total }) => {
                let mb = |b: u64| format!("{:.1}", b as f64 / 1e6);
                let frac = if *total > 0 { *done as f32 / *total as f32 } else { 0.0 };
                let text = trf("Downloading… {} / {} MB", &[&mb(*done), &mb(*total)]);
                ui.allocate_ui(egui::vec2(220.0, 22.0), |ui| widgets::progress(ui, frac, &text));
            }
            Some(Stage::Verifying) => {
                ui.spinner();
                ui.label(tr("Verifying checksum…"));
            }
            Some(Stage::Installing) => {
                ui.spinner();
                ui.label(tr("Starting the installer…"));
            }
            Some(Stage::Installed) => {
                widgets::pill(ui, tr("Installed. Restart CpuZeeZ to use the new version."), widgets::Tone::Success);
            }
            Some(Stage::OpenedInstaller) => {
                widgets::pill(
                    ui,
                    tr("The installer was opened: follow its steps, then restart CpuZeeZ."),
                    widgets::Tone::Accent,
                );
            }
            Some(Stage::OpenedFolder(d)) => {
                widgets::pill(ui, &trf("Downloaded and verified. Opened {}", &[&d.display()]), widgets::Tone::Success);
            }
            Some(Stage::Failed(e)) => {
                widgets::pill(ui, &trf("Update failed: {}", &[e]), widgets::Tone::Danger);
                if ui.button(tr("Retry")).clicked() {
                    start(installer);
                }
                if let Some(a) = &rel.asset
                    && ui.button(tr("Open download page")).clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(&a.url));
                }
            }
            Some(Stage::Quit) => {}
        }
        if ui.button(tr("What's new")).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(&rel.page));
        }
        let busy = matches!(stage, Some(Stage::Downloading { .. } | Stage::Verifying | Stage::Installing));
        if !busy && ui.small_button(icon::X).on_hover_text(tr("Dismiss")).clicked() {
            *dismissed = true;
        }
    });
}

impl App {
    fn tools_menu(&mut self, ui: &mut egui::Ui, report: &Option<Vec<Section>>) {
        self.mon.menu(ui);
        ui.separator();
        if ui.button(tr("Clocks")).clicked() {
            *self.show_clocks.lock().unwrap() = true;
        }
        if ui.button(tr("Timers")).clicked() {
            self.show_timers = true;
        }
        ui.separator();
        if ui.button(format!("{} {}", icon::CLOCK_COUNTER_CLOCKWISE, tr("Benchmark history"))).clicked() {
            self.history.reload();
            self.show_history = true;
        }
        if ui.button(format!("{} {}", icon::GIT_DIFF, tr("Compare reports"))).clicked() {
            self.show_compare = true;
        }
        if ui.button(format!("{} {}", icon::GEAR_SIX, tr("Settings"))).clicked() {
            self.show_settings = true;
        }
        ui.separator();
        if ui.button(tr("Save Report as .TXT")).clicked() {
            self.save(report, "txt");
        }
        if ui.button(tr("Save Report as .HTML")).clicked() {
            self.save(report, "html");
        }
        if ui.button(tr("Save .cvf (JSON)")).clicked() {
            self.save(report, "cvf");
        }
    }

    /// Settings / History / Compare pages, hosted in windows for now.
    fn tool_windows(&mut self, ectx: &egui::Context, report: Option<&[Section]>) {
        let win = |id: &str, title: &str| {
            egui::Window::new(title.to_string())
                .id(egui::Id::new(id))
                .default_pos([16.0, 40.0])
                .default_size([488.0, 470.0])
                .vscroll(true)
        };
        let persist = self.auto.is_none();
        let mut open = self.show_settings;
        let tray = self.mon.settings.tray;
        win("settings-window", tr("Settings"))
            .open(&mut open)
            .show(ectx, |ui| settings_ui::page(ui, &mut self.mon.settings, self.ctx.cfg.check_updates, persist));
        self.show_settings = open;
        if self.mon.settings.tray != tray {
            self.mon.set_tray(self.mon.settings.tray, ectx);
        }
        let mut open = self.show_history;
        win("history-window", tr("Benchmark history")).open(&mut open).show(ectx, |ui| self.history.page(ui));
        self.show_history = open;
        let mut open = self.show_compare;
        win("compare-window", tr("Compare reports")).open(&mut open).show(ectx, |ui| self.compare.page(ui, report));
        self.show_compare = open;
    }

    fn cpu_tab(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let c = crate::cpu::get(&self.ctx);
        let pi = |k: &str| sget(r, "Processors Information", k);
        let (fam, model, step) = c.sig;
        let live = self.live.lock().unwrap();
        let first_of_core = c.logical.iter().filter(|l| l.smt == 0).nth(self.core).map_or(0, |l| l.cpu);
        let core_mhz = live.mhz.get(first_of_core).copied().flatten();
        drop(live);
        let bus = crate::cpu::bus_mhz(c);
        boxed(
            ui,
            "Processor",
            &[
                ("Name", c.name.clone()),
                ("Code Name", c.info.codename.clone().unwrap_or_default()),
                ("Max TDP", c.info.tdp_w.map(|w| format!("{w:.1} W")).unwrap_or_default()),
                ("Package", c.info.package.clone().unwrap_or_default()),
                ("Technology", c.info.technology.clone().unwrap_or_default()),
                ("Specification", c.brand.clone()),
                (
                    "Family / Model / Stepping",
                    c.cpuid
                        .as_deref()
                        .map_or_else(|| format!("{fam:X} / {model:X} / {step:X}"), |s| s.replace('.', " / ")),
                ),
                ("Ext. Family / Model", c.ext_cpuid.clone().unwrap_or_default()),
                ("Revision", c.info.core_stepping.clone().unwrap_or_default()),
                ("Instructions", c.instructions.join(", ")),
                ("Microcode", c.microcode.clone().unwrap_or_default()),
            ],
        );
        ui.columns(2, |cols| {
            boxed(
                &mut cols[0],
                &trf("Clocks (Core #{})", &[&self.core]),
                &[
                    ("Core Speed", core_mhz.map(|m| format!("{m:.2} MHz")).unwrap_or_else(|| pi("Core Speed"))),
                    ("Multiplier", core_mhz.zip(bus).map(|(m, b)| format!("x {:.1}", m / b)).unwrap_or_default()),
                    ("Bus Speed", bus.map(|b| format!("{b:.2} MHz")).unwrap_or_default()),
                    ("Base", c.base_mhz.map(|b| format!("{b:.0} MHz")).unwrap_or_default()),
                ],
            );
            let cache = |lvl: u32, kind: char| {
                c.caches
                    .iter()
                    .find(|x| x.level == lvl && (x.kind == kind || kind == 'U'))
                    .map(|x| format!("{} x {}", x.count, crate::cpu::size_text(x.size)))
                    .unwrap_or_default()
            };
            boxed(
                &mut cols[1],
                "Cache",
                &[
                    ("L1 Data", cache(1, 'D')),
                    ("L1 Inst.", cache(1, 'I')),
                    ("Level 2", cache(2, 'U')),
                    ("Level 3", cache(3, 'U')),
                ],
            );
        });
        ui.horizontal(|ui| {
            ui.label(tr("Selection"));
            let n = c.cores.max(1);
            egui::ComboBox::from_id_salt("core").selected_text(trf("Core #{}", &[&self.core])).show_ui(ui, |ui| {
                for i in 0..n {
                    ui.selectable_value(&mut self.core, i, trf("Core #{}", &[&i]));
                }
            });
            ui.label(trf("Cores {}   Threads {}", &[&c.cores, &c.threads]));
        });
        for cl in &c.clusters {
            ui.label(trf("{}: {} cores, {} threads", &[&cl.kind, &cl.cores, &cl.threads]));
        }
    }

    fn caches_tab(&mut self, ui: &mut egui::Ui) {
        let c = crate::cpu::get(&self.ctx);
        for ca in &c.caches {
            let title = match (ca.level, ca.kind) {
                (1, 'D') => tr("L1 D-Cache").to_string(),
                (1, 'I') => tr("L1 I-Cache").to_string(),
                (l, _) => trf("Level {}", &[&l]),
            };
            let ways = if ca.ways == 0 {
                tr("fully associative").to_string()
            } else {
                trf("{}-way set associative", &[&ca.ways])
            };
            let mut rows = vec![
                ("Size", format!("{} x {}", ca.count, crate::cpu::size_text(ca.size))),
                ("Descriptor", trf("{}, {}-byte line size", &[&ways, &ca.line])),
                ("Shared by", trf("{} thread(s)", &[&ca.sharing])),
            ];
            if let Some(n) = &ca.note {
                rows.push(("Core set", n.clone()));
            }
            boxed(ui, &title, &rows);
        }
    }

    fn spd_tab(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        // Empty "DIMM #" groups are the headers of the raw dumps at the end.
        let mut dimms: Vec<(&str, &[Node])> =
            sec(r, "Memory SPD").map(|s| report::groups(&s.nodes, "DIMM #")).unwrap_or_default();
        dimms.retain(|(_, c)| !c.is_empty());
        if dimms.is_empty() {
            ui.label(no_data(ui, "SPD", &self.ctx));
            return;
        }
        self.slot = self.slot.min(dimms.len() - 1);
        ui.horizontal(|ui| {
            ui.label(tr("Memory Slot Selection"));
            egui::ComboBox::from_id_salt("slot").selected_text(trf("Slot #{}", &[&dimms[self.slot].0])).show_ui(
                ui,
                |ui| {
                    for (i, (v, _)) in dimms.iter().enumerate() {
                        ui.selectable_value(&mut self.slot, i, trf("Slot #{}", &[v]));
                    }
                },
            );
        });
        let rows: Vec<(&str, String)> = report::fields(dimms[self.slot].1).map(|(k, v)| (k, v.to_string())).collect();
        boxed(ui, "Module", &rows);
        // CPU-Z style timings table for the selected module: one column per
        // JEDEC bin / XMP / EXPO profile.
        let mods = crate::spd::modules(&self.ctx);
        let Some(m) = mods.get(self.slot) else { return };
        let cols: Vec<(String, &crate::spd::decode::Timing)> = m
            .jedec
            .iter()
            .enumerate()
            .map(|(i, t)| (trf("JEDEC #{}", &[&(i + 1)]), t))
            .chain(
                m.profiles
                    .iter()
                    .map(|p| (if p.name.is_empty() { p.kind.to_string() } else { p.name.clone() }, &p.timing)),
            )
            .collect();
        ui.group(|ui| {
            ui.label(RichText::new(tr("Timings Table")).strong());
            egui::ScrollArea::horizontal().show(ui, |ui| {
                egui::Grid::new("spd-timings").striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
                    ui.label("");
                    for (h, _) in &cols {
                        ui.label(RichText::new(h).strong());
                    }
                    ui.end_row();
                    type Cell = fn(&crate::spd::decode::Timing) -> String;
                    let rows: [(&str, Cell); 8] = [
                        ("Frequency", |t| format!("{:.0} MHz", t.freq)),
                        ("CAS# Latency", |t| format!("{:.1}", t.cl)),
                        ("RAS# to CAS#", |t| t.rcd.to_string()),
                        ("RAS# Precharge", |t| t.rp.to_string()),
                        ("tRAS", |t| t.ras.to_string()),
                        ("tRC", |t| t.rc.to_string()),
                        ("Command Rate", |t| t.cr.map_or("-".into(), |c| format!("{c}T"))),
                        ("Voltage", |t| t.volts.map_or("-".into(), |v| format!("{v:.3} V"))),
                    ];
                    for (name, cell) in rows {
                        ui.label(tr(name));
                        for (_, t) in &cols {
                            value(ui, &cell(t));
                        }
                        ui.end_row();
                    }
                });
            });
        });
    }

    fn graphics_tab(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let gpus: Vec<(&str, &[Node])> =
            sec(r, "Display Adapters").map(|s| report::groups(&s.nodes, "Display adapter")).unwrap_or_default();
        if gpus.is_empty() {
            ui.label(no_data(ui, "graphics", &self.ctx));
            return;
        }
        self.gpu = self.gpu.min(gpus.len() - 1);
        ui.horizontal(|ui| {
            ui.label(tr("Display Device Selection"));
            egui::ComboBox::from_id_salt("gpu").selected_text(gpu_name(gpus[self.gpu].1)).show_ui(ui, |ui| {
                for (i, (_, n)) in gpus.iter().enumerate() {
                    ui.selectable_value(&mut self.gpu, i, gpu_name(n));
                }
            });
        });
        let rows: Vec<(&str, String)> = report::fields(gpus[self.gpu].1).map(|(k, v)| (k, v.to_string())).collect();
        boxed(ui, "GPU", &rows);
        for (_, perf) in report::groups(gpus[self.gpu].1, "Performance Level") {
            let rows: Vec<(&str, String)> = report::fields(perf).map(|(k, v)| (k, v.to_string())).collect();
            boxed(ui, "Clocks", &rows);
        }
    }

    fn about_tab(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let logo = self.logo.get_or_insert_with(|| {
            let icon = decode_png(include_bytes!("../../assets/icon-128.png")).unwrap_or_default();
            let img = egui::ColorImage::from_rgba_unmultiplied([icon.width as usize, icon.height as usize], &icon.rgba);
            ui.ctx().load_texture("logo", img, egui::TextureOptions::LINEAR)
        });
        ui.horizontal(|ui| {
            ui.image((logo.id(), egui::vec2(64.0, 64.0)));
            ui.heading(RichText::new(format!("{} {}", crate::DISPLAY_NAME, crate::VERSION)).size(26.0));
        });
        ui.label(tr("Clean-room, open-source reimplementation of CPU-Z's feature set."));
        ui.label(trf("Hardware access: {}", &[&self.ctx.hw.name()]));
        let sw = sec(r, "Software")
            .map(|s| report::fields(&s.nodes).map(|(k, v)| format!("{}: {v}", tr(k))).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default();
        if !sw.is_empty() {
            ui.label(sw);
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("Save Report (.TXT)")).clicked() {
                self.save(&Some(r.to_vec()), "txt");
            }
            if ui.button(tr("Save Report (.HTML)")).clicked() {
                self.save(&Some(r.to_vec()), "html");
            }
            if ui.button(tr("Clocks")).clicked() {
                *self.show_clocks.lock().unwrap() = true;
            }
            if ui.button(tr("Timers")).clicked() {
                self.show_timers = true;
            }
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.hyperlink_to(tr("Project page"), crate::REPO_URL);
            if ui.button(tr("Check for updates")).clicked() {
                let slot = self.update_manual.clone();
                *slot.lock().unwrap() = Some(tr("Checking...").into());
                let found = self.update.clone();
                self.update_dismissed = false;
                std::thread::spawn(move || {
                    let r = crate::update::check();
                    // Translated when the check ends (the language can't change mid-check in practice).
                    let text = match &r {
                        Some(r) => trf("{} {} is available.", &[&crate::DISPLAY_NAME, &r.version]),
                        None => trf("You have the latest version ({}).", &[&crate::VERSION]),
                    };
                    *found.lock().unwrap() = r;
                    *slot.lock().unwrap() = Some(text);
                });
            }
            if let Some(t) = self.update_manual.lock().unwrap().as_deref() {
                ui.label(t);
            }
        });
        ui.label(RichText::new(tr("Not affiliated with CPUID. CPU-Z is a trademark of CPUID.")).small().weak());
    }

    fn update_banner(&mut self, ui: &mut egui::Ui) {
        if self.update_dismissed {
            return;
        }
        let Some(rel) = self.update.lock().unwrap().clone() else { return };
        egui::Panel::top("update").show(ui, |ui| update_bar(ui, &rel, &mut self.installer, &mut self.update_dismissed));
    }

    fn clocks_window(&mut self, ectx: &egui::Context) {
        let mut open = *self.show_clocks.lock().unwrap();
        if !open {
            return;
        }
        let c = crate::cpu::get(&self.ctx);
        let live = self.live.lock().unwrap();
        egui::Window::new(tr("Clocks")).id(egui::Id::new("clocks-window")).open(&mut open).default_width(260.0).show(
            ectx,
            |ui| {
                ui.label(RichText::new(trf("source: {}", &[&live.source])).small().weak());
                egui::Grid::new("clk").striped(true).show(ui, |ui| {
                    for (core, l) in c.logical.iter().filter(|l| l.smt == 0).enumerate() {
                        let kind = l.kind.as_deref().map(|k| format!(" ({k})")).unwrap_or_default();
                        ui.label(format!("{}{kind}", trf("Core #{}", &[&core])));
                        value(
                            ui,
                            &live.mhz.get(l.cpu).copied().flatten().map_or("…".into(), |m| format!("{m:.1} MHz")),
                        );
                        ui.end_row();
                    }
                    let names = crate::gpu::adapters(&self.ctx);
                    for g in &live.gpus {
                        let name = names.get(g.adapter).map_or("GPU", |a| a.name.as_str());
                        let opt = |v: Option<f64>, unit: &str| v.map_or(String::new(), |v| format!("{v:.0} {unit}"));
                        ui.label(name);
                        let mut s: Vec<String> = [
                            opt(g.core_mhz, "MHz core"),
                            opt(g.mem_mhz, "MHz mem"),
                            opt(g.temp_c, "°C"),
                            opt(g.hotspot_c, &format!("°C {}", tr("hot spot"))),
                            opt(g.fan_rpm, "RPM"),
                            opt(g.power_w, "W"),
                            opt(g.load_pct, "%"),
                        ]
                        .into_iter()
                        .filter(|s| !s.is_empty())
                        .collect();
                        if g.throttling() {
                            s.push(tr("throttling").into());
                        }
                        value(ui, &s.join(", "));
                        ui.end_row();
                    }
                });
            },
        );
        drop(live);
        *self.show_clocks.lock().unwrap() = open;
    }

    fn timers_window(&mut self, ectx: &egui::Context, r: Option<&[Section]>) {
        if !self.show_timers {
            return;
        }
        let mut open = true;
        egui::Window::new(tr("Timers")).id(egui::Id::new("timers-window")).open(&mut open).show(ectx, |ui| {
            let rows: Vec<(String, String)> = r
                .and_then(|r| sec(r, "Timers"))
                .map(|s| report::fields(&s.nodes).map(|(k, v)| (k.to_string(), v.to_string())).collect())
                .unwrap_or_default();
            if rows.is_empty() {
                ui.label(tr("No timer information."));
            }
            for (k, v) in rows {
                ui.horizontal(|ui| {
                    ui.label(k);
                    value(ui, &v);
                });
            }
        });
        self.show_timers = open;
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

    /// Put the UI in the state `step` captures; true when it is ready.
    fn stage_shot(&mut self, step: Step, first: bool, report: Option<&[Section]>, ectx: &egui::Context) -> bool {
        *self.show_clocks.lock().unwrap() = matches!(step, Step::Clocks);
        self.show_settings = matches!(step, Step::Settings);
        self.show_history = matches!(step, Step::History);
        self.show_compare = matches!(step, Step::Compare);
        self.tab = Tab::Cpu;
        if !matches!(step, Step::Monitor(_)) {
            self.mon.show_sensors = false;
            self.mon.show_stress = false;
            if self.mon.stress.running() {
                self.mon.stress.stop(None); // don't load the CPU during later captures
            }
        }
        match step {
            Step::Monitor(k) => self.mon.autoshot(k, ectx),
            Step::Tab(t) => {
                self.tab = t;
                match t {
                    Tab::Bench => self.bench.autorun(&self.ctx),
                    Tab::Devices => self.devices.loaded(&self.ctx),
                    _ => true,
                }
            }
            Step::Clocks | Step::Settings => true,
            Step::History => {
                if first {
                    // This session's results only: nothing from the user's history.json.
                    self.history.preview(self.bench.session_entries(&self.ctx));
                }
                true
            }
            Step::Compare => {
                if first {
                    let path = std::env::temp_dir().join(format!("{}-screenshot-report.json", crate::NAME));
                    let _ = std::fs::write(&path, report.map(report::to_json).unwrap_or_default());
                    self.compare.current = true;
                    self.compare.b = path.display().to_string();
                    self.compare.run(report);
                    // Published image: no user name from the temp path.
                    self.compare.b = path.file_name().map_or(String::new(), |n| n.to_string_lossy().into());
                }
                true
            }
            Step::MemBench => {
                self.tab = Tab::Bench;
                self.bench.autorun_memory(&self.ctx)
            }
            Step::AboutPt => {
                i18n::set(i18n::Lang::PtBr);
                self.tab = Tab::About;
                true
            }
        }
    }

    fn auto_shots(&mut self, ectx: &egui::Context, report: Option<&[Section]>) {
        let Some(a) = &self.auto else { return };
        ectx.request_repaint();
        self.mon.tel(); // start sampling early so the Sensors shot has history
        // Let the report load and the clock sampler produce a value first.
        if report.is_none() || a.started.elapsed() < Duration::from_secs(3) || self.shot.is_some() {
            return;
        }
        let idx = a.step;
        let Some(&(name, step)) = SHOTS.get(idx) else {
            self.mon.stress.stop(None);
            ectx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        let first = a.prepared != Some(idx);
        let ready = self.stage_shot(step, first, report, ectx);
        let Some(a) = &mut self.auto else { return };
        if first {
            a.prepared = Some(idx);
            a.since = std::time::Instant::now();
        }
        if !ready {
            return;
        }
        a.wait += 1;
        // A few frames so the page is laid out, and time for windows to fade in.
        if a.wait < 4 || a.since.elapsed() < Duration::from_millis(600) {
            return;
        }
        a.wait = 0;
        a.step += 1;
        self.mon.autoshot_next();
        self.shot = Some(Shot::Png(a.dir.join(format!("{name}.png"))));
        ectx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
    }

    fn take_screenshot(&mut self, ectx: &egui::Context) {
        if self.shot.is_none() {
            return;
        }
        let img = ectx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(img) = img else { return };
        match self.shot.take() {
            Some(Shot::Clipboard) => {
                ectx.copy_image((*img).clone());
                self.status = tr("Tab copied to clipboard").into();
            }
            Some(Shot::Bmp) => {
                let name =
                    format!("{}-{}.bmp", crate::NAME, TABS.iter().find(|t| t.0 == self.tab).map_or("tab", |t| t.1));
                self.status = match std::fs::write(&name, bmp::encode(&img)) {
                    Ok(()) => trf("Saved {}", &[&name]),
                    Err(e) => format!("{name}: {e}"),
                };
            }
            Some(Shot::Png(path)) => {
                if let Err(e) = write_png(&path, &img) {
                    eprintln!("{}: {e}", path.display());
                } else {
                    eprintln!("wrote {}", path.display());
                }
            }
            None => {}
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

fn mainboard_tab(ui: &mut egui::Ui, r: &[Section]) {
    boxed(
        ui,
        "Motherboard",
        &[
            ("Manufacturer", sgi(r, "DMI", "DMI Baseboard", "vendor")),
            ("Model", sgi(r, "DMI", "DMI Baseboard", "model")),
            ("Revision", sgi(r, "DMI", "DMI Baseboard", "revision")),
            ("Bus Specs.", sget(r, "Chipset", "Bus Specification")),
            ("Chipset", sget(r, "Chipset", "Northbridge")),
            ("Southbridge", sget(r, "Chipset", "Southbridge")),
            (
                "LPCIO",
                format!("{} {}", sget(r, "LPCIO", "LPCIO Vendor"), sget(r, "LPCIO", "LPCIO Model")).trim().to_string(),
            ),
        ],
    );
    boxed(
        ui,
        "BIOS",
        &[
            ("Brand", sgi(r, "DMI", "DMI BIOS", "vendor")),
            ("Version", sgi(r, "DMI", "DMI BIOS", "version")),
            ("Date", sgi(r, "DMI", "DMI BIOS", "date")),
            ("UEFI", sget(r, "BIOS", "UEFI")),
            ("Microcode", sget(r, "Processors Information", "Microcode Revision")),
        ],
    );
    boxed(
        ui,
        "Graphic Interface",
        &[
            ("Bus", sget(r, "Chipset", "Graphic Interface")),
            ("Current Link Width", sget(r, "Chipset", "PCI-E Link Width")),
            ("Current Link Speed", sget(r, "Chipset", "PCI-E Link Speed")),
        ],
    );
}

fn memory_tab(ui: &mut egui::Ui, r: &[Section]) {
    let k = |key: &str| sget(r, "Chipset", key);
    boxed(
        ui,
        "General",
        &[
            ("Type", k("Memory Type")),
            ("Size", k("Memory Size")),
            ("Channel #", k("Channels")),
            ("Uncore Frequency", k("Uncore Frequency")),
        ],
    );
    boxed(
        ui,
        "Timings",
        &[
            ("DRAM Frequency", k("Memory Frequency")),
            ("CAS# Latency (CL)", k("CAS# latency (CL)")),
            ("RAS# to CAS# Delay (tRCD)", k("RAS# to CAS# delay (tRCD)")),
            ("RAS# Precharge (tRP)", k("RAS# Precharge (tRP)")),
            ("Cycle Time (tRAS)", k("Cycle Time (tRAS)")),
            ("Bank Cycle Time (tRC)", k("Bank Cycle Time (tRC)")),
            ("Command Rate (CR)", k("Command Rate (CR)")),
            ("Row Refresh Cycle Time (tRFC)", k("Row Refresh Cycle Time (tRFC)")),
        ],
    );
    #[cfg(target_os = "macos")]
    if k("CAS# latency (CL)").is_empty() {
        let muted = theme::palette(ui).text_muted;
        ui.label(
            RichText::new(tr("macOS does not expose live DRAM timings. Dashes indicate unavailable fields."))
                .color(muted),
        );
    }
}

fn gpu_name(n: &[Node]) -> String {
    report::find(n, "Name").unwrap_or("GPU").to_string()
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

fn write_png(path: &std::path::Path, img: &egui::ColorImage) -> Result<(), Box<dyn std::error::Error>> {
    let f = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(f, img.size[0] as u32, img.size[1] as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let data: Vec<u8> = img.pixels.iter().flat_map(|p| p.to_array()).collect();
    enc.write_header()?.write_image_data(&data)?;
    Ok(())
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

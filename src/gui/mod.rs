//! CPU-Z style window: CPU, Caches, Mainboard, Memory, SPD, Graphics, Bench,
//! About tabs; Tools ▸ Clocks / Timers / Save report; F5 BMP, F6 clipboard,
//! F7 .cvf (our JSON). Every tab reads the same report tree as `-txt`.

mod bench_tab;
mod bmp;
pub mod i18n;
pub mod theme;
pub mod widgets;

/// This file's UI strings (see i18n.rs).
pub const PT_BR: &[(&str, &str)] = &[];
pub const ES: &[(&str, &str)] = &[];

use crate::Ctx;
use crate::report::{self, Node, Section};
use eframe::egui::{self, Color32, RichText};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Cpu,
    Caches,
    Mainboard,
    Memory,
    Spd,
    Graphics,
    Bench,
    About,
}

const TABS: [(Tab, &str); 8] = [
    (Tab::Cpu, "CPU"),
    (Tab::Caches, "Caches"),
    (Tab::Mainboard, "Mainboard"),
    (Tab::Memory, "Memory"),
    (Tab::Spd, "SPD"),
    (Tab::Graphics, "Graphics"),
    (Tab::Bench, "Bench"),
    (Tab::About, "About"),
];

enum Shot {
    Clipboard,
    Bmp,
    Png(std::path::PathBuf),
}

/// `-screenshots=DIR`: visit every tab (and the Clocks window), save a PNG of
/// each, then quit. Used for the README and by CI on every OS.
struct AutoShots {
    dir: std::path::PathBuf,
    step: usize,
    wait: u32,
    started: std::time::Instant,
}

#[derive(Default)]
struct Live {
    /// MHz per logical CPU (index = logical CPU), refreshed in the background.
    mhz: Vec<Option<f64>>,
    source: &'static str,
    gpus: Vec<crate::gpu::LiveRow>,
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
    status: String,
    shot: Option<Shot>,
    auto: Option<AutoShots>,
    bench: bench_tab::State,
    /// Newer release found by the background check (None = none / not yet).
    update: Arc<Mutex<Option<crate::update::Release>>>,
    update_dismissed: bool,
    logo: Option<egui::TextureHandle>,
    /// About > Check for updates: Some(result text) once finished.
    update_manual: Arc<Mutex<Option<String>>>,
    title_minimized: bool,
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

    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(crate::DISPLAY_NAME)
            .with_icon(app_icon())
            .with_inner_size([520.0, 560.0])
            .with_min_inner_size([460.0, 440.0]),
        ..Default::default()
    };
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
        status: String::new(),
        shot: None,
        auto: shots.map(|dir| {
            let _ = std::fs::create_dir_all(&dir);
            MASK_SERIALS.store(true, std::sync::atomic::Ordering::Relaxed);
            AutoShots { dir, step: 0, wait: 0, started: std::time::Instant::now() }
        }),
        bench: bench_tab::State::default(),
        update: Arc::new(Mutex::new(None)),
        update_dismissed: false,
        logo: None,
        update_manual: Arc::new(Mutex::new(None)),
        title_minimized: false,
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
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (t, name) in TABS {
                    ui.selectable_value(&mut self.tab, t, name);
                }
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} {}", crate::DISPLAY_NAME, crate::VERSION)).weak());
                ui.menu_button("Tools", |ui| {
                    if ui.button("Clocks").clicked() {
                        *self.show_clocks.lock().unwrap() = true;
                    }
                    if ui.button("Timers").clicked() {
                        self.show_timers = true;
                    }
                    if ui.button("Save Report as .TXT").clicked() {
                        self.save(&report, "txt");
                    }
                    if ui.button("Save Report as .HTML").clicked() {
                        self.save(&report, "html");
                    }
                    if ui.button("Save .cvf (JSON)").clicked() {
                        self.save(&report, "cvf");
                    }
                });
                ui.label(RichText::new(&self.status).small());
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let Some(r) = report.as_deref() else {
                ui.centered_and_justified(|ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Reading hardware…");
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
                Tab::Bench => self.bench.ui(ui, &self.ctx),
                Tab::About => self.about_tab(ui, r),
            });
        });
        self.clocks_window(&ectx);
        self.timers_window(&ectx, report.as_deref());
        self.take_screenshot(&ectx);
        self.auto_shots(&ectx, report.is_some());
    }
}

// ---- helpers ----

fn value(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).monospace()).wrap());
}

/// Set by `-screenshots`: published images shouldn't carry the machine's serial numbers.
static MASK_SERIALS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn shown<'a>(key: &str, v: &'a str) -> &'a str {
    let serial = key.to_ascii_lowercase().contains("serial") || key.contains("UUID");
    if serial && MASK_SERIALS.load(std::sync::atomic::Ordering::Relaxed) { "••••••••" } else { v }
}

/// A titled box of label/value rows.
fn boxed(ui: &mut egui::Ui, title: &str, rows: &[(&str, String)]) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).strong());
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
                    ui.label(*k);
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

impl App {
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
                &format!("Clocks (Core #{})", self.core),
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
            ui.label("Selection");
            let n = c.cores.max(1);
            egui::ComboBox::from_id_salt("core").selected_text(format!("Core #{}", self.core)).show_ui(ui, |ui| {
                for i in 0..n {
                    ui.selectable_value(&mut self.core, i, format!("Core #{i}"));
                }
            });
            ui.label(format!("Cores {}   Threads {}", c.cores, c.threads));
        });
        for cl in &c.clusters {
            ui.label(format!("{}: {} cores, {} threads", cl.kind, cl.cores, cl.threads));
        }
    }

    fn caches_tab(&mut self, ui: &mut egui::Ui) {
        let c = crate::cpu::get(&self.ctx);
        for ca in &c.caches {
            let title = match (ca.level, ca.kind) {
                (1, 'D') => "L1 D-Cache".to_string(),
                (1, 'I') => "L1 I-Cache".to_string(),
                (l, _) => format!("Level {l}"),
            };
            let ways =
                if ca.ways == 0 { "fully associative".into() } else { format!("{}-way set associative", ca.ways) };
            let mut rows = vec![
                ("Size", format!("{} x {}", ca.count, crate::cpu::size_text(ca.size))),
                ("Descriptor", format!("{ways}, {}-byte line size", ca.line)),
                ("Shared by", format!("{} thread(s)", ca.sharing)),
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
            ui.label(no_data("SPD", &self.ctx));
            return;
        }
        self.slot = self.slot.min(dimms.len() - 1);
        ui.horizontal(|ui| {
            ui.label("Memory Slot Selection");
            egui::ComboBox::from_id_salt("slot").selected_text(format!("Slot #{}", dimms[self.slot].0)).show_ui(
                ui,
                |ui| {
                    for (i, (v, _)) in dimms.iter().enumerate() {
                        ui.selectable_value(&mut self.slot, i, format!("Slot #{v}"));
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
            .map(|(i, t)| (format!("JEDEC #{}", i + 1), t))
            .chain(
                m.profiles
                    .iter()
                    .map(|p| (if p.name.is_empty() { p.kind.to_string() } else { p.name.clone() }, &p.timing)),
            )
            .collect();
        ui.group(|ui| {
            ui.label(RichText::new("Timings Table").strong());
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
                        ui.label(name);
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
            ui.label(no_data("graphics", &self.ctx));
            return;
        }
        self.gpu = self.gpu.min(gpus.len() - 1);
        ui.horizontal(|ui| {
            ui.label("Display Device Selection");
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
        ui.label("Clean-room, open-source reimplementation of CPU-Z's feature set.");
        ui.label(format!("Hardware access: {}", self.ctx.hw.name()));
        let sw = sec(r, "Software")
            .map(|s| report::fields(&s.nodes).map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("\n"))
            .unwrap_or_default();
        if !sw.is_empty() {
            ui.label(sw);
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui.button("Save Report (.TXT)").clicked() {
                self.save(&Some(r.to_vec()), "txt");
            }
            if ui.button("Save Report (.HTML)").clicked() {
                self.save(&Some(r.to_vec()), "html");
            }
            if ui.button("Clocks").clicked() {
                *self.show_clocks.lock().unwrap() = true;
            }
            if ui.button("Timers").clicked() {
                self.show_timers = true;
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.hyperlink_to("Project page", crate::REPO_URL);
            if ui.button("Check for updates").clicked() {
                let slot = self.update_manual.clone();
                *slot.lock().unwrap() = Some("Checking...".into());
                let found = self.update.clone();
                std::thread::spawn(move || {
                    let r = crate::update::check();
                    let text = match &r {
                        Some(r) => format!("{} {} is available.", crate::DISPLAY_NAME, r.version),
                        None => format!("You have the latest version ({}).", crate::VERSION),
                    };
                    *found.lock().unwrap() = r;
                    *slot.lock().unwrap() = Some(text);
                });
            }
            if let Some(t) = self.update_manual.lock().unwrap().as_deref() {
                ui.label(t);
            }
        });
        ui.label(RichText::new("Not affiliated with CPUID. CPU-Z is a trademark of CPUID.").small().weak());
    }

    fn update_banner(&mut self, ui: &mut egui::Ui) {
        if self.update_dismissed {
            return;
        }
        let Some(rel) = self.update.lock().unwrap().clone() else { return };
        egui::Panel::top("update").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{} {} is available", crate::DISPLAY_NAME, rel.version)).strong());
                ui.label(RichText::new(format!("(you have {})", crate::VERSION)).weak());
                if let Some(d) = &rel.download
                    && ui.button("Download").clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(d));
                }
                if ui.button("What's new").clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(&rel.page));
                }
                if ui.small_button("x").clicked() {
                    self.update_dismissed = true;
                }
            });
        });
    }

    fn clocks_window(&mut self, ectx: &egui::Context) {
        let mut open = *self.show_clocks.lock().unwrap();
        if !open {
            return;
        }
        let c = crate::cpu::get(&self.ctx);
        let live = self.live.lock().unwrap();
        egui::Window::new("Clocks").open(&mut open).default_width(260.0).show(ectx, |ui| {
            ui.label(RichText::new(format!("source: {}", live.source)).small().weak());
            egui::Grid::new("clk").striped(true).show(ui, |ui| {
                for (core, l) in c.logical.iter().filter(|l| l.smt == 0).enumerate() {
                    let kind = l.kind.as_deref().map(|k| format!(" ({k})")).unwrap_or_default();
                    ui.label(format!("Core #{core}{kind}"));
                    value(ui, &live.mhz.get(l.cpu).copied().flatten().map_or("…".into(), |m| format!("{m:.1} MHz")));
                    ui.end_row();
                }
                let names = crate::gpu::adapters(&self.ctx);
                for (idx, core, mem, temp, power, load) in &live.gpus {
                    let name = names.get(*idx).map_or("GPU", |a| a.name.as_str());
                    let opt = |v: &Option<f64>, unit: &str| v.map_or(String::new(), |v| format!("{v:.0} {unit}"));
                    ui.label(name);
                    value(
                        ui,
                        &[opt(core, "MHz core"), opt(mem, "MHz mem"), opt(temp, "°C"), opt(power, "W"), opt(load, "%")]
                            .iter()
                            .filter(|s| !s.is_empty())
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    ui.end_row();
                }
            });
        });
        drop(live);
        *self.show_clocks.lock().unwrap() = open;
    }

    fn timers_window(&mut self, ectx: &egui::Context, r: Option<&[Section]>) {
        if !self.show_timers {
            return;
        }
        let mut open = true;
        egui::Window::new("Timers").open(&mut open).show(ectx, |ui| {
            let rows: Vec<(String, String)> = r
                .and_then(|r| sec(r, "Timers"))
                .map(|s| report::fields(&s.nodes).map(|(k, v)| (k.to_string(), v.to_string())).collect())
                .unwrap_or_default();
            if rows.is_empty() {
                ui.label("No timer information.");
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

    fn auto_shots(&mut self, ectx: &egui::Context, loaded: bool) {
        let Some(a) = &mut self.auto else { return };
        ectx.request_repaint();
        // Let the report load and the clock sampler produce a value first.
        if !loaded || a.started.elapsed() < Duration::from_secs(3) || self.shot.is_some() {
            return;
        }
        if a.step > TABS.len() {
            ectx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let name = if a.step < TABS.len() {
            self.tab = TABS[a.step].0;
            TABS[a.step].1.to_lowercase()
        } else {
            self.tab = Tab::Cpu;
            *self.show_clocks.lock().unwrap() = true;
            "clocks".to_string()
        };
        if self.tab == Tab::Bench && !self.bench.autorun(&self.ctx) {
            return;
        }
        a.wait += 1;
        if a.wait < 4 {
            return; // a few frames so the tab is fully laid out
        }
        a.wait = 0;
        a.step += 1;
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
                self.status = "Tab copied to clipboard".into();
            }
            Some(Shot::Bmp) => {
                let name =
                    format!("{}-{}.bmp", crate::NAME, TABS.iter().find(|t| t.0 == self.tab).map_or("tab", |t| t.1));
                self.status = match std::fs::write(&name, bmp::encode(&img)) {
                    Ok(()) => format!("Saved {name}"),
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
                format!("Saved {}", std::fs::canonicalize(&name).map_or(name.clone(), |p| p.display().to_string()))
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
}

fn gpu_name(n: &[Node]) -> String {
    report::find(n, "Name").unwrap_or("GPU").to_string()
}

fn no_data(what: &str, ctx: &Ctx) -> RichText {
    let why = if ctx.hw.name().starts_with("none") {
        "this needs privileged hardware access (see README: PawnIO on Windows, root on Linux)"
    } else {
        "nothing was detected"
    };
    RichText::new(format!("No {what} information: {why}.")).color(Color32::GRAY)
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

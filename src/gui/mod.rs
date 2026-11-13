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
mod app_tasks;
#[cfg(test)]
mod app_tests;
mod bench_tab;
mod bmp;
mod bottleneck_ui;
pub mod detail;
mod devices;
#[cfg(test)]
mod fixture;
pub mod i18n;
mod nav;
pub mod overview;
#[cfg(test)]
mod page_tests;
mod pages;
mod palette;
pub mod pins;
mod screens;
mod settings_ui;
mod shots;
#[cfg(feature = "soak")]
mod soak;
mod split;
mod start;
mod strings;
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
use nav::{NAV, NAV_BOTTOM, Page, page_name};
use pages::benchmarks::BenchView;
use pages::common::{MASK_PRIVATE, boxed, cpuid_dump, frame_log, heading, no_data, sec, sget, sgi, shown, value};
use pages::tools::ToolView;
use pages::{mainboard_tab, memory_tab};
use shots::{AutoShots, Shot};
use start::decode_png;
pub use start::run;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use theme::icon;

struct App {
    ctx: Arc<Ctx>,
    report: Arc<Mutex<Option<Vec<Section>>>>,
    /// Capability banners and "Restart as administrator".
    access: access_ui::Access,
    /// The first-run privacy screen, until the user continues.
    welcome: Option<welcome::Welcome>,
    page: Page,
    /// Settings: the mini monitor editor.
    minis: settings_ui::Minis,
    status: String,
    shot: Option<Shot>,
    auto: Option<AutoShots>,
    /// Each page's own state.
    processor: pages::Processor,
    mainboard: pages::Mainboard,
    memory: pages::Memory,
    graphics: pages::Graphics,
    devices: devices::State,
    benches: pages::Benches,
    tools: pages::Tools,
    /// The update check, its banner and the installer.
    updates: update_ui::Updates,
    logo: Option<egui::TextureHandle>,
    /// Last window title sent (● REC, MHz while minimized).
    title: String,
    /// Sensors page, stress monitor, tray and mini monitor (gui/sensors.rs).
    /// Its `settings` is the app's one copy of settings.json.
    mon: sensors::Monitor,
    /// Ctrl+K search (S1-U02).
    palette: palette::Palette,
    overview: overview::State,
    system: system::State,
    live: live::State,
    /// This copy's single-instance slot (Windows); another launch asks through it.
    instance: Option<Arc<cpuzeez_platform::instance::Instance>>,
    /// Set by the listener when another launch asked to see this copy.
    show_asked: Arc<std::sync::atomic::AtomicBool>,
    #[cfg(feature = "soak")]
    soak: Option<soak::Tour>,
}

impl App {
    fn new(
        ctx: Arc<Ctx>,
        report: Arc<Mutex<Option<Vec<Section>>>>,
        mon: sensors::Monitor,
        instance: Option<Arc<cpuzeez_platform::instance::Instance>>,
        show_asked: Arc<std::sync::atomic::AtomicBool>,
    ) -> App {
        App {
            ctx,
            report,
            access: Default::default(),
            welcome: None,
            page: Page::Overview,
            minis: Default::default(),
            status: String::new(),
            shot: None,
            auto: None,
            processor: Default::default(),
            mainboard: Default::default(),
            memory: Default::default(),
            graphics: Default::default(),
            devices: Default::default(),
            benches: Default::default(),
            tools: Default::default(),
            updates: Default::default(),
            logo: None,
            title: String::new(),
            mon,
            palette: Default::default(),
            overview: Default::default(),
            system: Default::default(),
            live: Default::default(),
            instance,
            show_asked,
            #[cfg(feature = "soak")]
            soak: None,
        }
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
        self.take_widths(&ectx);
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
        if self.show_asked.swap(false, std::sync::atomic::Ordering::Relaxed) {
            // Another launch: come to the front (from the tray too) and tell it.
            self.mon.show_window(&ectx);
            if let Some(i) = &self.instance {
                i.shown();
            }
        }
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
            self.benches.view = BenchView::Stress;
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
            self.minis.edit = Some(i);
        }
        self.take_screenshot(&ectx);
        self.auto_shots(&ectx, report.as_deref());
    }
}

// ---- the page dispatch ----

impl App {
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
                    shots::request(ui.ctx());
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
            Page::Benchmarks => self.benchmarks_page(ui),
            Page::Tools => self.tools_page(ui, r),
            Page::Settings => self.settings_page(ui, r),
            Page::About => self.about_tab(ui, r),
        }
    }
}

// ---------------------------------------------------------------- strings

//! Starting the window: renderer, fonts, the tray, the first frame.

#[allow(unused_imports)]
use crate::gui::*;

pub fn run(ctx: Ctx, shots: Option<std::path::PathBuf>, instance: Option<cpuzeez_platform::instance::Instance>) {
    #[cfg(feature = "soak")]
    crate::soak::start();
    // Listen from now, not from the first frame: a launch while this copy is
    // still starting (a cold start can take seconds) must be answered at once,
    // since the window is about to appear anyway; after that the UI answers.
    let ui_ctx: Arc<std::sync::OnceLock<egui::Context>> = Default::default();
    let instance = instance.map(Arc::new);
    let show_asked: Arc<std::sync::atomic::AtomicBool> = Default::default();
    if let Some(i) = instance.clone() {
        let (asked, ui_ctx) = (show_asked.clone(), ui_ctx.clone());
        std::thread::spawn(move || {
            loop {
                i.wait_show();
                match ui_ctx.get() {
                    None => i.shown(),
                    Some(ectx) => {
                        asked.store(true, std::sync::atomic::Ordering::Relaxed);
                        ectx.request_repaint();
                    }
                }
            }
        });
    }
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
    let mut app = App::new(ctx, report, mon, instance, show_asked);
    app.welcome = (shots.is_none() && app.mon.settings.privacy_seen < welcome::PRIVACY_VERSION).then(Default::default);
    app.page = if shots.is_some() { Page::Overview } else { start_page };
    app.auto = shots.map(|dir| {
        let _ = std::fs::create_dir_all(&dir);
        MASK_PRIVATE.store(true, std::sync::atomic::Ordering::Relaxed);
        let now = std::time::Instant::now();
        AutoShots { dir, step: 0, wait: 0, started: now, prepared: None, since: now }
    });
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
        let (app, ui_ctx) = (app.clone(), ui_ctx.clone());
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
                let _ = ui_ctx.set(cc.egui_ctx.clone());
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

/// Window/taskbar icon (the same artwork as assets/icon.svg).
pub(super) fn app_icon() -> egui::IconData {
    decode_png(include_bytes!("../../assets/icon-256.png")).unwrap_or_default()
}

/// RGBA PNG -> pixels (our icons are always 8-bit RGBA).
pub(super) fn decode_png(png: &[u8]) -> Option<egui::IconData> {
    let mut r = png::Decoder::new(std::io::Cursor::new(png)).read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    (info.color_type == png::ColorType::Rgba).then_some(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}

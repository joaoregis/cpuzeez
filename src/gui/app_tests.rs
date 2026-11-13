//! Tests of `App` itself: masking, detail levels, pins.

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
    assert_eq!((p.card.as_str(), p.label.as_str(), p.value.as_str(), p.metric), ("Processor", "Name", "Ryzen", None));
}

//! Interaction tests for the pages and tabs that had none (M2-02), drawn
//! from `fixture`, and the list that makes a new page without a test fail
//! (M2-03).

use super::fixture::harness;
use super::*;
use cpuzeez_core::detail::Level;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

const PAGE: egui::Vec2 = egui::vec2(1120.0, 2400.0);

/// Buttons, links, check boxes, combo boxes and tabs without a name.
fn unnamed_controls(h: &Harness<'_, App>) -> Vec<String> {
    use egui::accesskit::Role;
    h.root()
        .children_recursive()
        .filter(|n| {
            let a = n.accesskit_node();
            matches!(
                a.role(),
                Role::Button
                    | Role::Link
                    | Role::CheckBox
                    | Role::ComboBox
                    | Role::Tab
                    | Role::RadioButton
                    | Role::Slider
            ) && !a.is_hidden()
                && a.label().is_none_or(|l| l.trim().is_empty())
                && a.value().is_none_or(|v| v.trim().is_empty())
        })
        .map(|n| format!("{:?} at {:?}", n.accesskit_node().role(), n.rect()))
        .collect()
}

fn named(h: &Harness<'_, App>, page: &str) {
    let u = unnamed_controls(h);
    assert!(u.is_empty(), "{page}: controls without a name: {}", u.join("; "));
}

/// The detail switch changes the page's saved level and what it shows.
fn detail_switch(h: &mut Harness<'_, App>, page: Page, advanced_only: &str) {
    assert!(h.query_all_by_label_contains(advanced_only).count() > 0, "{page:?} at Advanced shows {advanced_only:?}");
    h.get_by_label("Basic").click();
    h.run();
    h.run();
    assert_eq!(h.state().mon.settings.detail.get(&format!("{page:?}")), Some(&Level::Basic));
    assert_eq!(h.query_all_by_label_contains(advanced_only).count(), 0, "{page:?} at Basic hides {advanced_only:?}");
}

/// Right-click a value: "Copy value" copies it, "Pin to Overview" asks for a pin.
fn copy_and_pin(h: &mut Harness<'_, App>, value: &str) {
    h.get_all_by_label(value).next().unwrap().click_secondary();
    h.run();
    h.get_by_label("Copy value").click();
    h.step();
    let copied =
        h.output().platform_output.commands.iter().any(|c| matches!(c, egui::OutputCommand::CopyText(t) if t == value));
    assert!(copied, "Copy value copies {value:?}");
    h.run();
    h.get_all_by_label(value).next().unwrap().click_secondary();
    h.run();
    h.get_by_label("Pin to Overview").click();
    h.run();
    let pins = pins::take_requests(&h.ctx);
    assert!(pins.iter().any(|p| p.value == value), "Pin to Overview pins {value:?}: {pins:?}");
}

#[test]
fn processor_page() {
    let mut h = harness(Page::Processor, PAGE);
    named(&h, "Processor");
    assert!(h.query_all_by_label_contains("AMD Ryzen 7 7800X3D").count() > 0);
    copy_and_pin(&mut h, "Raphael");
    detail_switch(&mut h, Page::Processor, "0xA601209");
}

#[test]
fn mainboard_page() {
    let mut h = harness(Page::Mainboard, PAGE);
    named(&h, "Mainboard");
    copy_and_pin(&mut h, "AMD X670E rev. 51");
    detail_switch(&mut h, Page::Mainboard, "NCT6799D");
}

#[test]
fn memory_page() {
    let mut h = harness(Page::Memory, PAGE);
    named(&h, "Memory");
    assert!(h.query_all_by_label_contains("DDR5").count() > 0);
    copy_and_pin(&mut h, "32 GBytes");
}

#[test]
fn graphics_page() {
    let mut h = harness(Page::Graphics, PAGE);
    named(&h, "Graphics");
    h.get_by_label("Display Device Selection");
    copy_and_pin(&mut h, "12 GB");
    h.get_by_label("2475 MHz");
}

#[test]
fn devices_page() {
    let h = harness(Page::Devices, PAGE);
    named(&h, "Devices");
    h.get_by_label_contains("Refresh");
    h.get_by_label_contains("Test NVMe SSD 2TB");
    h.get_by_label_contains("Test 2.5GbE Controller");
}

#[test]
fn sensors_page() {
    let mut h = harness(Page::Sensors, PAGE);
    named(&h, "Sensors");
    h.get_by_label_contains("Reset min/max");
    // Right-click a sensor: pin it with its metric, so the Overview tile stays live.
    h.get_by_label_contains(", Package, 71.5").click_secondary();
    h.run();
    h.get_by_label("Pin to Overview").click();
    h.run();
    let pins = pins::take_requests(&h.ctx);
    assert!(pins.iter().any(|p| p.metric.as_deref() == Some("cpu/temp/package")), "{pins:?}");
}

#[test]
fn tools_page() {
    let mut h = harness(Page::Tools, PAGE);
    named(&h, "Tools");
    for (tab, view) in [
        ("Clocks", ToolView::Clocks),
        ("Timers", ToolView::Timers),
        ("Drive health", ToolView::Drives),
        ("Reports", ToolView::Reports),
        ("Compare reports", ToolView::Compare),
    ] {
        h.get_by_label_contains(tab).click();
        h.run();
        h.run();
        assert_eq!(h.state().tools.view, view, "the {tab} tab opens");
        named(&h, tab);
    }
    h.get_by_label_contains("Reports").click();
    h.run();
    h.run();
    h.get_by_label_contains("Save .TXT");
}

#[test]
fn about_page() {
    let mut h = harness(Page::About, PAGE);
    named(&h, "About");
    h.get_by_label("Check for updates");
    h.get_by_label("Project page");
    h.get_by_label("Clocks").click();
    h.run();
    assert_eq!(
        (h.state().page, h.state().tools.view),
        (Page::Tools, ToolView::Clocks),
        "About links to Tools ▸ Clocks"
    );
}

#[test]
fn live_tab() {
    let mut h = harness(Page::Live, PAGE);
    named(&h, "ZeeZ Live ▸ Live");
    h.get_by_label_contains("Show overlay");
    h.get_by_label_contains("Game");
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Overlay").click();
    h.run();
    h.run();
    assert_eq!(h.state().live.tab, live::Tab::Overlay);
}

#[test]
fn sessions_tab() {
    let mut h = harness(Page::Live, PAGE);
    h.state_mut().live.sessions = live_sessions::Sessions::fixture();
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Sessions").click();
    h.run();
    h.run();
    assert_eq!(h.state().live.tab, live::Tab::Sessions);
    named(&h, "ZeeZ Live ▸ Sessions");
    h.get_by_label_contains("Refresh");
    h.get_by_label_contains("fixture-game.exe");
}

// ------------------------------------------------------------- M2-03

/// Each page and tab, and the test that drives it. Adding a page or a tab
/// without a line here does not compile (the matches are exhaustive), and a
/// line naming a test that does not exist fails below.
fn page_test(p: Page) -> &'static str {
    match p {
        Page::Overview => "customize_presets_and_restore",
        Page::System => "performance_tab_switches_devices_and_summary",
        Page::Live => "live_tab",
        Page::Processor => "processor_page",
        Page::Mainboard => "mainboard_page",
        Page::Memory => "memory_page",
        Page::Graphics => "graphics_page",
        Page::Devices => "devices_page",
        Page::Sensors => "sensors_page",
        Page::Benchmarks => "cpu_suite_modes",
        Page::Tools => "tools_page",
        Page::Settings => "system_task_manager_settings",
        Page::About => "about_page",
    }
}

fn system_test(t: system::SysTab) -> &'static str {
    use system::SysTab::*;
    match t {
        Processes => "the_processes_tab_holds_together_at_every_scale",
        Performance => "performance_tab_switches_devices_and_summary",
        AppHistory => "the_app_history_tab_holds_together_at_every_scale",
        Startup => "the_startup_tab_holds_together_at_every_scale",
        Users => "the_users_tab_holds_together_at_every_scale",
        Services => "a_refused_change_leaves_no_undo_and_resets_the_editor",
    }
}

fn live_test(t: live::Tab) -> &'static str {
    match t {
        live::Tab::Live => "live_tab",
        live::Tab::Overlay => "live_tab",
        live::Tab::Sessions => "sessions_tab",
        live::Tab::Bottleneck => "the_bottleneck_page_holds_together_at_every_scale",
    }
}

#[test]
fn every_page_and_tab_has_an_interaction_test() {
    use system::SysTab;
    let mut names: Vec<(String, &str)> =
        NAV.iter().chain(NAV_BOTTOM.iter()).map(|n| (n.2.to_string(), page_test(n.0))).collect();
    for t in
        [SysTab::Processes, SysTab::Performance, SysTab::AppHistory, SysTab::Startup, SysTab::Users, SysTab::Services]
    {
        names.push((format!("System ▸ {t:?}"), system_test(t)));
    }
    for t in [live::Tab::Live, live::Tab::Overlay, live::Tab::Sessions, live::Tab::Bottleneck] {
        names.push((format!("ZeeZ Live ▸ {t:?}"), live_test(t)));
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gui");
    let mut text = String::new();
    for d in [dir.clone(), dir.join("pages")] {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            if e.path().extension().is_some_and(|x| x == "rs") {
                text += &std::fs::read_to_string(e.path()).unwrap();
            }
        }
    }
    let missing: Vec<String> = names
        .iter()
        .filter(|(_, t)| {
            !text.contains(&format!("#[test]\n    fn {t}()")) && !text.contains(&format!("#[test]\nfn {t}()"))
        })
        .map(|(p, t)| format!("{p} → {t}"))
        .collect();
    assert!(missing.is_empty(), "no such test: {}", missing.join(", "));
}

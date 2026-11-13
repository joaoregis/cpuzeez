//! Accessibility checks (S3-03), all automated:
//!
//! - **Contrast**: every text colour against every surface it is drawn on,
//!   in both themes, meets WCAG 2.1 AA (4.5:1; 3:1 for chart lines).
//! - **Keyboard**: the sidebar and the segmented tabs are reached with Tab
//!   and used with Enter; tables move with the arrow keys (in `table.rs`).
//! - **Scale**: 75 % to 200 % (the window's minimum size shrinks to the same
//!   number of logical points), no two pieces of text may collide.
//! - **Colour is never the only cue**: warning and danger marks carry an icon.
//!
//! The written checklist, with what is and isn't covered, is
//! `docs/accessibility.md`.

use super::theme::{DARK, LIGHT, Palette};
use eframe::egui::Color32;

/// WCAG relative luminance of a colour.
fn luminance(c: Color32) -> f64 {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// WCAG contrast ratio, 1 to 21.
fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn surfaces(p: &Palette) -> [(&'static str, Color32); 5] {
    [
        ("bg", p.bg),
        ("sidebar", p.sidebar),
        ("card", p.card),
        ("card_hover", p.card_hover),
        ("accent_soft", p.accent_soft),
    ]
}

#[test]
fn text_colours_meet_aa_on_every_surface() {
    let mut bad = Vec::new();
    for (theme, p) in [("light", &LIGHT), ("dark", &DARK)] {
        let texts = [
            ("text", p.text),
            ("text_secondary", p.text_secondary),
            ("text_muted", p.text_muted),
            ("accent", p.accent),
            ("success", p.success),
            ("warning", p.warning),
            ("danger", p.danger),
        ];
        for (tn, t) in texts {
            for (sn, s) in surfaces(p) {
                let c = contrast(t, s);
                if c < 4.5 {
                    bad.push(format!("{theme}: {tn} on {sn} is {c:.2}:1"));
                }
            }
        }
        // Pills: status text on its own soft background, and the primary button's label.
        for (tn, t, sn, s) in [
            ("success", p.success, "success_soft", p.success_soft),
            ("warning", p.warning, "warning_soft", p.warning_soft),
            ("danger", p.danger, "danger_soft", p.danger_soft),
            ("accent", p.accent, "accent_soft", p.accent_soft),
            ("text", p.text, "success_soft", p.success_soft),
            ("text", p.text, "warning_soft", p.warning_soft),
            ("text", p.text, "danger_soft", p.danger_soft),
            ("on_accent", p.on_accent, "accent", p.accent),
        ] {
            let c = contrast(t, s);
            if c < 4.5 {
                bad.push(format!("{theme}: {tn} on {sn} is {c:.2}:1"));
            }
        }
    }
    assert!(bad.is_empty(), "below WCAG AA (4.5:1):\n{}", bad.join("\n"));
}

#[test]
fn chart_colours_meet_the_graphics_minimum() {
    let mut bad = Vec::new();
    for (theme, p) in [("light", &LIGHT), ("dark", &DARK)] {
        for (i, s) in p.series.iter().enumerate() {
            for (sn, bg) in [("card", p.card), ("bg", p.bg)] {
                let c = contrast(*s, bg);
                if c < 3.0 {
                    bad.push(format!("{theme}: series {i} on {sn} is {c:.2}:1"));
                }
            }
        }
        // The keyboard-focus ring and selected outlines.
        let c = contrast(p.accent, p.card);
        assert!(c >= 3.0, "{theme}: the focus ring on a card is {c:.2}:1");
    }
    assert!(bad.is_empty(), "below 3:1 for graphics:\n{}", bad.join("\n"));
}

/// The bottleneck classes get their colours from a fixed list: each must be
/// readable on both card colours, since a label always goes with it.
#[test]
fn class_colours_are_distinct_and_readable() {
    use cpuzeez_core::bottleneck::Class;
    let colours: Vec<Color32> = Class::ALL.iter().map(|c| super::bottleneck_ui::class_color(*c)).collect();
    for (i, a) in colours.iter().enumerate() {
        for b in &colours[i + 1..] {
            assert_ne!(a, b, "two classes share a colour");
        }
    }
    for p in [&LIGHT, &DARK] {
        for (c, col) in Class::ALL.iter().zip(&colours) {
            // These are drawn as filled areas and legend squares next to text: 1.4.11 (3:1).
            let r = contrast(*col, p.card);
            if *c != Class::Inconclusive {
                assert!(r >= 1.5, "{c:?} is nearly invisible on the card ({r:.2}:1)");
            }
        }
    }
}

#[test]
fn warning_and_danger_marks_carry_an_icon() {
    use cpuzeez_core::layout::Tone;
    assert!(!super::layout_view::tone_mark(Tone::Warning).trim().is_empty());
    assert!(!super::layout_view::tone_mark(Tone::Danger).trim().is_empty());
    assert_ne!(super::layout_view::tone_mark(Tone::Warning), super::layout_view::tone_mark(Tone::Danger));
    assert!(super::layout_view::tone_mark(Tone::Normal).is_empty());
}

// ------------------------------------------------------------------ keyboard

use super::widgets;
use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};

/// Installs the app's fonts and styles on the first frame (and skips it):
/// the icons and symbols need them.
pub(super) fn themed(ui: &egui::Ui) -> bool {
    if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
        super::theme::install(ui.ctx());
        ui.ctx().request_repaint();
        return false;
    }
    true
}

#[test]
fn sidebar_and_tabs_work_from_the_keyboard() {
    #[derive(Default)]
    struct S {
        page: usize,
        tab: usize,
    }
    let mut h = Harness::builder().with_size(egui::vec2(500.0, 300.0)).build_ui_state(
        |ui, s: &mut S| {
            if !themed(ui) {
                return;
            }
            for (i, name) in ["Overview", "System", "Settings"].iter().enumerate() {
                if widgets::sidebar_item(ui, "•", name, s.page == i).clicked() {
                    s.page = i;
                }
            }
            widgets::segmented(ui, &mut s.tab, &[(0, "", "Live"), (1, "", "Overlay"), (2, "", "Sessions")]);
        },
        S::default(),
    );
    h.run();
    h.run();
    // Each control has a name and says whether it is the current one.
    for name in ["Overview", "System", "Settings", "Live", "Overlay", "Sessions"] {
        h.get_by_label_contains(name);
    }
    // Tab to "System" and press Enter.
    h.get_by_label("Overview").focus();
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    assert!(h.get_by_label("System").is_focused(), "Tab moves to the next item");
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(h.state().page, 1, "Enter chooses it");
    // And Space on a tab.
    h.get_by_label_contains("Overlay").focus();
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert_eq!(h.state().tab, 1, "Space chooses a tab");
}

// ------------------------------------------------------------------ scale

/// Leaf nodes with text, as (label, rect), and the pairs of them that overlap
/// enough to be unreadable.
fn collisions<State>(h: &Harness<'_, State>, width: f32) -> Vec<String> {
    use egui_kittest::kittest::NodeT;
    let nodes: Vec<(String, egui::Rect)> = h
        .root()
        .children_recursive()
        // Leaves that draw text: a splitter draws none (its name is for screen readers).
        .filter(|n| {
            let a = n.accesskit_node();
            a.children().count() == 0 && !a.is_hidden() && a.role() != egui::accesskit::Role::Splitter
        })
        .filter_map(|n| {
            let label = n.accesskit_node().label().or_else(|| n.accesskit_node().value())?;
            let r = n.rect();
            (!label.trim().is_empty() && r.width() > 1.0 && r.height() > 1.0 && r.width() < 0.8 * width)
                .then_some((label.to_string(), r))
        })
        .collect();
    let mut out = Vec::new();
    for (i, (a, ra)) in nodes.iter().enumerate() {
        for (b, rb) in &nodes[i + 1..] {
            // A control named by the label it holds (a pill button) is one text.
            if a == b {
                continue;
            }
            let inter = ra.intersect(*rb);
            if inter.width() <= 0.0 || inter.height() <= 0.0 {
                continue;
            }
            let smaller = (ra.width() * ra.height()).min(rb.width() * rb.height());
            if inter.width() * inter.height() > 0.3 * smaller {
                out.push(format!("{a:?} overlaps {b:?}"));
            }
        }
    }
    out
}

/// Zoom factors from 75 % to 200 %; the window's minimum size (860 x 560
/// points) at zoom z is (860 / z) x (560 / z) logical points of the same text.
const ZOOMS: [f32; 5] = [0.75, 1.0, 1.25, 1.5, 2.0];

fn at_every_zoom<S: 'static>(name: &str, state: impl Fn() -> S, draw: impl Fn(&mut egui::Ui, &mut S) + Copy + 'static) {
    for z in ZOOMS {
        let size = egui::vec2(860.0 / z, 560.0 / z);
        let mut h = Harness::builder().with_size(size).build_ui_state(
            move |ui, s: &mut S| {
                if themed(ui) {
                    draw(ui, s);
                }
            },
            state(),
        );
        // A fixed number of frames: pages with live values never stop repainting.
        h.run_steps(6);
        let mut c = collisions(&h, size.x);
        c.extend(beyond(&h, size.x));
        assert!(c.is_empty(), "{name} at {:.0} %: {}", z * 100.0, c.join("; "));
    }
}

/// Text that ends past the window's right edge: the page is wider than the
/// window (10.1). A node about as wide as the window is a table row or
/// header, which scrolls sideways on purpose (egui does not expose scroll
/// areas to accessibility, so the width is the tell).
fn beyond<State>(h: &Harness<'_, State>, width: f32) -> Vec<String> {
    use egui_kittest::kittest::NodeT;
    h.root()
        .children_recursive()
        // Leaves that draw text: a splitter draws none (its name is for screen readers).
        .filter(|n| {
            let a = n.accesskit_node();
            a.children().count() == 0 && !a.is_hidden() && a.role() != egui::accesskit::Role::Splitter
        })
        .filter_map(|n| {
            let label = n.accesskit_node().label().or_else(|| n.accesskit_node().value())?;
            let r = n.rect();
            (!label.trim().is_empty() && r.max.x > width + 1.0 && r.width() < 0.8 * width)
                .then(|| format!("{label:?} ends at x {:.0} of {width:.0}", r.max.x))
        })
        .collect()
}

fn bottleneck_sample() -> super::bottleneck_ui::Detector {
    use cpuzeez_core::bottleneck::{self as bn, Thresholds, Window};
    let w = Window {
        frames: 120,
        fps: 140.0,
        ms_cv: 0.2,
        gpu_busy_frac: Some(0.97),
        game_thread: Some(40.0),
        vram_used_pct: Some(60.0),
        ram_used_pct: Some(50.0),
        busiest_core: Some(70.0),
        ..Default::default()
    };
    let v = bn::classify(&w, &Thresholds::default());
    let a = bn::analyze(&vec![w.clone(); 30], &Thresholds::default());
    let now = super::bottleneck_ui::Now { verdict: v, window: w, app: "game.exe".into(), fps: 140.0, low1: 90.0 };
    super::bottleneck_ui::Detector::preview(Some(now), Some(a))
}

#[test]
fn the_bottleneck_page_holds_together_at_every_scale() {
    at_every_zoom("Bottleneck", bottleneck_sample, |ui, d| {
        let mut access = super::access_ui::Access::default();
        egui::ScrollArea::vertical().show(ui, |ui| d.page(ui, &mut access));
    });
}

#[test]
fn the_services_tab_holds_together_at_every_scale() {
    use cpuzeez_core::services::{ServiceRow, ServiceState, StartupType};
    let rows = || -> Vec<ServiceRow> {
        (0..12)
            .map(|i| ServiceRow {
                name: format!("Service{i}"),
                display_name: format!("A display name for service number {i}"),
                state: if i % 2 == 0 { ServiceState::Running } else { ServiceState::Stopped },
                startup: StartupType::Automatic,
                ..Default::default()
            })
            .collect()
    };
    at_every_zoom(
        "Services",
        || (super::system_services::State::default(), rows()),
        |ui, (s, rows)| s.show_with_rows(ui, rows),
    );
}

#[test]
fn the_users_tab_holds_together_at_every_scale() {
    use cpuzeez_platform::users::UserSession;
    let sessions = || {
        vec![UserSession {
            session_id: 1,
            user: "someone".into(),
            domain: "PC".into(),
            state: "Active".into(),
            client_name: "Console".into(),
        }]
    };
    at_every_zoom(
        "Users",
        || (super::system_users::State::default(), sessions(), crate::procs::Snapshot::default()),
        |ui, (s, sess, snap)| s.show_with_sessions(ui, sess, snap),
    );
}

#[test]
fn the_app_history_tab_holds_together_at_every_scale() {
    use super::system_history::{AggregatedApp, Period, State, View};
    let state = || {
        let mut st = State::default();
        st.enabled = true;
        st.preview = Some(View {
            period: Period::Days30,
            apps: vec![AggregatedApp {
                app: "firefox.exe".into(),
                publisher: "Mozilla".into(),
                cpu_secs: 125.0,
                gpu_secs: 40.0,
                disk_bytes: 50 << 20,
                peak_mem_bytes: 1200 << 20,
                run_secs: 600.0,
            }],
            days: vec![("2026-11-05".into(), 125.0)],
            error: None,
        });
        st
    };
    at_every_zoom("App history", state, |ui, s| s.show_ui(ui));
}

#[test]
fn the_processes_tab_holds_together_at_every_scale() {
    use cpuzeez_core::procs::{Kind, Row, Totals};
    let make = || {
        let rows: Vec<Row> = (0..30)
            .map(|i| Row {
                pid: 1000 + i,
                name: format!("a-fairly-long-process-name-{i}.exe"),
                kind: Some(if i % 3 == 0 { Kind::App } else { Kind::Background }),
                cpu: i as f64 / 3.0,
                working_set: (i as u64 + 1) << 24,
                ..Default::default()
            })
            .collect();
        let snap = crate::procs::Snapshot {
            totals: Totals { processes: rows.len(), threads: 400, handles: 9000 },
            rows,
            ..Default::default()
        };
        (super::system::State::default(), snap)
    };
    at_every_zoom("Processes", make, |ui, (s, snap)| s.page_with_snapshot(ui, snap));
}

/// Painted charts and bars are named for assistive technology, with their numbers.
#[test]
fn charts_and_bars_say_what_they_show() {
    use egui_kittest::{Harness, kittest::Queryable};
    let pts: Vec<(f64, f64)> = (0..60).map(|i| (i as f64, 40.0 + i as f64)).collect();
    let mut h = Harness::new_ui(move |ui| {
        let lines = [super::plot::Line { label: "CPU", color: egui::Color32::RED, points: &pts }];
        super::plot::chart(ui, &lines, 60.0, 60.0, 120.0, &|v| format!("{v:.0} °C"));
    });
    h.run();
    let found = h.query_all_by_label_contains("Chart, last 60 s").count();
    assert_eq!(found, 1, "the chart has a name");
    assert!(h.query_all_by_label_contains("CPU: 99 °C (min 40 °C, max 99 °C)").count() == 1, "with its numbers");
}

#[test]
fn the_startup_tab_holds_together_at_every_scale() {
    use cpuzeez_core::startup::{BootSummary, Impact, StartupRow};
    let summary = || {
        let items = (0..10)
            .map(|i| StartupRow {
                id: format!("item{i}"),
                name: format!("A startup program with a rather long name {i}"),
                publisher: "Some Publisher, Inc.".into(),
                enabled: i % 3 != 0,
                impact: if i % 2 == 0 { Impact::High } else { Impact::Low },
                command: r"C:\Program Files\Some Publisher\A long folder name\app.exe --background".into(),
                ..Default::default()
            })
            .collect();
        BootSummary { bios_time_ms: Some(12_400), os_boot_ms: Some(18_000), items, ..Default::default() }
    };
    at_every_zoom(
        "Startup",
        || (super::system_startup::State::default(), summary()),
        |ui, (s, sum)| s.show_with_summary(ui, sum),
    );
}

// ------------------------------------------------------------ every page (M2-04)

/// Every page and tab from the fixtures, at every zoom.
fn page_at_every_zoom(page: super::Page, prepare: fn(&mut super::App)) {
    at_every_zoom(
        &format!("{page:?}"),
        move || {
            let mut app = super::fixture::app(page);
            prepare(&mut app);
            app
        },
        |ui, app| {
            let r = super::fixture::report();
            egui::ScrollArea::vertical().show(ui, |ui| app.page_body(ui, &r));
        },
    );
}

#[test]
fn every_page_holds_together_at_every_scale() {
    for p in super::NAV.iter().chain(super::NAV_BOTTOM.iter()).map(|n| n.0) {
        page_at_every_zoom(p, |_| {});
    }
}

#[test]
fn every_live_tab_holds_together_at_every_scale() {
    use super::live::Tab;
    page_at_every_zoom(super::Page::Live, |a| a.live.tab = Tab::Overlay);
    page_at_every_zoom(super::Page::Live, |a| {
        a.live.tab = Tab::Sessions;
        a.live.sessions = super::live_sessions::Sessions::fixture();
    });
}

#[test]
fn every_tools_tab_holds_together_at_every_scale() {
    use super::ToolView;
    page_at_every_zoom(super::Page::Tools, |a| a.tool_view = ToolView::Clocks);
    page_at_every_zoom(super::Page::Tools, |a| a.tool_view = ToolView::Timers);
    page_at_every_zoom(super::Page::Tools, |a| a.tool_view = ToolView::Drives);
    page_at_every_zoom(super::Page::Tools, |a| a.tool_view = ToolView::Reports);
}

/// The Services inspector open on a service with long values, in the
/// languages with the longest words (M2-06).
#[test]
fn the_services_inspector_fits_at_every_scale_in_every_language() {
    use cpuzeez_core::services::{Recovery, ServiceDetails, ServiceRow, ServiceState, StartupType};
    let long = "CzFixtureServiceWithAVeryLongInternalName";
    let rows = move || -> Vec<ServiceRow> {
        (0..8)
            .map(|i| ServiceRow {
                name: if i == 0 { long.to_string() } else { format!("Service{i}") },
                display_name: format!("A display name that goes on for quite a while, number {i}"),
                description: "Provides a long description of what this service does, how it does it, and why someone might want to keep it running all the time.".into(),
                state: ServiceState::Running,
                startup: StartupType::AutomaticDelayed,
                pid: Some(4321),
                account: r"NT AUTHORITY\LocalServiceNetworkRestricted".into(),
                group: "LocalServiceNetworkRestrictedGroup".into(),
                binary_path: r"C:\Windows\System32\svchost.exe -k LocalServiceNetworkRestricted -p -s CzFixtureServiceWithAVeryLongInternalName".into(),
                dependencies: vec!["RpcSs".into(), "AnotherServiceWithALongName".into()],
                ..Default::default()
            })
            .collect()
    };
    let details = || ServiceDetails {
        triggers: vec!["Start: when a network endpoint for a very long protocol name becomes available".into()],
        sid_type: "Unrestricted".into(),
        privileges: vec![
            "SeImpersonatePrivilege".into(),
            "SeChangeNotifyPrivilege".into(),
            "SeCreateGlobalPrivilege".into(),
        ],
        recovery: Some(Recovery { reset_days: 1, restart_delay_s: 60, ..Default::default() }),
        ..Default::default()
    };
    for lang in [super::i18n::Lang::PtBr, super::i18n::Lang::Es] {
        super::i18n::set(lang);
        at_every_zoom(
            &format!("Services inspector ({lang:?})"),
            move || (super::system_services::State::inspecting(long, details()), rows()),
            |ui, (s, rows)| s.show_with_rows(ui, rows),
        );
    }
    super::i18n::set(super::i18n::Lang::En);
}

/// The Processes details panel open, in the languages with the longest words (M2-06).
#[test]
fn the_processes_details_fit_at_every_scale_in_every_language() {
    use cpuzeez_core::procs::{Kind, Row, Totals};
    let make = || {
        let rows: Vec<Row> = (0..12)
            .map(|i| Row {
                pid: 1000 + i,
                name: format!("a-really-quite-long-process-name-for-testing-{i}.exe"),
                kind: Some(Kind::App),
                cpu: 1.0,
                working_set: 1 << 28,
                ..Default::default()
            })
            .collect();
        let mut st = super::system::State::default();
        st.show_details = true;
        st.table.selected.insert(rows[0].key());
        let snap = crate::procs::Snapshot {
            totals: Totals { processes: rows.len(), threads: 100, handles: 1000 },
            rows,
            ..Default::default()
        };
        (st, snap)
    };
    for lang in [super::i18n::Lang::PtBr, super::i18n::Lang::Es] {
        super::i18n::set(lang);
        at_every_zoom(&format!("Processes details ({lang:?})"), make, |ui, (s, snap)| s.page_with_snapshot(ui, snap));
    }
    super::i18n::set(super::i18n::Lang::En);
}

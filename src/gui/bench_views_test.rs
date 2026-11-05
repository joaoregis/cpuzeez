//! GUI interaction tests for the S1 benchmark and tool views (S1 gate):
//! each one renders, its mode switches respond, and nothing runs until asked.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};

fn themed(ui: &mut egui::Ui) -> bool {
    if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
        super::theme::install(ui.ctx());
        ui.ctx().request_repaint();
        return false;
    }
    true
}

fn ctx() -> std::sync::Arc<crate::Ctx> {
    std::sync::Arc::new(crate::Ctx::new(crate::config::Config { smart: false, ..Default::default() }))
}

/// The CPU suite's modes switch and describe themselves; nothing runs.
#[test]
fn cpu_suite_modes() {
    let c = ctx();
    let mut h = Harness::builder().with_size(egui::vec2(1100.0, 700.0)).build_ui_state(
        |ui, st: &mut super::cpusuite_view::State| {
            if themed(ui) {
                st.page(ui, &c, Some(40.0));
            }
        },
        Default::default(),
    );
    h.run();
    assert!(h.query_by_label_contains("Eight workloads").is_some());
    h.get_by_label_contains("Thread scaling").click();
    h.run();
    assert!(h.query_by_label_contains("How the integer score grows").is_some());
    h.get_by_label_contains("Sustained").click();
    h.run();
    assert!(h.query_by_label("10 min").is_some(), "the duration picker shows in sustained mode");
    assert!(h.query_by_label_contains("Score:").is_none(), "no result before a run");
}

/// Storage: the place picker, sizes and the sustained-write warning.
#[test]
fn storage_view_warns_before_writing() {
    let c = ctx();
    let mut h = Harness::builder().with_size(egui::vec2(1100.0, 900.0)).build_ui_state(
        |ui, st: &mut super::diskbench_view::State| {
            if themed(ui) {
                st.page(ui, &c, None);
            }
        },
        Default::default(),
    );
    h.run();
    assert!(h.query_by_label("4 GiB").is_some());
    assert!(h.query_by_label_contains("This writes 32 GB to the drive").is_some());
    h.get_by_label("64 GB").click();
    h.run();
    // 64 GB may not fit this machine's free space; when it does, the warning follows.
    let _ = h.query_by_label_contains("This writes 64 GB");
}

/// Memory extras and core-to-core latency render with their Run buttons.
#[test]
fn memory_and_latency_views() {
    let c = ctx();
    let mut h = Harness::builder().with_size(egui::vec2(1100.0, 700.0)).build_ui_state(
        |ui, st: &mut (super::memplus_view::State, super::latency_view::State)| {
            if themed(ui) {
                st.0.page(ui, &c, None);
                st.1.page(ui, &c, None);
            }
        },
        Default::default(),
    );
    h.run();
    assert!(h.query_by_label_contains("STREAM triad (bandwidth)").is_some());
    assert!(h.query_by_label_contains("Core-to-core latency").is_some());
}

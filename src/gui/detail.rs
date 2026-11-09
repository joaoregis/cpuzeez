//! The detail-level switch (S1-U01) every information page puts in its
//! header, and the level the page is drawing at (read by `boxed`).

use super::i18n::tr;
use cpuzeez_core::detail::Level;
use eframe::egui;

pub const PT_BR: &[(&str, &str)] = &[
    ("Basic", "Básico"),
    ("Advanced", "Avançado"),
    ("Expert", "Especialista"),
    ("The essentials", "O essencial"),
    ("Everything CPU ZeeZ reads", "Tudo o que o CPU ZeeZ lê"),
    ("Plus raw values: CPUID leaves, registers", "Mais valores brutos: folhas CPUID, registradores"),
    ("CPUID leaves (thread 0)", "Folhas CPUID (thread 0)"),
    ("MSRs (thread 0)", "MSRs (thread 0)"),
];

pub const ES: &[(&str, &str)] = &[
    ("Basic", "Básico"),
    ("Advanced", "Avanzado"),
    ("Expert", "Experto"),
    ("The essentials", "Lo esencial"),
    ("Everything CPU ZeeZ reads", "Todo lo que lee CPU ZeeZ"),
    ("Plus raw values: CPUID leaves, registers", "Más valores en bruto: hojas CPUID, registros"),
    ("CPUID leaves (thread 0)", "Hojas CPUID (hilo 0)"),
    ("MSRs (thread 0)", "MSRs (hilo 0)"),
];

fn id() -> egui::Id {
    egui::Id::new("cz-detail-level")
}

/// The level the current page draws at.
pub fn get(ctx: &egui::Context) -> Level {
    ctx.data(|d| d.get_temp(id())).unwrap_or_default()
}

pub fn set(ctx: &egui::Context, l: Level) {
    ctx.data_mut(|d| d.insert_temp(id(), l));
}

/// Basic · Advanced · Expert; true when the choice changed.
pub fn switch(ui: &mut egui::Ui, l: &mut Level) -> bool {
    let before = *l;
    // Drawn right to left (page header actions): Expert first.
    for (lv, hint) in [
        (Level::Expert, "Plus raw values: CPUID leaves, registers"),
        (Level::Advanced, "Everything CPU ZeeZ reads"),
        (Level::Basic, "The essentials"),
    ] {
        ui.selectable_value(l, lv, tr(lv.label())).on_hover_text(tr(hint));
    }
    *l != before
}

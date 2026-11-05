//! Benchmarks › Latency (S1-K03): the core-to-core latency heat map.

use super::i18n::{tr, trf};
use super::theme::{self, icon};
use super::widgets;
use crate::Ctx;
use eframe::egui::{self, Color32, RichText};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub const PT_BR: &[(&str, &str)] = &[
    ("Core-to-core latency", "Latência entre núcleos"),
    (
        "How long one core takes to see a value another core wrote. Cores that share an L3 cache answer fastest.",
        "Quanto tempo um núcleo leva para ver um valor que outro escreveu. Núcleos que compartilham o cache L3 respondem mais rápido.",
    ),
    ("Run", "Executar"),
    ("Measuring… {} of {} pairs", "Medindo… {} de {} pares"),
    ("min {} · median {} · max {} (ns, one way)", "mín {} · mediana {} · máx {} (ns, um sentido)"),
    (
        "This OS doesn't let a program pin threads to cores.",
        "Este sistema não deixa um programa fixar threads em núcleos.",
    ),
    ("Latency", "Latência"),
];

pub const ES: &[(&str, &str)] = &[
    ("Core-to-core latency", "Latencia entre núcleos"),
    (
        "How long one core takes to see a value another core wrote. Cores that share an L3 cache answer fastest.",
        "Cuánto tarda un núcleo en ver un valor que escribió otro. Los núcleos que comparten la caché L3 responden más rápido.",
    ),
    ("Run", "Ejecutar"),
    ("Measuring… {} of {} pairs", "Midiendo… {} de {} pares"),
    ("min {} · median {} · max {} (ns, one way)", "mín {} · mediana {} · máx {} (ns, un sentido)"),
    ("This OS doesn't let a program pin threads to cores.", "Este sistema no permite fijar hilos a núcleos."),
    ("Latency", "Latencia"),
];

type Matrix = Vec<Vec<Option<f64>>>;

#[derive(Default)]
pub struct State {
    running: Option<(Arc<AtomicUsize>, usize, Arc<Mutex<Option<Matrix>>>)>,
    result: Option<Matrix>,
}

impl State {
    pub fn page(&mut self, ui: &mut egui::Ui, ctx: &Ctx) {
        let p = theme::palette(ui);
        let c = crate::cpu::get(ctx);
        // One thread per physical core: SMT siblings share the core's caches.
        let cpus: Vec<usize> = c.logical.iter().filter(|l| l.smt == 0).map(|l| l.cpu).collect();
        let finished =
            self.running.as_ref().and_then(|(_, _, slot)| slot.lock().unwrap_or_else(|e| e.into_inner()).take());
        if let Some(m) = finished {
            self.result = Some(m);
            self.running = None;
        }
        widgets::card(ui, icon::CPU, tr("Core-to-core latency"), |ui| {
            ui.label(
                RichText::new(tr("How long one core takes to see a value another core wrote. Cores that share an L3 cache answer fastest."))
                    .color(p.text_secondary),
            );
            if cfg!(target_os = "macos") {
                ui.label(RichText::new(tr("This OS doesn't let a program pin threads to cores.")).color(p.text_muted));
                return;
            }
            match &self.running {
                Some((done, total, _)) => {
                    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
                    let d = done.load(Ordering::Relaxed);
                    widgets::progress(
                        ui,
                        d as f32 / (*total).max(1) as f32,
                        &trf("Measuring… {} of {} pairs", &[&d, total]),
                    );
                }
                None => {
                    if widgets::primary_button(ui, &format!("{}  {}", icon::PLAY, tr("Run"))).clicked()
                        && cpus.len() > 1
                    {
                        let (done, slot) = (Arc::new(AtomicUsize::new(0)), Arc::new(Mutex::new(None)));
                        let (d, s, list) = (done.clone(), slot.clone(), cpus.clone());
                        std::thread::spawn(move || {
                            let m = crate::latency::measure(&list, crate::latency::ROUNDS, &d);
                            *s.lock().unwrap_or_else(|e| e.into_inner()) = Some(m);
                        });
                        self.running = Some((done, cpus.len() * (cpus.len() - 1) / 2, slot));
                    }
                }
            }
        });
        ui.add_space(theme::GAP);
        if let Some(m) = &self.result {
            heat_map(ui, m);
        }
    }
}

fn heat_map(ui: &mut egui::Ui, m: &Matrix) {
    let p = theme::palette(ui);
    let Some((lo, mid, hi)) = crate::latency::summary(m) else { return };
    let color = |v: f64| -> Color32 {
        let t = if hi > lo { ((v - lo) / (hi - lo)).clamp(0.0, 1.0) as f32 } else { 0.0 };
        let (a, b) = (p.success, p.danger);
        let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
        Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b())).gamma_multiply(0.75)
    };
    widgets::card(ui, "", "", |ui| {
        ui.label(
            RichText::new(trf(
                "min {} · median {} · max {} (ns, one way)",
                &[&format!("{lo:.0}"), &format!("{mid:.0}"), &format!("{hi:.0}")],
            ))
            .color(p.text_secondary),
        );
        ui.add_space(6.0);
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("latency-grid").spacing([2.0, 2.0]).show(ui, |ui| {
                ui.label("");
                for j in 0..m.len() {
                    ui.label(RichText::new(j.to_string()).small().color(p.text_muted));
                }
                ui.end_row();
                for (i, row) in m.iter().enumerate() {
                    ui.label(RichText::new(i.to_string()).small().color(p.text_muted));
                    for v in row {
                        let (rect, r) = ui.allocate_exact_size(egui::vec2(40.0, 24.0), egui::Sense::hover());
                        if let Some(v) = v {
                            ui.painter().rect_filled(rect, 3.0, color(*v));
                            ui.painter().text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{v:.0}"),
                                egui::FontId::monospace(11.0),
                                p.text,
                            );
                            r.on_hover_text(format!("{v:.1} ns"));
                        } else {
                            ui.painter().rect_filled(rect, 3.0, p.card_hover);
                        }
                    }
                    ui.end_row();
                }
            });
        });
    });
}

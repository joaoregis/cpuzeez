//! Bench tab: single-thread and multi-thread runs, thread count, SIMD variant,
//! core set (hybrid CPUs), and Stress CPU. Scores are cpuzeez's own workload.

use crate::Ctx;
use crate::bench::{self, Bench, Score, Variant};
use eframe::egui::{self, RichText};

pub struct State {
    variant: Variant,
    threads: usize,
    cluster: String,
    st: Option<Score>,
    mt: Option<Score>,
    running: Option<(bool, Bench)>, // (multi-thread?, run)
    queue_mt: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            variant: Variant::best(),
            threads: crate::os::logical_cpus(),
            cluster: "All".into(),
            st: None,
            mt: None,
            running: None,
            queue_mt: false,
        }
    }
}

fn bar(ui: &mut egui::Ui, label: &str, score: Option<Score>, best: f64, progress: Option<f32>) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(label).strong());
        ui.horizontal(|ui| {
            ui.label("This Processor");
            let (frac, text) = match (progress, score) {
                (Some(p), _) => (p, format!("{:.0}%", p * 100.0)),
                (None, Some(s)) => (
                    (s.points / best).min(1.0) as f32,
                    format!("{:.1}  ({} thr, {})", s.points, s.threads, s.variant.label()),
                ),
                (None, None) => (0.0, "-".into()),
            };
            ui.add(egui::ProgressBar::new(frac).text(text));
        });
    });
}

impl State {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &Ctx, mon: &mut super::sensors::Monitor) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        // Advance the ST -> MT sequence.
        if let Some((mt, b)) = &self.running
            && b.finished()
        {
            let (mt, r) = (*mt, b.result());
            if mt {
                self.mt = r;
            } else {
                self.st = r;
            }
            self.running = None;
            if !mt && self.queue_mt {
                self.queue_mt = false;
                self.running = Some((true, self.spawn(ctx, self.threads)));
            }
        }
        let prog = |mt: bool, s: &Self| s.running.as_ref().filter(|r| r.0 == mt).map(|r| r.1.progress());
        let best = [self.st, self.mt].iter().flatten().map(|s| s.points).fold(1.0, f64::max);
        bar(ui, "CPU Single Thread", self.st, best, prog(false, self));
        bar(ui, &format!("CPU Multi Thread ({} threads)", self.threads), self.mt, best, prog(true, self));

        let idle = self.running.is_none();
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(idle, |ui| {
                ui.label("Threads");
                let max = crate::os::logical_cpus();
                ui.add(egui::DragValue::new(&mut self.threads).range(1..=max));
                egui::ComboBox::from_id_salt("variant").selected_text(self.variant.label()).show_ui(ui, |ui| {
                    for v in Variant::all() {
                        ui.selectable_value(&mut self.variant, v, v.label());
                    }
                });
                let sets: Vec<String> = std::iter::once("All".to_string())
                    .chain(crate::cpu::get(ctx).clusters.iter().map(|c| c.kind.clone()))
                    .collect();
                if sets.len() > 2 {
                    egui::ComboBox::from_id_salt("cluster").selected_text(&self.cluster).show_ui(ui, |ui| {
                        for s in sets {
                            ui.selectable_value(&mut self.cluster, s.clone(), s);
                        }
                    });
                }
            });
        });
        ui.horizontal(|ui| {
            if idle {
                if ui.button("Bench CPU").clicked() {
                    self.st = None;
                    self.mt = None;
                    self.queue_mt = true;
                    self.running = Some((false, self.spawn(ctx, 1)));
                }
            } else if ui.button("Stop").clicked() {
                self.queue_mt = false;
                self.running = None; // dropping cancels
            }
            // ---- monitored stress (gui/stress.rs) ----
            let label = if mon.stress.running() { "Stop Stress" } else { "Stress CPU" };
            if ui.add_enabled(idle, egui::Button::new(label)).clicked() {
                if mon.stress.running() {
                    mon.stress.stop(None);
                } else {
                    mon.start_stress(self.threads);
                }
            }
        });
        if mon.stress.running() {
            super::stress::page(ui, mon, false);
        }
        // ---- end monitored stress ----
        ui.separator();
        ui.label(RichText::new("cpuzeez's own workload: scores are not comparable with CPU-Z's.").small().weak());
    }

    /// For `-screenshots`: run ST+MT once; true when both results are in.
    pub fn autorun(&mut self, ctx: &Ctx) -> bool {
        if self.st.is_none() && self.running.is_none() && !self.queue_mt {
            self.queue_mt = true;
            self.running = Some((false, self.spawn(ctx, 1)));
        }
        self.mt.is_some()
    }

    fn spawn(&self, ctx: &Ctx, threads: usize) -> Bench {
        let set = if self.cluster == "All" { Vec::new() } else { bench::cluster_cpus(ctx, &self.cluster) };
        if threads == 1 {
            // Single thread: first CPU of the chosen set, else unpinned.
            return match set.first() {
                Some(&c) => Bench::start_pinned(&[c], self.variant),
                None => Bench::start(1, self.variant),
            };
        }
        if set.is_empty() {
            let all: Vec<usize> = (0..threads).collect();
            Bench::start_pinned(&all, self.variant)
        } else {
            Bench::start_pinned(&set[..threads.min(set.len())], self.variant)
        }
    }
}

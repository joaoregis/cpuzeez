//! Bench tab: CPU (single-thread and multi-thread runs, thread count, SIMD
//! variant, core set on hybrid CPUs, Stress CPU) and Memory (latency by
//! working-set size, bandwidth per cache level / DRAM). Scores are cpuzeez's
//! own workloads; results can be saved to the benchmark history.

use super::i18n::{tr, trf};
use super::theme::{self, icon};
use super::tools::{Series, line_chart};
use super::widgets;
use crate::Ctx;
use crate::bench::{self, Bench, Score, Variant};
use crate::membench::{MemBench, MemResult};
use eframe::egui::{self, RichText};

pub const PT_BR: &[(&str, &str)] = &[
    ("CPU", "CPU"),
    ("Memory", "Memória"),
    ("CPU Single Thread", "CPU Single Thread"),
    ("CPU Multi Thread ({} threads)", "CPU Multi Thread ({} threads)"),
    ("This Processor", "Este processador"),
    ("Threads", "Threads"),
    ("All", "Todos"),
    ("Bench CPU", "Testar CPU"),
    ("Stop", "Parar"),
    ("Stress CPU", "Estressar CPU"),
    ("Stop Stress", "Parar estresse"),
    ("Stressing {} threads…", "Estressando {} threads…"),
    (
        "cpuzeez's own workload: scores are not comparable with CPU-Z's.",
        "Carga de trabalho própria do cpuzeez: as pontuações não são comparáveis com as do CPU-Z.",
    ),
    ("{} thr", "{} thr"),
    ("Label (optional)", "Rótulo (opcional)"),
    ("Save to history", "Salvar no histórico"),
    ("Saved to history.", "Salvo no histórico."),
    ("Quick", "Rápido"),
    ("Full", "Completo"),
    ("Run memory benchmark", "Rodar benchmark de memória"),
    ("Latency", "Latência"),
    ("Latency by block size", "Latência por tamanho de bloco"),
    ("DRAM latency", "Latência da DRAM"),
    ("Bandwidth", "Largura de banda"),
    ("Level", "Nível"),
    ("Read", "Leitura"),
    ("Write", "Escrita"),
    ("Copy", "Cópia"),
    ("ST", "ST"),
    ("MT", "MT"),
    ("Run the benchmark to see latency and bandwidth.", "Rode o benchmark para ver latência e largura de banda."),
    (
        "cpuzeez's own method: results are not comparable with other tools' numbers.",
        "Método próprio do cpuzeez: os resultados não são comparáveis com os de outras ferramentas.",
    ),
    (
        "Finished, no result (cancelled or not supported here).",
        "Concluído sem resultado (cancelado ou sem suporte aqui).",
    ),
];

pub const ES: &[(&str, &str)] = &[
    ("CPU", "CPU"),
    ("Memory", "Memoria"),
    ("CPU Single Thread", "CPU Single Thread"),
    ("CPU Multi Thread ({} threads)", "CPU Multi Thread ({} hilos)"),
    ("This Processor", "Este procesador"),
    ("Threads", "Hilos"),
    ("All", "Todos"),
    ("Bench CPU", "Probar CPU"),
    ("Stop", "Detener"),
    ("Stress CPU", "Estresar CPU"),
    ("Stop Stress", "Detener estrés"),
    ("Stressing {} threads…", "Estresando {} hilos…"),
    (
        "cpuzeez's own workload: scores are not comparable with CPU-Z's.",
        "Carga de trabajo propia de cpuzeez: las puntuaciones no son comparables con las de CPU-Z.",
    ),
    ("{} thr", "{} hilos"),
    ("Label (optional)", "Etiqueta (opcional)"),
    ("Save to history", "Guardar en el historial"),
    ("Saved to history.", "Guardado en el historial."),
    ("Quick", "Rápida"),
    ("Full", "Completa"),
    ("Run memory benchmark", "Ejecutar prueba de memoria"),
    ("Latency", "Latencia"),
    ("Latency by block size", "Latencia por tamaño de bloque"),
    ("DRAM latency", "Latencia de la DRAM"),
    ("Bandwidth", "Ancho de banda"),
    ("Level", "Nivel"),
    ("Read", "Lectura"),
    ("Write", "Escritura"),
    ("Copy", "Copia"),
    ("ST", "ST"),
    ("MT", "MT"),
    ("Run the benchmark to see latency and bandwidth.", "Ejecute la prueba para ver la latencia y el ancho de banda."),
    (
        "cpuzeez's own method: results are not comparable with other tools' numbers.",
        "Método propio de cpuzeez: los resultados no son comparables con los de otras herramientas.",
    ),
    (
        "Finished, no result (cancelled or not supported here).",
        "Terminado sin resultado (cancelado o no compatible aquí).",
    ),
];

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Cpu,
    Memory,
}

pub struct State {
    mode: Mode,
    variant: Variant,
    threads: usize,
    cluster: String,
    st: Option<Score>,
    mt: Option<Score>,
    running: Option<(bool, Bench)>, // (multi-thread?, run)
    queue_mt: bool,
    label: String,
    saved: String,
    mem_quick: bool,
    mem: Option<MemBench>,
    mem_result: Option<MemResult>,
    mem_done: bool,
    mem_label: String,
    mem_saved: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            mode: Mode::Cpu,
            variant: Variant::best(),
            threads: crate::os::logical_cpus(),
            cluster: "All".into(),
            st: None,
            mt: None,
            running: None,
            queue_mt: false,
            label: String::new(),
            saved: String::new(),
            mem_quick: true,
            mem: None,
            mem_result: None,
            mem_done: false,
            mem_label: String::new(),
            mem_saved: String::new(),
        }
    }
}

fn bar(ui: &mut egui::Ui, label: &str, score: Option<Score>, best: f64, progress: Option<f32>) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(label).strong());
        ui.horizontal(|ui| {
            ui.label(tr("This Processor"));
            let (frac, text) = match (progress, score) {
                (Some(p), _) => (p, format!("{:.0}%", p * 100.0)),
                (None, Some(s)) => (
                    (s.points / best).min(1.0) as f32,
                    format!("{:.1}  ({}, {})", s.points, trf("{} thr", &[&s.threads]), s.variant.label()),
                ),
                (None, None) => (0.0, "-".into()),
            };
            ui.add(egui::ProgressBar::new(frac).text(text));
        });
    });
}

/// Label field + "Save to history" button; true when clicked.
fn save_row(ui: &mut egui::Ui, label: &mut String, status: &str, enabled: bool) -> bool {
    let mut clicked = false;
    ui.horizontal_wrapped(|ui| {
        ui.add(egui::TextEdit::singleline(label).hint_text(tr("Label (optional)")).desired_width(160.0));
        clicked = ui.add_enabled(enabled, egui::Button::new(tr("Save to history"))).clicked();
        ui.label(RichText::new(status).small().weak());
    });
    clicked
}

/// "4 KiB", "32 MiB"…
pub fn bytes_text(b: f64) -> String {
    let (v, u) = if b >= (1u64 << 30) as f64 {
        (b / (1u64 << 30) as f64, "G")
    } else if b >= (1 << 20) as f64 {
        (b / (1 << 20) as f64, "M")
    } else {
        (b / 1024.0, "K")
    };
    if v.fract() == 0.0 { format!("{v:.0}{u}") } else { format!("{v:.1}{u}") }
}

impl State {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &Ctx, mon: &mut super::sensors::Monitor) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mode, Mode::Cpu, tr("CPU"));
            ui.selectable_value(&mut self.mode, Mode::Memory, tr("Memory"));
        });
        ui.separator();
        match self.mode {
            Mode::Cpu => self.cpu_page(ui, ctx, mon),
            Mode::Memory => self.memory_page(ui, ctx),
        }
    }

    /// CPU benchmark (existing CPU-Z style layout) with the monitored stress.
    pub fn cpu_page(&mut self, ui: &mut egui::Ui, ctx: &Ctx, mon: &mut super::sensors::Monitor) {
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
        bar(ui, tr("CPU Single Thread"), self.st, best, prog(false, self));
        bar(ui, &trf("CPU Multi Thread ({} threads)", &[&self.threads]), self.mt, best, prog(true, self));

        let idle = self.running.is_none();
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(idle, |ui| {
                ui.label(tr("Threads"));
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
                    let shown = |s: &str| if s == "All" { tr("All").to_string() } else { s.to_string() };
                    egui::ComboBox::from_id_salt("cluster").selected_text(shown(&self.cluster)).show_ui(ui, |ui| {
                        for s in sets {
                            let t = shown(&s);
                            ui.selectable_value(&mut self.cluster, s, t);
                        }
                    });
                }
            });
        });
        ui.horizontal(|ui| {
            if idle {
                if ui.button(tr("Bench CPU")).clicked() {
                    self.st = None;
                    self.mt = None;
                    self.saved.clear();
                    self.queue_mt = true;
                    self.running = Some((false, self.spawn(ctx, 1)));
                }
            } else if ui.button(tr("Stop")).clicked() {
                self.queue_mt = false;
                self.running = None; // dropping cancels
            }
            // ---- monitored stress (gui/stress.rs) ----
            let label = if mon.stress.running() { tr("Stop Stress") } else { tr("Stress CPU") };
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
        let have = idle && (self.st.is_some() || self.mt.is_some());
        if save_row(ui, &mut self.label, &self.saved, have && self.saved.is_empty()) {
            for (kind, s) in [("CPU Single Thread", self.st), ("CPU Multi Thread", self.mt)] {
                let Some(s) = s else { continue };
                let mut e = crate::history::system_entry(ctx, kind, s.points, "points");
                e.label = self.label.clone();
                e.threads = s.threads;
                e.variant = s.variant.label().to_string();
                crate::history::add(e);
            }
            self.saved = tr("Saved to history.").into();
        }
        ui.separator();
        ui.label(RichText::new(tr("cpuzeez's own workload: scores are not comparable with CPU-Z's.")).small().weak());
    }

    /// Memory benchmark: quick/full run, progress + stage, latency chart,
    /// bandwidth table, save to history.
    pub fn memory_page(&mut self, ui: &mut egui::Ui, ctx: &Ctx) {
        if let Some(m) = &self.mem {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
            if m.finished() {
                self.mem_result = m.result();
                self.mem = None;
                self.mem_done = true;
            }
        }
        let p = theme::palette(ui);
        widgets::card(ui, icon::MEMORY, tr("Memory"), |ui| {
            ui.horizontal(|ui| {
                let idle = self.mem.is_none();
                ui.add_enabled_ui(idle, |ui| {
                    ui.selectable_value(&mut self.mem_quick, true, tr("Quick"));
                    ui.selectable_value(&mut self.mem_quick, false, tr("Full"));
                });
                if idle {
                    if widgets::primary_button(ui, &format!("{} {}", icon::PLAY, tr("Run memory benchmark"))).clicked()
                    {
                        self.start_mem(ctx);
                    }
                } else if ui.button(format!("{} {}", icon::STOP, tr("Stop"))).clicked()
                    && let Some(m) = self.mem.take()
                {
                    m.cancel();
                }
            });
            if let Some(m) = &self.mem {
                widgets::progress(ui, m.progress(), &format!("{:.0}%", m.progress() * 100.0));
                ui.label(RichText::new(m.stage()).small().color(p.text_secondary));
            }
            ui.label(
                RichText::new(tr("cpuzeez's own method: results are not comparable with other tools' numbers."))
                    .small()
                    .color(p.text_muted),
            );
        });
        ui.add_space(theme::GAP);
        let Some(r) = &self.mem_result else {
            let msg = if self.mem_done && self.mem.is_none() {
                "Finished, no result (cancelled or not supported here)."
            } else {
                "Run the benchmark to see latency and bandwidth."
            };
            widgets::empty_state(ui, icon::GAUGE, tr("Latency by block size"), tr(msg));
            return;
        };
        let title = if r.dram_latency_ns > 0.0 {
            format!("{}  ·  {} {:.1} ns", tr("Latency by block size"), tr("DRAM latency"), r.dram_latency_ns)
        } else {
            tr("Latency by block size").to_string()
        };
        widgets::card(ui, icon::CHART_LINE, &title, |ui| {
            let s = [Series {
                name: String::new(),
                color: p.series[0],
                points: r.latency.iter().map(|p| (p.bytes as f64, p.ns)).collect(),
            }];
            line_chart(ui, &s, true, &bytes_text, &|y| format!("{y:.0} ns"));
        });
        ui.add_space(theme::GAP);
        widgets::card(ui, icon::SPEEDOMETER, &format!("{} (GB/s)", tr("Bandwidth")), |ui| {
            egui::Grid::new("mem-bw").striped(true).spacing([18.0, 4.0]).show(ui, |ui| {
                for h in ["Level", "Threads", "Read", "Write", "Copy"] {
                    ui.label(RichText::new(tr(h)).strong());
                }
                ui.end_row();
                for b in &r.bandwidth {
                    ui.label(&b.level);
                    ui.label(format!("{} ({})", tr(if b.threads > 1 { "MT" } else { "ST" }), b.threads));
                    for v in [b.read_gbs, b.write_gbs, b.copy_gbs] {
                        ui.add(egui::Label::new(RichText::new(format!("{v:.1}")).monospace()).extend());
                    }
                    ui.end_row();
                }
            });
        });
        ui.add_space(theme::GAP);
        let can_save = r.dram_latency_ns > 0.0 && self.mem_saved.is_empty();
        if save_row(ui, &mut self.mem_label, &self.mem_saved, can_save) {
            let mut e = crate::history::system_entry(ctx, "Memory", r.dram_latency_ns, "ns");
            e.label = self.mem_label.clone();
            e.threads = r.bandwidth.iter().map(|b| b.threads).max().unwrap_or(1);
            e.variant = if self.mem_quick { "Quick" } else { "Full" }.to_string();
            e.details = serde_json::to_value(r).unwrap_or_default();
            crate::history::add(e);
            self.mem_saved = tr("Saved to history.").into();
        }
    }

    fn start_mem(&mut self, ctx: &Ctx) {
        self.mem_result = None;
        self.mem_done = false;
        self.mem_saved.clear();
        self.mem = Some(MemBench::start(ctx, self.mem_quick));
    }

    /// For `-screenshots`: run ST+MT once; true when both results are in.
    pub fn autorun(&mut self, ctx: &Ctx) -> bool {
        self.mode = Mode::Cpu;
        if self.st.is_none() && self.running.is_none() && !self.queue_mt {
            self.queue_mt = true;
            self.running = Some((false, self.spawn(ctx, 1)));
        }
        self.mt.is_some()
    }

    /// For `-screenshots`: one quick memory run; true when it finished.
    pub fn autorun_memory(&mut self, ctx: &Ctx) -> bool {
        self.mode = Mode::Memory;
        if self.mem.is_none() && !self.mem_done {
            self.mem_quick = true;
            self.start_mem(ctx);
        }
        self.mem_done
    }

    /// Results of this session, as history entries (screenshot preview).
    pub fn session_entries(&self, ctx: &Ctx) -> Vec<crate::history::Entry> {
        let mut v = Vec::new();
        for (kind, s) in [("CPU Single Thread", self.st), ("CPU Multi Thread", self.mt)] {
            if let Some(s) = s {
                let mut e = crate::history::system_entry(ctx, kind, s.points, "points");
                e.threads = s.threads;
                e.variant = s.variant.label().into();
                v.push(e);
            }
        }
        if let Some(r) = self.mem_result.as_ref().filter(|r| r.dram_latency_ns > 0.0) {
            v.push(crate::history::system_entry(ctx, "Memory", r.dram_latency_ns, "ns"));
        }
        v
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

#[cfg(test)]
mod tests {
    #[test]
    fn sizes() {
        assert_eq!(super::bytes_text(4096.0), "4K");
        assert_eq!(super::bytes_text(1536.0 * 1024.0), "1.5M");
        assert_eq!(super::bytes_text((1u64 << 30) as f64), "1G");
    }
}

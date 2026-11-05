//! Benchmarks › Memory › More tests (S1-K04): STREAM triad, GUPS, the
//! huge-page effect and latency per CCD / NUMA node.

use super::benchkit_view::{self as kit, Saved};
use super::i18n::{tr, trf};
use super::theme::{self, icon};
use super::widgets;
use crate::Ctx;
use crate::benchkit::{self, Job, Sub};
use eframe::egui::{self, RichText};

pub const PT_BR: &[(&str, &str)] = &[
    ("More memory tests", "Mais testes de memória"),
    (
        "STREAM triad (bandwidth), GUPS (random updates), a random walk on small and on large pages (the cost of TLB misses) and DRAM latency from each CCD or NUMA node.",
        "STREAM triad (largura de banda), GUPS (atualizações aleatórias), um passeio aleatório em páginas pequenas e grandes (o custo das falhas de TLB) e a latência da DRAM a partir de cada CCD ou nó NUMA.",
    ),
    ("STREAM triad", "STREAM triad"),
    ("GUPS", "GUPS"),
    ("Random walk, 4 KiB pages", "Passeio aleatório, páginas de 4 KiB"),
    ("Random walk, large pages", "Passeio aleatório, páginas grandes"),
    ("Latency", "Latência"),
    ("Run", "Executar"),
];

pub const ES: &[(&str, &str)] = &[
    ("More memory tests", "Más pruebas de memoria"),
    (
        "STREAM triad (bandwidth), GUPS (random updates), a random walk on small and on large pages (the cost of TLB misses) and DRAM latency from each CCD or NUMA node.",
        "STREAM triad (ancho de banda), GUPS (actualizaciones aleatorias), un recorrido aleatorio en páginas pequeñas y grandes (el coste de los fallos de TLB) y la latencia de la DRAM desde cada CCD o nodo NUMA.",
    ),
    ("STREAM triad", "STREAM triad"),
    ("GUPS", "GUPS"),
    ("Random walk, 4 KiB pages", "Recorrido aleatorio, páginas de 4 KiB"),
    ("Random walk, large pages", "Recorrido aleatorio, páginas grandes"),
    ("Latency", "Latencia"),
    ("Run", "Ejecutar"),
];

#[derive(Default)]
pub struct State {
    job: Option<Job<(Vec<Sub>, Vec<String>)>>,
    cond: benchkit::Conditions,
    result: Option<Saved>,
    notes: Vec<String>,
    error: Option<String>,
    label: String,
    status: String,
}

impl State {
    pub fn page(&mut self, ui: &mut egui::Ui, ctx: &Ctx, start_temp: Option<f64>) {
        let p = theme::palette(ui);
        widgets::card(ui, icon::MEMORY, tr("More memory tests"), |ui| {
            ui.label(RichText::new(tr("STREAM triad (bandwidth), GUPS (random updates), a random walk on small and on large pages (the cost of TLB misses) and DRAM latency from each CCD or NUMA node.")).color(p.text_secondary));
            if let Some(r) = kit::poll(ui, &mut self.job) {
                match r {
                    Ok((subs, notes)) => {
                        let score = subs.iter().find(|s| s.name == "STREAM triad MT").map_or(0.0, |s| s.value);
                        self.notes = notes;
                        self.result = Some(Saved {
                            kind: "Memory (more)",
                            score,
                            unit: "GB/s",
                            variant: String::new(),
                            threads: crate::os::logical_cpus(),
                            subs,
                            conditions: self.cond.clone(),
                            series: Vec::new(),
                        });
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            if self.job.is_none() && widgets::primary_button(ui, &format!("{}  {}", icon::PLAY, tr("Run"))).clicked() {
                self.result = None;
                self.error = None;
                self.status.clear();
                self.cond = benchkit::conditions(start_temp, 1);
                self.job = Some(crate::memplus::start(crate::memplus::Plan {
                    cpus: (0..crate::os::logical_cpus()).collect(),
                    domains: crate::memplus::domains(ctx),
                    max_bytes: (crate::membench::ram_bytes(ctx) / 4) as usize,
                    secs: 1.5,
                }));
            }
            if let Some(e) = &self.error {
                ui.label(RichText::new(trf("Stopped: {}", &[e])).color(p.danger));
            }
        });
        if let Some(r) = &self.result {
            ui.add_space(theme::GAP);
            widgets::card(ui, "", "", |ui| {
                kit::conditions(ui, &r.conditions);
                kit::table(ui, "memplus", &r.subs);
                for n in &self.notes {
                    ui.label(RichText::new(n).small().color(p.text_muted));
                }
            });
            ui.add_space(theme::GAP);
            kit::save(ui, ctx, r, &mut self.label, &mut self.status);
        }
    }
}

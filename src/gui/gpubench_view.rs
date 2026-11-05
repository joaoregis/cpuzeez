//! Benchmarks › Graphics (S1-K05): GPU compute, memory, bus and raster.

use super::benchkit_view::{self as kit, Saved};
use super::i18n::{tr, trf};
use super::theme::{self, icon};
use super::widgets;
use crate::Ctx;
use crate::benchkit::{self, Job, Sub};
use eframe::egui::{self, RichText};

pub const PT_BR: &[(&str, &str)] = &[
    ("GPU", "GPU"),
    (
        "FP32 and FP16 throughput, video-memory bandwidth, the bus to the CPU in both directions (shows the real PCIe link) and a 1080p raster scene. Runs on DX12, Vulkan or Metal.",
        "Vazão FP32 e FP16, largura de banda da memória de vídeo, o barramento até a CPU nos dois sentidos (mostra o link PCIe real) e uma cena rasterizada em 1080p. Roda em DX12, Vulkan ou Metal.",
    ),
    ("No GPU adapter found (DX12, Vulkan or Metal).", "Nenhum adaptador de GPU encontrado (DX12, Vulkan ou Metal)."),
    ("VRAM bandwidth", "Largura de banda da VRAM"),
    ("Upload (host → GPU)", "Envio (CPU → GPU)"),
    ("Download (GPU → host)", "Leitura (GPU → CPU)"),
    ("Raster scene 1080p", "Cena rasterizada 1080p"),
    (
        "1000 points = the reference machine's RTX 4060 (PCIe 4.0 x8).",
        "1000 pontos = a RTX 4060 da máquina de referência (PCIe 4.0 x8).",
    ),
];

pub const ES: &[(&str, &str)] = &[
    ("GPU", "GPU"),
    (
        "FP32 and FP16 throughput, video-memory bandwidth, the bus to the CPU in both directions (shows the real PCIe link) and a 1080p raster scene. Runs on DX12, Vulkan or Metal.",
        "Rendimiento FP32 y FP16, ancho de banda de la memoria de vídeo, el bus hacia la CPU en ambos sentidos (muestra el enlace PCIe real) y una escena rasterizada a 1080p. Funciona con DX12, Vulkan o Metal.",
    ),
    ("No GPU adapter found (DX12, Vulkan or Metal).", "No se encontró un adaptador de GPU (DX12, Vulkan o Metal)."),
    ("VRAM bandwidth", "Ancho de banda de la VRAM"),
    ("Upload (host → GPU)", "Subida (CPU → GPU)"),
    ("Download (GPU → host)", "Bajada (GPU → CPU)"),
    ("Raster scene 1080p", "Escena rasterizada 1080p"),
    (
        "1000 points = the reference machine's RTX 4060 (PCIe 4.0 x8).",
        "1000 puntos = la RTX 4060 de la máquina de referencia (PCIe 4.0 x8).",
    ),
];

type Out = (Vec<Sub>, String, Vec<String>);

#[derive(Default)]
pub struct State {
    adapters: Option<Vec<String>>,
    pub pick: usize,
    job: Option<Job<Out>>,
    cond: benchkit::Conditions,
    result: Option<Saved>,
    notes: Vec<String>,
    error: Option<String>,
    label: String,
    status: String,
}

impl State {
    /// The adapter list (enumerated once; it takes a moment).
    pub fn adapters(&mut self) -> &[String] {
        self.adapters.get_or_insert_with(crate::gpubench::adapters)
    }

    pub fn page(&mut self, ui: &mut egui::Ui, ctx: &Ctx, start_temp: Option<f64>) {
        let p = theme::palette(ui);
        let adapters = self.adapters().to_vec();
        widgets::card(ui, icon::GRAPHICS_CARD, tr("GPU"), |ui| {
            ui.label(RichText::new(tr("FP32 and FP16 throughput, video-memory bandwidth, the bus to the CPU in both directions (shows the real PCIe link) and a 1080p raster scene. Runs on DX12, Vulkan or Metal.")).color(p.text_secondary));
            if adapters.is_empty() {
                ui.label(RichText::new(tr("No GPU adapter found (DX12, Vulkan or Metal).")).color(p.text_muted));
                return;
            }
            adapter_picker(ui, "gpu-adapter", &adapters, &mut self.pick, self.job.is_none());
            if let Some(r) = kit::poll(ui, &mut self.job) {
                match r {
                    Ok((subs, name, notes)) => {
                        self.notes = notes;
                        self.result = Some(Saved {
                            kind: "GPU",
                            score: benchkit::total(&subs).unwrap_or(0.0),
                            unit: "points",
                            variant: name,
                            threads: 0,
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
                let pick = self.pick;
                self.job = Some(Job::spawn(move |ctl| {
                    let mut notes = Vec::new();
                    crate::gpubench::run(pick, ctl, &mut notes).map(|(s, n)| (s, n, notes))
                }));
            }
            if let Some(e) = &self.error {
                ui.label(RichText::new(trf("Stopped: {}", &[e])).color(p.danger));
            }
            ui.label(
                RichText::new(tr("1000 points = the reference machine's RTX 4060 (PCIe 4.0 x8)."))
                    .small()
                    .color(p.text_muted),
            );
        });
        if let Some(r) = &self.result {
            ui.add_space(theme::GAP);
            widgets::card(ui, "", &format!("{}: {:.0} · {}", tr("Score"), r.score, r.variant), |ui| {
                kit::conditions(ui, &r.conditions);
                kit::table(ui, "gpubench", &r.subs);
                for n in &self.notes {
                    ui.label(RichText::new(n).small().color(p.text_muted));
                }
            });
            ui.add_space(theme::GAP);
            kit::save(ui, ctx, r, &mut self.label, &mut self.status);
        }
    }
}

/// The GPU picker (benchmark and GPU stress share it).
pub fn adapter_picker(ui: &mut egui::Ui, id: &str, adapters: &[String], pick: &mut usize, enabled: bool) {
    ui.add_enabled_ui(enabled, |ui| {
        egui::ComboBox::from_id_salt(id)
            .selected_text(adapters.get(*pick).cloned().unwrap_or_default())
            .width(360.0)
            .show_ui(ui, |ui| {
                for (i, a) in adapters.iter().enumerate() {
                    ui.selectable_value(pick, i, a);
                }
            });
    });
}

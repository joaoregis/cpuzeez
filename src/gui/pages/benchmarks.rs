//! Benchmarks: the view switch and each benchmark's page.

#[allow(unused_imports)]
use crate::gui::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::gui) enum BenchView {
    Cpu,
    Memory,
    Latency,
    Gpu,
    Storage,
    Stress,
    History,
}

impl App {
    pub(in crate::gui) fn benchmarks_page(&mut self, ui: &mut egui::Ui) {
        widgets::page_header(
            ui,
            tr("Benchmarks"),
            tr("Our own workloads: scores aren't comparable with CPU-Z's"),
            |_| {},
        );
        widgets::segmented(
            ui,
            &mut self.benches.view,
            &[
                (BenchView::Cpu, icon::CPU, tr("Processor")),
                (BenchView::Memory, icon::MEMORY, tr("Memory")),
                (BenchView::Latency, icon::GRID_FOUR, tr("Latency")),
                (BenchView::Gpu, icon::GRAPHICS_CARD, tr("Graphics")),
                (BenchView::Storage, icon::HARD_DRIVES, tr("Storage")),
                (BenchView::Stress, icon::FLAME, tr("Stress test")),
                (BenchView::History, icon::CLOCK_COUNTER_CLOCKWISE, tr("History")),
            ],
        );
        match self.benches.view {
            BenchView::Cpu => {
                self.benches.cpu.cpu_page(ui, &self.ctx, &mut self.mon);
                ui.add_space(theme::GAP);
                let t0 = benchkit_view::cpu_temp(&mut self.mon);
                self.benches.cpusuite.page(ui, &self.ctx, t0);
            }
            BenchView::Memory => {
                let t0 = benchkit_view::cpu_temp(&mut self.mon);
                self.benches.cpu.memory_page(ui, &self.ctx, t0);
                ui.add_space(theme::GAP);
                self.benches.memplus.page(ui, &self.ctx, t0);
            }
            BenchView::Latency => {
                let t0 = benchkit_view::cpu_temp(&mut self.mon);
                self.benches.latency.page(ui, &self.ctx, t0)
            }
            BenchView::Gpu => {
                let t0 = benchkit_view::cpu_temp(&mut self.mon);
                self.benches.gpu.page(ui, &self.ctx, t0)
            }
            BenchView::Storage => {
                let t0 = benchkit_view::cpu_temp(&mut self.mon);
                self.benches.disk.page(ui, &self.ctx, t0)
            }
            BenchView::Stress => {
                let gpus = cache::gpu_adapters(&self.ctx).map(|g| g.to_vec()).unwrap_or_default();
                stress::page(ui, &mut self.mon, true, &gpus)
            }
            BenchView::History => {
                if self.benches.history_dirty {
                    self.benches.history.reload();
                    self.benches.history_dirty = false;
                }
                self.benches.history.page(ui)
            }
        }
        if self.benches.view != BenchView::History {
            self.benches.history_dirty = true;
        }
    }
}

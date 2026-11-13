//! One file per page (S0-E04). Each adds methods to `App` or free page functions.

mod about;
mod graphics;
mod mainboard;
pub(crate) mod mainboard_more;
mod memory;
pub(crate) mod memory_more;
mod processor;
pub(crate) mod processor_more;
mod tools;
pub(crate) use mainboard::mainboard_tab;
pub(crate) use memory::memory_tab;

use super::{BenchView, ToolView, cpusuite_view, diskbench_view, drivehealth_view, gpubench_view, latency_view};
use super::{bench_tab, memplus_view, table, tools as tool};

/// Processor: the core shown and the Cores table.
#[derive(Default)]
pub(super) struct Processor {
    pub core: usize,
    pub cores: table::State,
}

/// Mainboard › PCIe table.
#[derive(Default)]
pub(super) struct Mainboard {
    pub pcie: table::State,
}

/// Memory: the module shown.
#[derive(Default)]
pub(super) struct Memory {
    pub slot: usize,
}

/// Graphics: the GPU shown.
#[derive(Default)]
pub(super) struct Graphics {
    pub gpu: usize,
}

/// Benchmarks: the view and each one's state.
pub(super) struct Benches {
    pub view: BenchView,
    pub cpu: bench_tab::State,
    pub cpusuite: cpusuite_view::State,
    pub memplus: memplus_view::State,
    pub latency: latency_view::State,
    pub gpu: gpubench_view::State,
    pub disk: diskbench_view::State,
    pub history: tool::History,
    /// Reload history.json when the History view is opened.
    pub history_dirty: bool,
}

impl Default for Benches {
    fn default() -> Self {
        Benches {
            view: BenchView::Cpu,
            cpu: Default::default(),
            cpusuite: Default::default(),
            memplus: Default::default(),
            latency: Default::default(),
            gpu: Default::default(),
            disk: Default::default(),
            history: Default::default(),
            history_dirty: true,
        }
    }
}

/// Tools: the view, Compare and Drives.
pub(super) struct Tools {
    pub view: ToolView,
    pub compare: tool::Compare,
    pub drives: drivehealth_view::State,
}

impl Default for Tools {
    fn default() -> Self {
        Tools { view: ToolView::Compare, compare: Default::default(), drives: Default::default() }
    }
}

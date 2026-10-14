//! Memory and cache benchmark: latency by working-set size (pointer chasing,
//! 4 KiB … 1 GiB, reveals L1/L2/L3/DRAM) and bandwidth (read / write / copy) per
//! cache level and DRAM, single- and multi-threaded. Our own method, like the
//! CPU benchmark: results are not comparable with other tools' numbers.

use crate::Ctx;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LatencyPoint {
    pub bytes: u64,
    pub ns: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bandwidth {
    /// "L1", "L2", "L3", "DRAM".
    pub level: String,
    pub threads: usize,
    pub read_gbs: f64,
    pub write_gbs: f64,
    pub copy_gbs: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MemResult {
    pub latency: Vec<LatencyPoint>,
    pub bandwidth: Vec<Bandwidth>,
    /// DRAM latency summary (ns) for history/CLI.
    pub dram_latency_ns: f64,
}

/// A run on background threads (GUI).
pub struct MemBench {
    _p: (),
}

impl MemBench {
    pub fn start(_ctx: &Ctx, _quick: bool) -> MemBench {
        MemBench { _p: () }
    }
    /// 0.0 ..= 1.0
    pub fn progress(&self) -> f32 {
        1.0
    }
    /// What is being measured now ("Latency 32 MiB", "Bandwidth L2 copy"…).
    pub fn stage(&self) -> String {
        String::new()
    }
    pub fn result(&self) -> Option<MemResult> {
        None
    }
    pub fn finished(&self) -> bool {
        true
    }
    pub fn cancel(&self) {}
}

/// `-membench`
pub fn cli(_ctx: &Ctx) {}

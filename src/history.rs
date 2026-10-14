//! Local benchmark history (settings folder, `history.json`): every CPU and
//! memory benchmark result with date, label/notes and the system it ran on,
//! so runs before/after an overclock, paste swap or BIOS update can be compared.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// RFC 3339 UTC, e.g. "2026-10-14T10:20:00Z".
    pub when: String,
    /// User label ("before BIOS 1.A3", "PBO on"…).
    pub label: String,
    /// "CPU Single Thread", "CPU Multi Thread", "Memory".
    pub kind: String,
    /// Main score (points, or GB/s / ns for memory: see `unit`).
    pub score: f64,
    pub unit: String,
    pub threads: usize,
    /// "AVX-512", "Scalar"…
    pub variant: String,
    pub cpu: String,
    pub board: String,
    pub bios: String,
    pub memory: String,
    pub os: String,
    pub app_version: String,
    /// Full result (e.g. memory latency curve / bandwidth table).
    #[serde(default)]
    pub details: serde_json::Value,
}

pub fn load() -> Vec<Entry> {
    Vec::new()
}

pub fn add(_e: Entry) {}

pub fn remove(_index: usize) {}

pub fn clear() {}

/// Fill the system fields (cpu/board/bios/memory/os/version/when) of a new entry.
pub fn system_entry(_ctx: &crate::Ctx, kind: &str, score: f64, unit: &str) -> Entry {
    Entry {
        when: String::new(),
        label: String::new(),
        kind: kind.into(),
        score,
        unit: unit.into(),
        threads: 0,
        variant: String::new(),
        cpu: String::new(),
        board: String::new(),
        bios: String::new(),
        memory: String::new(),
        os: String::new(),
        app_version: crate::VERSION.into(),
        details: serde_json::Value::Null,
    }
}

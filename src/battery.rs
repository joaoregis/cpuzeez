//! Batteries (laptops, UPS on Windows): design vs full-charge capacity
//! (health / wear), cycle count, chemistry, current charge, rate, status.

use crate::Ctx;
use crate::report::Section;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Battery {
    pub name: String,
    pub manufacturer: String,
    pub serial: String,
    pub chemistry: String,
    pub design_mwh: Option<f64>,
    pub full_mwh: Option<f64>,
    pub remaining_mwh: Option<f64>,
    /// full / design, %.
    pub health_pct: Option<f64>,
    pub cycles: Option<u32>,
    pub voltage_v: Option<f64>,
    /// Positive = charging, negative = discharging (W).
    pub rate_w: Option<f64>,
    pub percent: Option<f64>,
    /// "Charging", "Discharging", "Full", "AC"…
    pub status: String,
    pub temperature_c: Option<f64>,
}

pub fn batteries(_ctx: &Ctx) -> Vec<Battery> {
    Vec::new()
}

/// "Batteries" report section (empty on desktops).
pub fn collect(_ctx: &Ctx) -> Vec<Section> {
    Vec::new()
}

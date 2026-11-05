//! The display units chosen in Settings (S1-U04), for every place that
//! formats a value. Set once per frame from the settings, like the language.

use cpuzeez_core::units::Units;
use std::sync::RwLock;

static CURRENT: RwLock<Units> = RwLock::new(Units::DEFAULT);

pub fn set(u: Units) {
    if let Ok(mut c) = CURRENT.write()
        && *c != u
    {
        *c = u;
    }
}

pub fn get() -> Units {
    CURRENT.read().map(|u| *u).unwrap_or_default()
}

/// A °C value, `prec` decimals unless the user chose a fixed number.
pub fn temp(c: f64, prec: usize) -> String {
    get().temp(c, prec)
}

/// A MHz value; `ghz` is what this place shows on Auto.
pub fn clock(mhz: f64, ghz: bool, prec: usize) -> String {
    get().clock(mhz, ghz, prec)
}

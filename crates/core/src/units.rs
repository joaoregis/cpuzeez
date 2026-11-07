//! Display units (S1-U04): one global choice of °C/°F, MHz/GHz, MB/MiB and
//! decimal places, applied wherever a value is shown. Values are always
//! stored in the app's base units (°C, MHz, MiB); only formatting changes, so
//! thresholds, logs and reports stay comparable.

use crate::metrics::Unit;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Temp {
    #[default]
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Clock {
    /// Whatever each place shows by default (MHz in tables, GHz on tiles).
    #[default]
    Auto,
    Mhz,
    Ghz,
}

/// Memory sizes. The app measures in MiB (1024²).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Size {
    /// MiB values labelled "MB", as Windows and the Task Manager do.
    #[default]
    Windows,
    /// The same values with IEC labels (MiB, GiB).
    Iec,
    /// Decimal megabytes (1000²), as drive makers count.
    Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Units {
    pub temp: Temp,
    pub clock: Clock,
    pub size: Size,
    /// None: each place's own precision. Some(n): n decimals everywhere.
    pub decimals: Option<u8>,
}

impl Units {
    pub const DEFAULT: Units = Units { temp: Temp::Celsius, clock: Clock::Auto, size: Size::Windows, decimals: None };

    fn prec(&self, default: usize) -> usize {
        self.decimals.map_or(default, |d| d.min(4) as usize)
    }

    /// A temperature given in °C.
    pub fn temp(&self, c: f64, default_prec: usize) -> String {
        let p = self.prec(default_prec);
        match self.temp {
            Temp::Celsius => format!("{c:.p$} °C"),
            Temp::Fahrenheit => format!("{:.p$} °F", c * 9.0 / 5.0 + 32.0),
        }
    }

    /// A clock given in MHz. `ghz` is what this place shows on Auto.
    pub fn clock(&self, mhz: f64, ghz: bool, default_prec: usize) -> String {
        let ghz = match self.clock {
            Clock::Auto => ghz,
            Clock::Mhz => false,
            Clock::Ghz => true,
        };
        // A place that shows MHz with 0 decimals needs 2 to say as much in GHz.
        if ghz {
            let p = self.prec(if default_prec == 0 { 2 } else { default_prec });
            format!("{:.p$} GHz", mhz / 1000.0)
        } else {
            let p = self.prec(default_prec);
            format!("{mhz:.p$} MHz")
        }
    }

    /// A size given in MiB.
    pub fn size(&self, mib: f64) -> String {
        let (v, mb, gb) = match self.size {
            Size::Windows => (mib, "MB", "GB"),
            Size::Iec => (mib, "MiB", "GiB"),
            Size::Decimal => (mib * 1.048_576, "MB", "GB"),
        };
        let big = if self.size == Size::Decimal { 10_000.0 } else { 10_240.0 };
        let div = if self.size == Size::Decimal { 1000.0 } else { 1024.0 };
        if v >= big {
            format!("{:.p$} {gb}", v / div, p = self.prec(1))
        } else {
            format!("{v:.p$} {mb}", p = self.prec(0))
        }
    }

    /// A telemetry value in its unit, at the precision the Sensors list uses.
    pub fn format(&self, v: f64, u: Unit) -> String {
        match u {
            Unit::Celsius => self.temp(v, 1),
            Unit::Mhz => self.clock(v, false, 0),
            Unit::Megabytes => self.size(v),
            Unit::Volt => format!("{v:.p$} V", p = self.prec(3)),
            Unit::Watt => format!("{v:.p$} W", p = self.prec(1)),
            Unit::Ampere => format!("{v:.p$} A", p = self.prec(2)),
            Unit::Rpm => format!("{v:.p$} RPM", p = self.prec(0)),
            Unit::Percent => format!("{v:.p$} %", p = self.prec(1)),
            Unit::MegabytesPerSec => format!("{v:.p$} MB/s", p = self.prec(if v < 10.0 { 2 } else { 0 })),
            Unit::Mbps => format!("{v:.p$} Mbps", p = self.prec(if v < 10.0 { 2 } else { 0 })),
            Unit::PerSec => format!("{v:.p$} /s", p = self.prec(0)),
            Unit::Millis => format!("{v:.p$} ms", p = self.prec(2)),
            Unit::Count => format!("{v:.p$}", p = self.prec(1)),
        }
    }

    /// The unit's label as this choice shows it (for axis titles, CSV headers stay base units).
    pub fn suffix(&self, u: Unit) -> &'static str {
        match (u, self.temp, self.size) {
            (Unit::Celsius, Temp::Fahrenheit, _) => "°F",
            (Unit::Megabytes, _, Size::Iec) => "MiB",
            _ => u.suffix(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_todays_text() {
        let u = Units::default();
        assert_eq!(u.format(95.25, Unit::Celsius), "95.2 °C");
        assert_eq!(u.format(4950.0, Unit::Mhz), "4950 MHz");
        assert_eq!(u.clock(4950.0, true, 2), "4.95 GHz");
        assert_eq!(u.format(20480.0, Unit::Megabytes), "20.0 GB");
        assert_eq!(u.format(512.0, Unit::Megabytes), "512 MB");
        assert_eq!(u.format(1.23456, Unit::Volt), "1.235 V");
    }

    #[test]
    fn choices() {
        let f = Units { temp: Temp::Fahrenheit, ..Default::default() };
        assert_eq!(f.temp(100.0, 0), "212 °F");
        assert_eq!(f.suffix(Unit::Celsius), "°F");
        let g = Units { clock: Clock::Ghz, ..Default::default() };
        assert_eq!(g.format(4950.0, Unit::Mhz), "4.95 GHz");
        assert_eq!(Units { clock: Clock::Mhz, ..Default::default() }.clock(4950.0, true, 2), "4950.00 MHz");
        let iec = Units { size: Size::Iec, ..Default::default() };
        assert_eq!(iec.size(20480.0), "20.0 GiB");
        let dec = Units { size: Size::Decimal, ..Default::default() };
        assert_eq!(dec.size(1000.0), "1049 MB");
        let d0 = Units { decimals: Some(0), ..Default::default() };
        assert_eq!(d0.format(95.25, Unit::Celsius), "95 °C");
        assert_eq!(d0.format(1.23456, Unit::Volt), "1 V");
    }

    #[test]
    fn old_settings_files_load() {
        let u: Units = serde_json::from_str("{}").unwrap();
        assert_eq!(u, Units::default());
    }
}

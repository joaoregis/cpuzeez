//! Batteries (laptops, UPS on Windows): design vs full-charge capacity
//! (health / wear), cycle count, chemistry, current charge, rate, status.
//! Windows: battery class IOCTLs (Windows Driver Kit battery docs). Linux:
//! /sys/class/power_supply ABI. macOS: `ioreg -rc AppleSmartBattery -a`.
// Parsers stay OS-neutral so their unit tests run on every host.
#![allow(dead_code)]

use crate::Ctx;
use crate::report::{Section, f, gv, section};
use serde_json::Value;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Battery {
    pub name: String,
    pub manufacturer: String,
    pub serial: String,
    pub chemistry: String,
    pub design_mwh: Option<f64>,
    pub full_mwh: Option<f64>,
    pub remaining_mwh: Option<f64>,
    /// Charge-based capacities (Linux charge_*, macOS); mWh is derived from these
    /// with the design (Linux) or present (macOS) voltage.
    pub design_mah: Option<f64>,
    pub full_mah: Option<f64>,
    pub remaining_mah: Option<f64>,
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

impl Battery {
    /// Health from whichever unit pair is complete (the ratio is unit-free).
    fn finish(mut self) -> Battery {
        let ratio = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (Some(a), Some(b)) if b > 0.0 => Some(a * 100.0 / b),
            _ => None,
        };
        self.health_pct = ratio(self.full_mwh, self.design_mwh).or(ratio(self.full_mah, self.design_mah));
        self.percent = self
            .percent
            .or(ratio(self.remaining_mwh, self.full_mwh))
            .or(ratio(self.remaining_mah, self.full_mah))
            .map(|p| p.min(100.0));
        self
    }
}

pub fn batteries(_ctx: &Ctx) -> Vec<Battery> {
    imp::probe()
}

fn opt(v: Option<f64>, unit: &str, prec: usize) -> Option<String> {
    v.map(|x| format!("{x:.prec$} {unit}"))
}

/// "Batteries" report section (empty on desktops).
pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let bats = batteries(ctx);
    if bats.is_empty() {
        return Vec::new();
    }
    let nodes = bats
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let cap = |mwh: Option<f64>, mah: Option<f64>| match (mwh, mah) {
                (Some(w), Some(a)) => Some(format!("{w:.0} mWh ({a:.0} mAh)")),
                (Some(w), None) => Some(format!("{w:.0} mWh")),
                (None, Some(a)) => Some(format!("{a:.0} mAh")),
                _ => None,
            };
            let rows: [(&str, Option<String>); 15] = [
                ("Name", Some(b.name.clone()).filter(|s| !s.is_empty())),
                ("Manufacturer", Some(b.manufacturer.clone()).filter(|s| !s.is_empty())),
                ("Serial", Some(b.serial.clone()).filter(|s| !s.is_empty())),
                ("Chemistry", Some(b.chemistry.clone()).filter(|s| !s.is_empty())),
                ("Design Capacity", cap(b.design_mwh, b.design_mah)),
                ("Full Charge Capacity", cap(b.full_mwh, b.full_mah)),
                ("Remaining Capacity", cap(b.remaining_mwh, b.remaining_mah)),
                ("Health", opt(b.health_pct, "%", 1)),
                ("Wear Level", opt(b.health_pct.map(|h| (100.0 - h).max(0.0)), "%", 1)),
                ("Cycle Count", b.cycles.map(|c| c.to_string())),
                ("Charge Level", opt(b.percent, "%", 0)),
                ("Status", Some(b.status.clone()).filter(|s| !s.is_empty())),
                ("Voltage", opt(b.voltage_v, "V", 3)),
                ("Rate", opt(b.rate_w, "W", 2)),
                ("Temperature", opt(b.temperature_c, "C", 1)),
            ];
            let c = rows.into_iter().filter_map(|(k, v)| v.map(|v| f(k, v))).collect();
            gv("Battery", i, c)
        })
        .collect();
    vec![section("Batteries", nodes)]
}

// ---------------------------------------------------------------- Linux sysfs

/// One power_supply `uevent` (POWER_SUPPLY_*=value lines). None unless it's a
/// system battery (type Battery, scope not Device: peripherals report Device).
pub fn parse_power_supply(uevent: &str) -> Option<Battery> {
    let map: std::collections::HashMap<&str, &str> = uevent
        .lines()
        .filter_map(|l| l.trim().strip_prefix("POWER_SUPPLY_")?.split_once('='))
        .map(|(k, v)| (k, v.trim()))
        .collect();
    let s = |k: &str| map.get(k).map(|v| v.to_string()).unwrap_or_default();
    let n = |k: &str| map.get(k).and_then(|v| v.parse::<f64>().ok());
    if map.get("TYPE").is_some_and(|t| *t != "Battery") || map.get("SCOPE") == Some(&"Device") {
        return None;
    }
    if map.get("PRESENT") == Some(&"0") {
        return None;
    }
    let micro = |k: &str| n(k).map(|v| v / 1e6);
    let milli = |k: &str| n(k).map(|v| v / 1e3); // µWh / µAh → mWh / mAh
    let design_v = micro("VOLTAGE_MIN_DESIGN").or(micro("VOLTAGE_MAX_DESIGN")).filter(|&v| v > 0.0);
    let mut b = Battery {
        name: map.get("MODEL_NAME").or(map.get("NAME")).map_or(String::new(), |v| v.to_string()),
        manufacturer: s("MANUFACTURER"),
        serial: s("SERIAL_NUMBER"),
        chemistry: s("TECHNOLOGY"),
        design_mwh: milli("ENERGY_FULL_DESIGN"),
        full_mwh: milli("ENERGY_FULL"),
        remaining_mwh: milli("ENERGY_NOW"),
        design_mah: milli("CHARGE_FULL_DESIGN"),
        full_mah: milli("CHARGE_FULL"),
        remaining_mah: milli("CHARGE_NOW"),
        cycles: n("CYCLE_COUNT").filter(|&c| c > 0.0).map(|c| c as u32),
        voltage_v: micro("VOLTAGE_NOW"),
        percent: n("CAPACITY"),
        status: s("STATUS"),
        temperature_c: n("TEMP").map(|t| t / 10.0),
        ..Battery::default()
    };
    // Charge-only gauges: energy = charge × design voltage.
    if let Some(v) = design_v {
        b.design_mwh = b.design_mwh.or(b.design_mah.map(|a| a * v));
        b.full_mwh = b.full_mwh.or(b.full_mah.map(|a| a * v));
        b.remaining_mwh = b.remaining_mwh.or(b.remaining_mah.map(|a| a * v));
    }
    // power_now (µW) or current_now (µA) × voltage; sign from status (drivers differ).
    let watts = micro("POWER_NOW").map(f64::abs).or_else(|| Some(micro("CURRENT_NOW")?.abs() * b.voltage_v?));
    b.rate_w = watts.map(|w| if b.status == "Discharging" { -w } else { w });
    Some(b.finish())
}

// ---------------------------------------------------------------- macOS ioreg

/// Minimal XML property-list reader (dict/array/key/string/integer/real/
/// true/false/data/date) into a JSON value. Integers above i64::MAX are
/// two's-complement negatives (ioreg prints signed values unsigned).
pub fn parse_plist(xml: &str) -> Option<Value> {
    let mut stack: Vec<(Value, Option<String>)> = Vec::new();
    let mut key: Option<String> = None;
    let mut root = None;
    let mut rest = xml;
    fn unescape(s: &str) -> String {
        s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
    }
    while let Some(i) = rest.find('<') {
        rest = &rest[i + 1..];
        let end = rest.find('>')?;
        let tag = &rest[..end];
        rest = &rest[end + 1..];
        if tag.starts_with('?') || tag.starts_with('!') || tag.starts_with("plist") || tag == "/plist" {
            continue;
        }
        let mut text = |close: &str| -> Option<String> {
            let j = rest.find(close)?;
            let t = unescape(&rest[..j]);
            rest = &rest[j + close.len()..];
            Some(t)
        };
        let leaf = match tag {
            "dict" => {
                stack.push((Value::Object(Default::default()), key.take()));
                continue;
            }
            "array" => {
                stack.push((Value::Array(Vec::new()), key.take()));
                continue;
            }
            "/dict" | "/array" => {
                let (v, k) = stack.pop()?;
                key = k;
                v
            }
            "key" => {
                key = Some(text("</key>")?);
                continue;
            }
            "dict/" => Value::Object(Default::default()),
            "array/" => Value::Array(Vec::new()),
            "string/" => Value::String(String::new()),
            "true/" => Value::Bool(true),
            "false/" => Value::Bool(false),
            "string" | "data" | "date" => Value::String(text(&format!("</{tag}>"))?.trim().to_string()),
            "integer" => {
                let t = text("</integer>")?;
                let t = t.trim();
                t.parse::<i64>()
                    .ok()
                    .or_else(|| t.parse::<u64>().ok().map(|u| u as i64))
                    .map_or(Value::Null, Value::from)
            }
            "real" => text("</real>")?.trim().parse::<f64>().ok().map_or(Value::Null, Value::from),
            _ => continue,
        };
        match stack.last_mut() {
            Some((Value::Object(m), _)) => {
                m.insert(key.take()?, leaf);
            }
            Some((Value::Array(a), _)) => a.push(leaf),
            _ => root = Some(leaf),
        }
    }
    root
}

/// One AppleSmartBattery dictionary. Capacities are mAh; on Apple Silicon
/// MaxCapacity/CurrentCapacity are percentages and the AppleRaw* keys hold mAh.
pub fn parse_apple_battery(d: &Value) -> Option<Battery> {
    let i = |k: &str| d.get(k).and_then(Value::as_i64).or_else(|| d.pointer(&format!("/BatteryData/{k}"))?.as_i64());
    let b = |k: &str| d.get(k).and_then(Value::as_bool).unwrap_or(false);
    let s = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    let design = i("DesignCapacity")? as f64;
    let max = i("MaxCapacity").map(|v| v as f64);
    let pct_style = max.is_some_and(|m| m <= 100.0);
    let full = i("AppleRawMaxCapacity").map(|v| v as f64).or(max.filter(|_| !pct_style));
    let remaining = i("AppleRawCurrentCapacity")
        .map(|v| v as f64)
        .or(i("CurrentCapacity").map(|v| v as f64).filter(|_| !pct_style));
    let volts = i("Voltage").map(|mv| mv as f64 / 1000.0);
    let amps = i("InstantAmperage").or(i("Amperage")).map(|ma| ma as f64 / 1000.0);
    // ponytail: mWh uses the present voltage (no design voltage key); ±5 % at most.
    let mwh = |mah: Option<f64>| Some(mah? * volts?);
    let status = if b("FullyCharged") {
        "Full"
    } else if b("IsCharging") {
        "Charging"
    } else if b("ExternalConnected") {
        "AC"
    } else {
        "Discharging"
    };
    let bat = Battery {
        name: s("DeviceName"),
        manufacturer: s("Manufacturer"),
        serial: [s("Serial"), s("BatterySerialNumber")].into_iter().find(|x| !x.is_empty()).unwrap_or_default(),
        design_mah: Some(design),
        full_mah: full,
        remaining_mah: remaining,
        design_mwh: mwh(Some(design)),
        full_mwh: mwh(full),
        remaining_mwh: mwh(remaining),
        cycles: i("CycleCount").map(|c| c as u32),
        voltage_v: volts,
        rate_w: volts.zip(amps).map(|(v, a)| v * a),
        percent: if pct_style { i("CurrentCapacity").map(|v| v as f64) } else { None },
        status: status.into(),
        temperature_c: i("Temperature").map(|t| t as f64 / 100.0),
        ..Battery::default()
    };
    Some(bat.finish())
}

/// Windows BATTERY_INFORMATION.Chemistry (4 ASCII chars) to a readable name.
pub fn chemistry_name(code: &[u8]) -> String {
    let c = String::from_utf8_lossy(code).trim_matches(['\0', ' ']).to_string();
    match c.to_ascii_uppercase().as_str() {
        "LION" | "LI-I" | "LIION" => "Li-ion".into(),
        "LIP" | "LIPO" => "Li-polymer".into(),
        "PBAC" => "Lead acid".into(),
        "NICD" => "NiCd".into(),
        "NIMH" => "NiMH".into(),
        "NIZN" => "NiZn".into(),
        "RAM" => "Rechargeable alkaline-manganese".into(),
        _ => c,
    }
}

// ---------------------------------------------------------------- Windows

#[cfg(windows)]
mod imp {
    use super::{Battery, chemistry_name};
    use crate::storage::win::{Guid, Handle, interfaces, le32};

    /// GUID_DEVICE_BATTERY device interface class.
    const GUID_DEVICE_BATTERY: Guid =
        Guid(0x72631e54, 0x78a4, 0x11d0, [0xbc, 0xf7, 0x00, 0xaa, 0x00, 0xb7, 0xb3, 0x2a]);
    const IOCTL_BATTERY_QUERY_TAG: u32 = 0x294040;
    const IOCTL_BATTERY_QUERY_INFORMATION: u32 = 0x294044;
    const IOCTL_BATTERY_QUERY_STATUS: u32 = 0x29404C;
    const UNKNOWN: u32 = 0xFFFF_FFFF;

    fn info(h: &Handle, tag: u32, level: u32, len: usize) -> Option<Vec<u8>> {
        // BATTERY_QUERY_INFORMATION: BatteryTag, InformationLevel, AtRate.
        let mut q = [0u8; 12];
        q[0..4].copy_from_slice(&tag.to_le_bytes());
        q[4..8].copy_from_slice(&level.to_le_bytes());
        h.ioctl(IOCTL_BATTERY_QUERY_INFORMATION, &q, len)
    }

    fn wstr(h: &Handle, tag: u32, level: u32) -> String {
        info(h, tag, level, 512).map_or(String::new(), |b| {
            let u: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
            String::from_utf16_lossy(&u).trim_end_matches('\0').trim().to_string()
        })
    }

    pub fn probe() -> Vec<Battery> {
        let mut out = Vec::new();
        for path in interfaces(&GUID_DEVICE_BATTERY, None) {
            // The battery class needs read/write access; works without admin.
            let Some(h) = Handle::open_rw(&path).or_else(|| Handle::open(&path)) else { continue };
            let tag = h.ioctl(IOCTL_BATTERY_QUERY_TAG, &0u32.to_le_bytes(), 4).map(|b| le32(&b, 0));
            let Some(tag) = tag.filter(|&t| t != 0) else { continue };
            let mut b = Battery::default();
            // BATTERY_INFORMATION (36 bytes).
            if let Some(i) = info(&h, tag, 0, 36).filter(|i| i.len() >= 36) {
                let relative = le32(&i, 0) & 0x4000_0000 != 0; // BATTERY_CAPACITY_RELATIVE
                b.chemistry = chemistry_name(&i[8..12]);
                let cap = |o| Some(le32(&i, o) as f64).filter(|&v| v > 0.0 && v != UNKNOWN as f64 && !relative);
                b.design_mwh = cap(12);
                b.full_mwh = cap(16);
                b.cycles = Some(le32(&i, 32)).filter(|&c| c > 0);
            }
            b.name = wstr(&h, tag, 4); // BatteryDeviceName
            b.manufacturer = wstr(&h, tag, 6); // BatteryManufactureName
            b.serial = wstr(&h, tag, 8); // BatterySerialNumber
            // BatteryTemperature: tenths of a kelvin.
            b.temperature_c = info(&h, tag, 2, 4)
                .filter(|t| t.len() >= 4)
                .map(|t| le32(&t, 0))
                .filter(|&t| t > 0)
                .map(|t| t as f64 / 10.0 - 273.15);
            // BATTERY_WAIT_STATUS {tag, timeout, power state, low, high} → BATTERY_STATUS.
            let mut w = [0u8; 20];
            w[0..4].copy_from_slice(&tag.to_le_bytes());
            if let Some(s) = h.ioctl(IOCTL_BATTERY_QUERY_STATUS, &w, 16).filter(|s| s.len() >= 16) {
                let (state, cap, mv, rate) = (le32(&s, 0), le32(&s, 4), le32(&s, 8), le32(&s, 12) as i32);
                b.status = if state & 4 != 0 {
                    "Charging"
                } else if state & 2 != 0 {
                    "Discharging"
                } else if state & 1 != 0 {
                    "AC"
                } else {
                    ""
                }
                .into();
                if b.full_mwh.is_some() {
                    b.remaining_mwh = (cap != UNKNOWN).then_some(cap as f64);
                } else if cap != UNKNOWN {
                    b.percent = Some(cap as f64); // relative-capacity batteries report %
                }
                b.voltage_v = (mv != UNKNOWN && mv > 0).then_some(mv as f64 / 1000.0);
                b.rate_w = (rate != i32::MIN).then_some(rate as f64 / 1000.0);
                if b.status.is_empty() && b.percent.or(b.remaining_mwh).is_some() && rate == 0 && state & 1 != 0 {
                    b.status = "Full".into();
                }
            }
            out.push(b.finish());
        }
        out
    }
}

#[cfg(target_os = "linux")]
mod imp {
    pub fn probe() -> Vec<super::Battery> {
        let mut dirs: Vec<_> = std::fs::read_dir("/sys/class/power_supply")
            .map(|r| r.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        dirs.sort();
        dirs.iter()
            .filter_map(|d| super::parse_power_supply(&std::fs::read_to_string(d.join("uevent")).ok()?))
            .collect()
    }
}

#[cfg(target_os = "macos")]
mod imp {
    pub fn probe() -> Vec<super::Battery> {
        let Some(xml) = crate::os::cmd("ioreg", &["-rc", "AppleSmartBattery", "-a"]) else { return Vec::new() };
        let Some(v) = super::parse_plist(&xml) else { return Vec::new() };
        v.as_array().into_iter().flatten().filter_map(super::parse_apple_battery).collect()
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn probe() -> Vec<super::Battery> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysfs_energy() {
        let u = "POWER_SUPPLY_NAME=BAT0\nPOWER_SUPPLY_TYPE=Battery\nPOWER_SUPPLY_STATUS=Discharging\n\
POWER_SUPPLY_PRESENT=1\nPOWER_SUPPLY_TECHNOLOGY=Li-ion\nPOWER_SUPPLY_CYCLE_COUNT=123\n\
POWER_SUPPLY_VOLTAGE_MIN_DESIGN=11400000\nPOWER_SUPPLY_VOLTAGE_NOW=12000000\nPOWER_SUPPLY_POWER_NOW=8500000\n\
POWER_SUPPLY_ENERGY_FULL_DESIGN=57000000\nPOWER_SUPPLY_ENERGY_FULL=51300000\nPOWER_SUPPLY_ENERGY_NOW=25650000\n\
POWER_SUPPLY_CAPACITY=50\nPOWER_SUPPLY_MODEL_NAME=5B10W13930\nPOWER_SUPPLY_MANUFACTURER=SMP\n";
        let b = parse_power_supply(u).unwrap();
        assert_eq!((b.design_mwh, b.full_mwh, b.remaining_mwh), (Some(57000.0), Some(51300.0), Some(25650.0)));
        assert_eq!(b.health_pct, Some(90.0));
        assert_eq!((b.cycles, b.percent, b.voltage_v), (Some(123), Some(50.0), Some(12.0)));
        assert_eq!(b.rate_w, Some(-8.5));
        assert_eq!((b.name.as_str(), b.manufacturer.as_str(), b.chemistry.as_str()), ("5B10W13930", "SMP", "Li-ion"));
    }

    #[test]
    fn sysfs_charge() {
        let u = "POWER_SUPPLY_TYPE=Battery\nPOWER_SUPPLY_STATUS=Charging\nPOWER_SUPPLY_VOLTAGE_MIN_DESIGN=7600000\n\
POWER_SUPPLY_VOLTAGE_NOW=8000000\nPOWER_SUPPLY_CURRENT_NOW=1500000\nPOWER_SUPPLY_CHARGE_FULL_DESIGN=5000000\n\
POWER_SUPPLY_CHARGE_FULL=4000000\nPOWER_SUPPLY_CHARGE_NOW=1000000\nPOWER_SUPPLY_TEMP=315\nPOWER_SUPPLY_CYCLE_COUNT=0\n";
        let b = parse_power_supply(u).unwrap();
        assert_eq!((b.design_mah, b.full_mah), (Some(5000.0), Some(4000.0)));
        assert_eq!(b.design_mwh, Some(38000.0));
        assert_eq!((b.health_pct, b.percent, b.cycles), (Some(80.0), Some(25.0), None));
        assert_eq!((b.rate_w, b.temperature_c), (Some(12.0), Some(31.5)));
        assert!(parse_power_supply("POWER_SUPPLY_TYPE=Mains\nPOWER_SUPPLY_ONLINE=1\n").is_none());
        assert!(parse_power_supply("POWER_SUPPLY_TYPE=Battery\nPOWER_SUPPLY_SCOPE=Device\n").is_none());
    }

    #[test]
    fn ioreg_plist() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<dict>
		<key>AppleRawCurrentCapacity</key>
		<integer>3000</integer>
		<key>AppleRawMaxCapacity</key>
		<integer>4500</integer>
		<key>Amperage</key>
		<integer>18446744073709550616</integer>
		<key>BatteryData</key>
		<dict>
			<key>DesignCapacity</key>
			<integer>5000</integer>
			<key>Empty</key>
			<array/>
		</dict>
		<key>CurrentCapacity</key>
		<integer>66</integer>
		<key>CycleCount</key>
		<integer>321</integer>
		<key>DeviceName</key>
		<string>bq40z651</string>
		<key>ExternalConnected</key>
		<false/>
		<key>FullyCharged</key>
		<false/>
		<key>IsCharging</key>
		<false/>
		<key>Manufacturer</key>
		<string>SMP &amp; Co</string>
		<key>MaxCapacity</key>
		<integer>100</integer>
		<key>Temperature</key>
		<integer>3050</integer>
		<key>Voltage</key>
		<integer>12000</integer>
	</dict>
</array>
</plist>"#;
        let v = parse_plist(xml).unwrap();
        assert_eq!(v[0]["Amperage"].as_i64(), Some(-1000));
        assert_eq!(v[0]["Manufacturer"].as_str(), Some("SMP & Co"));
        let b = parse_apple_battery(&v[0]).unwrap();
        assert_eq!((b.design_mah, b.full_mah, b.remaining_mah), (Some(5000.0), Some(4500.0), Some(3000.0)));
        assert_eq!((b.health_pct, b.percent, b.cycles), (Some(90.0), Some(66.0), Some(321)));
        assert_eq!((b.full_mwh, b.voltage_v, b.rate_w), (Some(54000.0), Some(12.0), Some(-12.0)));
        assert_eq!((b.status.as_str(), b.temperature_c), ("Discharging", Some(30.5)));
        assert_eq!(chemistry_name(b"LION"), "Li-ion");
        assert_eq!(chemistry_name(b"PbAc"), "Lead acid");
    }
}

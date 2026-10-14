//! SMART / NVMe health decoding, OS-neutral.
//! Sources: ATA/ACS SMART READ DATA / READ THRESHOLDS layout (30 × 12-byte
//! entries from offset 2), NVMe base spec SMART / Health Information log page
//! (LID 02h), smartmontools' documented `smartctl -j` JSON output. Attribute
//! names follow the publicly documented common meanings; raw-value encodings
//! are vendor specific, so derived counters use the widely shared ones.

use crate::report::{Node, f, g, gv};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub enum HealthLevel {
    #[default]
    Unknown,
    Good,
    Caution,
    Bad,
}

impl HealthLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            HealthLevel::Unknown => "Unknown",
            HealthLevel::Good => "Good",
            HealthLevel::Caution => "Caution",
            HealthLevel::Bad => "Bad",
        }
    }
}

/// One ATA SMART attribute.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct AtaAttr {
    pub id: u8,
    pub name: String,
    /// Bit 0 pre-failure, bit 1 online collection, …
    pub flags: u16,
    pub current: u8,
    pub worst: u8,
    /// From READ THRESHOLDS (admin/root only on most systems).
    pub threshold: Option<u8>,
    /// 48-bit raw value, little endian as stored.
    pub raw: u64,
}

impl AtaAttr {
    pub fn failing(&self) -> bool {
        self.threshold.is_some_and(|t| t > 0 && self.current <= t)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Smart {
    /// "NVMe Health log (02h)", "ATA SMART", "smartctl", "system_profiler"…
    pub source: String,
    pub health: HealthLevel,
    pub reasons: Vec<String>,
    /// Drive's own overall self-assessment (ATA RETURN STATUS / predict failure / OS).
    pub passed: Option<bool>,
    pub temperature_c: Option<f64>,
    pub power_on_hours: Option<u64>,
    pub power_cycles: Option<u64>,
    pub data_read_bytes: Option<u64>,
    pub data_written_bytes: Option<u64>,
    /// Endurance used, % (NVMe Percentage Used / SSD wear attributes). May exceed 100.
    pub percent_used: Option<u32>,
    pub unsafe_shutdowns: Option<u64>,
    pub media_errors: Option<u64>,
    pub reallocated_sectors: Option<u64>,
    pub pending_sectors: Option<u64>,
    pub uncorrectable_sectors: Option<u64>,
    pub nvme_critical_warning: Option<u8>,
    pub nvme_available_spare: Option<(u8, u8)>,
    /// Complete ATA attribute table.
    pub ata_attributes: Vec<AtaAttr>,
    /// Every NVMe Health log field, in spec order.
    pub nvme_log: Vec<(String, String)>,
}

/// Common public meanings of ATA SMART attribute IDs.
pub fn ata_attr_name(id: u8) -> &'static str {
    match id {
        1 => "Raw Read Error Rate",
        2 => "Throughput Performance",
        3 => "Spin-Up Time",
        4 => "Start/Stop Count",
        5 => "Reallocated Sectors Count",
        6 => "Read Channel Margin",
        7 => "Seek Error Rate",
        8 => "Seek Time Performance",
        9 => "Power-On Hours",
        10 => "Spin Retry Count",
        11 => "Recalibration Retries",
        12 => "Power Cycle Count",
        13 => "Soft Read Error Rate",
        22 => "Current Helium Level",
        170 => "Available Reserved Space",
        171 => "Program Fail Count",
        172 => "Erase Fail Count",
        173 => "Wear Leveling Count",
        174 => "Unexpected Power Loss Count",
        175 => "Power Loss Protection Failure",
        176 => "Erase Fail Count",
        177 => "Wear Range Delta",
        179 => "Used Reserved Block Count Total",
        180 => "Unused Reserved Block Count Total",
        181 => "Program Fail Count Total",
        182 => "Erase Fail Count Total",
        183 => "SATA Downshift Error Count",
        184 => "End-to-End Error",
        185 => "Head Stability",
        186 => "Induced Op-Vibration Detection",
        187 => "Reported Uncorrectable Errors",
        188 => "Command Timeout",
        189 => "High Fly Writes",
        190 => "Airflow Temperature",
        191 => "G-Sense Error Rate",
        192 => "Power-off Retract Count",
        193 => "Load Cycle Count",
        194 => "Temperature",
        195 => "Hardware ECC Recovered",
        196 => "Reallocation Event Count",
        197 => "Current Pending Sector Count",
        198 => "Uncorrectable Sector Count",
        199 => "UltraDMA CRC Error Count",
        200 => "Multi-Zone Error Rate",
        201 => "Soft Read Error Rate",
        202 => "Data Address Mark Errors",
        203 => "Run Out Cancel",
        204 => "Soft ECC Correction",
        205 => "Thermal Asperity Rate",
        206 => "Flying Height",
        207 => "Spin High Current",
        208 => "Spin Buzz",
        209 => "Offline Seek Performance",
        210 | 211 => "Vibration During Write",
        212 => "Shock During Write",
        220 => "Disk Shift",
        221 => "G-Sense Error Rate",
        222 => "Loaded Hours",
        223 => "Load/Unload Retry Count",
        224 => "Load Friction",
        225 => "Load/Unload Cycle Count",
        226 => "Load-in Time",
        227 => "Torque Amplification Count",
        228 => "Power-Off Retract Cycle",
        230 => "GMR Head Amplitude",
        231 => "Life Left",
        232 => "Endurance Remaining",
        233 => "Media Wearout Indicator",
        234 => "Average/Maximum Erase Count",
        235 => "Good Block / Free Block Count",
        240 => "Head Flying Hours",
        241 => "Total LBAs Written",
        242 => "Total LBAs Read",
        243 => "Total LBAs Written Expanded",
        244 => "Total LBAs Read Expanded",
        249 => "NAND Writes (1 GiB)",
        250 => "Read Error Retry Rate",
        251 => "Minimum Spares Remaining",
        252 => "Newly Added Bad Flash Block",
        254 => "Free Fall Protection",
        _ => "Vendor Specific",
    }
}

/// Decode SMART READ DATA (512 bytes) plus optional READ THRESHOLDS.
pub fn ata_smart(data: &[u8], thresholds: Option<&[u8]>) -> Option<Smart> {
    let table = data.get(2..362)?;
    let thr = thresholds.and_then(|t| t.get(2..362));
    let attrs: Vec<AtaAttr> = table
        .as_chunks::<12>()
        .0
        .iter()
        .filter(|e| e[0] != 0)
        .map(|e| AtaAttr {
            id: e[0],
            name: ata_attr_name(e[0]).into(),
            flags: u16::from_le_bytes([e[1], e[2]]),
            current: e[3],
            worst: e[4],
            threshold: thr.and_then(|t| t.as_chunks::<12>().0.iter().find(|x| x[0] == e[0])).map(|x| x[1]),
            raw: u64::from_le_bytes([e[5], e[6], e[7], e[8], e[9], e[10], 0, 0]),
        })
        .collect();
    if attrs.is_empty() {
        return None;
    }
    let mut s = Smart { source: "ATA SMART".into(), ata_attributes: attrs, ..Smart::default() };
    s.derive_ata();
    Some(s)
}

impl Smart {
    /// Summary counters from the attribute table.
    // ponytail: common encodings only; vendors that store LBAs in 32 MiB units
    // or hours in minutes read off. Per-model quirk tables would fix that.
    fn derive_ata(&mut self) {
        let attrs = std::mem::take(&mut self.ata_attributes);
        let attr = |id: u8| attrs.iter().find(|a| a.id == id);
        let raw = |id| attr(id).map(|a| a.raw);
        let lo32 = |id| raw(id).map(|r| r & 0xFFFF_FFFF);
        self.temperature_c = raw(194).or(raw(190)).map(|r| (r & 0xFF) as f64).filter(|&t| t > 0.0 && t < 128.0);
        self.power_on_hours = lo32(9);
        self.power_cycles = lo32(12);
        self.data_written_bytes = raw(241).map(|r| r.saturating_mul(512));
        self.data_read_bytes = raw(242).map(|r| r.saturating_mul(512));
        self.unsafe_shutdowns = lo32(174).or(lo32(192));
        self.reallocated_sectors = lo32(5);
        self.pending_sectors = lo32(197);
        self.uncorrectable_sectors = lo32(198).or(lo32(187));
        // SSD life-left style attributes: the normalized value is % remaining.
        self.percent_used = [231, 233, 177]
            .iter()
            .find_map(|&id| attr(id))
            .filter(|a| a.current <= 100)
            .map(|a| 100 - a.current as u32);
        self.ata_attributes = attrs;
    }

    /// Fill `health` and `reasons` from whatever was read.
    pub fn evaluate(&mut self) {
        let (mut bad, mut caution) = (Vec::new(), Vec::new());
        if self.passed == Some(false) {
            bad.push("drive reports SMART failure".to_string());
        }
        for a in self.ata_attributes.iter().filter(|a| a.failing()) {
            bad.push(format!("{:02X} {} at or below threshold", a.id, a.name));
        }
        if let Some(w) = self.nvme_critical_warning {
            let bits = [
                (0, "available spare below threshold", true),
                (1, "temperature outside threshold", false),
                (2, "NVM subsystem reliability degraded", true),
                (3, "media in read-only mode", true),
                (4, "volatile memory backup failed", true),
                (5, "persistent memory region read-only", true),
            ];
            for (b, msg, is_bad) in bits {
                if w & (1 << b) != 0 {
                    if is_bad { &mut bad } else { &mut caution }.push(msg.to_string());
                }
            }
        }
        let counts = [
            (self.reallocated_sectors, "reallocated sectors"),
            (self.pending_sectors, "pending sectors"),
            (self.uncorrectable_sectors, "uncorrectable sectors"),
            (self.media_errors, "media errors"),
        ];
        for (n, what) in counts {
            if let Some(n) = n.filter(|&n| n > 0) {
                caution.push(format!("{n} {what}"));
            }
        }
        if let Some(p) = self.percent_used.filter(|&p| p >= 90) {
            caution.push(format!("{p}% of rated endurance used"));
        }
        // A temperature alone says nothing about health.
        let any = self.passed.is_some() || !self.ata_attributes.is_empty() || self.nvme_critical_warning.is_some();
        self.health = if !bad.is_empty() {
            HealthLevel::Bad
        } else if !caution.is_empty() {
            HealthLevel::Caution
        } else if any {
            HealthLevel::Good
        } else {
            HealthLevel::Unknown
        };
        self.reasons = bad.into_iter().chain(caution).collect();
    }

    pub fn health_line(&self) -> String {
        if self.reasons.is_empty() {
            self.health.as_str().into()
        } else {
            format!("{} ({})", self.health.as_str(), self.reasons.join("; "))
        }
    }

    /// Key/value summary in display order.
    pub fn summary(&self) -> Vec<(String, String)> {
        let mut v = vec![("Health Status".to_string(), self.health_line())];
        let mut add = |k: &str, val: Option<String>| {
            if let Some(val) = val {
                v.push((k.into(), val));
            }
        };
        add("SMART Status", self.passed.map(|p| if p { "Passed" } else { "Failed" }.into()));
        add("Temperature", self.temperature_c.map(|t| format!("{t:.0} C")));
        add("Power On Hours", self.power_on_hours.map(|h| format!("{h} hours")));
        add("Power Cycles", self.power_cycles.map(|n| n.to_string()));
        add("Data Read", self.data_read_bytes.map(tb));
        add("Data Written", self.data_written_bytes.map(tb));
        add("Percentage Used", self.percent_used.map(|p| format!("{p}% ({}% remaining)", 100u32.saturating_sub(p))));
        add("Available Spare", self.nvme_available_spare.map(|(a, t)| format!("{a}% (threshold {t}%)")));
        add("Unsafe Shutdowns", self.unsafe_shutdowns.map(|n| n.to_string()));
        add("Media Errors", self.media_errors.map(|n| n.to_string()));
        add("Reallocated Sectors", self.reallocated_sectors.map(|n| n.to_string()));
        add("Pending Sectors", self.pending_sectors.map(|n| n.to_string()));
        add("Uncorrectable Sectors", self.uncorrectable_sectors.map(|n| n.to_string()));
        v
    }

    /// "SMART" report group: summary, then the full attribute table / log.
    pub fn node(&self) -> Node {
        let mut c: Vec<Node> = vec![f("Source", &self.source)];
        c.extend(self.summary().into_iter().skip(1).map(|(k, v)| f(k, v))); // health is the group value
        if !self.ata_attributes.is_empty() {
            let mut t = vec![f("ID Name", "Cur Wst Thr Raw          Flags")];
            for a in &self.ata_attributes {
                let thr = a.threshold.map_or("---".into(), |t| format!("{t:>3}"));
                let fail = if a.failing() { "  FAILING" } else { "" };
                t.push(f(
                    format!("{:02X} {}", a.id, a.name),
                    format!("{:>3} {:>3} {thr} {:012X} {:04X}{fail}", a.current, a.worst, a.raw, a.flags),
                ));
            }
            c.push(g("Attributes", t));
        }
        if !self.nvme_log.is_empty() {
            c.push(g("NVMe Health Log", self.nvme_log.iter().map(|(k, v)| f(k, v)).collect()));
        }
        gv("SMART", self.health_line(), c)
    }
}

/// Decimal terabytes, e.g. "12.34 TB".
pub fn tb(bytes: u64) -> String {
    format!("{:.2} TB", bytes as f64 / 1e12)
}

/// NVMe SMART / Health Information log page (LID 02h), 512 bytes.
pub fn nvme_smart(l: &[u8]) -> Option<Smart> {
    if l.len() < 232 {
        return None;
    }
    let u16le = |o: usize| u16::from_le_bytes([l[o], l[o + 1]]);
    let u32le = |o: usize| u32::from_le_bytes([l[o], l[o + 1], l[o + 2], l[o + 3]]);
    let u128le = |o: usize| u128::from_le_bytes(l[o..o + 16].try_into().unwrap_or_default());
    let sat = |v: u128| u64::try_from(v).unwrap_or(u64::MAX);
    let kelvin = |k: u16| if k == 0 { "Not reported".to_string() } else { format!("{} C ({k} K)", k as i32 - 273) };
    let (temp_k, read_units, write_units) = (u16le(1), u128le(32), u128le(48));
    let mut s = Smart {
        source: "NVMe Health log (02h)".into(),
        nvme_critical_warning: Some(l[0]),
        nvme_available_spare: Some((l[3], l[4])),
        temperature_c: (temp_k > 0).then_some(temp_k as f64 - 273.0),
        percent_used: Some(l[5] as u32),
        data_read_bytes: Some(sat(read_units.saturating_mul(512_000))),
        data_written_bytes: Some(sat(write_units.saturating_mul(512_000))),
        power_cycles: Some(sat(u128le(112))),
        power_on_hours: Some(sat(u128le(128))),
        unsafe_shutdowns: Some(sat(u128le(144))),
        media_errors: Some(sat(u128le(160))),
        ..Smart::default()
    };
    let mut log = vec![
        ("Critical Warning".to_string(), format!("0x{:02X}", l[0])),
        ("Composite Temperature".into(), kelvin(temp_k)),
        ("Available Spare".into(), format!("{}%", l[3])),
        ("Available Spare Threshold".into(), format!("{}%", l[4])),
        ("Percentage Used".into(), format!("{}%", l[5])),
        ("Endurance Group Critical Warning Summary".into(), format!("0x{:02X}", l[6])),
        ("Data Units Read".into(), format!("{read_units} ({})", tb(sat(read_units.saturating_mul(512_000))))),
        ("Data Units Written".into(), format!("{write_units} ({})", tb(sat(write_units.saturating_mul(512_000))))),
        ("Host Read Commands".into(), u128le(64).to_string()),
        ("Host Write Commands".into(), u128le(80).to_string()),
        ("Controller Busy Time".into(), format!("{} minutes", u128le(96))),
        ("Power Cycles".into(), u128le(112).to_string()),
        ("Power On Hours".into(), u128le(128).to_string()),
        ("Unsafe Shutdowns".into(), u128le(144).to_string()),
        ("Media and Data Integrity Errors".into(), u128le(160).to_string()),
        ("Error Information Log Entries".into(), u128le(176).to_string()),
        ("Warning Composite Temperature Time".into(), format!("{} minutes", u32le(192))),
        ("Critical Composite Temperature Time".into(), format!("{} minutes", u32le(196))),
    ];
    for i in 0..8 {
        let k = u16le(200 + i * 2);
        if k != 0 {
            log.push((format!("Temperature Sensor {}", i + 1), kelvin(k)));
        }
    }
    log.extend([
        ("Thermal Mgmt Temp 1 Transition Count".into(), u32le(216).to_string()),
        ("Thermal Mgmt Temp 2 Transition Count".into(), u32le(220).to_string()),
        ("Total Time For Thermal Mgmt Temp 1".into(), format!("{} s", u32le(224))),
        ("Total Time For Thermal Mgmt Temp 2".into(), format!("{} s", u32le(228))),
    ]);
    s.nvme_log = log;
    s.evaluate();
    Some(s)
}

/// `smartctl -j -a DEV` output (smartmontools 7+ JSON schema).
pub fn from_smartctl(v: &Value) -> Option<Smart> {
    let u = |p: &str| v.pointer(p).and_then(Value::as_u64);
    let mut s = if let Some(n) = v.get("nvme_smart_health_information_log").filter(|n| n.is_object()) {
        // Rebuild a raw log page so NVMe has one decoder.
        let mut l = vec![0u8; 512];
        let n = |k: &str| n.get(k).and_then(Value::as_u64).unwrap_or(0);
        let put = |l: &mut [u8], o: usize, v: u64, w: usize| l[o..o + w].copy_from_slice(&v.to_le_bytes()[..w]);
        put(&mut l, 0, n("critical_warning"), 1);
        put(&mut l, 1, if n("temperature") > 0 { n("temperature") + 273 } else { 0 }, 2);
        put(&mut l, 3, n("available_spare"), 1);
        put(&mut l, 4, n("available_spare_threshold"), 1);
        put(&mut l, 5, n("percentage_used"), 1);
        for (o, k) in [
            (32, "data_units_read"),
            (48, "data_units_written"),
            (64, "host_reads"),
            (80, "host_writes"),
            (96, "controller_busy_time"),
            (112, "power_cycles"),
            (128, "power_on_hours"),
            (144, "unsafe_shutdowns"),
            (160, "media_errors"),
            (176, "num_err_log_entries"),
        ] {
            put(&mut l, o, n(k), 8);
        }
        put(&mut l, 192, n("warning_temp_time"), 4);
        put(&mut l, 196, n("critical_comp_time"), 4);
        let sensors = n_arr(v, "/nvme_smart_health_information_log/temperature_sensors");
        for (i, t) in sensors.iter().take(8).enumerate() {
            put(&mut l, 200 + i * 2, t + 273, 2);
        }
        nvme_smart(&l)?
    } else if let Some(t) = v.pointer("/ata_smart_attributes/table").and_then(Value::as_array) {
        let attrs: Vec<AtaAttr> = t
            .iter()
            .filter_map(|a| {
                let n = |k: &str| a.get(k).and_then(Value::as_u64);
                Some(AtaAttr {
                    id: n("id")? as u8,
                    name: a
                        .get("name")
                        .and_then(Value::as_str)
                        .map_or_else(|| ata_attr_name(n("id").unwrap_or(0) as u8).into(), |s| s.replace('_', " ")),
                    flags: a.pointer("/flags/value").and_then(Value::as_u64).unwrap_or(0) as u16,
                    current: n("value").unwrap_or(0) as u8,
                    worst: n("worst").unwrap_or(0) as u8,
                    threshold: n("thresh").map(|t| t as u8),
                    raw: a.pointer("/raw/value").and_then(Value::as_u64).unwrap_or(0),
                })
            })
            .collect();
        let mut s = Smart { ata_attributes: attrs, ..Smart::default() };
        s.derive_ata();
        s
    } else {
        Smart::default()
    };
    s.source = "smartctl".into();
    s.passed = v.pointer("/smart_status/passed").and_then(Value::as_bool);
    s.temperature_c = s.temperature_c.or(u("/temperature/current").map(|t| t as f64));
    s.power_on_hours = s.power_on_hours.or(u("/power_on_time/hours"));
    s.power_cycles = s.power_cycles.or(u("/power_cycle_count"));
    s.evaluate();
    (s.health != HealthLevel::Unknown).then_some(s)
}

fn n_arr(v: &Value, p: &str) -> Vec<u64> {
    v.pointer(p).and_then(Value::as_array).map_or(Vec::new(), |a| a.iter().filter_map(Value::as_u64).collect())
}

/// Only an overall verdict (macOS system_profiler "Verified" / "Failing").
pub fn from_status(status: &str, source: &str) -> Smart {
    let st = status.to_ascii_lowercase();
    let passed = if st.contains("verified") || st.contains("pass") || st == "ok" {
        Some(true)
    } else if st.contains("fail") {
        Some(false)
    } else {
        None
    };
    let mut s = Smart { source: source.into(), passed, ..Smart::default() };
    s.evaluate();
    s
}

/// `smartctl -j -a <dev>` when smartmontools is installed. Exit bits 0/1 mean
/// the command or the device open failed; higher bits only report findings.
#[cfg(unix)]
pub fn smartctl(dev: &str) -> Option<Smart> {
    for exe in [
        "smartctl",
        "/usr/sbin/smartctl",
        "/usr/local/sbin/smartctl",
        "/opt/homebrew/bin/smartctl",
        "/usr/local/bin/smartctl",
    ] {
        let Ok(o) = std::process::Command::new(exe).args(["-j", "-a", dev]).output() else { continue };
        if o.status.code().is_some_and(|c| c & 3 == 0)
            && let Ok(v) = serde_json::from_slice::<Value>(&o.stdout)
        {
            return from_smartctl(&v);
        }
        return None; // found smartctl but it couldn't read this device
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(t: &mut [u8], slot: usize, id: u8, flags: u16, cur: u8, worst: u8, raw: u64) {
        let o = 2 + slot * 12;
        t[o] = id;
        t[o + 1..o + 3].copy_from_slice(&flags.to_le_bytes());
        t[o + 3] = cur;
        t[o + 4] = worst;
        t[o + 5..o + 11].copy_from_slice(&raw.to_le_bytes()[..6]);
    }

    #[test]
    fn ata_attributes() {
        let mut d = vec![0u8; 512];
        entry(&mut d, 0, 5, 0x33, 100, 100, 8);
        entry(&mut d, 1, 9, 0x32, 95, 95, 0x0000_0001_0000_1234); // high bytes vendor data
        entry(&mut d, 2, 12, 0x32, 99, 99, 321);
        entry(&mut d, 3, 194, 0x22, 64, 50, 0x0014_0028_0024); // 36 C (min/max packed above)
        entry(&mut d, 4, 197, 0x12, 100, 100, 0);
        entry(&mut d, 5, 241, 0x32, 100, 100, 2_000_000_000);
        entry(&mut d, 6, 231, 0x13, 97, 97, 0);
        let mut t = vec![0u8; 512];
        for (i, (id, th)) in [(5u8, 10u8), (9, 0), (231, 98)].iter().enumerate() {
            t[2 + i * 12] = *id;
            t[3 + i * 12] = *th;
        }
        let s = ata_smart(&d, Some(&t)).unwrap();
        assert_eq!(s.ata_attributes.len(), 7);
        let a = &s.ata_attributes[0];
        assert_eq!(
            (a.id, a.name.as_str(), a.flags, a.current, a.threshold, a.raw),
            (5, "Reallocated Sectors Count", 0x33, 100, Some(10), 8)
        );
        assert_eq!(s.ata_attributes[1].threshold, Some(0));
        assert_eq!(s.ata_attributes[2].threshold, None);
        assert_eq!(s.power_on_hours, Some(0x1234));
        assert_eq!(s.power_cycles, Some(321));
        assert_eq!(s.temperature_c, Some(36.0));
        assert_eq!(s.reallocated_sectors, Some(8));
        assert_eq!(s.data_written_bytes, Some(1_024_000_000_000));
        assert_eq!(s.percent_used, Some(3));
        let mut s = s;
        s.evaluate();
        // 231 at 97 <= threshold 98 → Bad; 8 reallocated → also listed.
        assert_eq!(s.health, HealthLevel::Bad);
        assert!(s.reasons[0].starts_with("E7 Life Left"));
        assert!(s.reasons.iter().any(|r| r == "8 reallocated sectors"));
        assert!(ata_smart(&[0u8; 512], None).is_none());
        assert!(ata_smart(&[0u8; 100], None).is_none());
    }

    #[test]
    fn nvme_log() {
        let mut l = vec![0u8; 512];
        l[1..3].copy_from_slice(&310u16.to_le_bytes());
        l[3] = 100;
        l[4] = 10;
        l[5] = 3;
        l[32] = 10; // 10 data units read
        l[48..50].copy_from_slice(&1000u16.to_le_bytes());
        l[112] = 50;
        l[128..130].copy_from_slice(&1234u16.to_le_bytes());
        l[144] = 7;
        l[200..202].copy_from_slice(&305u16.to_le_bytes());
        let s = nvme_smart(&l).unwrap();
        assert_eq!(s.temperature_c, Some(37.0));
        assert_eq!(s.percent_used, Some(3));
        assert_eq!(s.power_on_hours, Some(1234));
        assert_eq!(s.power_cycles, Some(50));
        assert_eq!(s.unsafe_shutdowns, Some(7));
        assert_eq!(s.data_read_bytes, Some(5_120_000));
        assert_eq!(s.data_written_bytes, Some(512_000_000));
        assert_eq!(s.health, HealthLevel::Good);
        let get = |k: &str| s.nvme_log.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        assert_eq!(get("Composite Temperature"), Some("37 C (310 K)"));
        assert_eq!(get("Temperature Sensor 1"), Some("32 C (305 K)"));
        assert_eq!(get("Temperature Sensor 2"), None);
        assert_eq!(get("Data Units Written"), Some("1000 (0.00 TB)"));
        l[0] = 0x04; // reliability degraded
        l[160] = 2;
        let s = nvme_smart(&l).unwrap();
        assert_eq!(s.health, HealthLevel::Bad);
        assert_eq!(s.health_line(), "Bad (NVM subsystem reliability degraded; 2 media errors)");
        assert!(nvme_smart(&l[..100]).is_none());
    }

    #[test]
    fn smartctl_json() {
        let v: Value = serde_json::from_str(
            r#"{"smart_status":{"passed":true},"temperature":{"current":41},"power_on_time":{"hours":900},
            "ata_smart_attributes":{"table":[{"id":5,"name":"Reallocated_Sector_Ct","value":100,"worst":100,
            "thresh":10,"flags":{"value":51},"raw":{"value":0}},{"id":9,"name":"Power_On_Hours","value":99,
            "worst":99,"thresh":0,"flags":{"value":50},"raw":{"value":900}}]}}"#,
        )
        .unwrap();
        let s = from_smartctl(&v).unwrap();
        assert_eq!(s.ata_attributes[0].name, "Reallocated Sector Ct");
        assert_eq!((s.passed, s.power_on_hours, s.temperature_c), (Some(true), Some(900), Some(41.0)));
        assert_eq!(s.health, HealthLevel::Good);
        let n: Value = serde_json::from_str(
            r#"{"nvme_smart_health_information_log":{"critical_warning":0,"temperature":35,"percentage_used":95,
            "data_units_written":2000000,"power_on_hours":10}}"#,
        )
        .unwrap();
        let s = from_smartctl(&n).unwrap();
        assert_eq!((s.temperature_c, s.percent_used, s.health), (Some(35.0), Some(95), HealthLevel::Caution));
        assert_eq!(s.data_written_bytes, Some(1_024_000_000_000));
        assert_eq!(from_status("Verified", "x").health, HealthLevel::Good);
        assert_eq!(from_status("Failing", "x").health, HealthLevel::Bad);
    }
}

//! Startup applications model, `StartupApproved` 12-byte decoder/encoder,
//! WDI `StartupInfo` XML impact classifier, and boot timeline (S2a-08, docs/v1/task-manager.md §1, §6).

use serde::{Deserialize, Serialize};

/// Startup impact classification (Microsoft Task Manager thresholds, docs/v1/task-manager.md §1):
/// - **High**: > 1 s (1_000 ms) of CPU or > 3 MB (3_000_000 B) of disk I/O
/// - **Medium**: 300 ms..=1_000 ms of CPU or 300 KB..=3 MB of disk I/O
/// - **Low**: < 300 ms of CPU and < 300 KB of disk I/O
/// - **NotMeasured**: no WDI / boot measurement available
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Impact {
    High,
    Medium,
    Low,
    #[default]
    NotMeasured,
}

impl Impact {
    pub fn label(self) -> &'static str {
        match self {
            Impact::High => "High",
            Impact::Medium => "Medium",
            Impact::Low => "Low",
            Impact::NotMeasured => "Not measured",
        }
    }

    pub fn from_metrics(cpu_ms: u64, disk_bytes: u64) -> Self {
        if cpu_ms == 0 && disk_bytes == 0 {
            return Impact::NotMeasured;
        }
        if cpu_ms > 1_000 || disk_bytes > 3_000_000 {
            Impact::High
        } else if cpu_ms >= 300 || disk_bytes >= 300_000 {
            Impact::Medium
        } else {
            Impact::Low
        }
    }
}

/// Where the startup entry lives (used by the UI and by `StartupOp::Toggle` to know what to modify).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StartupSource {
    #[default]
    HkcuRun,
    HkcuRunOnce,
    HklmRun,
    HklmRun32,
    HklmRunOnce,
    UserFolder,
    CommonFolder,
    XdgUser,
    XdgSystem,
    SystemdUser,
    LaunchAgentUser,
    LaunchAgentSystem,
}

impl StartupSource {
    pub fn label(self) -> &'static str {
        match self {
            StartupSource::HkcuRun => "Registry · HKCU\\…\\Run",
            StartupSource::HkcuRunOnce => "Registry · HKCU\\…\\RunOnce",
            StartupSource::HklmRun => "Registry · HKLM\\…\\Run",
            StartupSource::HklmRun32 => "Registry · HKLM\\…\\Wow6432Node\\…\\Run",
            StartupSource::HklmRunOnce => "Registry · HKLM\\…\\RunOnce",
            StartupSource::UserFolder => "Startup folder (User)",
            StartupSource::CommonFolder => "Startup folder (All users)",
            StartupSource::XdgUser => "XDG Autostart (~/.config/autostart)",
            StartupSource::XdgSystem => "XDG Autostart (/etc/xdg/autostart)",
            StartupSource::SystemdUser => "systemd --user",
            StartupSource::LaunchAgentUser => "~/Library/LaunchAgents",
            StartupSource::LaunchAgentSystem => "/Library/LaunchAgents",
        }
    }

    pub fn needs_admin(self) -> bool {
        matches!(
            self,
            StartupSource::HklmRun
                | StartupSource::HklmRun32
                | StartupSource::HklmRunOnce
                | StartupSource::CommonFolder
                | StartupSource::LaunchAgentSystem
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StartupRow {
    /// Entry identifier (registry value name, `.lnk` filename, `.desktop` filename, or unit/plist label).
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub enabled: bool,
    /// ISO-8601 date (`YYYY-MM-DD`) when disabled, decoded from `StartupApproved` FILETIME.
    pub disabled_date: Option<String>,
    pub impact: Impact,
    pub cpu_ms: Option<u64>,
    pub disk_bytes: Option<u64>,
    pub source: StartupSource,
    pub command: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootSummary {
    /// Firmware POST time in milliseconds (`FwPOSTTime`, `FirmwareTimestampMonotonic`, or ACPI FPDT).
    pub bios_time_ms: Option<u64>,
    /// Kernel + OS init time in milliseconds, when available.
    pub os_boot_ms: Option<u64>,
    /// Age or date of the WDI `StartupInfo` XML file if present.
    pub impact_source_note: Option<String>,
    pub items: Vec<StartupRow>,
}

/// Decodes a 12-byte Windows `StartupApproved` binary value (docs/v1/task-manager.md §1):
/// - bytes 0..4: LE u32; bit 0 set (`value & 1 != 0`, e.g. `0x03`) => **disabled**; clear (`0x02` / `0x06`) => **enabled**.
/// - bytes 4..12: Windows `FILETIME` (100-ns intervals since 1601-01-01 UTC) when disabled.
///
/// Returns `(enabled, optional YYYY-MM-DD disabled date)`.
pub fn decode_startup_approved(bytes: &[u8]) -> (bool, Option<String>) {
    if bytes.len() < 4 {
        return (true, None);
    }
    let flags = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let enabled = (flags & 1) == 0;
    if enabled || bytes.len() < 12 {
        return (enabled, None);
    }
    let ft = u64::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11]]);
    let date = filetime_to_ymd(ft);
    (false, date)
}

/// Encodes a 12-byte Windows `StartupApproved` binary value:
/// - enabled: `02 00 00 00` + 8 zero bytes
/// - disabled: `03 00 00 00` + 8-byte `FILETIME` for `unix_secs`
pub fn encode_startup_approved(enabled: bool, unix_secs: u64) -> [u8; 12] {
    let mut out = [0u8; 12];
    if enabled {
        out[0] = 0x02;
    } else {
        out[0] = 0x03;
        let ft = (unix_secs.saturating_add(11_644_473_600)).saturating_mul(10_000_000);
        out[4..12].copy_from_slice(&ft.to_le_bytes());
    }
    out
}

/// Converts a Windows `FILETIME` (100-ns intervals since 1601-01-01) to `YYYY-MM-DD`.
pub fn filetime_to_ymd(ft: u64) -> Option<String> {
    const EPOCH_DIFF_SECS: u64 = 11_644_473_600;
    let secs = ft / 10_000_000;
    if secs <= EPOCH_DIFF_SECS {
        return None;
    }
    let unix_secs = secs - EPOCH_DIFF_SECS;
    let days = (unix_secs / 86_400) as i64;
    // Howard Hinnant civil_from_days algorithm
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    if !(1990..=2100).contains(&year) {
        return None;
    }
    Some(format!("{year:04}-{m:02}-{d:02}"))
}

/// Extracts the executable path from a command line (e.g. `"C:\Program Files\App\app.exe" --minimized`).
pub fn extract_exe_path(cmd: &str) -> String {
    let s = cmd.trim();
    if let Some(rest) = s.strip_prefix('"')
        && let Some(end) = rest.find('"')
    {
        return rest[..end].to_string();
    }
    if let Some(pos) = s.to_ascii_lowercase().find(".exe") {
        return s[..pos + 4].trim().to_string();
    }
    s.split_whitespace().next().unwrap_or("").to_string()
}

/// Parses a Windows WDI `StartupInfo*.xml` document into `(exe_name_lowercase, cpu_ms, disk_bytes)` entries.
pub fn parse_wdi_startup_xml(xml: &str) -> Vec<(String, u64, u64)> {
    let mut out = Vec::new();
    for block in xml.split("<Process ").skip(1) {
        let name = attr_value(block, "Name=\"").unwrap_or_default().to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        let cpu_us = tag_u64(block, "<CpuUsage").unwrap_or(0);
        let disk_b = tag_u64(block, "<DiskUsage").unwrap_or(0);
        out.push((name, cpu_us / 1_000, disk_b));
    }
    out
}

fn attr_value<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let start = s.find(prefix)? + prefix.len();
    let rest = &s[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn tag_u64(s: &str, open_tag: &str) -> Option<u64> {
    let start = s.find(open_tag)?;
    let after_open = &s[start..];
    let gt = after_open.find('>')? + 1;
    let body = &after_open[gt..];
    let lt = body.find('<')?;
    body[..lt].trim().parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_approved_round_trip_and_filetime_decoding() {
        let enabled_bytes = encode_startup_approved(true, 1_762_214_400);
        assert_eq!(decode_startup_approved(&enabled_bytes), (true, None));

        // 2026-11-04 00:00:00 UTC = 1793750400
        let disabled_bytes = encode_startup_approved(false, 1_793_750_400);
        let (en, date) = decode_startup_approved(&disabled_bytes);
        assert!(!en);
        assert_eq!(date.as_deref(), Some("2026-11-04"));
    }

    #[test]
    fn impact_thresholds_and_wdi_xml_parser() {
        assert_eq!(Impact::from_metrics(0, 0), Impact::NotMeasured);
        assert_eq!(Impact::from_metrics(150, 100_000), Impact::Low);
        assert_eq!(Impact::from_metrics(450, 100_000), Impact::Medium);
        assert_eq!(Impact::from_metrics(1500, 100_000), Impact::High);
        assert_eq!(Impact::from_metrics(100, 4_000_000), Impact::High);

        let xml = r#"
        < StartupInfo >
            <Process Name=" OneDrive.exe" PID="4120">
                <DiskUsage Units="bytes">4500000</DiskUsage>
                <CpuUsage Units="us">1250000</CpuUsage>
            </Process>
        </StartupInfo>
        "#;
        let parsed = parse_wdi_startup_xml(xml);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0.trim(), "onedrive.exe");
        assert_eq!(parsed[0].1, 1250);
        assert_eq!(parsed[0].2, 4_500_000);
    }
}

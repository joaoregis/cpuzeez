//! Linux hwmon sysfs ABI (Documentation/hwmon/sysfs-interface): one directory
//! per chip with `name` and `<type><n>_input` files in milli-units.
//! Pure file parsing, so it builds (and is tested) on every OS.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use super::{Kind, Monitor, Reading};
use std::path::Path;

pub const ROOT: &str = "/sys/class/hwmon";

/// CPU-Z style chip name for a hwmon `name`.
pub fn display_name(name: &str, cpu_family: u32) -> String {
    let up = name.to_ascii_uppercase();
    let pre = |p: &str| name.starts_with(p) && name[p.len()..].starts_with(|c: char| c.is_ascii_digit());
    match name {
        _ if pre("nct") => format!("Nuvoton {up}"),
        _ if pre("w83") => format!("Winbond {up}"),
        _ if pre("it") => format!("ITE {up}"),
        _ if pre("f71") || pre("f81") || pre("f8") => format!("Fintek {up}"),
        "k10temp" | "zenpower" if cpu_family >= 0x17 => "AMD Ryzen Power Management".into(),
        "k10temp" | "fam15h_power" => "AMD K10".into(),
        "coretemp" => "Intel Core DTS".into(),
        "spd5118" | "jc42" => "DIMM".into(),
        "amdgpu" => "AMD GPU".into(),
        "nouveau" => "NVIDIA GPU".into(),
        "i915" | "xe" => "Intel GPU".into(),
        "nvme" => "NVMe".into(),
        "drivetemp" => "Drive".into(),
        "acpitz" => "ACPI Thermal Zone".into(),
        _ if name.starts_with("pch_") => "Intel PCH".into(), // intel_pch_thermal zones: pch_skylake, pch_cannonlake, ...
        _ => name.to_string(),
    }
}

/// Every hwmon chip under `root`, sorted like the kernel numbers them.
pub fn read(root: &Path, cpu_family: u32) -> Vec<Monitor> {
    let Ok(rd) = std::fs::read_dir(root) else { return Vec::new() };
    let mut dirs: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    dirs.sort_by_key(|p| {
        p.file_name().and_then(|n| n.to_str()).and_then(|n| n.trim_start_matches("hwmon").parse::<u32>().ok())
    });
    dirs.iter()
        .filter_map(|d| {
            let name = crate::os::read_trim(d.join("name"))?;
            Some(Monitor { name: display_name(&name, cpu_family), readings: readings(d), ..Default::default() })
        })
        .collect()
}

fn readings(dir: &Path) -> Vec<Reading> {
    // (sysfs prefix, file suffixes tried in order, kind, scale to our unit)
    const TYPES: [(&str, &[&str], Kind, f64); 6] = [
        ("in", &["input"], Kind::Voltage, 1e-3),
        ("temp", &["input"], Kind::Temperature, 1e-3),
        ("fan", &["input"], Kind::Fan, 1.0),
        ("power", &["input", "average"], Kind::Power, 1e-6),
        ("curr", &["input"], Kind::Current, 1e-3),
        ("freq", &["input"], Kind::Clock, 1e-6),
    ];
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().filter_map(|e| e.file_name().into_string().ok()).collect())
        .unwrap_or_default();
    names.sort();
    let mut out = Vec::new();
    for (pre, sufs, kind, scale) in TYPES {
        // Channel numbers present for this type, in numeric order.
        let mut chans: Vec<u32> = names
            .iter()
            .filter_map(|n| {
                n.strip_prefix(pre)?
                    .split_once('_')
                    .and_then(|(c, s)| sufs.contains(&s).then(|| c.parse().ok()).flatten())
            })
            .collect();
        chans.sort();
        chans.dedup();
        for c in chans {
            let Some(v) =
                sufs.iter().find_map(|s| crate::os::read_trim(dir.join(format!("{pre}{c}_{s}")))?.parse::<i64>().ok())
            else {
                continue;
            };
            let label = crate::os::read_trim(dir.join(format!("{pre}{c}_label"))).unwrap_or_default();
            out.push(Reading { kind, label, value: v as f64 * scale, raw: None, channel: None });
        }
    }
    out
}

/// CPU temperatures from k10temp/zenpower/coretemp, relabelled like CPU-Z's
/// CPU rows ("Package", "CCD #0", "Core #3").
pub fn cpu_temps(root: &Path) -> Vec<Reading> {
    let mut out = Vec::new();
    for m in read(root, 0x17) {
        if !matches!(m.name.as_str(), "AMD Ryzen Power Management" | "AMD K10" | "Intel Core DTS") {
            continue;
        }
        // Tdie (offset removed) only exists where Tctl carries an offset.
        let has_tdie = m.readings.iter().any(|r| r.label == "Tdie");
        for mut r in m.readings.into_iter().filter(|r| r.kind == Kind::Temperature && !(has_tdie && r.label == "Tctl"))
        {
            r.label = cpu_label(&r.label);
            out.push(r);
        }
    }
    // Package first, as CPU-Z orders them.
    out.sort_by_key(|r| r.label != "Package");
    out
}

fn cpu_label(l: &str) -> String {
    let num = |p: &str| l.strip_prefix(p).and_then(|n| n.trim().parse::<u32>().ok());
    if l == "Tdie" || l == "Tctl" || l.starts_with("Package id") {
        "Package".into()
    } else if let Some(n) = num("Tccd") {
        format!("CCD #{}", n.saturating_sub(1))
    } else if let Some(n) = num("Core") {
        format!("Core #{n}")
    } else {
        l.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(dir: &Path, files: &[(&str, &str)]) {
        std::fs::create_dir_all(dir).unwrap();
        for (n, v) in files {
            std::fs::write(dir.join(n), format!("{v}\n")).unwrap();
        }
    }

    #[test]
    fn fake_sysfs() {
        let root = std::env::temp_dir().join(format!("cpuzeez-hwmon-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        put(&root.join("hwmon10"), &[("name", "spd5118"), ("temp1_input", "39250")]);
        put(
            &root.join("hwmon2"),
            &[
                ("name", "nct6687"),
                ("in0_input", "12048"),
                ("in0_label", "+12V"),
                ("in10_input", "1360"),
                ("in2_input", "1310"),
                ("temp1_input", "71000"),
                ("fan2_input", "868"),
                ("fan2_min", "0"),
                ("power1_average", "79610000"),
            ],
        );
        put(
            &root.join("hwmon3"),
            &[
                ("name", "k10temp"),
                ("temp1_input", "70125"),
                ("temp1_label", "Tctl"),
                ("temp3_input", "56000"),
                ("temp3_label", "Tccd1"),
            ],
        );
        let m = read(&root, 0x19);
        assert_eq!(
            m.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
            ["Nuvoton NCT6687", "AMD Ryzen Power Management", "DIMM"]
        );
        let r = &m[0].readings;
        assert_eq!(
            r.iter().filter(|r| r.kind == Kind::Voltage).map(|r| r.value).collect::<Vec<_>>(),
            [12.048, 1.31, 1.36]
        );
        assert_eq!(r[0].label, "+12V");
        assert!(r.iter().any(|r| r.kind == Kind::Fan && r.value == 868.0));
        assert!(r.iter().any(|r| r.kind == Kind::Power && (r.value - 79.61).abs() < 1e-9));
        assert_eq!(r.iter().filter(|r| r.kind == Kind::Fan).count(), 1, "fan2_min is not a reading");
        let t = cpu_temps(&root);
        assert_eq!(t.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(), ["Package", "CCD #0"]);
        assert_eq!(display_name("pch_cannonlake", 0), "Intel PCH");
        let _ = std::fs::remove_dir_all(&root);
    }
}

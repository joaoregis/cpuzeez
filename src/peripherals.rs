//! Devices page extras (S1-G07), no privileges (in the binary for now: the
//! Windows part uses the CfgMgr32 helpers in `storage::win`):
//! - audio endpoints and the format the OS mixes at (Windows: MMDevices
//!   registry, the same `PKEY_AudioEngine_DeviceFormat` the Sound panel shows;
//!   Linux: ALSA `hw_params` while a stream is open; macOS: CoreAudio via
//!   `system_profiler`);
//! - Bluetooth peripherals' battery (Windows device property, Linux
//!   power_supply with scope Device, macOS `BatteryPercent`);
//! - Thunderbolt / USB4 (Linux `/sys/bus/thunderbolt`; Windows lists the
//!   USB4 host routers on the PCI page's data).

#[derive(Debug, Clone, PartialEq)]
pub struct Audio {
    pub name: String,
    /// "Realtek(R) Audio", "NVIDIA High Definition Audio"…
    pub device: String,
    pub output: bool,
    /// "24-bit, 48 kHz, 2 ch"; None while unknown (Linux: no stream open).
    pub format: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Peripheral {
    pub name: String,
    pub battery: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Thunderbolt {
    pub name: String,
    /// "Thunderbolt 3", "USB4"…
    pub generation: Option<String>,
    /// "40 Gb/s × 2"…
    pub link: Option<String>,
}

pub fn audio() -> Vec<Audio> {
    imp::audio()
}

/// A display adapter's BARs (S1-G02/G04): the memory windows the CPU sees it through.
#[derive(Debug, Clone, PartialEq)]
pub struct Bars {
    pub vendor: u16,
    pub device: u16,
    /// Largest window, bytes.
    pub largest: u64,
    /// Some window sits above 4 GiB (Above 4G decoding on).
    pub above_4g: bool,
}

impl Bars {
    /// Resizable BAR / Smart Access Memory: a window larger than the classic 256 MiB.
    pub fn rebar(&self) -> bool {
        self.largest > 256 << 20
    }
}

/// From (base, size) ranges.
pub fn bars_of(vendor: u16, device: u16, ranges: &[(u64, u64)]) -> Bars {
    Bars {
        vendor,
        device,
        largest: ranges.iter().map(|r| r.1).max().unwrap_or(0),
        above_4g: ranges.iter().any(|r| r.0 >= 1 << 32),
    }
}

/// Every display adapter's BARs: Windows from the resources it was given
/// (no admin), Linux from sysfs `resource`; read once.
pub fn bars() -> &'static [Bars] {
    static V: std::sync::OnceLock<Vec<Bars>> = std::sync::OnceLock::new();
    V.get_or_init(imp::bars)
}

pub fn bluetooth_batteries() -> Vec<Peripheral> {
    imp::batteries()
}

pub fn thunderbolt() -> Vec<Thunderbolt> {
    imp::thunderbolt()
}

/// "24-bit, 48 kHz, 2 ch" from a WAVEFORMATEX(TENSIBLE).
pub fn wave_format(w: &[u8]) -> Option<String> {
    let u16_at = |o: usize| w.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |o: usize| w.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let (tag, ch, rate, bits) = (u16_at(0)?, u16_at(2)?, u32_at(4)?, u16_at(14)?);
    let (bits, float) = if tag == 0xFFFE {
        (u16_at(18).filter(|&b| b > 0).unwrap_or(bits), u32_at(24) == Some(3))
    } else {
        (bits, tag == 3)
    };
    let khz = if rate % 1000 == 0 { format!("{}", rate / 1000) } else { format!("{:.1}", rate as f64 / 1000.0) };
    Some(format!("{bits}-bit{}, {khz} kHz, {ch} ch", if float { " float" } else { "" }))
}

#[cfg(windows)]
mod imp {
    use super::{Audio, Peripheral, Thunderbolt, wave_format};
    use crate::os::win::{reg_raw, reg_str, reg_subkeys, reg_u64};
    use crate::storage::win::{
        DRP_COMPATIBLEIDS, DRP_DEVICEDESC, DRP_FRIENDLYNAME, Guid, PropKey, id_list, locate, prop_bytes,
        reg_str as dev_str, reg_strs,
    };

    /// DEVPKEY_Bluetooth_Battery: the level Windows' Bluetooth settings show.
    const BATTERY: PropKey =
        PropKey(Guid(0x104E_A319, 0x6EE2, 0x4701, [0xBD, 0x47, 0x8D, 0xDB, 0xF4, 0x25, 0xBB, 0xE5]), 2);

    const ROOT: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio";
    const NAME: &str = "{a45c254e-df1c-4efd-8020-67d146a850e0},2";
    const DEVICE: &str = "{b3f8fa53-0004-438e-9003-51a46e139bfc},6";
    const FORMAT: &str = "{f19f064d-082c-4e27-bc73-6882a1bb8e4c},0";

    pub fn audio() -> Vec<Audio> {
        let mut v = Vec::new();
        for (dir, output) in [("Render", true), ("Capture", false)] {
            for id in reg_subkeys(&format!(r"{ROOT}\{dir}")) {
                let key = format!(r"{ROOT}\{dir}\{id}");
                // DEVICE_STATE_ACTIVE = 1 (unplugged / disabled ones are left out).
                if reg_u64(&key, "DeviceState").is_none_or(|s| s & 0xF != 1) {
                    continue;
                }
                let props = format!(r"{key}\Properties");
                let Some(name) = reg_str(&props, NAME) else { continue };
                // The value is a serialized PROPVARIANT: 8 bytes of header, then the blob.
                let format = reg_raw(&props, FORMAT).and_then(|(_, b)| wave_format(b.get(8..)?));
                v.push(Audio { name, device: reg_str(&props, DEVICE).unwrap_or_default(), output, format });
            }
        }
        v
    }

    pub fn batteries() -> Vec<Peripheral> {
        let mut v: Vec<Peripheral> = Vec::new();
        for id in id_list("BTHENUM").into_iter().chain(id_list("BTHLE")) {
            let Some(dn) = locate(&id) else { continue };
            let Some(&battery) = prop_bytes(dn, &BATTERY).as_deref().and_then(|b| b.first()) else { continue };
            let name = dev_str(dn, DRP_FRIENDLYNAME).or_else(|| dev_str(dn, DRP_DEVICEDESC)).unwrap_or_default();
            if !v.iter().any(|p| p.name == name) {
                v.push(Peripheral { name, battery });
            }
        }
        v
    }

    pub fn bars() -> Vec<super::Bars> {
        use crate::storage::win::memory_ranges;
        id_list("PCI")
            .into_iter()
            .filter_map(|id| {
                let dn = locate(&id)?;
                reg_strs(dn, DRP_COMPATIBLEIDS)
                    .iter()
                    .any(|c| c.to_ascii_uppercase().contains("CC_03"))
                    .then_some(())?;
                let (v, d) = (crate::usb::id_field(&id, "VEN_")?, crate::usb::id_field(&id, "DEV_")?);
                Some(super::bars_of(v as u16, d as u16, &memory_ranges(dn)))
            })
            .collect()
    }

    /// USB4 host routers (PCI class 0C0340) and Thunderbolt controllers.
    pub fn thunderbolt() -> Vec<Thunderbolt> {
        id_list("PCI")
            .into_iter()
            .filter_map(|id| {
                let dn = locate(&id)?;
                let name = dev_str(dn, DRP_DEVICEDESC).unwrap_or_default();
                let usb4 = reg_strs(dn, DRP_COMPATIBLEIDS).iter().any(|c| c.to_ascii_uppercase().contains("CC_0C0340"));
                let tb = name.to_ascii_lowercase().contains("thunderbolt");
                (usb4 || tb).then(|| Thunderbolt {
                    name,
                    generation: Some(if usb4 { "USB4" } else { "Thunderbolt" }.into()),
                    link: None,
                })
            })
            .collect()
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{Audio, Peripheral, Thunderbolt};
    use crate::os::read_trim;

    pub fn audio() -> Vec<Audio> {
        let cards = std::fs::read_to_string("/proc/asound/cards").unwrap_or_default();
        let mut v = Vec::new();
        for line in cards.lines() {
            // " 0 [PCH            ]: HDA-Intel - HDA Intel PCH"
            let Some((num, rest)) = line.trim_start().split_once(' ') else { continue };
            let Ok(n) = num.parse::<u32>() else { continue };
            let name = rest.split_once(" - ").map_or(rest.trim(), |x| x.1.trim()).to_string();
            for (dir, output) in [("p", true), ("c", false)] {
                let Ok(rd) = std::fs::read_dir(format!("/proc/asound/card{n}")) else { continue };
                let has = rd.flatten().any(|e| {
                    let f = e.file_name().to_string_lossy().into_owned();
                    f.starts_with("pcm") && f.ends_with(dir)
                });
                if !has {
                    continue;
                }
                let params = read_trim(format!("/proc/asound/card{n}/pcm0{dir}/sub0/hw_params")).unwrap_or_default();
                let field = |k: &str| params.lines().find_map(|l| l.strip_prefix(k).map(|x| x.trim().to_string()));
                let format = match (field("format:"), field("rate:"), field("channels:")) {
                    (Some(f), Some(r), Some(c)) => {
                        Some(format!("{f}, {}, {c} ch", r.split_whitespace().next().unwrap_or(&r)))
                    }
                    _ => None,
                };
                v.push(Audio { name: name.clone(), device: String::new(), output, format });
            }
        }
        v
    }

    pub fn batteries() -> Vec<Peripheral> {
        let Ok(rd) = std::fs::read_dir("/sys/class/power_supply") else { return Vec::new() };
        rd.flatten()
            .filter_map(|e| {
                let p = e.path();
                (read_trim(p.join("scope")).as_deref() == Some("Device")).then_some(())?;
                let battery = read_trim(p.join("capacity"))?.parse().ok()?;
                let name =
                    read_trim(p.join("model_name")).unwrap_or_else(|| e.file_name().to_string_lossy().into_owned());
                Some(Peripheral { name, battery })
            })
            .collect()
    }

    pub fn bars() -> Vec<super::Bars> {
        let Ok(rd) = std::fs::read_dir("/sys/bus/pci/devices") else { return Vec::new() };
        let hex = |s: &str| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok();
        rd.flatten()
            .filter_map(|e| {
                let p = e.path();
                read_trim(p.join("class")).filter(|c| c.starts_with("0x03"))?;
                let (v, d) = (hex(&read_trim(p.join("vendor"))?)?, hex(&read_trim(p.join("device"))?)?);
                let res = std::fs::read_to_string(p.join("resource")).ok()?;
                // "start end flags" per BAR; IORESOURCE_MEM = 0x200.
                let ranges: Vec<(u64, u64)> = res
                    .lines()
                    .filter_map(|l| {
                        let f: Vec<u64> = l.split_whitespace().filter_map(hex).collect();
                        (f.len() == 3 && f[2] & 0x200 != 0 && f[1] > f[0]).then(|| (f[0], f[1] - f[0] + 1))
                    })
                    .collect();
                Some(super::bars_of(v as u16, d as u16, &ranges))
            })
            .collect()
    }

    pub fn thunderbolt() -> Vec<Thunderbolt> {
        let Ok(rd) = std::fs::read_dir("/sys/bus/thunderbolt/devices") else { return Vec::new() };
        let mut v: Vec<Thunderbolt> = rd
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                let name = read_trim(p.join("device_name"))?;
                let vendor = read_trim(p.join("vendor_name")).unwrap_or_default();
                let generation = read_trim(p.join("generation")).map(|g| match g.as_str() {
                    "4" => "USB4".to_string(),
                    g => format!("Thunderbolt {g}"),
                });
                let link = match (read_trim(p.join("rx_speed")), read_trim(p.join("rx_lanes"))) {
                    (Some(s), Some(l)) => Some(format!("{s} × {l}")),
                    (Some(s), None) => Some(s),
                    _ => None,
                };
                Some(Thunderbolt { name: format!("{vendor} {name}").trim().to_string(), generation, link })
            })
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{Audio, Peripheral, Thunderbolt};

    pub fn audio() -> Vec<Audio> {
        let out = crate::os::cmd("system_profiler", &["SPAudioDataType", "-json"]).unwrap_or_default();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap_or_default();
        let mut list = Vec::new();
        for group in v["SPAudioDataType"].as_array().into_iter().flatten() {
            for d in group["_items"].as_array().into_iter().flatten() {
                let name = d["_name"].as_str().unwrap_or_default().to_string();
                let rate = d["coreaudio_device_srate"].as_f64();
                let format = rate.map(|r| format!("{} kHz", r / 1000.0));
                let output = d.get("coreaudio_device_output").is_some();
                if d.get("coreaudio_device_input").is_some() {
                    list.push(Audio {
                        name: name.clone(),
                        device: String::new(),
                        output: false,
                        format: format.clone(),
                    });
                }
                if output || d.get("coreaudio_device_input").is_none() {
                    list.push(Audio { name, device: String::new(), output: true, format });
                }
            }
        }
        list
    }

    pub fn batteries() -> Vec<Peripheral> {
        let out = crate::os::cmd("ioreg", &["-r", "-l", "-k", "BatteryPercent"]).unwrap_or_default();
        out.split("+-o ")
            .filter_map(|node| {
                let get = |k: &str| {
                    node.lines().find_map(|l| {
                        l.trim().strip_prefix(&format!("\"{k}\" = ")).map(|v| v.trim_matches('"').to_string())
                    })
                };
                Some(Peripheral { name: get("Product")?, battery: get("BatteryPercent")?.parse().ok()? })
            })
            .collect()
    }

    pub fn thunderbolt() -> Vec<Thunderbolt> {
        Vec::new()
    }

    pub fn bars() -> Vec<super::Bars> {
        Vec::new()
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn bars() -> Vec<super::Bars> {
        Vec::new()
    }
    pub fn audio() -> Vec<super::Audio> {
        Vec::new()
    }
    pub fn batteries() -> Vec<super::Peripheral> {
        Vec::new()
    }
    pub fn thunderbolt() -> Vec<super::Thunderbolt> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn wave_formats() {
        // As read from this machine's registry: extensible, 2 ch, 48 kHz, 24 valid bits, PCM.
        let w = [
            0xFE, 0xFF, 0x02, 0x00, 0x80, 0xBB, 0x00, 0x00, 0x00, 0xDC, 0x05, 0x00, 0x08, 0x00, 0x20, 0x00, 0x16, 0x00,
            0x18, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        ];
        assert_eq!(super::wave_format(&w).as_deref(), Some("24-bit, 48 kHz, 2 ch"));
        let mut f = w;
        f[24] = 3;
        assert_eq!(super::wave_format(&f).as_deref(), Some("24-bit float, 48 kHz, 2 ch"));
        assert_eq!(super::wave_format(&[1, 0]), None);
        // RTX 4060 here: 16 MiB, 32 MiB and an 8 GiB window above 4 GiB = ReBAR on.
        let b = super::bars_of(
            0x10DE,
            0x2882,
            &[(0xDD00_0000, 16 << 20), (0xFA_0000_0000, 32 << 20), (0xF8_0000_0000, 8 << 30)],
        );
        assert!(b.rebar() && b.above_4g);
        assert!(!super::bars_of(1, 2, &[(0xE000_0000, 256 << 20)]).rebar(), "the classic 256 MiB window");
        let _ = (super::audio(), super::bluetooth_batteries(), super::thunderbolt());
    }
}

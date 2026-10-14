//! Linux: DRM sysfs (PCI IDs, link, amdgpu/i915/xe clocks and VRAM), NVML,
//! connector EDIDs. Documented in the kernel's sysfs ABI (Documentation/ABI).

use super::{Adapter, Link, Live, Perf, Probe, edid};
use crate::hw::Bdf;
use crate::os::{read_trim, read_u64};
use crate::report::f;
use std::path::{Path, PathBuf};

fn cards() -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir("/sys/class/drm") else { return Vec::new() };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            n.strip_prefix("card").is_some_and(|r| !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    v.sort();
    v
}

fn parse_bdf(s: &str) -> Option<Bdf> {
    // "0000:01:00.0"
    let (seg, rest) = s.split_once(':')?;
    let (bus, rest) = rest.split_once(':')?;
    let (dev, func) = rest.split_once('.')?;
    Some(Bdf {
        seg: u16::from_str_radix(seg, 16).ok()?,
        bus: u8::from_str_radix(bus, 16).ok()?,
        dev: u8::from_str_radix(dev, 16).ok()?,
        func: func.parse().ok()?,
    })
}

fn name_of(p: &Path) -> Option<String> {
    p.file_name()?.to_str().map(str::to_string)
}

fn link(dev: &Path, kind: &str) -> Link {
    let speed = read_trim(dev.join(format!("{kind}_link_speed")))
        .and_then(|s| s.split_whitespace().next().and_then(|x| x.parse().ok()));
    let width = read_trim(dev.join(format!("{kind}_link_width"))).and_then(|s| s.parse().ok()).filter(|&w| w > 0);
    Link { width, speed }
}

/// amdgpu `pp_dpm_*`: "0: 500Mhz\n1: 2100Mhz *" → (lowest, highest, current)
fn dpm(p: &Path) -> Option<(f64, f64, Option<f64>)> {
    let s = std::fs::read_to_string(p).ok()?;
    let mut v = Vec::new();
    let mut cur = None;
    for l in s.lines() {
        let Some(mhz) =
            l.split_whitespace().nth(1).and_then(|m| m.to_ascii_lowercase().trim_end_matches("mhz").parse().ok())
        else {
            continue;
        };
        if l.trim_end().ends_with('*') {
            cur = Some(mhz);
        }
        v.push(mhz);
    }
    let min = v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = v.iter().copied().fold(0.0, f64::max);
    (!v.is_empty()).then_some((min, max, cur))
}

/// Marketing name from the system's pci.ids database, when installed.
fn pci_name(ven: u16, dev: u16) -> Option<String> {
    let db = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids", "/usr/share/pci.ids"]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())?;
    let (vh, dh) = (format!("{ven:04x}  "), format!("\t{dev:04x}  "));
    let mut in_vendor = false;
    for l in db.lines() {
        if !l.starts_with('\t') && !l.starts_with('#') {
            if in_vendor {
                return None;
            }
            in_vendor = l.starts_with(&vh);
        } else if in_vendor && l.starts_with(&dh) {
            let n = &l[dh.len()..];
            // "AD107 [GeForce RTX 4060]" → "GeForce RTX 4060"
            let n = n.find('[').and_then(|i| n[i + 1..].split(']').next()).unwrap_or(n);
            return Some(format!("{} {n}", super::db::vendor_name(ven)));
        }
    }
    None
}

fn freq(p: &Path, names: &[&str]) -> Option<f64> {
    names.iter().find_map(|n| read_u64(p.join(n))).filter(|&v| v > 0).map(|v| v as f64)
}

fn adapter(card: &Path) -> Option<Adapter> {
    let dev = std::fs::canonicalize(card.join("device")).ok()?;
    let id = |n: &str| read_u64(dev.join(n)).map(|v| v as u32);
    let (ven, did) = (id("vendor")? as u16, id("device")? as u16);
    let driver = std::fs::read_link(dev.join("driver")).ok().and_then(|p| name_of(&p));
    let mut a = Adapter {
        vendor_id: ven,
        device_id: did,
        subsys: (id("subsystem_vendor").unwrap_or(0) as u16, id("subsystem_device").unwrap_or(0) as u16),
        rev: id("revision").unwrap_or(0) as u8,
        pci: name_of(&dev).and_then(|n| parse_bdf(&n)),
        root: dev.parent().and_then(name_of).and_then(|n| parse_bdf(&n)),
        link: link(&dev, "current"),
        link_max: link(&dev, "max"),
        primary: read_trim(dev.join("boot_vga")).as_deref() == Some("1"),
        ..Default::default()
    };
    a.name = read_trim(dev.join("product_name"))
        .or_else(|| pci_name(ven, did))
        .unwrap_or_else(|| format!("{} GPU 0x{did:04X}", super::db::vendor_name(ven)));
    a.mem_size_mb = read_u64(dev.join("mem_info_vram_total")).map(|b| b >> 20).filter(|&m| m > 0);
    a.mem_vendor = read_trim(dev.join("mem_info_vram_vendor")).map(|v| {
        let mut c = v.chars();
        c.next().map_or(v.clone(), |f| f.to_uppercase().chain(c).collect())
    });
    match driver.as_deref() {
        Some("amdgpu") => {
            let (s, m) = (dpm(&dev.join("pp_dpm_sclk")), dpm(&dev.join("pp_dpm_mclk")));
            if s.is_some() || m.is_some() {
                a.perf.push(Perf { level: "Base", core: s.map(|x| x.0), shader: None, mem: m.map(|x| x.0) });
                a.perf.push(Perf { level: "Boost", core: s.map(|x| x.1), shader: None, mem: m.map(|x| x.1) });
                a.perf.push(Perf {
                    level: "Current",
                    core: s.and_then(|x| x.2),
                    shader: None,
                    mem: m.and_then(|x| x.2),
                });
            }
        }
        Some("i915") => {
            let level = |l, n: &[&str]| Perf { level: l, core: freq(card, n), shader: None, mem: None };
            a.perf = vec![
                level("Base", &["gt_RP1_freq_mhz"]),
                level("Boost", &["gt_RP0_freq_mhz"]),
                level("Current", &["gt_act_freq_mhz", "gt_cur_freq_mhz"]),
            ];
        }
        Some("xe") => {
            let fq = dev.join("tile0/gt0/freq0");
            let level = |l, n: &[&str]| Perf { level: l, core: freq(&fq, n), shader: None, mem: None };
            a.perf = vec![
                level("Base", &["rpe_freq"]),
                level("Boost", &["rp0_freq"]),
                level("Current", &["act_freq", "cur_freq"]),
            ];
        }
        _ => {}
    }
    a.perf.retain(|p| p.core.is_some() || p.mem.is_some());
    if let Some(d) = &driver {
        a.driver_version = read_trim(format!("/sys/module/{d}/version"))
            .or_else(|| read_trim("/proc/sys/kernel/osrelease").map(|k| format!("kernel {k}")));
    }
    a.driver_model = driver;
    Some(a)
}

pub fn probe(use_api: bool) -> Probe {
    let mut p = Probe::default();
    p.adapters = cards().iter().filter_map(|c| adapter(c)).collect();
    if !p.adapters.is_empty() {
        p.apis.push(f("API", "DRM sysfs"));
    }
    if use_api {
        super::apply_nvml(&mut p.adapters, &mut p.apis);
        if let Some(v) = super::nvml::get().and_then(|n| n.driver.clone()) {
            for a in p.adapters.iter_mut().filter(|a| a.vendor_id == 0x10DE) {
                a.driver_version.get_or_insert(v.clone());
            }
        }
    }
    // Connected connectors' EDIDs: /sys/class/drm/card0-DP-1/edid
    if let Ok(rd) = std::fs::read_dir("/sys/class/drm") {
        let mut conns: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|c| c.join("edid").exists()).collect();
        conns.sort();
        for c in conns {
            if read_trim(c.join("status")).as_deref() != Some("connected") {
                continue;
            }
            if let Some(m) = std::fs::read(c.join("edid")).ok().and_then(|e| edid::parse(&e))
                && !p.monitors.iter().any(|x| x.raw == m.raw)
            {
                p.monitors.push(m);
            }
        }
    }
    p
}

pub fn live(ads: &[Adapter], use_api: bool) -> Vec<Live> {
    let mut out = Vec::new();
    if use_api {
        super::nvml_live(ads, &mut out);
    }
    for c in cards() {
        let Ok(dev) = std::fs::canonicalize(c.join("device")) else { continue };
        let bdf = name_of(&dev).and_then(|n| parse_bdf(&n));
        let Some(i) = ads.iter().position(|a| a.pci.is_some() && a.pci == bdf) else { continue };
        if out.iter().any(|r| r.adapter == i) {
            continue;
        }
        let hw =
            std::fs::read_dir(dev.join("hwmon")).ok().and_then(|mut r| r.next()).and_then(|e| e.ok()).map(|e| e.path());
        let mut l = hw.as_deref().map(hwmon_live).unwrap_or_default();
        l.adapter = i;
        l.load_pct = read_u64(dev.join("gpu_busy_percent")).map(|v| v as f64);
        l.mem_load_pct = read_u64(dev.join("mem_busy_percent")).map(|v| v as f64);
        l.core_mhz = dpm(&dev.join("pp_dpm_sclk"))
            .and_then(|x| x.2)
            .or_else(|| freq(&c, &["gt_act_freq_mhz"]))
            .or_else(|| freq(&dev.join("tile0/gt0/freq0"), &["act_freq"]));
        l.mem_mhz = dpm(&dev.join("pp_dpm_mclk")).and_then(|x| x.2);
        out.push(l);
    }
    out
}

/// GPU hwmon (amdgpu: temp1/2/3 = edge/junction/mem, fan1, pwm1, power1).
/// Labels win over channel numbers when the driver provides them.
fn hwmon_live(h: &Path) -> Live {
    let v = |n: &str| read_u64(h.join(n));
    let temp = |label: &str, ch: u32| {
        let by_label = (1..=8).find(|i| read_trim(h.join(format!("temp{i}_label"))).as_deref() == Some(label));
        let any_label = h.join(format!("temp{ch}_label")).exists();
        let ch = by_label.or((!any_label).then_some(ch))?;
        v(&format!("temp{ch}_input")).map(|m| m as f64 / 1000.0)
    };
    Live {
        temp_c: temp("edge", 1),
        hotspot_c: temp("junction", 2),
        mem_junction_c: temp("mem", 3),
        fan_rpm: v("fan1_input").filter(|&r| r > 0).map(|r| r as f64),
        fan_pct: v("pwm1").map(|p| (p as f64 * 100.0 / 255.0).round()),
        power_w: v("power1_average").or_else(|| v("power1_input")).map(|u| u as f64 / 1e6),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bdf_and_dpm() {
        let b = super::parse_bdf("0000:74:00.0").unwrap();
        assert_eq!((b.bus, b.dev, b.func), (0x74, 0, 0));
        let p = std::env::temp_dir().join("cpuzeez_pp_dpm_test");
        std::fs::write(&p, "0: 500Mhz\n1: 1200Mhz *\n2: 2400Mhz\n").unwrap();
        assert_eq!(super::dpm(&p), Some((500.0, 2400.0, Some(1200.0))));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn amdgpu_hwmon() {
        let h = std::env::temp_dir().join(format!("cpuzeez-amdgpu-{}", std::process::id()));
        std::fs::create_dir_all(&h).unwrap();
        for (n, v) in [
            ("temp1_input", "45000"),
            ("temp1_label", "edge"),
            ("temp2_input", "61000"),
            ("temp2_label", "junction"),
            ("temp3_input", "58000"),
            ("temp3_label", "mem"),
            ("fan1_input", "1200"),
            ("pwm1", "255"),
            ("power1_average", "150000000"),
        ] {
            std::fs::write(h.join(n), v).unwrap();
        }
        let l = super::hwmon_live(&h);
        assert_eq!((l.temp_c, l.hotspot_c, l.mem_junction_c), (Some(45.0), Some(61.0), Some(58.0)));
        assert_eq!((l.fan_rpm, l.fan_pct, l.power_w), (Some(1200.0), Some(100.0), Some(150.0)));
        let _ = std::fs::remove_dir_all(h);
    }
}

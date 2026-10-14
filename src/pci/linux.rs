//! Linux: /sys/bus/pci/devices (config file is 64 bytes unprivileged, 4 KB as root).
use super::{Device, Link};
use crate::hw::Bdf;
use crate::os::{read_trim, read_u64};
use std::path::Path;

pub fn parse_bdf(s: &str) -> Option<Bdf> {
    // "0000:00:01.1"
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

/// "16.0 GT/s PCIe" -> 4.
pub fn speed_code(s: &str) -> Option<u8> {
    let gt: f32 = s.split_whitespace().next()?.parse().ok()?;
    Some(match gt {
        x if x < 3.0 => 1,
        x if x < 6.0 => 2,
        x if x < 9.0 => 3,
        x if x < 17.0 => 4,
        x if x < 33.0 => 5,
        _ => 6,
    })
}

pub fn list() -> Vec<Device> {
    let Ok(rd) = std::fs::read_dir("/sys/bus/pci/devices") else { return Vec::new() };
    rd.flatten().filter_map(|e| one(&e.path(), &e.file_name().to_string_lossy())).collect()
}

fn one(p: &Path, name: &str) -> Option<Device> {
    let mut d = Device::new(parse_bdf(name)?);
    let num = |f: &str| read_u64(p.join(f));
    d.vendor = num("vendor")? as u16;
    d.device = num("device").unwrap_or(0) as u16;
    d.subvendor = num("subsystem_vendor").unwrap_or(0) as u16;
    d.subdevice = num("subsystem_device").unwrap_or(0) as u16;
    d.class = num("class").unwrap_or(0) as u32;
    d.config = std::fs::read(p.join("config")).unwrap_or_default();
    d.revision = num("revision").map(|r| r as u8).or_else(|| d.config.get(8).copied()).unwrap_or(0);
    let link = |s: &str, w: &str| Some((speed_code(&read_trim(p.join(s))?)?, read_u64(p.join(w))? as u8));
    if let (Some((s, w)), Some((ms, mw))) =
        (link("current_link_speed", "current_link_width"), link("max_link_speed", "max_link_width"))
    {
        d.link = Some(Link { speed: s, width: w, max_speed: ms, max_width: mw });
    }
    // Symlink target is .../pci0000:00/0000:00:01.1/0000:01:00.0: the parent is the previous component.
    d.parent = std::fs::canonicalize(p)
        .ok()
        .and_then(|c| c.parent().and_then(|x| x.file_name()).and_then(|n| parse_bdf(&n.to_string_lossy())));
    Some(d)
}

/// SMBus host controller I/O bases from /proc/ioports (addresses need root).
pub fn smbus_bases() -> Vec<u64> {
    let t = std::fs::read_to_string("/proc/ioports").unwrap_or_default();
    let mut v: Vec<u64> = t
        .lines()
        .filter(|l| l.to_ascii_lowercase().contains("smbus"))
        .filter_map(|l| u64::from_str_radix(l.trim().split('-').next()?, 16).ok())
        .filter(|&b| b != 0)
        .collect();
    v.dedup();
    v
}

/// "e0000000-efffffff : PCI ECAM 0000 [bus 00-ff]" / "PCI MMCONFIG" (root only).
pub fn iomem_ecam() -> Option<u64> {
    let t = std::fs::read_to_string("/proc/iomem").ok()?;
    t.lines()
        .filter(|l| l.contains("MMCONFIG") || l.contains("PCI ECAM"))
        .find_map(|l| u64::from_str_radix(l.trim().split('-').next()?, 16).ok())
        .filter(|&b| b != 0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse() {
        let b = super::parse_bdf("0000:74:00.6").unwrap();
        assert_eq!((b.seg, b.bus, b.dev, b.func), (0, 0x74, 0, 6));
        assert_eq!(super::speed_code("16.0 GT/s PCIe"), Some(4));
        assert_eq!(super::speed_code("2.5 GT/s PCIe"), Some(1));
        assert_eq!(super::speed_code("Unknown"), None);
    }
}

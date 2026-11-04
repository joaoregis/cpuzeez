//! PCI Devices and Register Spaces sections (FEATURES.md 3.13, 3.22).
//!
//! Enumeration comes from the OS (Linux sysfs, Windows SetupAPI, macOS IOKit);
//! raw config space from the OS when it hands it out (sysfs) or from the
//! privileged backend (`Hw::pci_read`). The decoder in `decode` is pure.

pub use cpuzeez_core::pci::*;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(any(target_os = "macos", test))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod macos;
#[cfg(windows)]
mod windows;

use crate::Ctx;
use crate::report::{Section, f, g, section};
use std::sync::OnceLock;

/// All PCI functions, sorted by location. Empty when `PCI=0` in the ini.
pub fn devices(ctx: &Ctx) -> &[Device] {
    ctx.pci.get_or_init(|| if ctx.cfg.pci { enumerate(ctx) } else { Vec::new() })
}

fn enumerate(ctx: &Ctx) -> Vec<Device> {
    #[cfg(target_os = "linux")]
    let mut v = linux::list();
    #[cfg(windows)]
    let mut v = windows::list();
    #[cfg(target_os = "macos")]
    let mut v = macos::list();
    #[cfg(not(any(target_os = "linux", windows, target_os = "macos")))]
    let mut v: Vec<Device> = Vec::new();

    v.retain(|d| (d.bdf.bus as u16) < ctx.cfg.max_pci_bus);
    for d in &mut v {
        if d.config.len() < 4096 {
            // Privileged backend: whole extended space, else the legacy 256 bytes.
            for n in [4096usize, 256] {
                if n <= d.config.len() {
                    break;
                }
                let mut b = vec![0u8; n];
                if ctx.hw.pci_read(d.bdf, 0, &mut b) && decode::u16_at(&b, 0) == Some(d.vendor) {
                    d.config = b;
                    break;
                }
            }
        }
        if d.vendor == 0 {
            d.ids_from_config();
        }
        if d.name.is_none() {
            d.name = ids_name(d.vendor, d.device);
        }
    }
    // Parents from bridge bus numbers when the OS didn't tell us.
    let bridges: Vec<(Bdf, u8, u8)> = v
        .iter()
        .filter(|d| d.config.len() >= 0x1B && d.config[0x0E] & 0x7F == 1)
        .map(|d| (d.bdf, d.config[0x19], d.config[0x1A]))
        .collect();
    for d in &mut v {
        if d.parent.is_none() {
            d.parent = bridges.iter().find(|b| b.0.seg == d.bdf.seg && b.1 == d.bdf.bus).map(|b| b.0);
        }
    }
    v.sort_by_key(|d| d.bdf);
    v.dedup_by_key(|d| d.bdf);
    v
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let devs = devices(ctx);
    if devs.is_empty() {
        return Vec::new();
    }
    let mut n = Vec::new();
    if let Some(base) = ecam_base() {
        n.push(f("Register space", format!("PCI Express, base address = 0x{base:09X}")));
    }
    n.extend(devs.iter().map(|d| g("", decode::device_nodes(d))));
    vec![section("PCI Devices", n)]
}

/// "Register Spaces": the config mechanisms we know are in use, plus the
/// SMBus I/O base when the OS lists it.
pub fn register_spaces(ctx: &Ctx) -> Vec<Section> {
    if devices(ctx).is_empty() {
        return Vec::new();
    }
    let mut n = vec![f("Register space", "PCI")];
    if let Some(base) = ecam_base() {
        n.push(f("Register space", format!("PCI Express, base address = 0x{base:09X}")));
    }
    #[cfg(target_os = "linux")]
    for base in linux::smbus_bases() {
        n.push(f("Register space", format!("SMBus, base address = 0x{base:04X}")));
    }
    vec![section("Register Spaces", n)]
}

/// PCIe ECAM base of segment 0 (ACPI MCFG; Linux falls back to /proc/iomem).
pub fn ecam_base() -> Option<u64> {
    static B: OnceLock<Option<u64>> = OnceLock::new();
    *B.get_or_init(|| {
        #[cfg(windows)]
        let t = windows::acpi_table(*b"MCFG");
        #[cfg(target_os = "linux")]
        let t = std::fs::read("/sys/firmware/acpi/tables/MCFG").ok();
        #[cfg(not(any(windows, target_os = "linux")))]
        let t: Option<Vec<u8>> = None;
        let b = t.and_then(|t| mcfg_base(&t));
        #[cfg(target_os = "linux")]
        let b = b.or_else(linux::iomem_ecam);
        b
    })
}

// ---- pci.ids (read from the system at runtime; never bundled) ----

fn ids() -> Option<&'static Ids> {
    static I: OnceLock<Option<Ids>> = OnceLock::new();
    I.get_or_init(|| {
        let paths = [
            "/usr/share/hwdata/pci.ids",
            "/usr/share/misc/pci.ids",
            "/usr/share/pci.ids",
            "/usr/local/share/pci.ids",
            "/opt/homebrew/share/pciids/pci.ids",
            "/usr/local/share/pciids/pci.ids",
        ];
        let text = paths.iter().find_map(|p| std::fs::read(p).ok())?;
        Some(parse_ids(&String::from_utf8_lossy(&text)))
    })
    .as_ref()
}

/// "Vendor Device" from the system's pci.ids.
pub fn ids_name(vendor: u16, device: u16) -> Option<String> {
    ids()?.name(vendor, device)
}

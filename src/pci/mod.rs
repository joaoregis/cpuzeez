//! PCI Devices and Register Spaces sections (FEATURES.md 3.13, 3.22).
//!
//! Enumeration comes from the OS (Linux sysfs, Windows SetupAPI, macOS IOKit);
//! raw config space from the OS when it hands it out (sysfs) or from the
//! privileged backend (`Hw::pci_read`). The decoder in `decode` is pure.

pub mod decode;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(any(target_os = "macos", test))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod macos;
#[cfg(windows)]
mod windows;

use crate::Ctx;
use crate::hw::Bdf;
use crate::report::{Section, f, g, section};
use std::collections::HashMap;
use std::sync::OnceLock;

/// PCIe link, speeds in Link Status encoding (1 = 2.5 GT/s ... 6 = 64 GT/s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub speed: u8,
    pub width: u8,
    pub max_speed: u8,
    pub max_width: u8,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub bdf: Bdf,
    pub vendor: u16,
    pub device: u16,
    pub subvendor: u16,
    pub subdevice: u16,
    pub revision: u8,
    pub class: u32, // base<<16 | sub<<8 | prog-if
    /// Config space bytes we could read (0, 64, 256 or 4096).
    pub config: Vec<u8>,
    /// OS device name, else pci.ids "vendor device".
    pub name: Option<String>,
    /// Link as reported by the OS (used when config space is not readable).
    pub link: Option<Link>,
    /// PCIe device/port type (PCIe capability encoding), OS-reported.
    pub port_type: Option<u8>,
    /// Upstream bridge.
    pub parent: Option<Bdf>,
}

impl Device {
    pub(crate) fn new(bdf: Bdf) -> Device {
        Device {
            bdf,
            vendor: 0,
            device: 0,
            subvendor: 0,
            subdevice: 0,
            revision: 0,
            class: 0,
            config: Vec::new(),
            name: None,
            link: None,
            port_type: None,
            parent: None,
        }
    }

    /// Current/max link, from config space or else from the OS.
    pub fn pcie_link(&self) -> Option<Link> {
        decode::pcie_link(&self.config).or(self.link)
    }

    pub fn pcie_type(&self) -> Option<u8> {
        decode::pcie_port(&self.config).map(|p| p.0).or(self.port_type)
    }

    /// Slot implemented bit of a root/downstream port (None = unknown).
    pub fn has_slot(&self) -> Option<bool> {
        decode::pcie_port(&self.config).map(|p| p.1)
    }

    /// Fill IDs from config bytes (when the OS gave only those).
    fn ids_from_config(&mut self) {
        let c = &self.config;
        if c.len() < 0x30 {
            return;
        }
        self.vendor = decode::u16_at(c, 0).unwrap_or(0);
        self.device = decode::u16_at(c, 2).unwrap_or(0);
        self.revision = c[8];
        self.class = (c[0x0B] as u32) << 16 | (c[0x0A] as u32) << 8 | c[9] as u32;
        if c[0x0E] & 0x7F == 0 {
            self.subvendor = decode::u16_at(c, 0x2C).unwrap_or(0);
            self.subdevice = decode::u16_at(c, 0x2E).unwrap_or(0);
        }
    }
}

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

/// First allocation of an ACPI MCFG table (header 36 bytes + 8 reserved, then
/// 16-byte entries: base u64, segment u16, start bus, end bus, reserved).
pub fn mcfg_base(t: &[u8]) -> Option<u64> {
    let e = t.get(44..52)?;
    let b = u64::from_le_bytes(e.try_into().ok()?);
    (b != 0 && t.get(0..4) == Some(b"MCFG")).then_some(b)
}

/// "0x1022 (AMD)", or bare hex for vendors not in the short table.
pub fn vendor_label(v: u16) -> String {
    match vendor_short(v) {
        Some(n) => format!("0x{v:04X} ({n})"),
        None => format!("0x{v:04X}"),
    }
}

/// Short vendor names (public PCI-SIG vendor ID assignments).
pub fn vendor_short(v: u16) -> Option<&'static str> {
    Some(match v {
        0x1002 | 0x1022 => "AMD",
        0x8086 | 0x8087 => "Intel",
        0x10DE => "NVIDIA",
        0x1462 => "MSI",
        0x1043 => "ASUS",
        0x1458 => "Gigabyte",
        0x1849 => "ASRock",
        0x10EC => "Realtek",
        0x144D => "Samsung",
        0x1B21 => "ASMedia",
        0x2646 => "Kingston",
        0x15B7 => "Sandisk",
        0x1E0F => "KIOXIA",
        0x1C5C => "SK Hynix",
        0x1344 => "Micron",
        0x1987 => "Phison",
        0x126F => "Silicon Motion",
        0x1CC1 => "ADATA",
        0x1D97 => "Shenzhen Longsys",
        0x14E4 => "Broadcom",
        0x168C => "Qualcomm Atheros",
        0x1969 => "Qualcomm Atheros",
        0x11AB => "Marvell",
        0x1B4B => "Marvell",
        0x1912 => "Renesas",
        0x1106 => "VIA",
        0x1039 => "SiS",
        0x1028 => "Dell",
        0x103C => "HP",
        0x17AA => "Lenovo",
        0x1025 => "Acer",
        0x1179 => "Toshiba",
        0x104D => "Sony",
        0x106B => "Apple",
        0x1414 => "Microsoft",
        0x1AF4 => "Red Hat",
        0x15AD => "VMware",
        0x80EE => "Oracle",
        0x1AB8 => "Parallels",
        0x5853 => "XenSource",
        0x1D17 => "Zhaoxin",
        0x1D94 => "Hygon",
        0x1A03 => "ASPEED",
        0x102B => "Matrox",
        0x8384 => "SigmaTel",
        0x1102 => "Creative",
        0x13F6 => "C-Media",
        0x1274 => "Ensoniq",
        0x1000 => "Broadcom / LSI",
        0x9005 => "Adaptec",
        0x15B3 => "Mellanox",
        0x1077 => "QLogic",
        0x1217 => "O2 Micro",
        0x1180 => "Ricoh",
        0x104C => "Texas Instruments",
        0x1033 => "NEC",
        0x197B => "JMicron",
        0x1CC4 => "Union Memory",
        0x1E4B => "MAXIO",
        0x1BB1 => "Seagate",
        0x1C58 => "HGST",
        0x1E49 => "YMTC",
        0x8088 => "Wangxun",
        _ => return None,
    })
}

/// CPU-Z's "Description": the class code in words (PCI Code and ID Assignment spec).
pub fn class_desc(class: u32) -> String {
    let (b, s, p) = ((class >> 16) as u8, (class >> 8) as u8, class as u8);
    let t = match (b, s) {
        (0x00, 0x01) => "VGA Compatible Device",
        (0x00, _) => "Unclassified Device",
        (0x01, 0x00) => "SCSI Controller",
        (0x01, 0x01) => "IDE Controller",
        (0x01, 0x02) => "Floppy Controller",
        (0x01, 0x04) => "RAID Controller",
        (0x01, 0x05) => "ATA Controller",
        (0x01, 0x06) if p == 1 => "Serial ATA Controller - AHCI",
        (0x01, 0x06) => "Serial ATA Controller",
        (0x01, 0x07) => "SAS Controller",
        (0x01, 0x08) if p == 2 => "NVM Express (NVMe) Controller",
        (0x01, 0x08) => "Non-Volatile Memory Controller",
        (0x01, 0x09) => "UFS Controller",
        (0x01, _) => "Mass Storage Controller",
        (0x02, 0x00) => "Ethernet Controller",
        (0x02, 0x01) => "Token Ring Controller",
        (0x02, 0x02) => "FDDI Controller",
        (0x02, 0x03) => "ATM Controller",
        (0x02, 0x04) => "ISDN Controller",
        (0x02, 0x07) => "InfiniBand Controller",
        (0x02, _) => "Network Controller",
        (0x03, 0x00) if p == 1 => "8514 Compatible Controller",
        (0x03, 0x00) => "VGA Controller",
        (0x03, 0x01) => "XGA Controller",
        (0x03, 0x02) => "3D Controller",
        (0x03, _) => "Display Controller",
        (0x04, 0x00) => "Video Device",
        (0x04, 0x01) => "Audio Device",
        (0x04, 0x02) => "Telephony Device",
        (0x04, 0x03) => "High Definition Audio (HDA)",
        (0x04, _) => "Multimedia Device",
        (0x05, 0x00) => "RAM Controller",
        (0x05, 0x01) => "Flash Controller",
        (0x05, _) => "Memory Controller",
        (0x06, 0x00) => "Host Bridge",
        (0x06, 0x01) => "PCI to ISA Bridge",
        (0x06, 0x02) => "PCI to EISA Bridge",
        (0x06, 0x03) => "PCI to MCA Bridge",
        (0x06, 0x04) => "PCI to PCI Bridge",
        (0x06, 0x05) => "PCI to PCMCIA Bridge",
        (0x06, 0x06) => "PCI to NuBus Bridge",
        (0x06, 0x07) => "PCI to CardBus Bridge",
        (0x06, 0x08) => "RACEway Bridge",
        (0x06, 0x09) => "Semi-transparent PCI to PCI Bridge",
        (0x06, 0x0A) => "InfiniBand to PCI Bridge",
        (0x06, _) => "Bridge Device",
        (0x07, 0x00) => "Serial Controller",
        (0x07, 0x01) => "Parallel Port",
        (0x07, 0x03) => "Modem",
        (0x07, _) => "Communication Controller",
        (0x08, 0x00) => "Interrupt Controller",
        (0x08, 0x01) => "DMA Controller",
        (0x08, 0x02) => "System Timer",
        (0x08, 0x03) => "RTC Controller",
        (0x08, 0x04) => "PCI Hot-Plug Controller",
        (0x08, 0x05) => "SD Host Controller",
        (0x08, 0x06) => "IOMMU",
        (0x08, _) => "System Peripheral",
        (0x09, _) => "Input Device",
        (0x0A, _) => "Docking Station",
        (0x0B, _) => "Processor",
        (0x0C, 0x00) => "FireWire (IEEE 1394) Controller",
        (0x0C, 0x03) => match p {
            0x00 => "USB Controller (UHCI)",
            0x10 => "USB Controller (OHCI)",
            0x20 => "USB Controller (EHCI)",
            0x30 => "USB Controller (xHCI)",
            _ => "USB Controller",
        },
        (0x0C, 0x04) => "Fibre Channel Controller",
        (0x0C, 0x05) => "SMBus Controller",
        (0x0C, 0x06) => "InfiniBand Controller",
        (0x0C, 0x07) => "IPMI Interface",
        (0x0C, _) => "Serial Bus Controller",
        (0x0D, _) => "Wireless Controller",
        (0x0E, _) => "Intelligent I/O Controller",
        (0x0F, _) => "Satellite Communication Controller",
        (0x10, _) => "Encryption Device",
        (0x11, _) => "Signal Processing Controller",
        (0x12, _) => "Processing Accelerator",
        (0x13, _) => "Non-Essential Instrumentation",
        _ => "Unknown Device",
    };
    t.into()
}

// ---- pci.ids (read from the system at runtime; never bundled) ----

struct Ids {
    vendors: HashMap<u16, String>,
    devices: HashMap<(u16, u16), String>,
}

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

fn parse_ids(text: &str) -> Ids {
    let mut ids = Ids { vendors: HashMap::new(), devices: HashMap::new() };
    let mut cur: Option<u16> = None;
    for l in text.lines() {
        if l.starts_with('#') || l.is_empty() {
            continue;
        }
        if l.starts_with("C ") {
            break; // class list follows the vendor list
        }
        let hex = |s: &str| s.get(..4).and_then(|h| u16::from_str_radix(h, 16).ok());
        let name = |s: &str| s.get(4..).map(|n| n.trim().to_string());
        if !l.starts_with('\t') {
            cur = hex(l);
            if let (Some(v), Some(n)) = (cur, name(l)) {
                ids.vendors.insert(v, n);
            }
        } else if !l.starts_with("\t\t")
            && let (Some(v), Some(d), Some(n)) = (cur, hex(&l[1..]), name(&l[1..]))
        {
            ids.devices.insert((v, d), n);
        }
    }
    ids
}

/// "Vendor Device" from pci.ids.
pub fn ids_name(vendor: u16, device: u16) -> Option<String> {
    let i = ids()?;
    let d = i.devices.get(&(vendor, device))?;
    let v = vendor_short(vendor).map(str::to_string).or_else(|| i.vendors.get(&vendor).cloned());
    Some(match v {
        Some(v) => format!("{v} {d}"),
        None => d.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_parse_and_labels() {
        let i = parse_ids(
            "# c\n1022  Advanced Micro Devices, Inc. [AMD]\n\t790e  FCH LPC Bridge\n\t\t1462 7e47  sub\nC 00  x\n",
        );
        assert_eq!(i.devices.get(&(0x1022, 0x790E)).map(String::as_str), Some("FCH LPC Bridge"));
        assert_eq!(i.vendors.len(), 1);
        assert_eq!(vendor_label(0x1022), "0x1022 (AMD)");
        assert_eq!(vendor_label(0x17CB), "0x17CB");
        assert_eq!(class_desc(0x0C0330), "USB Controller (xHCI)");
        assert_eq!(class_desc(0x0C0340), "USB Controller");
        assert_eq!(class_desc(0x010802), "NVM Express (NVMe) Controller");
        assert_eq!(class_desc(0x108000), "Encryption Device");
    }

    #[test]
    fn mcfg() {
        let mut t = vec![0u8; 60];
        t[..4].copy_from_slice(b"MCFG");
        t[44..52].copy_from_slice(&0xE000_0000u64.to_le_bytes());
        assert_eq!(mcfg_base(&t), Some(0xE000_0000));
        assert_eq!(format!("0x{:09X}", 0xE000_0000u64), "0x0E0000000");
    }
}

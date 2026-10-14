//! USB Devices section (FEATURES.md 3.16).
//! Windows: CfgMgr32 "USB" enumerator + hub IOCTL for the device descriptor.
//! Linux: /sys/bus/usb/devices. macOS: `ioreg -p IOUSB`.
// Parsers stay OS-neutral so their unit tests run on every host.
#![allow(dead_code)]

use crate::Ctx;
use crate::report::{Section, f, section};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsbDevice {
    pub name: String,
    pub class: u8,
    pub subclass: u8,
    pub vendor: u16,
    pub product: u16,
}

pub fn line(d: &UsbDevice) -> String {
    format!(
        "{}, class=0x{:02X}, subclass=0x{:02X}, vendor=0x{:04X}, product=0x{:04X}",
        d.name, d.class, d.subclass, d.vendor, d.product
    )
}

pub fn devices(_ctx: &Ctx) -> Vec<UsbDevice> {
    imp::devices()
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    vec![section("USB Devices", devices(ctx).iter().map(|d| f("USB Device", line(d))).collect())]
}

/// Hex value following `tag` in a hardware/compatible ID ("VID_" -> 0x0DB0).
pub fn id_field(id: &str, tag: &str) -> Option<u32> {
    let up = id.to_ascii_uppercase();
    let rest = &up[up.find(tag)? + tag.len()..];
    let hex: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
    u32::from_str_radix(&hex, 16).ok()
}

/// (class, subclass) from compatible IDs, preferring the device class.
pub fn class_from_compat(ids: &[String]) -> Option<(u8, u8)> {
    let pick = |tag: &str| {
        ids.iter().find_map(|id| {
            let c = id_field(id, tag)?;
            Some((c as u8, id_field(id, "SUBCLASS_").unwrap_or(0) as u8))
        })
    };
    if ids.iter().any(|i| i.eq_ignore_ascii_case("USB\\COMPOSITE")) {
        return Some((0, 0));
    }
    pick("DEVCLASS_").or_else(|| pick("\\CLASS_"))
}

/// Product (or vendor) name from a usb.ids / pci.ids style database.
pub fn ids_lookup(db: &str, vendor: u16, product: u16) -> Option<String> {
    let (v, p) = (format!("{vendor:04x}"), format!("{product:04x}"));
    let mut vname = None;
    for l in db.lines() {
        if let Some(rest) = l.strip_prefix('\t') {
            if vname.is_some() && !rest.starts_with('\t') && rest.get(..4) == Some(p.as_str()) {
                return Some(rest[4..].trim().to_string());
            }
        } else if vname.is_some() {
            break; // left our vendor block
        } else if !l.starts_with('#') && l.get(..4) == Some(v.as_str()) && l.as_bytes().get(4) == Some(&b' ') {
            vname = Some(l[4..].trim().to_string());
        }
    }
    vname
}

/// Blocks of `"key" = value` pairs from `ioreg -l` text, one per `+-o` node.
pub fn ioreg_nodes(text: &str) -> Vec<(String, std::collections::HashMap<String, String>)> {
    let mut out: Vec<(String, std::collections::HashMap<String, String>)> = Vec::new();
    for l in text.lines() {
        let t = l.trim_start_matches([' ', '|']);
        if let Some(n) = t.strip_prefix("+-o ") {
            let name = n.split(['@', '<']).next().unwrap_or("").trim().to_string();
            out.push((name, Default::default()));
        } else if let (Some((k, v)), Some(last)) = (t.split_once(" = "), out.last_mut()) {
            last.1.insert(k.trim_matches('"').to_string(), v.trim().trim_matches('"').to_string());
        }
    }
    out
}

#[cfg(windows)]
mod imp {
    use super::{UsbDevice, class_from_compat, id_field};
    use crate::storage::win::*;

    const IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX: u32 = 0x220448;

    /// bDeviceClass/SubClass from the parent hub's view of this port.
    fn hub_class(dn: u32) -> Option<(u8, u8)> {
        let port = reg_u32(dn, DRP_ADDRESS)?;
        let hub = dev_id(parent(dn)?)?;
        let path = interfaces(&GUID_USB_HUB, Some(&hub)).into_iter().next()?;
        let h = Handle::open(&path)?;
        let mut q = [0u8; 64];
        q[..4].copy_from_slice(&port.to_le_bytes());
        // Packed struct: ConnectionIndex(4) then USB_DEVICE_DESCRIPTOR.
        let b = h.ioctl(IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX, &q, 512)?;
        (b.len() >= 16 && b[5] == 1).then(|| (b[8], b[9]))
    }

    pub fn devices() -> Vec<UsbDevice> {
        let mut out = Vec::new();
        for id in id_list("USB") {
            let up = id.to_ascii_uppercase();
            if up.contains("&MI_") || up.starts_with("USB\\ROOT_HUB") {
                continue; // interfaces of composite devices, root hubs
            }
            let (Some(vendor), Some(product)) = (id_field(&id, "VID_"), id_field(&id, "PID_")) else { continue };
            let Some(dn) = locate(&id) else { continue };
            let (class, subclass) =
                hub_class(dn).or_else(|| class_from_compat(&reg_strs(dn, DRP_COMPATIBLEIDS))).unwrap_or((0, 0));
            out.push(UsbDevice {
                name: reg_str(dn, DRP_DEVICEDESC).unwrap_or_else(|| "USB Device".into()),
                class,
                subclass,
                vendor: vendor as u16,
                product: product as u16,
            });
        }
        out
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{UsbDevice, ids_lookup};
    use crate::os::read_trim;

    pub fn devices() -> Vec<UsbDevice> {
        let db = ["/usr/share/hwdata/usb.ids", "/usr/share/misc/usb.ids", "/usr/share/usb.ids"]
            .iter()
            .find_map(|p| std::fs::read_to_string(p).ok());
        let mut names: Vec<_> = std::fs::read_dir("/sys/bus/usb/devices")
            .map(|r| r.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        names.sort();
        let hex = |p: &std::path::Path, f: &str| read_trim(p.join(f)).and_then(|s| u32::from_str_radix(&s, 16).ok());
        names
            .iter()
            .filter(|p| {
                p.file_name()
                    .is_some_and(|n| !n.to_string_lossy().contains(':') && !n.to_string_lossy().starts_with("usb"))
            })
            .filter_map(|p| {
                let (vendor, product) = (hex(p, "idVendor")? as u16, hex(p, "idProduct")? as u16);
                let name = read_trim(p.join("product"))
                    .or_else(|| db.as_deref().and_then(|d| ids_lookup(d, vendor, product)))
                    .or_else(|| read_trim(p.join("manufacturer")))
                    .unwrap_or_else(|| "USB Device".into());
                Some(UsbDevice {
                    name,
                    class: hex(p, "bDeviceClass").unwrap_or(0) as u8,
                    subclass: hex(p, "bDeviceSubClass").unwrap_or(0) as u8,
                    vendor,
                    product,
                })
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{UsbDevice, ioreg_nodes};

    pub fn devices() -> Vec<UsbDevice> {
        let text = crate::os::cmd("ioreg", &["-p", "IOUSB", "-l", "-w0"]).unwrap_or_default();
        ioreg_nodes(&text)
            .into_iter()
            .filter_map(|(node, kv)| {
                let n = |k: &str| kv.get(k).and_then(|v| v.parse::<u32>().ok());
                let name = ["USB Product Name", "kUSBProductString"].iter().find_map(|k| kv.get(*k).cloned());
                Some(UsbDevice {
                    name: name.unwrap_or(node),
                    class: n("bDeviceClass").unwrap_or(0) as u8,
                    subclass: n("bDeviceSubClass").unwrap_or(0) as u8,
                    vendor: n("idVendor")? as u16,
                    product: n("idProduct")? as u16,
                })
            })
            .collect()
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn devices() -> Vec<super::UsbDevice> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_ids() {
        let id = r"USB\VID_0DB0&PID_0076\5&1234";
        assert_eq!((id_field(id, "VID_"), id_field(id, "PID_")), (Some(0x0DB0), Some(0x0076)));
        let c = |v: &[&str]| class_from_compat(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(c(&[r"USB\Class_E0&SubClass_01&Prot_01", r"USB\Class_E0"]), Some((0xE0, 1)));
        assert_eq!(c(&[r"USB\DevClass_00&SubClass_00&Prot_00", r"USB\COMPOSITE"]), Some((0, 0)));
        assert_eq!(c(&[]), None);
        let d = UsbDevice { name: "Generic USB Hub".into(), class: 9, subclass: 0, vendor: 0x05E3, product: 0x0608 };
        assert_eq!(line(&d), "Generic USB Hub, class=0x09, subclass=0x00, vendor=0x05E3, product=0x0608");
    }

    #[test]
    fn usb_ids() {
        let db = "# comment\n05e3  Genesys Logic, Inc.\n\t0608  Hub\n\t0610  Hub\n\t\t0001  iface\n0db0  Micro Star\n\t0076  Thing\n";
        assert_eq!(ids_lookup(db, 0x05e3, 0x0610).as_deref(), Some("Hub"));
        assert_eq!(ids_lookup(db, 0x05e3, 0x9999).as_deref(), Some("Genesys Logic, Inc."));
        assert_eq!(ids_lookup(db, 0x0db0, 0x0076).as_deref(), Some("Thing"));
        assert_eq!(ids_lookup(db, 0x1234, 0x0001), None);
    }

    #[test]
    fn ioreg() {
        let t = "+-o Root  <class IORegistryEntry>\n  +-o USB3.1 Hub@01100000  <class IOUSBHostDevice>\n  |   {\n  |     \"idProduct\" = 1553\n  |     \"USB Product Name\" = \"USB3.1 Hub\"\n  |     \"idVendor\" = 3034\n  |     \"bDeviceClass\" = 9\n  |   }\n";
        let n = ioreg_nodes(t);
        assert_eq!(n.len(), 2);
        assert_eq!(n[1].0, "USB3.1 Hub");
        assert_eq!(n[1].1.get("idVendor").map(String::as_str), Some("3034"));
        assert_eq!(n[1].1.get("USB Product Name").map(String::as_str), Some("USB3.1 Hub"));
    }
}

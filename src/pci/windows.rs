//! Windows: SetupAPI enumeration of the PCI bus driver's devices (user mode).
use super::{Device, Link};
use crate::hw::Bdf;
use std::collections::HashMap;

/// Fields of "PCI\VEN_10DE&DEV_2882&SUBSYS_51611462&REV_A1" and "...&CC_030000".
fn hwid_field(ids: &[String], key: &str) -> Option<u32> {
    ids.iter().find_map(|id| {
        id.split(['\\', '&']).find_map(|p| p.strip_prefix(key)).and_then(|h| u32::from_str_radix(h, 16).ok())
    })
}

/// Windows DEVPKEY_PciDevice_DeviceType -> PCIe capability port type.
fn port_type(t: u32) -> Option<u8> {
    Some(match t {
        2 => 0,
        3 => 1,
        4 => 9,
        8 => 4,
        9 => 5,
        10 => 6,
        11 => 7,
        12 => 8,
        14 => 0xA,
        _ => return None,
    })
}

pub fn list() -> Vec<Device> {
    let raw = cpuzeez_platform::setupapi::pci_devices();
    let mut out = Vec::new();
    let mut inst: Vec<(u32, u32)> = Vec::new(); // (devinst, parent devinst) per device
    for r in raw {
        let ids = &r.hardware_ids;
        let (Some(bus), Some(addr), Some(vendor)) = (r.bus, r.address, hwid_field(ids, "VEN_")) else {
            continue;
        };
        let mut dev = Device::new(Bdf { seg: 0, bus: bus as u8, dev: (addr >> 16) as u8, func: addr as u8 });
        dev.vendor = vendor as u16;
        dev.device = hwid_field(ids, "DEV_").unwrap_or(0) as u16;
        let ss = hwid_field(ids, "SUBSYS_").unwrap_or(0);
        (dev.subvendor, dev.subdevice) = (ss as u16, (ss >> 16) as u16);
        dev.revision = hwid_field(ids, "REV_").unwrap_or(0) as u8;
        let p = |pid: usize| r.pci[pid];
        dev.class = match hwid_field(ids, "CC_") {
            Some(c) if c > 0xFFFF => c,
            _ => p(3).unwrap_or(0) << 16 | p(4).unwrap_or(0) << 8 | p(5).unwrap_or(0),
        };
        dev.name = r.name.clone();
        dev.port_type = p(1).and_then(port_type);
        if let (Some(s), Some(w), Some(ms), Some(mw)) = (p(9), p(10), p(11), p(12))
            && mw > 0
        {
            dev.link = Some(Link { speed: s as u8, width: w as u8, max_speed: ms as u8, max_width: mw as u8 });
        }
        inst.push((r.inst, r.parent));
        out.push(dev);
    }
    let by_inst: HashMap<u32, Bdf> = inst.iter().zip(&out).map(|(i, d)| (i.0, d.bdf)).collect();
    for (d, (_, p)) in out.iter_mut().zip(&inst) {
        d.parent = by_inst.get(p).copied();
    }
    out
}

/// Raw ACPI table by signature (GetSystemFirmwareTable 'ACPI').
pub fn acpi_table(sig: [u8; 4]) -> Option<Vec<u8>> {
    cpuzeez_platform::firmware::table(u32::from_be_bytes(*b"ACPI"), u32::from_le_bytes(sig))
}

#[cfg(test)]
mod tests {
    #[test]
    fn hwid() {
        let ids = vec![
            "PCI\\VEN_10DE&DEV_2882&SUBSYS_51611462&REV_A1".to_string(),
            "PCI\\VEN_10DE&DEV_2882&CC_030000".to_string(),
        ];
        assert_eq!(super::hwid_field(&ids, "VEN_"), Some(0x10DE));
        assert_eq!(super::hwid_field(&ids, "SUBSYS_"), Some(0x51611462));
        assert_eq!(super::hwid_field(&ids, "REV_"), Some(0xA1));
        assert_eq!(super::hwid_field(&ids, "CC_"), Some(0x030000));
    }
}

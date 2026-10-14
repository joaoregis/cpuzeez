//! Windows: SetupAPI enumeration of the PCI bus driver's devices (user mode).
#![allow(non_snake_case)]
use super::{Device, Link};
use crate::hw::Bdf;
use std::collections::HashMap;

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid(u32, u16, u16, [u8; 8]);

#[repr(C)]
struct DevPropKey {
    fmtid: Guid,
    pid: u32,
}

#[repr(C)]
struct DevInfoData {
    cb_size: u32,
    class_guid: Guid,
    dev_inst: u32,
    reserved: usize,
}

#[link(name = "setupapi")]
unsafe extern "system" {
    fn SetupDiGetClassDevsW(guid: *const Guid, enumerator: *const u16, hwnd: isize, flags: u32) -> isize;
    fn SetupDiEnumDeviceInfo(set: isize, idx: u32, data: *mut DevInfoData) -> i32;
    fn SetupDiGetDeviceRegistryPropertyW(
        set: isize,
        data: *const DevInfoData,
        prop: u32,
        regtype: *mut u32,
        buf: *mut u8,
        size: u32,
        req: *mut u32,
    ) -> i32;
    fn SetupDiGetDevicePropertyW(
        set: isize,
        data: *const DevInfoData,
        key: *const DevPropKey,
        ty: *mut u32,
        buf: *mut u8,
        size: u32,
        req: *mut u32,
        flags: u32,
    ) -> i32;
    fn SetupDiDestroyDeviceInfoList(set: isize) -> i32;
}

#[link(name = "cfgmgr32")]
unsafe extern "system" {
    fn CM_Get_Parent(out: *mut u32, dev: u32, flags: u32) -> u32;
}

const DIGCF_PRESENT: u32 = 0x2;
const DIGCF_ALLCLASSES: u32 = 0x4;
const SPDRP_DEVICEDESC: u32 = 0x0;
const SPDRP_HARDWAREID: u32 = 0x1;
const SPDRP_FRIENDLYNAME: u32 = 0xC;
const SPDRP_BUSNUMBER: u32 = 0x15;
const SPDRP_ADDRESS: u32 = 0x1C;
/// DEVPKEY_PciDevice_* (pciprop.h).
const PCI_PROPS: Guid = Guid(0x3ab22e31, 0x8264, 0x4b4e, [0x9a, 0xf5, 0xa8, 0xd2, 0xd8, 0xe3, 0x3e, 0x62]);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn reg_prop(set: isize, d: &mut DevInfoData, prop: u32) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; 2048];
    let (mut ty, mut req) = (0u32, 0u32);
    // SAFETY: buffer and sizes are valid; set/d come from SetupDiEnumDeviceInfo.
    let ok = unsafe {
        SetupDiGetDeviceRegistryPropertyW(set, d, prop, &mut ty, buf.as_mut_ptr(), buf.len() as u32, &mut req)
    };
    (ok != 0).then(|| {
        buf.truncate(req as usize);
        buf
    })
}

fn pci_u32(set: isize, d: &mut DevInfoData, pid: u32) -> Option<u32> {
    let key = DevPropKey { fmtid: PCI_PROPS, pid };
    let mut b = [0u8; 4];
    let (mut ty, mut req) = (0u32, 0u32);
    // SAFETY: as above; 4-byte buffer for DEVPROP_TYPE_UINT32.
    let ok = unsafe { SetupDiGetDevicePropertyW(set, d, &key, &mut ty, b.as_mut_ptr(), 4, &mut req, 0) };
    (ok != 0 && req == 4).then(|| u32::from_le_bytes(b))
}

/// REG_MULTI_SZ / REG_SZ UTF-16 bytes -> strings.
fn strings(b: &[u8]) -> Vec<String> {
    let u: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    u.split(|&c| c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect()
}

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
    let mut out = Vec::new();
    let mut inst: Vec<(u32, u32)> = Vec::new(); // (devinst, parent devinst) per device
    let en = wide("PCI");
    // SAFETY: standard SetupAPI enumeration; the set is destroyed below.
    let set = unsafe { SetupDiGetClassDevsW(std::ptr::null(), en.as_ptr(), 0, DIGCF_PRESENT | DIGCF_ALLCLASSES) };
    if set == -1 {
        return out;
    }
    for i in 0.. {
        let mut d = DevInfoData {
            cb_size: std::mem::size_of::<DevInfoData>() as u32,
            class_guid: Guid(0, 0, 0, [0; 8]),
            dev_inst: 0,
            reserved: 0,
        };
        // SAFETY: d.cb_size is set as the API requires.
        if unsafe { SetupDiEnumDeviceInfo(set, i, &mut d) } == 0 {
            break;
        }
        let ids = reg_prop(set, &mut d, SPDRP_HARDWAREID).map(|b| strings(&b)).unwrap_or_default();
        let u32p =
            |b: Option<Vec<u8>>| b.filter(|b| b.len() >= 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let (Some(bus), Some(addr), Some(vendor)) = (
            u32p(reg_prop(set, &mut d, SPDRP_BUSNUMBER)),
            u32p(reg_prop(set, &mut d, SPDRP_ADDRESS)),
            hwid_field(&ids, "VEN_"),
        ) else {
            continue;
        };
        let mut dev = Device::new(Bdf { seg: 0, bus: bus as u8, dev: (addr >> 16) as u8, func: addr as u8 });
        dev.vendor = vendor as u16;
        dev.device = hwid_field(&ids, "DEV_").unwrap_or(0) as u16;
        let ss = hwid_field(&ids, "SUBSYS_").unwrap_or(0);
        (dev.subvendor, dev.subdevice) = (ss as u16, (ss >> 16) as u16);
        dev.revision = hwid_field(&ids, "REV_").unwrap_or(0) as u8;
        dev.class = match hwid_field(&ids, "CC_") {
            Some(c) if c > 0xFFFF => c,
            _ => {
                let mut b = |pid| pci_u32(set, &mut d, pid).unwrap_or(0);
                b(3) << 16 | b(4) << 8 | b(5)
            }
        };
        dev.name = reg_prop(set, &mut d, SPDRP_FRIENDLYNAME)
            .or_else(|| reg_prop(set, &mut d, SPDRP_DEVICEDESC))
            .and_then(|b| strings(&b).into_iter().next());
        dev.port_type = pci_u32(set, &mut d, 1).and_then(port_type);
        if let (Some(s), Some(w), Some(ms), Some(mw)) =
            (pci_u32(set, &mut d, 9), pci_u32(set, &mut d, 10), pci_u32(set, &mut d, 11), pci_u32(set, &mut d, 12))
            && mw > 0
        {
            dev.link = Some(Link { speed: s as u8, width: w as u8, max_speed: ms as u8, max_width: mw as u8 });
        }
        let mut parent = 0u32;
        // SAFETY: plain out-parameter call.
        let _ = unsafe { CM_Get_Parent(&mut parent, d.dev_inst, 0) };
        inst.push((d.dev_inst, parent));
        out.push(dev);
    }
    // SAFETY: set came from SetupDiGetClassDevsW.
    unsafe { SetupDiDestroyDeviceInfoList(set) };
    let by_inst: HashMap<u32, Bdf> = inst.iter().zip(&out).map(|(i, d)| (i.0, d.bdf)).collect();
    for (d, (_, p)) in out.iter_mut().zip(&inst) {
        d.parent = by_inst.get(p).copied();
    }
    out
}

/// Raw ACPI table by signature (GetSystemFirmwareTable 'ACPI').
pub fn acpi_table(sig: [u8; 4]) -> Option<Vec<u8>> {
    let prov = u32::from_be_bytes(*b"ACPI");
    let id = u32::from_le_bytes(sig);
    // SAFETY: size query then fill, as documented.
    unsafe {
        let n = crate::os::win::GetSystemFirmwareTable(prov, id, std::ptr::null_mut(), 0);
        if n == 0 {
            return None;
        }
        let mut b = vec![0u8; n as usize];
        let m = crate::os::win::GetSystemFirmwareTable(prov, id, b.as_mut_ptr(), n);
        (m == n).then_some(b)
    }
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

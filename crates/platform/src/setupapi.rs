//! Windows: SetupAPI enumeration of the PCI bus driver's devices (user
//! mode), as plain values. Decoding stays with the caller.
#![allow(non_snake_case)]

/// One device under the PCI enumerator.
#[derive(Debug, Clone, Default)]
pub struct PciDevice {
    /// SPDRP_HARDWAREID: "PCI\VEN_10DE&DEV_2882&SUBSYS_51611462&REV_A1", …
    pub hardware_ids: Vec<String>,
    pub bus: Option<u32>,
    /// SPDRP_ADDRESS: device << 16 | function.
    pub address: Option<u32>,
    /// SPDRP_FRIENDLYNAME, else SPDRP_DEVICEDESC.
    pub name: Option<String>,
    /// DEVPKEY_PciDevice_* by property id (pciprop.h), index = pid, 1..=12.
    pub pci: [Option<u32>; 13],
    /// The device instance and its parent's (CM_Get_Parent).
    pub inst: u32,
    pub parent: u32,
}

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

/// Every present device under the PCI enumerator.
pub fn pci_devices() -> Vec<PciDevice> {
    let mut out = Vec::new();
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
        let u32p =
            |b: Option<Vec<u8>>| b.filter(|b| b.len() >= 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        let mut dev = PciDevice {
            hardware_ids: reg_prop(set, &mut d, SPDRP_HARDWAREID).map(|b| strings(&b)).unwrap_or_default(),
            bus: u32p(reg_prop(set, &mut d, SPDRP_BUSNUMBER)),
            address: u32p(reg_prop(set, &mut d, SPDRP_ADDRESS)),
            name: reg_prop(set, &mut d, SPDRP_FRIENDLYNAME)
                .or_else(|| reg_prop(set, &mut d, SPDRP_DEVICEDESC))
                .and_then(|b| strings(&b).into_iter().next()),
            inst: d.dev_inst,
            ..Default::default()
        };
        for pid in 1..=12 {
            dev.pci[pid] = pci_u32(set, &mut d, pid as u32);
        }
        // SAFETY: plain out-parameter call.
        let _ = unsafe { CM_Get_Parent(&mut dev.parent, d.dev_inst, 0) };
        out.push(dev);
    }
    // SAFETY: set came from SetupDiGetClassDevsW.
    unsafe { SetupDiDestroyDeviceInfoList(set) };
    out
}

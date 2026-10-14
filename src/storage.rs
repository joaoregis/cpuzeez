//! Storage section (FEATURES.md 3.15).
//! Windows: storage IOCTLs + CfgMgr32. Linux: /sys/block. macOS: system_profiler.
//! Read-only everywhere: drives are opened with zero access rights (Windows) or
//! only through sysfs / system tools, never written.
// Parsers stay OS-neutral so their unit tests run on every host.
#![allow(dead_code)]

use crate::Ctx;
use crate::report::{Node, Section, f, gv, section};

#[derive(Debug, Clone, Default)]
pub struct Volume {
    pub name: String,
    pub total: u64,
    pub free: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Drive {
    /// "index/device number/-1/-1", the key CPU-Z prints after "Drive".
    pub id: String,
    pub path: String,
    pub name: String,
    pub revision: Option<String>,
    pub serial: Option<String>,
    pub capacity: u64,
    pub removable: bool,
    pub ssd: bool,
    /// Windows STORAGE_BUS_TYPE numbering on every OS.
    pub bus: u32,
    pub rpm: Option<u32>,
    pub features: Vec<String>,
    pub controller: Option<String>,
    pub link: Option<String>,
    pub volumes: Vec<Volume>,
    /// SMART / NVMe health values (only when `cfg.smart`).
    pub health: Vec<(String, String)>,
}

const BUS: [&str; 21] = [
    "Unknown",
    "SCSI",
    "ATAPI",
    "ATA",
    "1394",
    "SSA",
    "Fibre",
    "USB",
    "RAID",
    "iSCSI",
    "SAS",
    "SATA",
    "SD",
    "MMC",
    "Virtual",
    "File Backed Virtual",
    "Spaces",
    "NVMe",
    "SCM",
    "UFS",
    "NVMe-oF",
];

pub fn bus_name(code: u32) -> &'static str {
    BUS.get(code as usize).copied().unwrap_or("Unknown")
}

impl Drive {
    pub fn kind(&self) -> String {
        let base = if self.removable { "Removable" } else { "Fixed" };
        if self.ssd { format!("{base}, SSD") } else { base.into() }
    }
}

fn gib(b: u64) -> String {
    format!("{:.1}", b as f64 / (1u64 << 30) as f64)
}

pub fn volume_line(v: &Volume) -> String {
    let pct = if v.total > 0 { v.free as f64 * 100.0 / v.total as f64 } else { 0.0 };
    format!("{}, {} GBytes ({pct:.1} percent available)", v.name, gib(v.total))
}

/// PCI class code (base<<16|sub<<8|prog-if) to a controller name.
pub fn controller_class(class: u32) -> Option<&'static str> {
    Some(match class {
        0x010601 => "Serial ATA Controller - AHCI",
        0x010802 => "NVM Express (NVMe) Controller",
        0x0C0330 => "USB xHCI Controller",
        0x0C0320 => "USB EHCI Controller",
        0x0C0310 => "USB OHCI Controller",
        0x0C0300 => "USB UHCI Controller",
        _ => match class >> 8 {
            0x0100 => "SCSI Controller",
            0x0101 => "IDE Controller",
            0x0104 => "RAID Controller",
            0x0105 => "ATA Controller",
            0x0106 => "Serial ATA Controller",
            0x0107 => "SAS Controller",
            0x0108 => "Non-Volatile Memory Controller",
            0x0805 => "SD Host Controller",
            _ => return None,
        },
    })
}

pub fn controller_line(name: &str, bus: u32, dev: u32, func: u32) -> String {
    format!("{name} at bus {bus}, device {dev}, function {func}")
}

pub fn link_line(width: u32, gts: &str) -> String {
    format!("PCI-E {width}x @ {gts} GT/s")
}

/// ATA IDENTIFY string in words [from, to): bytes are swapped within each word.
pub fn ata_string(id: &[u8], from: usize, to: usize) -> Option<String> {
    let b: Vec<u8> = id.get(from * 2..to * 2)?.as_chunks::<2>().0.iter().flat_map(|w| [w[1], w[0]]).collect();
    let s = String::from_utf8_lossy(&b).trim_matches([' ', '\0']).to_string();
    (!s.is_empty()).then_some(s)
}

/// (rotation rate, features) from a 512-byte ATA IDENTIFY DEVICE block.
pub fn ata_identify(id: &[u8]) -> (Option<u32>, Vec<String>) {
    let w = |i: usize| id.get(i * 2..i * 2 + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]));
    if id.len() < 512 {
        return (None, Vec::new());
    }
    let rate = w(217);
    let rpm = match rate {
        1 => Some(1), // non-rotating
        0x0401..=0xFFFE => Some(rate as u32),
        _ => None,
    };
    let mut feat = Vec::new();
    // Word 82/83 are valid only when bit 14 set and bit 15 clear.
    if w(83) & 0xC000 == 0x4000 && w(82) & 1 != 0 {
        feat.push("SMART".into());
    }
    if w(169) & 1 != 0 {
        feat.push("TRIM".into());
    }
    (rpm, feat)
}

pub fn drives(ctx: &Ctx) -> Vec<Drive> {
    imp::probe(ctx)
}

fn node(d: &Drive) -> Node {
    let mut c = vec![f("Device Path", &d.path), f("Name", &d.name)];
    if let Some(r) = &d.revision {
        c.push(f("Revision", r));
    }
    if let Some(s) = &d.serial {
        c.push(f("Serial", s));
    }
    c.push(f("Capacity", format!("{} GB", gib(d.capacity))));
    c.push(f("Type", d.kind()));
    c.push(f("Bus Type", format!("{} ({})", bus_name(d.bus), d.bus)));
    if let Some(r) = d.rpm.filter(|&r| r > 1) {
        c.push(f("Rotation speed", format!("{r} RPM")));
    }
    c.push(f("Features", d.features.join(", ")));
    if let Some(s) = &d.controller {
        c.push(f("Controller", s));
    }
    if let Some(s) = &d.link {
        c.push(f("Link Speed", s));
    }
    c.extend(d.volumes.iter().map(|v| f("Volume", volume_line(v))));
    c.extend(d.health.iter().map(|(k, v)| f(k, v)));
    gv("Drive", &d.id, c)
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    vec![section("Storage", drives(ctx).iter().map(node).collect())]
}

// ---------------------------------------------------------------- Windows

#[cfg(windows)]
pub(crate) mod win {
    #![allow(non_snake_case)]
    use std::ffi::c_void;

    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    fn multi_sz(b: &[u16]) -> Vec<String> {
        b.split(|&c| c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect()
    }

    pub fn le32(b: &[u8], o: usize) -> u32 {
        b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Guid(pub u32, pub u16, pub u16, pub [u8; 8]);
    #[repr(C)]
    pub struct PropKey(pub Guid, pub u32);

    pub const GUID_DISK: Guid = Guid(0x53f56307, 0xb6bf, 0x11d0, [0x94, 0xf2, 0x00, 0xa0, 0xc9, 0x1e, 0xfb, 0x8b]);
    pub const GUID_USB_HUB: Guid = Guid(0xf18a0e88, 0xc30c, 0x11d0, [0x88, 0x15, 0x00, 0xa0, 0xc9, 0x06, 0xbe, 0xd8]);
    pub const KEY_INSTANCE_ID: PropKey =
        PropKey(Guid(0x78c34fc8, 0x104a, 0x4aca, [0x9e, 0xa4, 0x52, 0x4d, 0x52, 0x99, 0x6e, 0x57]), 256);
    const PCI_PROPS: Guid = Guid(0x3ab22e31, 0x8264, 0x4b4e, [0x9a, 0xf5, 0xa8, 0xd2, 0xd8, 0xe3, 0x3e, 0x62]);
    pub const KEY_LINK_SPEED: PropKey = PropKey(PCI_PROPS, 9);
    pub const KEY_LINK_WIDTH: PropKey = PropKey(PCI_PROPS, 10);

    pub const DRP_DEVICEDESC: u32 = 1;
    pub const DRP_COMPATIBLEIDS: u32 = 3;
    pub const DRP_BUSNUMBER: u32 = 0x16;
    pub const DRP_ADDRESS: u32 = 0x1D;

    #[link(name = "cfgmgr32")]
    unsafe extern "system" {
        fn CM_Get_Device_Interface_List_SizeW(len: *mut u32, g: *const Guid, dev: *const u16, fl: u32) -> u32;
        fn CM_Get_Device_Interface_ListW(g: *const Guid, dev: *const u16, buf: *mut u16, len: u32, fl: u32) -> u32;
        fn CM_Get_Device_Interface_PropertyW(
            path: *const u16,
            key: *const PropKey,
            ty: *mut u32,
            buf: *mut u8,
            len: *mut u32,
            fl: u32,
        ) -> u32;
        fn CM_Get_Device_ID_List_SizeW(len: *mut u32, filter: *const u16, fl: u32) -> u32;
        fn CM_Get_Device_ID_ListW(filter: *const u16, buf: *mut u16, len: u32, fl: u32) -> u32;
        fn CM_Locate_DevNodeW(dn: *mut u32, id: *const u16, fl: u32) -> u32;
        fn CM_Get_Parent(dn: *mut u32, child: u32, fl: u32) -> u32;
        fn CM_Get_Device_IDW(dn: u32, buf: *mut u16, len: u32, fl: u32) -> u32;
        fn CM_Get_DevNode_Registry_PropertyW(
            dn: u32,
            p: u32,
            ty: *mut u32,
            buf: *mut c_void,
            len: *mut u32,
            fl: u32,
        ) -> u32;
        fn CM_Get_DevNode_PropertyW(
            dn: u32,
            key: *const PropKey,
            ty: *mut u32,
            buf: *mut u8,
            len: *mut u32,
            fl: u32,
        ) -> u32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(n: *const u16, acc: u32, share: u32, sa: *mut c_void, disp: u32, fl: u32, t: isize) -> isize;
        fn DeviceIoControl(
            h: isize,
            code: u32,
            inb: *const c_void,
            inl: u32,
            outb: *mut c_void,
            outl: u32,
            ret: *mut u32,
            ov: *mut c_void,
        ) -> i32;
        fn CloseHandle(h: isize) -> i32;
        pub fn GetLogicalDriveStringsW(len: u32, buf: *mut u16) -> u32;
        pub fn GetDiskFreeSpaceExW(root: *const u16, avail: *mut u64, total: *mut u64, free: *mut u64) -> i32;
    }

    /// Device interfaces of class `g` (present only), optionally for one instance.
    pub fn interfaces(g: &Guid, dev: Option<&str>) -> Vec<String> {
        let d = dev.map(wide);
        let dp = d.as_ref().map_or(std::ptr::null(), |d| d.as_ptr());
        let mut len = 0u32;
        unsafe {
            if CM_Get_Device_Interface_List_SizeW(&mut len, g, dp, 0) != 0 || len == 0 {
                return Vec::new();
            }
            let mut buf = vec![0u16; len as usize];
            if CM_Get_Device_Interface_ListW(g, dp, buf.as_mut_ptr(), len, 0) != 0 {
                return Vec::new();
            }
            multi_sz(&buf)
        }
    }

    /// Present device instance IDs under an enumerator ("USB", "PCI"...).
    pub fn id_list(enumerator: &str) -> Vec<String> {
        let e = wide(enumerator);
        const FLAGS: u32 = 0x1 | 0x100; // FILTER_ENUMERATOR | FILTER_PRESENT
        let mut len = 0u32;
        unsafe {
            if CM_Get_Device_ID_List_SizeW(&mut len, e.as_ptr(), FLAGS) != 0 || len == 0 {
                return Vec::new();
            }
            let mut buf = vec![0u16; len as usize];
            if CM_Get_Device_ID_ListW(e.as_ptr(), buf.as_mut_ptr(), len, FLAGS) != 0 {
                return Vec::new();
            }
            multi_sz(&buf)
        }
    }

    pub fn iface_instance(path: &str) -> Option<String> {
        let p = wide(path);
        let mut buf = vec![0u8; 1024];
        let (mut ty, mut len) = (0u32, buf.len() as u32);
        let r = unsafe {
            CM_Get_Device_Interface_PropertyW(p.as_ptr(), &KEY_INSTANCE_ID, &mut ty, buf.as_mut_ptr(), &mut len, 0)
        };
        (r == 0).then(|| utf16(&buf[..len as usize]))?
    }

    fn utf16(b: &[u8]) -> Option<String> {
        let u: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        multi_sz(&u).into_iter().next()
    }

    pub fn locate(id: &str) -> Option<u32> {
        let (w, mut dn) = (wide(id), 0u32);
        (unsafe { CM_Locate_DevNodeW(&mut dn, w.as_ptr(), 0) } == 0).then_some(dn)
    }

    pub fn parent(dn: u32) -> Option<u32> {
        let mut p = 0u32;
        (unsafe { CM_Get_Parent(&mut p, dn, 0) } == 0).then_some(p)
    }

    pub fn dev_id(dn: u32) -> Option<String> {
        let mut buf = [0u16; 512];
        (unsafe { CM_Get_Device_IDW(dn, buf.as_mut_ptr(), buf.len() as u32, 0) } == 0)
            .then(|| multi_sz(&buf).into_iter().next())?
    }

    // ponytail: fixed 4 KiB buffer; IDs and descriptions are far shorter.
    fn reg_prop(dn: u32, p: u32) -> Option<Vec<u8>> {
        let mut buf = vec![0u8; 4096];
        let (mut ty, mut len) = (0u32, buf.len() as u32);
        let r = unsafe { CM_Get_DevNode_Registry_PropertyW(dn, p, &mut ty, buf.as_mut_ptr().cast(), &mut len, 0) };
        (r == 0).then(|| {
            buf.truncate(len as usize);
            buf
        })
    }

    pub fn reg_strs(dn: u32, p: u32) -> Vec<String> {
        reg_prop(dn, p).map_or(Vec::new(), |b| {
            let u: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            multi_sz(&u)
        })
    }

    pub fn reg_str(dn: u32, p: u32) -> Option<String> {
        reg_strs(dn, p).into_iter().next()
    }

    pub fn reg_u32(dn: u32, p: u32) -> Option<u32> {
        reg_prop(dn, p).filter(|b| b.len() >= 4).map(|b| le32(&b, 0))
    }

    pub fn prop_u32(dn: u32, key: &PropKey) -> Option<u32> {
        let mut buf = [0u8; 8];
        let (mut ty, mut len) = (0u32, 8u32);
        let r = unsafe { CM_Get_DevNode_PropertyW(dn, key, &mut ty, buf.as_mut_ptr(), &mut len, 0) };
        (r == 0 && len == 4).then(|| le32(&buf, 0))
    }

    pub struct Handle(isize);

    impl Handle {
        /// Zero access rights: enough for the query IOCTLs, never for I/O.
        pub fn open(path: &str) -> Option<Handle> {
            Self::open_with(path, 0)
        }

        /// GENERIC_READ|GENERIC_WRITE, required by pass-through IOCTLs (admin only).
        pub fn open_rw(path: &str) -> Option<Handle> {
            Self::open_with(path, 0xC000_0000)
        }

        fn open_with(path: &str, access: u32) -> Option<Handle> {
            let w = wide(path);
            const SHARE: u32 = 3; // FILE_SHARE_READ | FILE_SHARE_WRITE
            let h = unsafe { CreateFileW(w.as_ptr(), access, SHARE, std::ptr::null_mut(), 3, 0, 0) };
            (h != -1).then_some(Handle(h))
        }

        pub fn ioctl(&self, code: u32, inp: &[u8], out: usize) -> Option<Vec<u8>> {
            let mut buf = vec![0u8; out];
            let mut ret = 0u32;
            let ok = unsafe {
                DeviceIoControl(
                    self.0,
                    code,
                    inp.as_ptr().cast(),
                    inp.len() as u32,
                    buf.as_mut_ptr().cast(),
                    out as u32,
                    &mut ret,
                    std::ptr::null_mut(),
                )
            };
            (ok != 0).then(|| {
                buf.truncate(ret as usize);
                buf
            })
        }
    }

    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::win::*;
    use super::{Drive, Volume, ata_identify, controller_class, controller_line, link_line};
    use crate::Ctx;
    use std::collections::HashMap;

    const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x2D1400;
    const IOCTL_STORAGE_GET_DEVICE_NUMBER: u32 = 0x2D1080;
    const IOCTL_DISK_GET_DRIVE_GEOMETRY_EX: u32 = 0x700A0;
    const IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS: u32 = 0x560000;

    fn query(h: &Handle, prop: u32) -> Option<Vec<u8>> {
        let mut q = [0u8; 12];
        q[..4].copy_from_slice(&prop.to_le_bytes());
        h.ioctl(IOCTL_STORAGE_QUERY_PROPERTY, &q, 4096)
    }

    /// ATA IDENTIFY DEVICE (0xEC, data-in only) via IOCTL_ATA_PASS_THROUGH.
    /// Needs a read/write handle, i.e. admin; returns None otherwise.
    fn ata_pass_identify(path: &str) -> Option<Vec<u8>> {
        const IOCTL_ATA_PASS_THROUGH: u32 = 0x4D02C;
        // ATA_PASS_THROUGH_EX: DataBufferOffset is a ULONG_PTR.
        let (hdr, off_at) = if cfg!(target_pointer_width = "64") { (48, 24) } else { (40, 20) };
        let mut q = vec![0u8; hdr + 512];
        q[0..2].copy_from_slice(&(hdr as u16).to_le_bytes());
        q[2] = 0x3; // ATA_FLAGS_DRDY_REQUIRED | ATA_FLAGS_DATA_IN
        q[8..12].copy_from_slice(&512u32.to_le_bytes());
        q[12..16].copy_from_slice(&3u32.to_le_bytes()); // timeout, s
        q[off_at..off_at + 4].copy_from_slice(&(hdr as u32).to_le_bytes());
        q[hdr - 8 + 6] = 0xEC; // CurrentTaskFile.Command
        let h = Handle::open_rw(path)?;
        let out = h.ioctl(IOCTL_ATA_PASS_THROUGH, &q, q.len())?;
        out.get(hdr..hdr + 512).map(<[u8]>::to_vec)
    }

    /// StorageAdapter/DeviceProtocolSpecificProperty (49/50) query.
    fn protocol(h: &Handle, prop: u32, proto: u32, dtype: u32, value: u32, len: u32) -> Option<Vec<u8>> {
        let mut q = vec![0u8; 48 + len as usize];
        for (i, v) in [prop, 0, proto, dtype, value, 0, 40, len].iter().enumerate() {
            q[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        let out = h.ioctl(IOCTL_STORAGE_QUERY_PROPERTY, &q, q.len())?;
        let (off, n) = (le32(&out, 24) as usize, le32(&out, 28) as usize);
        out.get(8 + off..8 + off + n).filter(|d| !d.is_empty()).map(<[u8]>::to_vec)
    }

    fn cstr(b: &[u8], off: u32) -> Option<String> {
        let s = b.get(off as usize..).filter(|_| off != 0)?;
        let s = &s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())];
        let s = String::from_utf8_lossy(s).trim_end().to_string();
        (!s.trim().is_empty()).then_some(s)
    }

    fn ascii(b: &[u8]) -> Option<String> {
        let s = String::from_utf8_lossy(b).trim_matches(|c: char| c == ' ' || c == '\0').to_string();
        (!s.is_empty()).then_some(s)
    }

    fn volumes() -> HashMap<u32, Vec<Volume>> {
        let mut map: HashMap<u32, Vec<Volume>> = HashMap::new();
        let mut buf = [0u16; 512];
        let n = unsafe { GetLogicalDriveStringsW(buf.len() as u32, buf.as_mut_ptr()) } as usize;
        for root in buf[..n.min(buf.len())].split(|&c| c == 0).filter(|s| !s.is_empty()) {
            let root = String::from_utf16_lossy(root); // "C:\"
            let Some(h) = Handle::open(&format!("\\\\.\\{}", root.trim_end_matches('\\'))) else { continue };
            let Some(ext) = h.ioctl(IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, &[], 8 + 24 * 32) else { continue };
            let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
            let w = wide(&root);
            if unsafe { GetDiskFreeSpaceExW(w.as_ptr(), &mut avail, &mut total, &mut free) } == 0 {
                continue;
            }
            // A volume spanning several disks is listed under each.
            for i in 0..le32(&ext, 0) as usize {
                let disk = le32(&ext, 8 + i * 24);
                let v = Volume { name: root.to_lowercase(), total, free };
                map.entry(disk).or_default().push(v);
            }
        }
        map
    }

    /// Nearest PCI ancestor of a disk devnode: (name, bus, dev, func, link).
    fn controller(dn: u32) -> Option<(String, Option<String>)> {
        let mut p = parent(dn)?;
        while !dev_id(p)?.to_ascii_uppercase().starts_with("PCI\\") {
            p = parent(p)?;
        }
        let class = reg_strs(p, DRP_COMPATIBLEIDS).iter().find_map(|id| {
            let up = id.to_ascii_uppercase();
            let cc = &up[up.find("CC_")? + 3..];
            let hex: String = cc.chars().take_while(char::is_ascii_hexdigit).collect();
            let v = u32::from_str_radix(&hex, 16).ok()?;
            Some(if hex.len() >= 6 { v } else { v << 8 })
        });
        let name = class.and_then(controller_class).map(str::to_string).or_else(|| reg_str(p, DRP_DEVICEDESC))?;
        let (bus, addr) = (reg_u32(p, DRP_BUSNUMBER)?, reg_u32(p, DRP_ADDRESS)?);
        let link = match (prop_u32(p, &KEY_LINK_WIDTH), prop_u32(p, &KEY_LINK_SPEED)) {
            (Some(w), Some(s @ 1..=6)) if w > 0 => {
                Some(link_line(w, ["2.5", "5.0", "8.0", "16.0", "32.0", "64.0"][s as usize - 1]))
            }
            _ => None,
        };
        Some((controller_line(&name, bus, addr >> 16, addr & 0xFFFF), link))
    }

    pub fn probe(ctx: &Ctx) -> Vec<Drive> {
        let vols = volumes();
        let mut out: Vec<(u32, Drive)> = Vec::new();
        for path in interfaces(&GUID_DISK, None) {
            let Some(h) = Handle::open(&path) else { continue };
            let Some(num) = h.ioctl(IOCTL_STORAGE_GET_DEVICE_NUMBER, &[], 12).map(|b| le32(&b, 4)) else {
                continue;
            };
            let Some(desc) = query(&h, 0) else { continue };
            let mut d = Drive { path: path.to_lowercase(), bus: le32(&desc, 28), ..Drive::default() };
            d.removable = desc.get(10).is_some_and(|&b| b != 0);
            let vendor = cstr(&desc, le32(&desc, 12));
            let product = cstr(&desc, le32(&desc, 16)).unwrap_or_default();
            d.name = match vendor.map(|v| v.trim().to_string()) {
                Some(v) if !product.starts_with(&v) => format!("{v} {}", product.trim()),
                _ => product.trim().to_string(),
            };
            d.revision = cstr(&desc, le32(&desc, 20)).map(|s| s.trim().to_string());
            d.serial = cstr(&desc, le32(&desc, 24)); // keeps ATA leading spaces, like CPU-Z
            // Seek penalty descriptor: IncursSeekPenalty at offset 8.
            d.ssd = query(&h, 7).and_then(|b| b.get(8).copied()) == Some(0);
            d.capacity = h
                .ioctl(IOCTL_DISK_GET_DRIVE_GEOMETRY_EX, &[], 64)
                .map_or(0, |b| b.get(24..32).map_or(0, |s| u64::from_le_bytes(s.try_into().unwrap_or_default())));
            if d.bus == 17 {
                // NVMe Identify Controller: real SN/MN/FR (the descriptor may carry an EUI-64).
                if let Some(id) = protocol(&h, 49, 3, 1, 1, 4096).filter(|b| b.len() >= 72) {
                    d.serial = ascii(&id[4..24]).or(d.serial);
                    d.name = ascii(&id[24..64]).unwrap_or(d.name);
                    d.revision = ascii(&id[64..72]).or(d.revision);
                }
                if ctx.cfg.smart {
                    let log = protocol(&h, 50, 3, 2, 2, 512).or_else(|| protocol(&h, 49, 3, 2, 2, 512));
                    d.health = log.map(|l| super::nvme_health(&l)).unwrap_or_default();
                }
            } else if let Some(id) = protocol(&h, 50, 2, 1, 0, 512).or_else(|| ata_pass_identify(&path)) {
                let (rpm, feat) = ata_identify(&id);
                d.rpm = rpm;
                d.features = feat;
                if rpm == Some(1) {
                    d.ssd = true;
                }
            }
            if let Some(dn) = iface_instance(&path).and_then(|id| locate(&id))
                && let Some((c, l)) = controller(dn)
            {
                d.controller = Some(c);
                d.link = l;
            }
            d.volumes = vols.get(&num).cloned().unwrap_or_default();
            out.push((num, d));
        }
        out.sort_by_key(|(n, _)| *n);
        out.into_iter()
            .enumerate()
            .map(|(i, (n, mut d))| {
                d.id = format!("{i}/{n}/-1/-1");
                d
            })
            .collect()
    }
}

/// Temperature / wear / hours from an NVMe SMART / Health log page (LID 02h).
pub fn nvme_health(l: &[u8]) -> Vec<(String, String)> {
    if l.len() < 144 {
        return Vec::new();
    }
    let k = u16::from_le_bytes([l[1], l[2]]) as i32;
    let hours = u64::from_le_bytes(l[128..136].try_into().unwrap_or_default());
    let mut v = Vec::new();
    if k > 0 {
        v.push(("Temperature".into(), format!("{} C", k - 273)));
    }
    v.push(("Percentage Used".into(), format!("{}%", l[5])));
    v.push(("Power On Hours".into(), hours.to_string()));
    v
}

// ---------------------------------------------------------------- Linux

/// Bus from a canonical /sys/block path.
pub fn linux_bus(path: &str) -> u32 {
    if path.contains("/nvme") {
        17
    } else if path.contains("/usb") {
        7
    } else if path.contains("/ata") {
        11
    } else if path.contains("/mmc_host/") {
        12
    } else if path.contains("/virtio") {
        14
    } else if path.contains("/host") {
        1
    } else {
        0
    }
}

/// Deepest PCI address ("0000:02:00.0") in a sysfs path.
pub fn last_bdf(path: &str) -> Option<&str> {
    path.split('/').rfind(|s| is_bdf(s))
}

fn is_bdf(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 12
        && b[4] == b':'
        && b[7] == b':'
        && b[10] == b'.'
        && b.iter().enumerate().all(|(i, c)| [4, 7, 10].contains(&i) || c.is_ascii_hexdigit())
}

/// /proc/mounts escapes spaces etc. as \ooo.
pub fn unescape_mount(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\' {
            let oct: String = it.clone().take(3).collect();
            if let Ok(v) = u8::from_str_radix(&oct, 8) {
                out.push(v as char);
                it.nth(2);
                continue;
            }
        }
        out.push(c);
    }
    out
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use crate::os::{read_trim, read_u64};
    use std::path::Path;

    fn statvfs(mp: &str) -> Option<(u64, u64)> {
        let c = std::ffi::CString::new(mp).ok()?;
        // SAFETY: plain out-struct filled by libc.
        unsafe {
            let mut s: libc::statvfs = std::mem::zeroed();
            (libc::statvfs(c.as_ptr(), &mut s) == 0)
                .then(|| (s.f_blocks as u64 * s.f_frsize as u64, s.f_bavail as u64 * s.f_frsize as u64))
        }
    }

    fn volumes(disk: &str) -> Vec<Volume> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for line in std::fs::read_to_string("/proc/mounts").unwrap_or_default().lines() {
            let mut it = line.split_whitespace();
            let (Some(dev), Some(mp)) = (it.next(), it.next()) else { continue };
            if !dev.starts_with("/dev/") {
                continue;
            }
            let real = std::fs::canonicalize(dev).unwrap_or_else(|_| dev.into());
            let Some(base) = real.file_name().and_then(|n| n.to_str()) else { continue };
            let ours = base == disk || Path::new(&format!("/sys/block/{disk}/{base}")).exists();
            if !ours || !seen.insert(base.to_string()) {
                continue;
            }
            let mp = unescape_mount(mp);
            if let Some((total, free)) = statvfs(&mp) {
                out.push(Volume { name: mp, total, free });
            }
        }
        out
    }

    fn vpd_serial(p: &Path) -> Option<String> {
        let b = std::fs::read(p.join("device/vpd_pg80")).ok()?;
        let n = *b.get(3)? as usize;
        let s = String::from_utf8_lossy(b.get(4..4 + n)?).trim().to_string();
        (!s.is_empty()).then_some(s)
    }

    fn pci_controller(bdf: &str) -> (Option<String>, Option<String>) {
        let p = Path::new("/sys/bus/pci/devices").join(bdf);
        let class = read_u64(p.join("class")).unwrap_or(0) as u32;
        let name = controller_class(class).map(str::to_string).unwrap_or_else(|| {
            let id = |f: &str| read_trim(p.join(f)).unwrap_or_default().trim_start_matches("0x").to_string();
            format!("PCI device {}:{}", id("vendor"), id("device"))
        });
        let n: Vec<u32> = bdf[5..].split([':', '.']).filter_map(|x| u32::from_str_radix(x, 16).ok()).collect();
        let ctl = (n.len() == 3).then(|| controller_line(&name, n[0], n[1], n[2]));
        let speed = read_trim(p.join("current_link_speed"));
        let gts = speed.as_deref().and_then(|s| s.split_whitespace().next()).filter(|s| s.contains('.'));
        let link = match (read_u64(p.join("current_link_width")), gts) {
            (Some(w), Some(g)) if w > 0 => Some(link_line(w as u32, g)),
            _ => None,
        };
        (ctl, link)
    }

    pub fn probe(ctx: &Ctx) -> Vec<Drive> {
        let mut names: Vec<String> = std::fs::read_dir("/sys/block")
            .map(|r| r.flatten().filter_map(|e| e.file_name().into_string().ok()).collect())
            .unwrap_or_default();
        names.retain(|n| !["loop", "ram", "zram", "dm-", "md", "nbd", "fd"].iter().any(|p| n.starts_with(p)));
        names.sort();
        let mut out = Vec::new();
        for name in names {
            let p = Path::new("/sys/block").join(&name);
            let sectors = read_u64(p.join("size")).unwrap_or(0);
            if sectors == 0 {
                continue;
            }
            let real = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone()).to_string_lossy().into_owned();
            let dev = p.join("device");
            let i = out.len();
            let mut d = Drive {
                id: format!("{i}/{i}/-1/-1"),
                path: format!("/dev/{name}"),
                capacity: sectors * 512,
                removable: read_u64(p.join("removable")) == Some(1),
                ssd: read_u64(p.join("queue/rotational")) == Some(0),
                bus: linux_bus(&real),
                ..Drive::default()
            };
            let model = read_trim(dev.join("model")).unwrap_or_else(|| name.clone());
            d.name = match read_trim(dev.join("vendor")) {
                Some(v) if v != "ATA" && !model.starts_with(&v) => format!("{v} {model}"),
                _ => model,
            };
            d.revision = read_trim(dev.join("firmware_rev")).or_else(|| read_trim(dev.join("rev")));
            d.serial = read_trim(dev.join("serial")).or_else(|| vpd_serial(&p));
            // ATA Information VPD page carries the IDENTIFY block at offset 60.
            if let Some(id) = std::fs::read(dev.join("vpd_pg89")).ok().filter(|b| b.len() >= 572) {
                let id = &id[60..572];
                (d.rpm, d.features) = ata_identify(id);
                d.revision = ata_string(id, 23, 27).or(d.revision); // sysfs `rev` keeps 4 chars
                d.serial = d.serial.or_else(|| ata_string(id, 10, 20));
            } else if let Some(b) = std::fs::read(dev.join("vpd_pgb1")).ok().filter(|b| b.len() >= 6) {
                // Block Device Characteristics: medium rotation rate.
                d.rpm = Some(u16::from_be_bytes([b[4], b[5]]) as u32).filter(|&r| r == 1 || r > 0x400);
            }
            // NVMe namespaces may sit under a virtual nvme-subsystem: look at the controller.
            let ctl_path = if last_bdf(&real).is_none() {
                std::fs::read_dir(&dev)
                    .ok()
                    .and_then(|r| r.flatten().find(|e| e.file_name().to_string_lossy().starts_with("nvme")))
                    .and_then(|e| std::fs::canonicalize(e.path()).ok())
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                real.clone()
            };
            if let Some(bdf) = last_bdf(&ctl_path) {
                (d.controller, d.link) = pci_controller(bdf);
            }
            if ctx.cfg.smart && d.bus == 17 {
                let hw = std::fs::read_dir(dev.join("hwmon")).ok().and_then(|mut r| r.next()).and_then(|e| e.ok());
                if let Some(t) = hw.and_then(|e| read_u64(e.path().join("temp1_input"))) {
                    d.health.push(("Temperature".into(), format!("{} C", t / 1000)));
                }
            }
            d.volumes = volumes(&name);
            out.push(d);
        }
        out
    }
}

// ---------------------------------------------------------------- macOS

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use serde_json::Value;

    fn s(v: &Value, k: &str) -> Option<String> {
        v.get(k).and_then(Value::as_str).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }

    /// First string value whose key (minus underscores) contains `needle`.
    fn find_key(v: &Value, needle: &str) -> Option<String> {
        v.as_object()?
            .iter()
            .find(|(k, _)| k.replace('_', "").contains(needle))
            .and_then(|(_, v)| v.as_str().map(str::to_string))
    }

    fn yes(v: Option<String>) -> bool {
        v.is_some_and(|s| s.eq_ignore_ascii_case("yes"))
    }

    /// Visit every object with a whole-disk "bsd_name" (diskN), with its parent.
    fn walk<'a>(v: &'a Value, parent: Option<&'a Value>, out: &mut Vec<(&'a Value, Option<&'a Value>)>) {
        match v {
            Value::Array(a) => a.iter().for_each(|x| walk(x, parent, out)),
            Value::Object(o) => {
                let bsd = o.get("bsd_name").and_then(Value::as_str).unwrap_or("");
                if bsd.strip_prefix("disk").is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit())) {
                    out.push((v, parent));
                }
                o.values().for_each(|x| walk(x, Some(v), out));
            }
            _ => {}
        }
    }

    pub fn probe(ctx: &Ctx) -> Vec<Drive> {
        let args = ["-json", "SPNVMeDataType", "SPSerialATADataType", "SPUSBDataType", "SPStorageDataType"];
        let Some(v) = crate::os::cmd("system_profiler", &args).and_then(|j| serde_json::from_str::<Value>(&j).ok())
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (key, bus) in [("SPNVMeDataType", 17), ("SPSerialATADataType", 11), ("SPUSBDataType", 7)] {
            let mut items = Vec::new();
            walk(&v[key], None, &mut items);
            for (it, par) in items {
                let i = out.len();
                let bsd = s(it, "bsd_name").unwrap_or_default();
                let medium = find_key(it, "mediumtype").unwrap_or_default().to_ascii_lowercase();
                let mut d = Drive {
                    id: format!("{i}/{}/-1/-1", bsd.trim_start_matches("disk")),
                    path: format!("/dev/{bsd}"),
                    name: s(it, "device_model").or_else(|| s(it, "_name")).unwrap_or_default(),
                    revision: s(it, "device_revision"),
                    serial: s(it, "device_serial").or_else(|| par.and_then(|p| s(p, "serial_num"))),
                    capacity: it.get("size_in_bytes").and_then(Value::as_u64).unwrap_or(0),
                    removable: yes(s(it, "removable_media")) || bus == 7,
                    ssd: bus == 17 || medium.contains("solid") || medium.contains("ssd"),
                    bus,
                    ..Drive::default()
                };
                if yes(find_key(it, "trimsupport")) {
                    d.features.push("TRIM".into());
                }
                if let Some(st) = s(it, "smart_status") {
                    d.features.insert(0, "SMART".into());
                    if ctx.cfg.smart {
                        d.health.push(("SMART Status".into(), st));
                    }
                }
                let look = |n: &str| find_key(it, n).or_else(|| par.and_then(|p| find_key(p, n)));
                if let Some(sp) = look("linkspeed") {
                    // NVMe: "8.0 GT/s" + "x4"; SATA: "6 Gigabit" alone.
                    let w = look("linkwidth").and_then(|w| w.trim_start_matches('x').parse::<u32>().ok());
                    d.link = Some(match (w, sp.strip_suffix(" GT/s")) {
                        (Some(w), Some(g)) => link_line(w, g),
                        _ => sp,
                    });
                }
                d.controller = par.and_then(|p| s(p, "_name"));
                out.push(d);
            }
        }
        for vol in v["SPStorageDataType"].as_array().into_iter().flatten() {
            let dev = vol.get("physical_drive").and_then(|p| s(p, "device_name"));
            let (Some(dev), Some(mp)) = (dev, s(vol, "mount_point")) else { continue };
            if let Some(d) = out.iter_mut().find(|d| d.name == dev) {
                let n = |k: &str| vol.get(k).and_then(Value::as_u64).unwrap_or(0);
                d.volumes.push(Volume { name: mp, total: n("size_in_bytes"), free: n("free_space_in_bytes") });
            }
        }
        out
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn probe(_: &crate::Ctx) -> Vec<super::Drive> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        let v = Volume { name: "c:\\".into(), total: 239_002_013_696, free: 17_208_144_896 };
        assert_eq!(volume_line(&v), "c:\\, 222.6 GBytes (7.2 percent available)");
        assert_eq!(gib(240_057_409_536), "223.6");
        assert_eq!(bus_name(17), "NVMe");
        assert_eq!(bus_name(99), "Unknown");
        assert_eq!(controller_class(0x010802), Some("NVM Express (NVMe) Controller"));
        assert_eq!(controller_class(0x010601), Some("Serial ATA Controller - AHCI"));
        assert_eq!(link_line(2, "8.0"), "PCI-E 2x @ 8.0 GT/s");
    }

    #[test]
    fn ata() {
        let mut id = vec![0u8; 512];
        let mut set = |w: usize, v: u16| id[w * 2..w * 2 + 2].copy_from_slice(&v.to_le_bytes());
        set(217, 7200);
        set(83, 0x4000);
        set(82, 1);
        assert_eq!(ata_identify(&id), (Some(7200), vec!["SMART".to_string()]));
        assert_eq!(ata_identify(&[0u8; 10]).0, None);
        id[46..54].copy_from_slice(b"100.A110");
        assert_eq!(ata_string(&id, 23, 27).as_deref(), Some("01.01A01"));
    }

    #[test]
    fn sysfs_paths() {
        let p = "/sys/devices/pci0000:00/0000:00:01.1/0000:02:00.0/nvme/nvme0/nvme0n1";
        assert_eq!(last_bdf(p), Some("0000:02:00.0"));
        assert_eq!(linux_bus(p), 17);
        let s = "/sys/devices/pci0000:00/0000:00:17.0/ata3/host2/target2:0:0/2:0:0:0/block/sda";
        assert_eq!(linux_bus(s), 11);
        assert_eq!(last_bdf("/sys/devices/virtual/block/x"), None);
        assert_eq!(unescape_mount("/mnt/my\\040disk"), "/mnt/my disk");
    }

    #[test]
    fn health() {
        let mut l = vec![0u8; 512];
        l[1..3].copy_from_slice(&310u16.to_le_bytes());
        l[5] = 3;
        l[128] = 100;
        let h = nvme_health(&l);
        assert_eq!(h[0].1, "37 C");
        assert_eq!(h[2].1, "100");
    }
}

//! Windows: SetupAPI (PCI IDs, location, link), DXGI (names, VRAM, LUID),
//! D3DKMT (WDDM version), registry (driver, EDID), plus NVML, NVAPI and ADL
//! loaded at run time. Raw FFI against the public Windows SDK headers.

use super::dl::{Lib, sym};
use super::{Adapter, Link, Live, Perf, Probe, edid};
use crate::hw::Bdf;
use crate::os::win::{reg_str, reg_u64};
use crate::report::{f, gv};
use std::ffi::c_void;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct Guid(pub u32, pub u16, pub u16, pub [u8; 8]);

/// Method `i` of a COM object's vtable.
///
/// # Safety
/// `obj` must be a live COM interface pointer and `F` the method's exact signature.
pub unsafe fn vt<F: Copy>(obj: *mut c_void, i: usize) -> F {
    unsafe {
        let table = *(obj as *const *const usize);
        std::mem::transmute_copy(&*table.add(i))
    }
}

/// # Safety
/// `obj` must be a live COM interface pointer (or null).
pub unsafe fn release(obj: *mut c_void) {
    if !obj.is_null() {
        unsafe { vt::<unsafe extern "system" fn(*mut c_void) -> u32>(obj, 2)(obj) };
    }
}

pub fn wstr(b: &[u16]) -> String {
    String::from_utf16_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]).trim().to_string()
}

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

// ---- SetupAPI / cfgmgr32 ----

#[repr(C)]
struct DevInfo {
    cb: u32,
    class: Guid,
    devinst: u32,
    reserved: usize,
}

#[repr(C)]
struct PropKey {
    fmtid: Guid,
    pid: u32,
}

#[link(name = "setupapi")]
unsafe extern "system" {
    fn SetupDiGetClassDevsW(class: *const Guid, en: *const u16, hwnd: isize, flags: u32) -> isize;
    fn SetupDiEnumDeviceInfo(h: isize, i: u32, d: *mut DevInfo) -> i32;
    fn SetupDiGetDeviceRegistryPropertyW(
        h: isize,
        d: *const DevInfo,
        p: u32,
        ty: *mut u32,
        buf: *mut u8,
        sz: u32,
        req: *mut u32,
    ) -> i32;
    fn SetupDiGetDevicePropertyW(
        h: isize,
        d: *const DevInfo,
        k: *const PropKey,
        ty: *mut u32,
        buf: *mut u8,
        sz: u32,
        req: *mut u32,
        fl: u32,
    ) -> i32;
    fn SetupDiDestroyDeviceInfoList(h: isize) -> i32;
}

#[link(name = "cfgmgr32")]
unsafe extern "system" {
    fn CM_Get_Parent(out: *mut u32, dn: u32, fl: u32) -> u32;
    fn CM_Get_DevNode_Registry_PropertyW(
        dn: u32,
        p: u32,
        ty: *mut u32,
        buf: *mut c_void,
        len: *mut u32,
        fl: u32,
    ) -> u32;
}

const DISPLAY_CLASS: Guid = Guid(0x4d36e968, 0xe325, 0x11ce, [0xbf, 0xc1, 0x08, 0x00, 0x2b, 0xe1, 0x03, 0x18]);
// pciprop.h: DEVPKEY_PciDevice_{Current,Max}Link{Speed,Width}
const PCI_PROPS: Guid = Guid(0x3ab22e31, 0x8264, 0x4b4e, [0x9a, 0xf5, 0xa8, 0xd2, 0xd8, 0xe3, 0x3e, 0x62]);

struct PnpDev {
    hwid: String,
    desc: String,
    driver_key: Option<String>,
    bdf: Option<Bdf>,
    root: Option<Bdf>,
    link: Link,
    link_max: Link,
}

fn dev_prop(h: isize, d: &DevInfo, p: u32) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; 1024];
    let (mut ty, mut req) = (0, 0);
    (unsafe { SetupDiGetDeviceRegistryPropertyW(h, d, p, &mut ty, buf.as_mut_ptr(), 1024, &mut req) } != 0).then(|| {
        buf.truncate(req as usize);
        buf
    })
}

fn u16s(b: &[u8]) -> Vec<u16> {
    b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect()
}

fn le32(b: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(..4)?.try_into().ok()?))
}

fn cm_u32(dn: u32, p: u32) -> Option<u32> {
    let (mut ty, mut v, mut len) = (0u32, 0u32, 4u32);
    (unsafe { CM_Get_DevNode_Registry_PropertyW(dn, p, &mut ty, (&mut v as *mut u32).cast(), &mut len, 0) } == 0)
        .then_some(v)
}

/// SPDRP_BUSNUMBER/SPDRP_ADDRESS (CM_DRP_* = SPDRP_* + 1) → BDF.
fn cm_bdf(dn: u32) -> Option<Bdf> {
    let bus = cm_u32(dn, 0x16)?;
    let addr = cm_u32(dn, 0x1D)?;
    Some(Bdf { seg: 0, bus: bus as u8, dev: (addr >> 16) as u8, func: addr as u8 })
}

fn link_speed(code: u32) -> Option<f64> {
    super::gen_speed(code)
}

fn display_devices() -> Vec<PnpDev> {
    let mut out = Vec::new();
    unsafe {
        let h = SetupDiGetClassDevsW(&DISPLAY_CLASS, std::ptr::null(), 0, 2 /* DIGCF_PRESENT */);
        if h == -1 {
            return out;
        }
        for i in 0.. {
            let mut d =
                DevInfo { cb: std::mem::size_of::<DevInfo>() as u32, class: DISPLAY_CLASS, devinst: 0, reserved: 0 };
            if SetupDiEnumDeviceInfo(h, i, &mut d) == 0 {
                break;
            }
            let s = |p| dev_prop(h, &d, p).map(|b| wstr(&u16s(&b))).unwrap_or_default();
            let pk = |pid| {
                let k = PropKey { fmtid: PCI_PROPS, pid };
                let (mut ty, mut v, mut req) = (0u32, 0u32, 0u32);
                (SetupDiGetDevicePropertyW(h, &d, &k, &mut ty, (&mut v as *mut u32).cast(), 4, &mut req, 0) != 0)
                    .then_some(v)
            };
            let mut parent = 0;
            let root = (CM_Get_Parent(&mut parent, d.devinst, 0) == 0).then(|| cm_bdf(parent)).flatten();
            let hwid = s(1); // SPDRP_HARDWAREID (first string of the multi-sz)
            out.push(PnpDev {
                bdf: if hwid.starts_with("PCI\\") { cm_bdf(d.devinst) } else { None },
                root: if hwid.starts_with("PCI\\") { root } else { None },
                desc: s(0),
                driver_key: Some(s(9)).filter(|k| !k.is_empty()),
                link: Link { speed: pk(9).and_then(link_speed), width: pk(10) },
                link_max: Link { speed: pk(11).and_then(link_speed), width: pk(12) },
                hwid,
            });
        }
        SetupDiDestroyDeviceInfoList(h);
    }
    out
}

/// "PCI\VEN_10DE&DEV_2882&SUBSYS_51611462&REV_A1" → (ven, dev, subsys, rev)
fn parse_hwid(id: &str) -> Option<(u16, u16, u32, u8)> {
    let part = |k: &str| {
        let i = id.find(k)? + k.len();
        id.get(i..).map(|s| s.split('&').next().unwrap_or(""))
    };
    let hex = |k: &str| part(k).and_then(|s| u32::from_str_radix(s, 16).ok());
    Some((hex("VEN_")? as u16, hex("DEV_")? as u16, hex("SUBSYS_").unwrap_or(0), hex("REV_").unwrap_or(0) as u8))
}

// ---- DXGI ----

#[repr(C)]
struct Desc1 {
    desc: [u16; 128],
    vendor: u32,
    device: u32,
    subsys: u32,
    rev: u32,
    dedicated_video: usize,
    dedicated_system: usize,
    shared_system: usize,
    luid: [u32; 2],
    flags: u32,
}

#[link(name = "dxgi")]
unsafe extern "system" {
    fn CreateDXGIFactory1(iid: *const Guid, out: *mut *mut c_void) -> i32;
}

const IID_FACTORY1: Guid = Guid(0x770aae78, 0xf26f, 0x4dba, [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87]);

fn dxgi_adapters() -> Option<Vec<Desc1>> {
    let mut out = Vec::new();
    unsafe {
        let mut fac = std::ptr::null_mut();
        if CreateDXGIFactory1(&IID_FACTORY1, &mut fac) < 0 || fac.is_null() {
            return None;
        }
        type Enum = unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32;
        type GetDesc1 = unsafe extern "system" fn(*mut c_void, *mut Desc1) -> i32;
        for i in 0..16 {
            let mut ad = std::ptr::null_mut();
            if vt::<Enum>(fac, 12)(fac, i, &mut ad) < 0 || ad.is_null() {
                break;
            }
            let mut d: Desc1 = std::mem::zeroed();
            if vt::<GetDesc1>(ad, 10)(ad, &mut d) >= 0 && d.flags & 2 == 0 {
                out.push(d); // skip DXGI_ADAPTER_FLAG_SOFTWARE
            }
            release(ad);
        }
        release(fac);
    }
    Some(out)
}

// ---- D3DKMT (WDDM version) ----

#[repr(C)]
struct OpenLuid {
    luid: [u32; 2],
    h: u32,
}
#[repr(C)]
struct QueryInfo {
    h: u32,
    ty: u32,
    data: *mut c_void,
    size: u32,
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn D3DKMTOpenAdapterFromLuid(p: *mut OpenLuid) -> i32;
    fn D3DKMTQueryAdapterInfo(p: *const QueryInfo) -> i32;
    fn D3DKMTCloseAdapter(p: *const u32) -> i32;
}

fn wddm(luid: [u32; 2]) -> Option<String> {
    unsafe {
        let mut o = OpenLuid { luid, h: 0 };
        if D3DKMTOpenAdapterFromLuid(&mut o) != 0 {
            return None;
        }
        let mut v = 0u32;
        // KMTQAITYPE_DRIVERVERSION → D3DKMT_DRIVERVERSION (e.g. 3200 = WDDM 3.2)
        let q = QueryInfo { h: o.h, ty: 13, data: (&mut v as *mut u32).cast(), size: 4 };
        let ok = D3DKMTQueryAdapterInfo(&q) == 0;
        D3DKMTCloseAdapter(&o.h);
        (ok && v >= 1000).then(|| format!("{}.{}", v / 1000, v % 1000 / 100))
    }
}

// ---- Monitors (EnumDisplayDevices + registry EDID) ----

#[repr(C)]
struct DisplayDevice {
    cb: u32,
    name: [u16; 32],
    string: [u16; 128],
    flags: u32,
    id: [u16; 128],
    key: [u16; 128],
}

#[link(name = "user32")]
unsafe extern "system" {
    fn EnumDisplayDevicesW(dev: *const u16, i: u32, dd: *mut DisplayDevice, fl: u32) -> i32;
}

fn dd() -> DisplayDevice {
    // SAFETY: plain data.
    let mut d: DisplayDevice = unsafe { std::mem::zeroed() };
    d.cb = std::mem::size_of::<DisplayDevice>() as u32;
    d
}

/// (primary adapter PnP ID, active monitors' EDIDs)
fn displays() -> (Option<String>, Vec<Vec<u8>>) {
    let (mut primary, mut edids) = (None, Vec::new());
    for i in 0..32 {
        let mut a = dd();
        if unsafe { EnumDisplayDevicesW(std::ptr::null(), i, &mut a, 0) } == 0 {
            break;
        }
        if a.flags & 4 != 0 {
            primary = Some(wstr(&a.id)); // DISPLAY_DEVICE_PRIMARY_DEVICE
        }
        for j in 0..8 {
            let mut m = dd();
            if unsafe {
                EnumDisplayDevicesW(a.name.as_ptr(), j, &mut m, 1 /* EDD_GET_DEVICE_INTERFACE_NAME */)
            } == 0
            {
                break;
            }
            if m.flags & 1 == 0 {
                continue; // DISPLAY_DEVICE_ACTIVE
            }
            // \\?\DISPLAY#AOC2702#5&b30cc9f&0&UID4357#{e6f07b5f-...}
            let id = wstr(&m.id);
            let p: Vec<&str> = id.split('#').collect();
            if p.len() < 3 {
                continue;
            }
            let key = format!("SYSTEM\\CurrentControlSet\\Enum\\DISPLAY\\{}\\{}\\Device Parameters", p[1], p[2]);
            if let Some((_, e)) = crate::os::win::reg_raw(&key, "EDID")
                && !edids.contains(&e)
            {
                edids.push(e);
            }
        }
    }
    (primary, edids)
}

// ---- NVAPI (public NVAPI SDK, MIT; function IDs from nvapi_interface.h) ----
// The RAM maker/type, tach and thermal-sensor calls are not in the public
// header; their QueryInterface IDs and layouts are the ones documented by
// open-source NVAPI bindings and monitors. All are optional.

type NvU32Fn = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;

struct NvGpu {
    h: *mut c_void,
    bus: u32,
    /// Sensors NvAPI_GPU_ThermalGetSensors accepts (probed once), 0 = none.
    thermal_mask: u32,
}

struct NvApi {
    _lib: Lib,
    gpus: Vec<NvGpu>,
    clocks: Option<unsafe extern "C" fn(*mut c_void, *mut NvClocks) -> i32>,
    arch: Option<unsafe extern "C" fn(*mut c_void, *mut [u32; 4]) -> i32>,
    ram_maker: Option<NvU32Fn>,
    ram_type: Option<NvU32Fn>,
    tach: Option<NvU32Fn>,
    thermal: Option<unsafe extern "C" fn(*mut c_void, *mut NvThermal) -> i32>,
}
unsafe impl Send for NvApi {}
unsafe impl Sync for NvApi {}

/// NvAPI_GPU_ThermalGetSensors, struct version 2: temperatures in 1/256 °C.
#[repr(C)]
struct NvThermal {
    version: u32,
    mask: u32,
    reserved: [i32; 8],
    temps: [i32; 32],
}

impl NvThermal {
    fn new(mask: u32) -> NvThermal {
        NvThermal { version: std::mem::size_of::<NvThermal>() as u32 | 2 << 16, mask, reserved: [0; 8], temps: [0; 32] }
    }
}

/// NV_GPU_RAM_MAKER values.
const RAM_MAKERS: [&str; 11] =
    ["", "Samsung", "Qimonda", "Elpida", "Etron", "Nanya", "SK Hynix", "Mosel", "Winbond", "Elite", "Micron"];
/// NV_GPU_RAM_TYPE values as far as they are documented (newer types come from the GPU table).
const RAM_TYPES: [&str; 11] =
    ["", "SDRAM", "DDR", "DDR2", "GDDR2", "GDDR3", "GDDR4", "DDR3", "GDDR5", "LPDDR2", "GDDR5X"];

#[repr(C)]
struct NvClocks {
    version: u32,
    clock_type: u32,        // ClockType:4, reserved bits
    domain: [[u32; 2]; 32], // {bIsPresent:1, frequency kHz}
}

fn nvapi() -> Option<&'static NvApi> {
    static N: OnceLock<Option<NvApi>> = OnceLock::new();
    N.get_or_init(|| {
        let lib = Lib::open(&["nvapi64.dll", "nvapi.dll"])?;
        let qi: unsafe extern "C" fn(u32) -> *mut c_void = sym!(lib, "nvapi_QueryInterface")?;
        macro_rules! q {
            ($id:expr) => {{
                let p = unsafe { qi($id) };
                (!p.is_null()).then(|| unsafe { std::mem::transmute_copy(&p) })
            }};
        }
        let init: unsafe extern "C" fn() -> i32 = q!(0x0150_E828)?;
        if unsafe { init() } != 0 {
            return None;
        }
        let enumg: unsafe extern "C" fn(*mut [*mut c_void; 64], *mut u32) -> i32 = q!(0xE5AC_921F)?;
        let busid: unsafe extern "C" fn(*mut c_void, *mut u32) -> i32 = q!(0x1BE0_B8E5)?;
        let mut hs = [std::ptr::null_mut(); 64];
        let mut n = 0;
        if unsafe { enumg(&mut hs, &mut n) } != 0 {
            return None;
        }
        let thermal: Option<unsafe extern "C" fn(*mut c_void, *mut NvThermal) -> i32> = q!(0x65FE_3AAD);
        let gpus = hs[..(n as usize).min(64)]
            .iter()
            .filter_map(|&h| {
                let mut b = 0;
                (unsafe { busid(h, &mut b) } == 0).then(|| NvGpu { h, bus: b, thermal_mask: thermal_mask(thermal, h) })
            })
            .collect();
        Some(NvApi {
            gpus,
            clocks: q!(0xDCB6_16C3),
            arch: q!(0xD826_5D24),
            ram_maker: q!(0x42AE_A16A),
            ram_type: q!(0x57F7_CAAC),
            tach: q!(0x5F60_8315),
            thermal,
            _lib: lib,
        })
    })
    .as_ref()
}

/// The driver rejects a mask naming a sensor the GPU lacks, so grow it one
/// bit at a time; the accepted bits are contiguous from bit 0.
fn thermal_mask(f: Option<unsafe extern "C" fn(*mut c_void, *mut NvThermal) -> i32>, h: *mut c_void) -> u32 {
    let Some(f) = f else { return 0 };
    let n = (0..32).take_while(|&b| unsafe { f(h, &mut NvThermal::new(1 << b)) } == 0).count();
    ((1u64 << n) - 1) as u32
}

impl NvApi {
    fn u32_of(f: Option<NvU32Fn>, h: *mut c_void) -> Option<u32> {
        let mut v = 0;
        (unsafe { f?(h, &mut v) } == 0).then_some(v)
    }

    /// Raw thermal-sensor slots in °C (0 = slot not reported).
    fn sensors(&self, g: &NvGpu) -> Option<[f64; 32]> {
        let mut t = NvThermal::new(g.thermal_mask);
        (g.thermal_mask != 0 && unsafe { self.thermal?(g.h, &mut t) } == 0).then(|| t.temps.map(|v| v as f64 / 256.0))
    }

    /// (graphics MHz, memory MHz) for clock type 0 current / 1 base / 2 boost.
    fn clocks(&self, h: *mut c_void, ty: u32) -> Option<(Option<f64>, Option<f64>)> {
        let f = self.clocks?;
        for ver in [3u32, 2] {
            let mut c = NvClocks {
                version: std::mem::size_of::<NvClocks>() as u32 | ver << 16,
                clock_type: ty,
                domain: [[0; 2]; 32],
            };
            if unsafe { f(h, &mut c) } == 0 {
                let d =
                    |i: usize| (c.domain[i][0] & 1 != 0 && c.domain[i][1] != 0).then(|| c.domain[i][1] as f64 / 1000.0);
                return Some((d(0), d(4)));
            }
        }
        None
    }

    /// NV_GPU_ARCH_INFO_V2: (architecture | implementation id)
    fn arch(&self, h: *mut c_void) -> Option<u32> {
        let mut a = [16u32 | 2 << 16, 0, 0, 0];
        (unsafe { self.arch?(h, &mut a) } == 0).then_some(a[1] | a[2])
    }
}

fn nv_gpu<'a>(n: &'a NvApi, a: &Adapter) -> Option<&'a NvGpu> {
    n.gpus.iter().find(|g| a.pci.is_some_and(|p| p.bus as u32 == g.bus))
}

/// Hot spot and memory junction slots of the thermal-sensor array per
/// generation (Ada: 1 and 7, Blackwell: none and 2, older: 1 and 9).
fn thermal_slots(codename: &str) -> (Option<usize>, usize) {
    match codename.get(..2) {
        Some("AD") => (Some(1), 7),
        Some("GB") => (None, 2),
        _ => (Some(1), 9),
    }
}

fn plausible(t: f64) -> Option<f64> {
    (t > 0.0 && t <= 150.0).then_some(t)
}

fn apply_nvapi(a: &mut Adapter, n: &NvApi) {
    let Some(g) = nv_gpu(n, a) else { return };
    let h = g.h;
    if let Some(v) = NvApi::u32_of(n.ram_maker, h).and_then(|m| RAM_MAKERS.get(m as usize)).filter(|v| !v.is_empty()) {
        a.mem_vendor = Some(v.to_string());
    }
    if a.mem_type.is_none() {
        a.mem_type = NvApi::u32_of(n.ram_type, h)
            .and_then(|t| RAM_TYPES.get(t as usize))
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string());
    }
    for (ty, level) in [(1, "Base"), (2, "Boost"), (0, "Current")] {
        if let Some((core, mem)) = n.clocks(h, ty)
            && (core.is_some() || mem.is_some())
        {
            a.perf.push(Perf { level, core, shader: None, mem });
        }
    }
    if let (Some(id), Some(s)) = (n.arch(h), super::db::lookup(a.vendor_id, a.device_id, a.rev)) {
        let chip = s.codename.split('-').next().unwrap_or(s.codename);
        let step = format!("{:X}", a.rev).chars().next().unwrap_or('A');
        a.core_family = Some(format!("0x{id:X} ({chip}-{step})"));
    }
}

// ---- AMD Display Library (public ADL SDK, MIT) ----

#[repr(C)]
struct AdlInfo {
    size: i32,
    index: i32,
    udid: [u8; 256],
    bus: i32,
    dev: i32,
    func: i32,
    vendor: i32,
    name: [u8; 256],
    display: [u8; 256],
    present: i32,
    exist: i32,
    path: [u8; 256],
    path_ext: [u8; 256],
    pnp: [u8; 256],
    os_index: i32,
}

#[repr(C)]
struct PmLog {
    size: i32,
    sensors: [[i32; 2]; 256], // {supported, value}
}

struct Adl {
    _lib: Lib,
    ctx: *mut c_void,
    gpus: Vec<(i32, i32)>, // (adapter index, bus)
    pmlog: Option<unsafe extern "C" fn(*mut c_void, i32, *mut PmLog) -> i32>,
}
unsafe impl Send for Adl {}
unsafe impl Sync for Adl {}

extern "system" fn adl_alloc(n: i32) -> *mut c_void {
    // ponytail: leaked; ADL only calls this for the adapter list at start-up.
    Box::leak(vec![0u8; n.max(0) as usize].into_boxed_slice()).as_mut_ptr().cast()
}

fn adl() -> Option<&'static Adl> {
    static A: OnceLock<Option<Adl>> = OnceLock::new();
    A.get_or_init(|| {
        let lib = Lib::open(&["atiadlxx.dll", "atiadlxy.dll"])?;
        type Create = unsafe extern "C" fn(extern "system" fn(i32) -> *mut c_void, i32, *mut *mut c_void) -> i32;
        let create: Create = sym!(lib, "ADL2_Main_Control_Create")?;
        let count: unsafe extern "C" fn(*mut c_void, *mut i32) -> i32 = sym!(lib, "ADL2_Adapter_NumberOfAdapters_Get")?;
        let info: unsafe extern "C" fn(*mut c_void, *mut AdlInfo, i32) -> i32 =
            sym!(lib, "ADL2_Adapter_AdapterInfo_Get")?;
        let mut ctx = std::ptr::null_mut();
        if unsafe { create(adl_alloc, 1, &mut ctx) } != 0 {
            return None;
        }
        let mut n = 0;
        unsafe { count(ctx, &mut n) };
        let n = n.clamp(0, 64) as usize;
        let mut v: Vec<AdlInfo> = (0..n).map(|_| unsafe { std::mem::zeroed() }).collect();
        let sz = (n * std::mem::size_of::<AdlInfo>()) as i32;
        let mut gpus = Vec::new();
        if n > 0 && unsafe { info(ctx, v.as_mut_ptr(), sz) } == 0 {
            for a in &v {
                if !gpus.iter().any(|g: &(i32, i32)| g.1 == a.bus) {
                    gpus.push((a.index, a.bus));
                }
            }
        }
        Some(Adl { pmlog: sym!(lib, "ADL2_New_QueryPMLogData_Get"), ctx, gpus, _lib: lib })
    })
    .as_ref()
}

// ADL_PMLOG_SENSORS: CLK_GFXCLK 1, CLK_MEMCLK 2, CLK_SOCCLK 3, TEMPERATURE_EDGE 8,
// TEMPERATURE_MEM 9, FAN_RPM 14, FAN_PERCENTAGE 15, INFO_ACTIVITY_GFX 19,
// INFO_ACTIVITY_MEM 20, ASIC_POWER 23, TEMPERATURE_HOTSPOT 27, TEMPERATURE_GFX 28.
impl Adl {
    fn sensors(&self, bus: u32) -> Option<Vec<(usize, i32)>> {
        let &(idx, _) = self.gpus.iter().find(|g| g.1 as u32 == bus)?;
        // SAFETY: plain data, filled by ADL.
        let mut p: PmLog = unsafe { std::mem::zeroed() };
        p.size = std::mem::size_of::<PmLog>() as i32;
        (unsafe { self.pmlog?(self.ctx, idx, &mut p) } == 0)
            .then(|| p.sensors.iter().enumerate().filter(|(_, s)| s[0] != 0).map(|(i, s)| (i, s[1])).collect())
    }
}

fn sensor(s: &[(usize, i32)], id: usize) -> Option<f64> {
    s.iter().find(|x| x.0 == id).map(|x| x.1 as f64)
}

// ---- probe ----

pub fn probe(use_api: bool) -> Probe {
    let mut p = Probe::default();
    let dxgi = dxgi_adapters();
    if dxgi.is_some() {
        p.apis.push(f("API", "D3D"));
    }
    let dxgi = dxgi.unwrap_or_default();
    let (primary, edids) = displays();
    let devs = display_devices();
    let class = "SYSTEM\\CurrentControlSet\\Control\\Class\\";

    for d in &devs {
        let Some((ven, dev, subsys, rev)) = parse_hwid(&d.hwid).filter(|_| d.bdf.is_some()) else { continue };
        let mut a = Adapter {
            name: d.desc.clone(),
            vendor_id: ven,
            device_id: dev,
            subsys: (subsys as u16, (subsys >> 16) as u16),
            rev,
            pci: d.bdf,
            root: d.root,
            link: d.link,
            link_max: d.link_max,
            ..Default::default()
        };
        if let Some(x) = dxgi
            .iter()
            .find(|x| x.vendor == ven as u32 && x.device == dev as u32 && x.subsys == subsys && x.rev == rev as u32)
        {
            let n = wstr(&x.desc);
            if !n.is_empty() {
                a.name = n;
            }
            a.mem_size_mb = Some(x.dedicated_video as u64 >> 20);
            a.driver_model = wddm(x.luid);
        }
        if let Some(k) = &d.driver_key {
            a.driver_version = reg_str(&format!("{class}{k}"), "DriverVersion");
            // The driver's own figure; DXGI reports a little less (reserved memory).
            if let Some(b) = reg_u64(&format!("{class}{k}"), "HardwareInformation.qwMemorySize") {
                a.mem_size_mb = Some(b >> 20);
            }
        }
        // EnumDisplayDevices IDs are "PCI\VEN_xxxx&DEV_xxxx&SUBSYS_xxxxxxxx&REV_xx".
        a.primary =
            primary.as_deref().is_some_and(|id| d.hwid.to_ascii_uppercase().starts_with(&id.to_ascii_uppercase()));
        p.adapters.push(a);
    }

    if use_api {
        // NVAPI first (base/boost clocks); NVML then fills what is still missing.
        let nv = nvapi();
        if let Some(n) = nv {
            for a in p.adapters.iter_mut().filter(|a| a.vendor_id == 0x10DE) {
                apply_nvapi(a, n);
            }
        }
        super::apply_nvml(&mut p.adapters, &mut p.apis);
        if nv.is_some() {
            p.apis.push(f("API", "NVIDIA NVAPI"));
        }
        if let Some(adl) = adl() {
            let mut ids = Vec::new();
            for a in p.adapters.iter_mut().filter(|a| a.vendor_id == 0x1002) {
                let Some(s) = a.pci.and_then(|b| adl.sensors(b.bus as u32)) else { continue };
                ids = s.iter().map(|x| x.0.to_string()).collect();
                // GFXCLK as core, SOCCLK on the "shader" line (CPU-Z's layout), MEMCLK.
                a.perf.push(Perf { level: "Current", core: sensor(&s, 1), shader: sensor(&s, 3), mem: sensor(&s, 2) });
            }
            let list = format!("[{}]", ids.iter().map(|i| format!("{i} ")).collect::<String>());
            p.apis.push(gv("API", "ADL SDK", if ids.is_empty() { vec![] } else { vec![f("sensors", list)] }));
        }
    }

    // The same per-device lines WMI's Win32_VideoController reports, from the registry.
    let keys: Vec<String> = devs.iter().filter_map(|d| d.driver_key.as_ref().map(|k| format!("{class}{k}"))).collect();
    for k in &keys {
        if let Some(m) = reg_u64(k, "HardwareInformation.MemorySize").map(|v| v as u32)
            && m != 0
        {
            p.extra.push(f("Win32_VideoController", format!("AdapterRAM = 0x{m:08X} ({m})")));
        }
    }
    for k in &keys {
        if let Some(v) = reg_str(k, "DriverVersion") {
            p.extra.push(f("Win32_VideoController", format!("DriverVersion = {v}")));
        }
    }
    for k in &keys {
        if let Some(v) = reg_str(k, "DriverDate") {
            let d: Vec<&str> = v.split('-').collect();
            let v = match d[..] {
                [m, dd, y] => format!("{m:0>2}/{dd:0>2}/{y}"),
                _ => v.clone(),
            };
            p.extra.push(f("Win32_VideoController", format!("DriverDate = {v}")));
        }
    }

    p.monitors = edids.iter().filter_map(|e| edid::parse(e)).collect();
    p
}

pub fn live(ads: &[Adapter], use_api: bool) -> Vec<Live> {
    let mut out = Vec::new();
    if !use_api {
        return out;
    }
    super::nvml_live(ads, &mut out);
    if let Some(n) = nvapi() {
        for (i, a) in ads.iter().enumerate().filter(|(_, a)| a.vendor_id == 0x10DE) {
            let Some(g) = nv_gpu(n, a) else { continue };
            if !out.iter().any(|l| l.adapter == i) {
                out.push(Live { adapter: i, ..Default::default() });
            }
            let Some(l) = out.iter_mut().find(|l| l.adapter == i) else { continue };
            if let Some(t) = n.sensors(g) {
                let (hot, mem) = thermal_slots(a.codename.as_deref().unwrap_or(""));
                l.hotspot_c = hot.and_then(|s| plausible(t[s]));
                l.mem_junction_c = plausible(t[mem]);
                l.temp_c = l.temp_c.or(plausible(t[0]));
            }
            if l.fan_rpm.is_none() {
                l.fan_rpm = NvApi::u32_of(n.tach, g.h).filter(|&r| r > 0).map(f64::from);
            }
        }
    }
    if let Some(adl) = adl() {
        for (i, a) in ads.iter().enumerate().filter(|(_, a)| a.vendor_id == 0x1002) {
            let Some(s) = a.pci.and_then(|b| adl.sensors(b.bus as u32)) else { continue };
            let v = |id| sensor(&s, id);
            out.push(Live {
                adapter: i,
                core_mhz: v(1),
                mem_mhz: v(2),
                temp_c: v(28).or(v(8)),
                hotspot_c: v(27),
                mem_junction_c: v(9),
                fan_rpm: v(14).filter(|&r| r > 0.0),
                fan_pct: v(15),
                power_w: v(23),
                load_pct: v(19),
                mem_load_pct: v(20),
                throttle: None,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn hwid() {
        assert_eq!(
            super::parse_hwid("PCI\\VEN_10DE&DEV_2882&SUBSYS_51611462&REV_A1"),
            Some((0x10DE, 0x2882, 0x51611462, 0xA1))
        );
        assert_eq!(super::parse_hwid("ROOT\\BasicDisplay"), None);
    }

    #[test]
    fn nvapi_tables() {
        assert_eq!(std::mem::size_of::<super::NvThermal>(), 168);
        assert_eq!(super::RAM_MAKERS[1], "Samsung");
        assert_eq!(super::thermal_slots("AD107-400"), (Some(1), 7));
        assert_eq!(super::thermal_slots("GB206"), (None, 2));
        assert_eq!(super::thermal_slots(""), (Some(1), 9));
    }

    /// Real NVAPI when present: thermal slots and RAM maker of each GPU.
    #[test]
    fn nvapi_live_probe() {
        let Some(n) = super::nvapi() else { return };
        for g in &n.gpus {
            println!(
                "bus {} mask {:#x} sensors {:?} maker {:?} type {:?} tach {:?}",
                g.bus,
                g.thermal_mask,
                n.sensors(g),
                super::NvApi::u32_of(n.ram_maker, g.h),
                super::NvApi::u32_of(n.ram_type, g.h),
                super::NvApi::u32_of(n.tach, g.h)
            );
        }
    }
}

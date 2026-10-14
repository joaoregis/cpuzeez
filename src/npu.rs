//! AI Devices and NPU Devices sections (FEATURES.md 3.20).
//! Windows: DXCore compute/ML adapters without graphics. Linux: the kernel's
//! compute accelerator class (`/sys/class/accel`). macOS: the Apple Neural Engine.
use crate::Ctx;
use crate::report::{Node, Section, f, g, section};

#[derive(Debug, Clone, Default)]
pub struct Npu {
    pub name: String,
    pub vendor_id: u16,
    pub device_id: u16,
    pub driver: Option<String>,
    pub driver_version: Option<String>,
    pub mem_mb: Option<u64>,
    pub cores: Option<u32>,
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let list = if ctx.cfg.display { devices() } else { Vec::new() };
    let ai = list.iter().map(|n| f("AI Device", &n.name)).collect();
    let npu = list
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mut c = vec![f("Name", &n.name)];
            if n.vendor_id != 0 {
                c.push(f("Vendor", crate::gpu::db::vendor_name(n.vendor_id)));
                c.push(f("Vendor ID", format!("0x{:04X}", n.vendor_id)));
                c.push(f("Model ID", format!("0x{:04X}", n.device_id)));
            }
            let opt = [
                ("Cores", n.cores.map(|v| v.to_string())),
                ("Dedicated memory", n.mem_mb.map(|v| format!("{v} MB"))),
                ("Driver", n.driver.clone()),
                ("Driver version", n.driver_version.clone()),
            ];
            c.extend(opt.into_iter().filter_map(|(k, v)| v.map(|v| f(k, v))));
            g(format!("NPU {i}"), c)
        })
        .collect::<Vec<Node>>();
    vec![section("AI Devices", ai), section("NPU Devices", npu)]
}

#[cfg(windows)]
fn devices() -> Vec<Npu> {
    win::devices()
}

#[cfg(target_os = "linux")]
fn devices() -> Vec<Npu> {
    let Ok(rd) = std::fs::read_dir("/sys/class/accel") else { return Vec::new() };
    let mut v: Vec<Npu> = rd
        .flatten()
        .filter_map(|e| {
            let d = e.path().join("device");
            let id = |n: &str| crate::os::read_u64(d.join(n)).unwrap_or(0) as u16;
            let driver = std::fs::read_link(d.join("driver")).ok()?.file_name()?.to_string_lossy().into_owned();
            let name = match driver.as_str() {
                "intel_vpu" => "Intel AI Boost (NPU)".to_string(),
                "amdxdna" => "AMD XDNA NPU".into(),
                "qaic" => "Qualcomm Cloud AI".into(),
                "habanalabs" => "Intel Gaudi".into(),
                "ethosu" | "rocket" => "Arm/Rockchip NPU".into(),
                d => format!("{d} accelerator"),
            };
            let driver_version = crate::os::read_trim(format!("/sys/module/{driver}/version"));
            Some(Npu {
                name,
                vendor_id: id("vendor"),
                device_id: id("device"),
                driver: Some(driver),
                driver_version,
                ..Default::default()
            })
        })
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

#[cfg(target_os = "macos")]
fn devices() -> Vec<Npu> {
    if !cfg!(target_arch = "aarch64") {
        return Vec::new(); // Intel Macs have no Neural Engine
    }
    // Every Apple M-series SoC has a 16-core Neural Engine; Ultra (two dies) has 32.
    let brand = crate::os::cmd("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_default();
    let cores = if brand.contains("Ultra") { 32 } else { 16 };
    vec![Npu { name: "Apple Neural Engine".into(), vendor_id: 0x106B, cores: Some(cores), ..Default::default() }]
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn devices() -> Vec<Npu> {
    Vec::new()
}

#[cfg(windows)]
mod win {
    use super::Npu;
    use crate::gpu::dl::{Lib, sym};
    use crate::gpu::win::{Guid, release, vt};
    use std::ffi::c_void;

    const IID_FACTORY: Guid = Guid(0x78ee5945, 0xc36e, 0x4b13, [0xa6, 0x69, 0x00, 0x5d, 0xd1, 0x1c, 0x0f, 0x06]);
    const IID_LIST: Guid = Guid(0x526c7776, 0x40e9, 0x459b, [0xb7, 0x11, 0xf3, 0x2a, 0xd7, 0x6d, 0xfc, 0x28]);
    const IID_ADAPTER: Guid = Guid(0xf0db4c7f, 0xfe5a, 0x42a2, [0xbd, 0x62, 0xf2, 0xa6, 0xcf, 0x6f, 0xc8, 0x3e]);
    // DXCORE_ADAPTER_ATTRIBUTE_D3D12_CORE_COMPUTE / D3D12_GENERIC_ML / D3D12_GRAPHICS
    const GENERIC_ML: Guid = Guid(0xb71b0d41, 0x1088, 0x422f, [0xa2, 0x7c, 0x02, 0x50, 0xb7, 0xd3, 0xa9, 0x88]);
    const CORE_COMPUTE: Guid = Guid(0x248e2800, 0xa793, 0x4724, [0xab, 0xaa, 0x23, 0xa6, 0xde, 0x1b, 0xe0, 0x90]);
    const GRAPHICS: Guid = Guid(0x0c9ece4d, 0x2f6e, 0x4f01, [0x8c, 0x96, 0xe8, 0x9e, 0x33, 0x1b, 0x47, 0xb1]);

    type CreateList = unsafe extern "system" fn(*mut c_void, u32, *const Guid, *const Guid, *mut *mut c_void) -> i32;
    type GetAdapter = unsafe extern "system" fn(*mut c_void, u32, *const Guid, *mut *mut c_void) -> i32;
    type Count = unsafe extern "system" fn(*mut c_void) -> u32;
    type IsAttr = unsafe extern "system" fn(*mut c_void, *const Guid) -> bool;
    type GetProp = unsafe extern "system" fn(*mut c_void, u32, usize, *mut c_void) -> i32;
    type PropSize = unsafe extern "system" fn(*mut c_void, u32, *mut usize) -> i32;

    /// DXCoreAdapterProperty value as bytes.
    unsafe fn prop(a: *mut c_void, p: u32) -> Option<Vec<u8>> {
        unsafe {
            let mut n = 0usize;
            if vt::<PropSize>(a, 7)(a, p, &mut n) < 0 || n == 0 || n > 4096 {
                return None;
            }
            let mut b = vec![0u8; n];
            (vt::<GetProp>(a, 6)(a, p, n, b.as_mut_ptr().cast()) >= 0).then_some(b)
        }
    }

    fn u64le(b: &[u8]) -> Option<u64> {
        Some(u64::from_le_bytes(b.get(..8)?.try_into().ok()?))
    }

    pub fn devices() -> Vec<Npu> {
        list(false)
    }

    /// Compute adapters; `all` keeps GPUs too (used by the self-check).
    fn list(all: bool) -> Vec<Npu> {
        let mut out = Vec::new();
        let Some(lib) = Lib::open(&["dxcore.dll"]) else { return out };
        let Some(create): Option<unsafe extern "system" fn(*const Guid, *mut *mut c_void) -> i32> =
            sym!(lib, "DXCoreCreateAdapterFactory")
        else {
            return out;
        };
        unsafe {
            let mut fac = std::ptr::null_mut();
            if create(&IID_FACTORY, &mut fac) < 0 || fac.is_null() {
                return out;
            }
            // NPUs may expose only GENERIC_ML, GPUs and some NPUs CORE_COMPUTE.
            for attr in [CORE_COMPUTE, GENERIC_ML] {
                let mut list = std::ptr::null_mut();
                if vt::<CreateList>(fac, 3)(fac, 1, &attr, &IID_LIST, &mut list) >= 0 && !list.is_null() {
                    for i in 0..vt::<Count>(list, 4)(list).min(32) {
                        let mut a = std::ptr::null_mut();
                        if vt::<GetAdapter>(list, 3)(list, i, &IID_ADAPTER, &mut a) < 0 || a.is_null() {
                            continue;
                        }
                        // IsHardware (11) and no graphics: an NPU / compute-only accelerator.
                        let hw = prop(a, 11).is_some_and(|b| b.first() == Some(&1));
                        if all || (hw && !vt::<IsAttr>(a, 4)(a, &GRAPHICS)) {
                            let name = prop(a, 2)
                                .map(|b| String::from_utf8_lossy(&b).trim_end_matches('\0').trim().to_string());
                            let ids = prop(a, 3).unwrap_or_default(); // DXCoreHardwareID {vendor, device, subsys, rev}
                            let id =
                                |i: usize| ids.get(i * 4..i * 4 + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]));
                            let ver = prop(a, 1).and_then(|b| u64le(&b)).map(|v| {
                                format!("{}.{}.{}.{}", v >> 48, (v >> 32) & 0xFFFF, (v >> 16) & 0xFFFF, v & 0xFFFF)
                            });
                            let n = Npu {
                                name: name.unwrap_or_else(|| "NPU".into()),
                                vendor_id: id(0),
                                device_id: id(1),
                                driver_version: ver,
                                mem_mb: prop(a, 7).and_then(|b| u64le(&b)).map(|v| v >> 20).filter(|&m| m > 0),
                                ..Default::default()
                            };
                            if !out.iter().any(|o: &Npu| {
                                (&o.name, o.vendor_id, o.device_id) == (&n.name, n.vendor_id, n.device_id)
                            }) {
                                out.push(n);
                            }
                        }
                        release(a);
                    }
                    release(list);
                }
            }
            release(fac);
        }
        out
    }

    #[test]
    fn dxcore_vtable() {
        // Any GPU on the box shows up as a compute adapter with its real name.
        for a in list(true) {
            println!("{a:?}");
            assert!(!a.name.is_empty() && a.name != "NPU");
        }
    }
}

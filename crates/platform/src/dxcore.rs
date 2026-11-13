//! DXCore compute adapters (Windows 10 2004+): NPUs and compute-only
//! accelerators, read through the COM vtables of `dxcore.dll`.

use std::ffi::c_void;

/// One adapter with a compute attribute, as DXCore reports it.
#[derive(Debug, Clone, Default)]
pub struct ComputeAdapter {
    /// DriverDescription.
    pub name: Option<String>,
    /// DXCoreHardwareID: vendor, device, subsystem, revision (u32 each).
    pub hardware_id: Vec<u8>,
    /// DriverVersion, as four 16-bit parts high to low.
    pub driver_version: Option<u64>,
    /// DedicatedAdapterMemory, bytes.
    pub dedicated_memory: Option<u64>,
    /// IsHardware.
    pub hardware: bool,
    /// Has the D3D12 graphics attribute (a GPU, not an NPU).
    pub graphics: bool,
}

#[repr(C)]
struct Guid(u32, u16, u16, [u8; 8]);

const IID_FACTORY: Guid = Guid(0x78ee5945, 0xc36e, 0x4b13, [0xa6, 0x69, 0x00, 0x5d, 0xd1, 0x1c, 0x0f, 0x06]);
const IID_LIST: Guid = Guid(0x526c7776, 0x40e9, 0x459b, [0xb7, 0x11, 0xf3, 0x2a, 0xd7, 0x6d, 0xfc, 0x28]);
const IID_ADAPTER: Guid = Guid(0xf0db4c7f, 0xfe5a, 0x42a2, [0xbd, 0x62, 0xf2, 0xa6, 0xcf, 0x6f, 0xc8, 0x3e]);
// DXCORE_ADAPTER_ATTRIBUTE_D3D12_GENERIC_ML / D3D12_CORE_COMPUTE / D3D12_GRAPHICS
const GENERIC_ML: Guid = Guid(0xb71b0d41, 0x1088, 0x422f, [0xa2, 0x7c, 0x02, 0x50, 0xb7, 0xd3, 0xa9, 0x88]);
const CORE_COMPUTE: Guid = Guid(0x248e2800, 0xa793, 0x4724, [0xab, 0xaa, 0x23, 0xa6, 0xde, 0x1b, 0xe0, 0x90]);
const GRAPHICS: Guid = Guid(0x0c9ece4d, 0x2f6e, 0x4f01, [0x8c, 0x96, 0xe8, 0x9e, 0x33, 0x1b, 0x47, 0xb1]);

type Create = unsafe extern "system" fn(*const Guid, *mut *mut c_void) -> i32;
type CreateList = unsafe extern "system" fn(*mut c_void, u32, *const Guid, *const Guid, *mut *mut c_void) -> i32;
type GetAdapter = unsafe extern "system" fn(*mut c_void, u32, *const Guid, *mut *mut c_void) -> i32;
type Count = unsafe extern "system" fn(*mut c_void) -> u32;
type IsAttr = unsafe extern "system" fn(*mut c_void, *const Guid) -> bool;
type GetProp = unsafe extern "system" fn(*mut c_void, u32, usize, *mut c_void) -> i32;
type PropSize = unsafe extern "system" fn(*mut c_void, u32, *mut usize) -> i32;
type Release = unsafe extern "system" fn(*mut c_void) -> u32;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

/// Method `i` of a COM object's vtable.
///
/// # Safety
/// `obj` must be a live COM interface pointer and `F` the method's exact signature.
unsafe fn vt<F: Copy>(obj: *mut c_void, i: usize) -> F {
    // SAFETY: per the contract above.
    unsafe {
        let table = *(obj as *const *const usize);
        std::mem::transmute_copy(&*table.add(i))
    }
}

/// IUnknown::Release.
///
/// # Safety
/// `obj` is null or a live COM interface pointer the caller owns.
unsafe fn release(obj: *mut c_void) {
    if !obj.is_null() {
        // SAFETY: per the contract above; Release is method 2 of every interface.
        unsafe { vt::<Release>(obj, 2)(obj) };
    }
}

/// A DXCoreAdapterProperty value as bytes.
///
/// # Safety
/// `a` is a live IDXCoreAdapter.
unsafe fn prop(a: *mut c_void, p: u32) -> Option<Vec<u8>> {
    // SAFETY: IDXCoreAdapter::GetPropertySize (7) and GetProperty (6); the
    // buffer is the size DXCore reported.
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

/// Every adapter with the core-compute or generic-ML attribute (a device can
/// appear under both; callers dedupe). Empty without DXCore.
pub fn compute_adapters() -> Vec<ComputeAdapter> {
    let mut out = Vec::new();
    let dll: Vec<u16> = "dxcore.dll".encode_utf16().chain([0]).collect();
    // SAFETY: plain loader calls; the module stays loaded for the process.
    let create = unsafe {
        let m = LoadLibraryW(dll.as_ptr());
        if m.is_null() {
            return out;
        }
        let f = GetProcAddress(m, c"DXCoreCreateAdapterFactory".as_ptr().cast());
        if f.is_null() {
            return out;
        }
        std::mem::transmute::<*mut c_void, Create>(f)
    };
    // SAFETY: the documented DXCore interfaces and vtable slots
    // (IDXCoreAdapterFactory, IDXCoreAdapterList, IDXCoreAdapter); every
    // object obtained here is released here.
    unsafe {
        let mut fac = std::ptr::null_mut();
        if create(&IID_FACTORY, &mut fac) < 0 || fac.is_null() {
            return out;
        }
        // NPUs may expose only GENERIC_ML, GPUs and some NPUs CORE_COMPUTE.
        for attr in [CORE_COMPUTE, GENERIC_ML] {
            let mut list = std::ptr::null_mut();
            if vt::<CreateList>(fac, 3)(fac, 1, &attr, &IID_LIST, &mut list) < 0 || list.is_null() {
                continue;
            }
            for i in 0..vt::<Count>(list, 4)(list).min(32) {
                let mut a = std::ptr::null_mut();
                if vt::<GetAdapter>(list, 3)(list, i, &IID_ADAPTER, &mut a) < 0 || a.is_null() {
                    continue;
                }
                out.push(ComputeAdapter {
                    name: prop(a, 2).map(|b| String::from_utf8_lossy(&b).trim_end_matches('\0').trim().to_string()),
                    hardware_id: prop(a, 3).unwrap_or_default(),
                    driver_version: prop(a, 1).and_then(|b| u64le(&b)),
                    dedicated_memory: prop(a, 7).and_then(|b| u64le(&b)),
                    hardware: prop(a, 11).is_some_and(|b| b.first() == Some(&1)),
                    graphics: vt::<IsAttr>(a, 4)(a, &GRAPHICS),
                });
                release(a);
            }
            release(list);
        }
        release(fac);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn dxcore_vtable() {
        // Any GPU on the box shows up as a compute adapter with its real name.
        for a in super::compute_adapters() {
            println!("{a:?}");
            assert!(a.name.as_deref().is_some_and(|n| !n.is_empty()));
        }
    }
}

//! GPU engine load per adapter (S1-D04), no privileges needed: Windows `GPU
//! Engine` performance counters (what the Task Manager shows), Linux amdgpu's
//! `gpu_busy_percent`, macOS IOAccelerator `PerformanceStatistics`. NVIDIA's
//! own load on Linux already comes from NVML.

/// Which adapter a set of engines belongs to.
#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    /// Windows LUID as the counters print it, "0x00000000_0x0000D1A3" (high_low).
    Luid(String),
    /// Linux PCI address, "0000:03:00.0".
    Pci(String),
    /// macOS: IOAccelerator order.
    Index(usize),
}

/// (adapter, [(engine type, load %)]), busiest engine type first.
pub fn read() -> Vec<(Key, Vec<(String, f64)>)> {
    imp::read()
}

/// DXGI's LUID ([low, high]) as the counters print it.
pub fn luid_key(luid: [u32; 2]) -> String {
    format!("0x{:08X}_0x{:08X}", luid[1], luid[0])
}

#[cfg(windows)]
mod imp {
    use super::Key;
    use std::sync::Mutex;
    use windows_sys::Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW,
        PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    };

    struct Query(PDH_HQUERY, PDH_HCOUNTER);
    // SAFETY: PDH handles may be used from any thread; access is serialized by the Mutex.
    unsafe impl Send for Query {}
    static QUERY: Mutex<Option<Query>> = Mutex::new(None);

    pub fn read() -> Vec<(Key, Vec<(String, f64)>)> {
        let mut g = QUERY.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: standard PDH use; the item buffer is sized by PDH itself and
        // its names point into that buffer, which outlives their use.
        unsafe {
            if g.is_none() {
                let (mut q, mut c) = (std::ptr::null_mut(), std::ptr::null_mut());
                let path: Vec<u16> = r"\GPU Engine(*)\Utilization Percentage".encode_utf16().chain([0]).collect();
                if PdhOpenQueryW(std::ptr::null(), 0, &mut q) != 0
                    || PdhAddEnglishCounterW(q, path.as_ptr(), 0, &mut c) != 0
                {
                    return Vec::new();
                }
                PdhCollectQueryData(q); // a rate: the first value needs a second sample
                *g = Some(Query(q, c));
            }
            let Some(Query(q, c)) = g.as_ref() else { return Vec::new() };
            if PdhCollectQueryData(*q) != 0 {
                return Vec::new();
            }
            let (mut size, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(*c, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut())
                != PDH_MORE_DATA
            {
                return Vec::new();
            }
            // u64 elements: the buffer holds the items, then their names.
            let mut buf = vec![0u64; (size as usize).div_ceil(8)];
            let items = buf.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            if PdhGetFormattedCounterArrayW(*c, PDH_FMT_DOUBLE, &mut size, &mut count, items) != 0 {
                return Vec::new();
            }
            let pairs: Vec<(String, f64)> = std::slice::from_raw_parts(items, count as usize)
                .iter()
                .filter(|i| i.FmtValue.CStatus == 0 && !i.szName.is_null())
                .map(|i| {
                    let n = (0..).take_while(|&k| *i.szName.add(k) != 0).count();
                    (
                        String::from_utf16_lossy(std::slice::from_raw_parts(i.szName, n)),
                        i.FmtValue.Anonymous.doubleValue,
                    )
                })
                .collect();
            cpuzeez_core::gpu::engines::aggregate(pairs.iter().map(|(n, v)| (n.as_str(), *v)))
                .into_iter()
                .map(|a| (Key::Luid(a.luid), a.engines))
                .collect()
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::Key;

    pub fn read() -> Vec<(Key, Vec<(String, f64)>)> {
        let Ok(dir) = std::fs::read_dir("/sys/class/drm") else { return Vec::new() };
        let mut v: Vec<(Key, Vec<(String, f64)>)> = dir
            .flatten()
            .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("card") && !n.contains('-')))
            .filter_map(|e| {
                let dev = e.path().join("device");
                let busy: f64 = std::fs::read_to_string(dev.join("gpu_busy_percent")).ok()?.trim().parse().ok()?;
                let pci = std::fs::canonicalize(&dev).ok()?.file_name()?.to_str()?.to_string();
                Some((Key::Pci(pci), vec![("Graphics".to_string(), busy.clamp(0.0, 100.0))]))
            })
            .collect();
        v.sort_by(|a, b| format!("{:?}", a.0).cmp(&format!("{:?}", b.0)));
        v
    }
}

#[cfg(target_os = "macos")]
mod imp {
    #![allow(non_upper_case_globals)]
    use super::Key;
    use std::ffi::{c_char, c_void};

    type CFTypeRef = *const c_void;
    const kCFStringEncodingUTF8: u32 = 0x0800_0100;
    const kCFNumberDoubleType: i32 = 13;

    #[link(name = "IOKit", kind = "framework")]
    unsafe extern "C" {
        fn IOServiceMatching(name: *const c_char) -> *mut c_void;
        fn IOServiceGetMatchingServices(main: u32, matching: *mut c_void, it: *mut u32) -> i32;
        fn IOIteratorNext(it: u32) -> u32;
        fn IOObjectRelease(o: u32) -> i32;
        fn IORegistryEntryCreateCFProperty(e: u32, key: CFTypeRef, alloc: CFTypeRef, opts: u32) -> CFTypeRef;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithCString(alloc: CFTypeRef, s: *const c_char, enc: u32) -> CFTypeRef;
        fn CFDictionaryGetValue(d: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
        fn CFNumberGetValue(n: CFTypeRef, ty: i32, out: *mut c_void) -> bool;
        fn CFRelease(o: CFTypeRef);
    }

    /// (dictionary key, engine label)
    const KEYS: [(&std::ffi::CStr, &str); 3] = [
        (c"Device Utilization %", "Device"),
        (c"Renderer Utilization %", "Renderer"),
        (c"Tiler Utilization %", "Tiler"),
    ];

    pub fn read() -> Vec<(Key, Vec<(String, f64)>)> {
        let mut v = Vec::new();
        let mut it = 0u32;
        // SAFETY: standard IOKit iteration; every object and CF value is released.
        unsafe {
            if IOServiceGetMatchingServices(0, IOServiceMatching(c"IOAccelerator".as_ptr()), &mut it) != 0 {
                return v;
            }
            let stats =
                CFStringCreateWithCString(std::ptr::null(), c"PerformanceStatistics".as_ptr(), kCFStringEncodingUTF8);
            loop {
                let e = IOIteratorNext(it);
                if e == 0 {
                    break;
                }
                let d = IORegistryEntryCreateCFProperty(e, stats, std::ptr::null(), 0);
                if !d.is_null() {
                    let mut engines = Vec::new();
                    for (k, label) in KEYS {
                        let key = CFStringCreateWithCString(std::ptr::null(), k.as_ptr(), kCFStringEncodingUTF8);
                        let n = CFDictionaryGetValue(d, key);
                        let mut x = 0f64;
                        if !n.is_null() && CFNumberGetValue(n, kCFNumberDoubleType, (&mut x as *mut f64).cast()) {
                            engines.push((label.to_string(), x.clamp(0.0, 100.0)));
                        }
                        CFRelease(key);
                    }
                    CFRelease(d);
                    if !engines.is_empty() {
                        v.push((Key::Index(v.len()), engines));
                    }
                }
                IOObjectRelease(e);
            }
            CFRelease(stats);
            IOObjectRelease(it);
        }
        v
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn read() -> Vec<(super::Key, Vec<(String, f64)>)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn luid_prints_like_the_counters() {
        assert_eq!(super::luid_key([0xD1A3, 0]), "0x00000000_0x0000D1A3");
    }

    #[test]
    fn loads_are_percentages() {
        super::read();
        std::thread::sleep(std::time::Duration::from_millis(100));
        for (k, engines) in super::read() {
            for (name, v) in engines {
                assert!((0.0..=100.0).contains(&v), "{k:?} {name} = {v}");
            }
        }
    }
}

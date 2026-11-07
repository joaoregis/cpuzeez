//! Cumulative I/O counters per physical disk (S1-D01), no privileges needed:
//! Windows `IOCTL_DISK_PERFORMANCE` on `\\.\PhysicalDriveN`, Linux
//! `/proc/diskstats`, macOS IOKit `IOBlockStorageDriver` statistics. Rates are
//! `cpuzeez_core::diskio::rates`.

pub use cpuzeez_core::diskio::Counters;

/// (display name, counters) for every disk the OS reports right now.
pub fn read() -> Vec<(String, Counters)> {
    imp::read()
}

#[cfg(target_os = "linux")]
mod imp {
    pub fn read() -> Vec<(String, super::Counters)> {
        std::fs::read_to_string("/proc/diskstats")
            .map(|t| cpuzeez_core::diskio::parse_diskstats(&t))
            .unwrap_or_default()
    }
}

#[cfg(windows)]
mod imp {
    use super::Counters;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{DISK_PERFORMANCE, IOCTL_DISK_PERFORMANCE};

    /// Disk numbers are dense in practice; stop after a run of missing ones.
    const MAX: u32 = 64;
    const GAP: u32 = 8;

    pub fn read() -> Vec<(String, Counters)> {
        let mut v = Vec::new();
        let mut missing = 0;
        for n in 0..MAX {
            match perf(n) {
                Some(c) => {
                    missing = 0;
                    v.push((format!("Disk {n}"), c));
                }
                None => {
                    missing += 1;
                    if missing >= GAP {
                        break;
                    }
                }
            }
        }
        v
    }

    fn perf(n: u32) -> Option<Counters> {
        let path: Vec<u16> = format!(r"\\.\PhysicalDrive{n}").encode_utf16().chain([0]).collect();
        // SAFETY: path is NUL-terminated; access 0 (query only) needs no admin.
        let h = unsafe {
            CreateFileW(
                path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if h == INVALID_HANDLE_VALUE {
            return None;
        }
        // SAFETY: DISK_PERFORMANCE is plain data; the buffer outlives the call.
        let (ok, p) = unsafe {
            let mut p: DISK_PERFORMANCE = std::mem::zeroed();
            let mut got = 0u32;
            let ok = DeviceIoControl(
                h,
                IOCTL_DISK_PERFORMANCE,
                std::ptr::null(),
                0,
                (&mut p as *mut DISK_PERFORMANCE).cast(),
                size_of::<DISK_PERFORMANCE>() as u32,
                &mut got,
                std::ptr::null_mut(),
            );
            CloseHandle(h);
            (ok != 0, p)
        };
        if !ok {
            return None;
        }
        // Times are in 100 ns units; busy = elapsed − idle (a delta of it is busy time).
        let t = |x: i64| x.max(0) as u64 * 100;
        Some(Counters {
            read_bytes: p.BytesRead.max(0) as u64,
            write_bytes: p.BytesWritten.max(0) as u64,
            reads: p.ReadCount as u64,
            writes: p.WriteCount as u64,
            read_ns: t(p.ReadTime),
            write_ns: t(p.WriteTime),
            busy_ns: Some(t(p.QueryTime.saturating_sub(p.IdleTime))),
            queue: Some(p.QueueDepth as f64),
        })
    }
}

#[cfg(target_os = "macos")]
mod imp {
    //! IOKit: every `IOBlockStorageDriver` has a "Statistics" dictionary.
    #![allow(non_upper_case_globals)]
    use super::Counters;
    use std::ffi::{c_char, c_void};

    type CFTypeRef = *const c_void;
    const kCFStringEncodingUTF8: u32 = 0x0800_0100;
    const kCFNumberSInt64Type: i32 = 4;

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

    fn cfstr(s: &std::ffi::CStr) -> CFTypeRef {
        // SAFETY: s is NUL-terminated UTF-8; the caller releases the result.
        unsafe { CFStringCreateWithCString(std::ptr::null(), s.as_ptr(), kCFStringEncodingUTF8) }
    }

    fn number(d: CFTypeRef, key: &std::ffi::CStr) -> u64 {
        let k = cfstr(key);
        let mut v = 0i64;
        // SAFETY: d is a live CFDictionary; the key is released after the lookup.
        unsafe {
            let n = CFDictionaryGetValue(d, k);
            if !n.is_null() {
                CFNumberGetValue(n, kCFNumberSInt64Type, (&mut v as *mut i64).cast());
            }
            CFRelease(k);
        }
        v.max(0) as u64
    }

    pub fn read() -> Vec<(String, Counters)> {
        let mut v = Vec::new();
        let mut it = 0u32;
        // SAFETY: standard IOKit iteration; every object and CF value is released.
        unsafe {
            if IOServiceGetMatchingServices(0, IOServiceMatching(c"IOBlockStorageDriver".as_ptr()), &mut it) != 0 {
                return v;
            }
            let key = cfstr(c"Statistics");
            loop {
                let e = IOIteratorNext(it);
                if e == 0 {
                    break;
                }
                let s = IORegistryEntryCreateCFProperty(e, key, std::ptr::null(), 0);
                if !s.is_null() {
                    v.push((
                        format!("Disk {}", v.len()),
                        Counters {
                            read_bytes: number(s, c"Bytes (Read)"),
                            write_bytes: number(s, c"Bytes (Write)"),
                            reads: number(s, c"Operations (Read)"),
                            writes: number(s, c"Operations (Write)"),
                            read_ns: number(s, c"Total Time (Read)"),
                            write_ns: number(s, c"Total Time (Write)"),
                            busy_ns: None,
                            queue: None,
                        },
                    ));
                    CFRelease(s);
                }
                IOObjectRelease(e);
            }
            CFRelease(key);
            IOObjectRelease(it);
        }
        v
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn read() -> Vec<(String, super::Counters)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn counters_only_grow_between_two_reads() {
        let a = super::read();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let b = super::read();
        for ((na, ca), (nb, cb)) in a.iter().zip(&b) {
            if na == nb {
                assert!(cb.reads >= ca.reads && cb.read_bytes >= ca.read_bytes, "{na}: {ca:?} -> {cb:?}");
            }
        }
    }
}

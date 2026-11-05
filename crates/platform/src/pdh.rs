//! A wildcard PDH counter read as (instance, value) pairs (Windows). Each
//! consumer owns its own query: rate counters measure since that query's
//! previous collection, so two readers can't share one.

/// One `\Object(*)\Counter` query.
pub struct Array {
    #[cfg(windows)]
    q: Option<(isize, isize)>,
}

// SAFETY: PDH handles may be used from any thread; callers keep one owner.
unsafe impl Send for Array {}

impl Array {
    /// The counter at `path` (English names). On other OSes it reads nothing.
    pub fn new(path: &str) -> Array {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Performance::{PdhAddEnglishCounterW, PdhCollectQueryData, PdhOpenQueryW};
            let w: Vec<u16> = path.encode_utf16().chain([0]).collect();
            let (mut q, mut c) = (std::ptr::null_mut(), std::ptr::null_mut());
            // SAFETY: standard PDH setup; the first collect primes rate counters.
            let ok = unsafe {
                PdhOpenQueryW(std::ptr::null(), 0, &mut q) == 0 && PdhAddEnglishCounterW(q, w.as_ptr(), 0, &mut c) == 0
            };
            if ok {
                // SAFETY: our query.
                unsafe { PdhCollectQueryData(q) };
            }
            Array { q: ok.then_some((q as isize, c as isize)) }
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Array {}
        }
    }

    /// (instance, value) pairs since the previous call.
    pub fn read(&mut self) -> Vec<(String, f64)> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Performance::{
                PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MORE_DATA, PdhCollectQueryData,
                PdhGetFormattedCounterArrayW,
            };
            let Some((q, c)) = self.q else { return Vec::new() };
            let (q, c) = (q as *mut core::ffi::c_void, c as *mut core::ffi::c_void);
            // SAFETY: standard PDH use; PDH sizes the buffer, the names point into it.
            unsafe {
                if PdhCollectQueryData(q) != 0 {
                    return Vec::new();
                }
                let (mut size, mut count) = (0u32, 0u32);
                if PdhGetFormattedCounterArrayW(c, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut())
                    != PDH_MORE_DATA
                {
                    return Vec::new();
                }
                let mut buf = vec![0u64; (size as usize).div_ceil(8)];
                let items = buf.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
                if PdhGetFormattedCounterArrayW(c, PDH_FMT_DOUBLE, &mut size, &mut count, items) != 0 {
                    return Vec::new();
                }
                std::slice::from_raw_parts(items, count as usize)
                    .iter()
                    .filter(|i| i.FmtValue.CStatus == 0 && !i.szName.is_null())
                    .map(|i| {
                        let n = (0..).take_while(|&k| *i.szName.add(k) != 0).count();
                        (
                            String::from_utf16_lossy(std::slice::from_raw_parts(i.szName, n)),
                            i.FmtValue.Anonymous.doubleValue,
                        )
                    })
                    .collect()
            }
        }
        #[cfg(not(windows))]
        Vec::new()
    }
}

impl Drop for Array {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some((q, _)) = self.q {
            // SAFETY: our query, closed once.
            unsafe { windows_sys::Win32::System::Performance::PdhCloseQuery(q as *mut core::ffi::c_void) };
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_a_wildcard_counter() {
        let mut a = super::Array::new(r"\Process(*)\Thread Count");
        std::thread::sleep(std::time::Duration::from_millis(50));
        let v = a.read();
        if cfg!(windows) {
            assert!(v.iter().any(|(_, n)| *n > 0.0), "{} instances", v.len());
        } else {
            assert!(v.is_empty());
        }
    }
}

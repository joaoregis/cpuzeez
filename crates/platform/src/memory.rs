//! RAM composition (S1-D03), no privileges needed: Windows
//! `GetPerformanceInfo` + the memory-list performance counters, Linux
//! `/proc/meminfo`, macOS `vm_statistics64` + `vm.swapusage`.

pub use cpuzeez_core::memory::Composition;

pub fn composition() -> Composition {
    imp::composition()
}

/// Hard page faults per second (pages read in from disk to resolve a fault):
/// the rate since the previous call, so the first call has nothing to say.
/// Windows `\Memory\Pages Input/sec`, Linux `pgmajfault` from `/proc/vmstat`.
pub fn hard_faults_per_sec() -> Option<f64> {
    imp::hard_faults_per_sec()
}

/// `pgmajfault` from `/proc/vmstat` text.
pub fn parse_major_faults(vmstat: &str) -> Option<u64> {
    vmstat.lines().find_map(|l| l.strip_prefix("pgmajfault ")?.trim().parse().ok())
}

#[cfg(any(windows, target_os = "macos"))]
const MB: f64 = 1024.0 * 1024.0;

#[cfg(target_os = "linux")]
mod imp {
    use std::sync::Mutex;
    use std::time::Instant;

    pub fn composition() -> super::Composition {
        std::fs::read_to_string("/proc/meminfo").map(|t| cpuzeez_core::memory::parse_meminfo(&t)).unwrap_or_default()
    }

    static LAST: Mutex<Option<(Instant, u64)>> = Mutex::new(None);

    pub fn hard_faults_per_sec() -> Option<f64> {
        let now = super::parse_major_faults(&std::fs::read_to_string("/proc/vmstat").ok()?)?;
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        let prev = last.replace((Instant::now(), now));
        let (at, was) = prev?;
        let dt = at.elapsed().as_secs_f64();
        (dt > 0.2).then(|| now.saturating_sub(was) as f64 / dt)
    }
}

#[cfg(windows)]
mod imp {
    use super::{Composition, MB};
    use std::sync::Mutex;
    use windows_sys::Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW, PdhCollectQueryData,
        PdhGetFormattedCounterValue, PdhOpenQueryW,
    };
    use windows_sys::Win32::System::ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION};

    /// The memory-list counters the Task Manager's memory bar shows. Gauges,
    /// so one collection per read is enough. "Memory Compression" is a
    /// process; it's missing when compression is off.
    const PATHS: [&str; 7] = [
        r"\Memory\Standby Cache Reserve Bytes",
        r"\Memory\Standby Cache Normal Priority Bytes",
        r"\Memory\Standby Cache Core Bytes",
        r"\Memory\Modified Page List Bytes",
        r"\Memory\Free & Zero Page List Bytes",
        r"\Process(Memory Compression)\Working Set - Private",
        r"\Paging File(_Total)\% Usage",
    ];

    struct Query {
        q: PDH_HQUERY,
        counters: [Option<PDH_HCOUNTER>; 7],
    }
    // SAFETY: PDH handles may be used from any thread; access is serialized by the Mutex.
    unsafe impl Send for Query {}

    static QUERY: Mutex<Option<Query>> = Mutex::new(None);

    /// A query of its own for the rate counter: a rate is measured since that
    /// query's previous collection, so it can't share the gauges' query.
    struct Handles(PDH_HQUERY, PDH_HCOUNTER);
    // SAFETY: PDH handles may be used from any thread; access is serialized by the Mutex.
    unsafe impl Send for Handles {}

    pub fn hard_faults_per_sec() -> Option<f64> {
        static H: Mutex<Option<Handles>> = Mutex::new(None);
        let mut g = H.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            let mut q: PDH_HQUERY = std::ptr::null_mut();
            let mut c: PDH_HCOUNTER = std::ptr::null_mut();
            let w: Vec<u16> = r"\Memory\Pages Input/sec".encode_utf16().chain([0]).collect();
            // SAFETY: out-params; the path is a NUL-terminated wide string.
            let ok = unsafe {
                PdhOpenQueryW(std::ptr::null(), 0, &mut q) == 0 && PdhAddEnglishCounterW(q, w.as_ptr(), 0, &mut c) == 0
            };
            if !ok {
                return None;
            }
            // SAFETY: our query; the first collection primes the rate.
            unsafe { PdhCollectQueryData(q) };
            *g = Some(Handles(q, c));
            return None;
        }
        let h = g.as_ref()?;
        // SAFETY: our live query; the value is read into a local.
        unsafe {
            if PdhCollectQueryData(h.0) != 0 {
                return None;
            }
            let mut v = PDH_FMT_COUNTERVALUE::default();
            (PdhGetFormattedCounterValue(h.1, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut v) == 0 && v.CStatus == 0)
                .then_some(v.Anonymous.doubleValue)
        }
    }

    fn open() -> Option<Query> {
        let mut q: PDH_HQUERY = std::ptr::null_mut();
        // SAFETY: out-params; paths are NUL-terminated wide strings.
        unsafe {
            if PdhOpenQueryW(std::ptr::null(), 0, &mut q) != 0 {
                return None;
            }
            let counters = PATHS.map(|p| {
                let w: Vec<u16> = p.encode_utf16().chain([0]).collect();
                let mut c: PDH_HCOUNTER = std::ptr::null_mut();
                (PdhAddEnglishCounterW(q, w.as_ptr(), 0, &mut c) == 0).then_some(c)
            });
            Some(Query { q, counters })
        }
    }

    /// Bytes per counter, None where it's missing or failed.
    fn lists() -> [Option<f64>; 7] {
        let mut g = QUERY.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            *g = open();
        }
        let Some(q) = g.as_ref() else { return [None; 7] };
        // SAFETY: q.q is a live query; values are read into a local.
        unsafe {
            if PdhCollectQueryData(q.q) != 0 {
                return [None; 7];
            }
            q.counters.map(|c| {
                let mut v = PDH_FMT_COUNTERVALUE::default();
                (PdhGetFormattedCounterValue(c?, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut v) == 0 && v.CStatus == 0)
                    .then_some(v.Anonymous.doubleValue)
            })
        }
    }

    pub fn composition() -> Composition {
        let mut p: PERFORMANCE_INFORMATION = unsafe { std::mem::zeroed() };
        p.cb = size_of::<PERFORMANCE_INFORMATION>() as u32;
        // SAFETY: cb set to the struct size.
        if unsafe { GetPerformanceInfo(&mut p, p.cb) } == 0 {
            return Composition::default();
        }
        let pages = |n: usize| Some(n as f64 * p.PageSize as f64 / MB);
        let [reserve, normal, core, modified, free, compressed, pagefile_pct] = lists();
        let [reserve, normal, core, modified, free, compressed] =
            [reserve, normal, core, modified, free, compressed].map(|b| b.map(|b| b / MB));
        let standby = match (reserve, normal, core) {
            (None, None, None) => None,
            (a, b, c) => Some(a.unwrap_or(0.0) + b.unwrap_or(0.0) + c.unwrap_or(0.0)),
        };
        let (total, available) = (pages(p.PhysicalTotal), pages(p.PhysicalAvailable));
        let (commit, commit_limit) = (pages(p.CommitTotal), pages(p.CommitLimit));
        // The commit limit is RAM plus every page file.
        let swap_total = commit_limit.zip(total).map(|(l, t)| (l - t).max(0.0));
        Composition {
            total,
            available,
            used: total.zip(available).map(|(t, a)| t - a),
            free,
            cached: standby,
            modified,
            compressed,
            wired: pages(p.KernelNonpaged),
            paged_pool: pages(p.KernelPaged),
            commit,
            commit_limit,
            swap_used: pagefile_pct.zip(swap_total).map(|(pct, s)| (pct / 100.0 * s).clamp(0.0, s)),
            swap_total,
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{Composition, MB};

    pub fn hard_faults_per_sec() -> Option<f64> {
        None
    }

    pub fn composition() -> Composition {
        let mut total = 0u64;
        let mut len = size_of::<u64>();
        let mut st: libc::vm_statistics64 = unsafe { std::mem::zeroed() };
        let mut n = libc::HOST_VM_INFO64_COUNT;
        let mut swap: libc::xsw_usage = unsafe { std::mem::zeroed() };
        let mut slen = size_of::<libc::xsw_usage>();
        // SAFETY: sysctl/host_statistics64 out-params sized as documented.
        let ok = unsafe {
            #[allow(deprecated)]
            let host = libc::mach_host_self();
            libc::sysctlbyname(
                c"hw.memsize".as_ptr(),
                (&mut total as *mut u64).cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            ) == 0
                && libc::host_statistics64(
                    host,
                    libc::HOST_VM_INFO64,
                    (&mut st as *mut libc::vm_statistics64).cast(),
                    &mut n,
                ) == 0
        };
        if !ok || total == 0 {
            return Composition::default();
        }
        let swap_ok = unsafe {
            libc::sysctlbyname(
                c"vm.swapusage".as_ptr(),
                (&mut swap as *mut libc::xsw_usage).cast(),
                &mut slen,
                std::ptr::null_mut(),
                0,
            ) == 0
        };
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as f64;
        let pg = |c: u32| Some(c as f64 * page / MB);
        let free = pg(st.free_count + st.speculative_count);
        // Activity Monitor: "Cached Files" = file-backed + purgeable pages.
        let cached = pg(st.external_page_count + st.purgeable_count);
        let total = Some(total as f64 / MB);
        let available = free.zip(cached).map(|(f, c)| f + c);
        Composition {
            total,
            available,
            used: total.zip(available).map(|(t, a)| (t - a).max(0.0)),
            free,
            cached,
            modified: None,
            compressed: pg(st.compressor_page_count),
            wired: pg(st.wire_count),
            paged_pool: None,
            commit: None,
            commit_limit: None,
            swap_used: swap_ok.then(|| swap.xsu_used as f64 / MB),
            swap_total: swap_ok.then(|| swap.xsu_total as f64 / MB),
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn hard_faults_per_sec() -> Option<f64> {
        None
    }

    pub fn composition() -> super::Composition {
        super::Composition::default()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parts_fit_in_the_total() {
        let c = super::composition();
        let Some(total) = c.total else { return };
        for (name, v) in [("used", c.used), ("free", c.free), ("cached", c.cached), ("wired", c.wired)] {
            if let Some(v) = v {
                assert!((0.0..=total * 1.01).contains(&v), "{name} = {v} of {total} MB");
            }
        }
    }
}

#[cfg(test)]
mod fault_tests {
    #[test]
    fn major_faults_from_vmstat() {
        let t = "pgfault 1000\npgmajfault 42\npgrefill 1\n";
        assert_eq!(super::parse_major_faults(t), Some(42));
        assert_eq!(super::parse_major_faults("nr_free_pages 3\n"), None);
    }

    /// The rate needs two reads; neither panics on any OS.
    #[test]
    fn hard_fault_rate_after_two_reads() {
        let _ = super::hard_faults_per_sec();
        std::thread::sleep(std::time::Duration::from_millis(300));
        if let Some(v) = super::hard_faults_per_sec() {
            assert!(v >= 0.0 && v.is_finite(), "{v}");
        }
    }
}

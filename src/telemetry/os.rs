//! OS-level telemetry inputs that no hardware monitor covers: per-CPU load,
//! RAM in use, kernel throttle counters and the Ctrl+C flag for `-log=`.
//! Windows load comes from the sensors module's PDH query (one open query per
//! process); Linux reads /proc/stat and /proc/meminfo, macOS uses Mach host
//! statistics.

use std::sync::atomic::{AtomicBool, Ordering};

/// (whole machine %, per logical CPU %).
#[cfg(windows)]
pub fn cpu_load() -> (Option<f64>, Vec<Option<f64>>) {
    crate::sensors::cpu_load()
}

/// (whole machine %, per logical CPU %) from busy/total tick deltas since the
/// previous call (or over 100 ms the first time).
#[cfg(not(windows))]
pub fn cpu_load() -> (Option<f64>, Vec<Option<f64>>) {
    use std::sync::Mutex;
    static LAST: Mutex<Vec<(u64, u64)>> = Mutex::new(Vec::new());
    let Ok(mut last) = LAST.lock() else { return (None, Vec::new()) };
    let Some(mut now) = ticks() else { return (None, Vec::new()) };
    if last.len() != now.len() {
        *last = now;
        std::thread::sleep(std::time::Duration::from_millis(100));
        let Some(n) = ticks() else { return (None, Vec::new()) };
        now = n;
    }
    let pct: Vec<Option<f64>> = now
        .iter()
        .zip(last.iter())
        .map(|(&(b1, t1), &(b0, t0))| {
            let dt = t1.saturating_sub(t0);
            (dt > 0).then(|| (b1.saturating_sub(b0) as f64 * 100.0 / dt as f64).clamp(0.0, 100.0))
        })
        .collect();
    *last = now;
    match pct.split_first() {
        Some((total, per)) => (*total, per.to_vec()),
        None => (None, Vec::new()),
    }
}

/// (busy, total) ticks: whole machine first, then every logical CPU.
#[cfg(target_os = "linux")]
fn ticks() -> Option<Vec<(u64, u64)>> {
    let s = std::fs::read_to_string("/proc/stat").ok()?;
    let v = parse_proc_stat(&s);
    (!v.is_empty()).then_some(v)
}

/// `cpu`/`cpuN` lines: busy = everything but idle + iowait (proc(5)).
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn parse_proc_stat(s: &str) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for l in s.lines().filter(|l| l.starts_with("cpu")) {
        // guest/guest_nice are already counted in user/nice.
        let f: Vec<u64> = l.split_whitespace().skip(1).take(8).filter_map(|x| x.parse().ok()).collect();
        if f.len() < 4 {
            continue;
        }
        let total: u64 = f.iter().sum();
        let idle = f[3] + f.get(4).copied().unwrap_or(0);
        out.push((total - idle, total));
    }
    out
}

#[cfg(target_os = "macos")]
mod mach {
    unsafe extern "C" {
        pub fn mach_host_self() -> u32;
        pub static mach_task_self_: u32;
        pub fn host_processor_info(host: u32, flavor: i32, count: *mut u32, info: *mut *mut i32, n: *mut u32) -> i32;
        pub fn host_statistics64(host: u32, flavor: i32, info: *mut i32, n: *mut u32) -> i32;
        pub fn vm_deallocate(task: u32, addr: usize, size: usize) -> i32;
    }
    pub const PROCESSOR_CPU_LOAD_INFO: i32 = 2;
    pub const HOST_VM_INFO64: i32 = 4;
    /// sizeof(vm_statistics64) / sizeof(integer_t).
    pub const HOST_VM_INFO64_COUNT: u32 = 38;
}

/// host_processor_info(PROCESSOR_CPU_LOAD_INFO): user, system, idle, nice ticks per CPU.
#[cfg(target_os = "macos")]
fn ticks() -> Option<Vec<(u64, u64)>> {
    let (mut cpus, mut info, mut n) = (0u32, std::ptr::null_mut::<i32>(), 0u32);
    // SAFETY: out-params per mach/host_info.h; the kernel-allocated array is
    // copied, then released with vm_deallocate.
    unsafe {
        if mach::host_processor_info(
            mach::mach_host_self(),
            mach::PROCESSOR_CPU_LOAD_INFO,
            &mut cpus,
            &mut info,
            &mut n,
        ) != 0
            || info.is_null()
        {
            return None;
        }
        let a = std::slice::from_raw_parts(info, n as usize);
        let mut per: Vec<(u64, u64)> = a
            .as_chunks::<4>()
            .0
            .iter()
            .take(cpus as usize)
            .map(|c| {
                let [u, s, i, ni] = c.map(|x| x as u32 as u64);
                (u + s + ni, u + s + i + ni)
            })
            .collect();
        mach::vm_deallocate(mach::mach_task_self_, info as usize, n as usize * 4);
        let total = per.iter().fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        per.insert(0, total);
        Some(per)
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn ticks() -> Option<Vec<(u64, u64)>> {
    None
}

/// RAM (total MB, in use MB).
#[cfg(windows)]
pub fn memory() -> Option<(f64, f64)> {
    #[repr(C)]
    struct MemoryStatusEx {
        len: u32,
        load: u32,
        total_phys: u64,
        avail_phys: u64,
        rest: [u64; 5],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(m: *mut MemoryStatusEx) -> i32;
    }
    let mut m = MemoryStatusEx { len: 64, load: 0, total_phys: 0, avail_phys: 0, rest: [0; 5] };
    // SAFETY: dwLength set to sizeof(MEMORYSTATUSEX).
    (unsafe { GlobalMemoryStatusEx(&mut m) } != 0 && m.total_phys > 0)
        .then(|| (m.total_phys as f64 / 1048576.0, (m.total_phys - m.avail_phys) as f64 / 1048576.0))
}

#[cfg(target_os = "linux")]
pub fn memory() -> Option<(f64, f64)> {
    let s = std::fs::read_to_string("/proc/meminfo").ok()?;
    let kb = |k: &str| {
        s.lines().find_map(|l| l.strip_prefix(k)?.strip_prefix(':')?.split_whitespace().next()?.parse::<f64>().ok())
    };
    let (total, avail) = (kb("MemTotal")?, kb("MemAvailable")?);
    Some((total / 1024.0, (total - avail) / 1024.0))
}

/// In use = total minus free, inactive and speculative pages (what can be
/// handed out without paging).
#[cfg(target_os = "macos")]
pub fn memory() -> Option<(f64, f64)> {
    let mut total = 0u64;
    let mut len = std::mem::size_of::<u64>();
    let mut st = [0i32; 64];
    let mut n = mach::HOST_VM_INFO64_COUNT;
    // SAFETY: sysctl/host_statistics64 out-params sized as documented.
    unsafe {
        if libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&mut total as *mut u64).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        ) != 0
            || mach::host_statistics64(mach::mach_host_self(), mach::HOST_VM_INFO64, st.as_mut_ptr(), &mut n) != 0
        {
            return None;
        }
    }
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(4096) as u64;
    // vm_statistics64: free_count [0], inactive_count [2], speculative_count at byte 92.
    let free = (st[0] as u32 as u64 + st[2] as u32 as u64 + st[23] as u32 as u64) * page;
    let mb = |b: u64| b as f64 / 1048576.0;
    (total > 0).then(|| (mb(total), mb(total.saturating_sub(free))))
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub fn memory() -> Option<(f64, f64)> {
    None
}

/// Sum of the kernel's thermal_throttle counters over all CPUs:
/// (`*_throttle_count`, `*_power_limit_count`). Intel-only on current kernels.
#[cfg(target_os = "linux")]
pub fn throttle_counts() -> Option<(u64, u64)> {
    let rd = std::fs::read_dir("/sys/devices/system/cpu").ok()?;
    let (mut th, mut pw, mut any) = (0, 0, false);
    for e in rd.flatten() {
        let Ok(files) = std::fs::read_dir(e.path().join("thermal_throttle")) else { continue };
        for f in files.flatten() {
            let name = f.file_name().to_string_lossy().into_owned();
            let Some(v) = crate::os::read_u64(f.path()) else { continue };
            any = true;
            if name.ends_with("_throttle_count") {
                th += v;
            } else if name.ends_with("_power_limit_count") {
                pw += v;
            }
        }
    }
    any.then_some((th, pw))
}

#[cfg(not(target_os = "linux"))]
pub fn throttle_counts() -> Option<(u64, u64)> {
    None
}

static STOP: AtomicBool = AtomicBool::new(false);

/// Set by Ctrl+C once `catch_ctrl_c` ran.
pub fn ctrl_c() -> bool {
    STOP.load(Ordering::SeqCst)
}

#[cfg(unix)]
pub fn catch_ctrl_c() {
    extern "C" fn on_sigint(_: libc::c_int) {
        STOP.store(true, Ordering::SeqCst);
    }
    // SAFETY: the handler only stores to an atomic (async-signal-safe).
    unsafe {
        libc::signal(libc::SIGINT, on_sigint as extern "C" fn(libc::c_int) as libc::sighandler_t);
    }
}

#[cfg(windows)]
pub fn catch_ctrl_c() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(h: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
    }
    unsafe extern "system" fn on_ctrl(ty: u32) -> i32 {
        // CTRL_C_EVENT / CTRL_BREAK_EVENT: handled; anything else: default action.
        if ty <= 1 {
            STOP.store(true, Ordering::SeqCst);
            1
        } else {
            0
        }
    }
    // SAFETY: registers a handler that only stores to an atomic.
    unsafe {
        SetConsoleCtrlHandler(Some(on_ctrl), 1);
    }
}

#[cfg(not(any(unix, windows)))]
pub fn catch_ctrl_c() {}

#[cfg(test)]
mod tests {
    #[test]
    fn proc_stat() {
        let s = "cpu  10 0 10 70 10 0 0 0 0 0\ncpu0 5 0 5 35 5 0 0 0 0 0\nintr 1 2 3\n";
        assert_eq!(super::parse_proc_stat(s), vec![(20, 100), (10, 50)]);
    }
}

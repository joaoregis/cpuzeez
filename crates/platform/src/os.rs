//! Small OS helpers shared by probes.

use std::path::Path;
use std::process::Command;

pub fn read_trim(p: impl AsRef<Path>) -> Option<String> {
    let s = std::fs::read_to_string(p).ok()?;
    let s = s.trim().trim_end_matches('\0').trim();
    (!s.is_empty()).then(|| s.to_string())
}

pub fn read_u64(p: impl AsRef<Path>) -> Option<u64> {
    let s = read_trim(p)?;
    s.strip_prefix("0x").map_or_else(|| s.parse().ok(), |h| u64::from_str_radix(h, 16).ok())
}

/// stdout of a command, if it ran and succeeded.
pub fn cmd(prog: &str, args: &[&str]) -> Option<String> {
    let mut c = Command::new(prog);
    c.args(args);
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: helpers must not pop up a console from the GUI.
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let o = c.output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

pub fn logical_cpus() -> usize {
    #[cfg(windows)]
    {
        // Counts every processor group, unlike available_parallelism().
        let n = unsafe { win::GetActiveProcessorCount(0xFFFF) } as usize;
        if n > 0 {
            return n;
        }
    }
    std::thread::available_parallelism().map_or(1, |n| n.get())
}

/// System uptime in seconds since boot.
pub fn uptime_secs() -> u64 {
    #[cfg(windows)]
    {
        // SAFETY: GetTickCount64 takes no arguments and never fails.
        (unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() }) / 1000
    }
    #[cfg(target_os = "linux")]
    {
        read_trim("/proc/uptime").and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok()).map_or(0, |s| s as u64)
    }
    #[cfg(target_os = "macos")]
    {
        let mut tv = libc::timeval { tv_sec: 0, tv_usec: 0 };
        let mut len = std::mem::size_of::<libc::timeval>();
        // SAFETY: sysctlbyname with a timeval buffer.
        let ok = unsafe {
            libc::sysctlbyname(
                c"kern.boottime".as_ptr(),
                (&mut tv as *mut libc::timeval).cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            ) == 0
        };
        if ok && tv.tv_sec > 0 {
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            now.saturating_sub(tv.tv_sec as u64)
        } else {
            0
        }
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        0
    }
}

/// Run `f` on the calling thread while it is pinned to logical CPU `cpu`
/// (global index), then restore the thread's previous affinity.
/// Returns None if the OS can't pin (macOS) so callers can fall back.
///
/// The sampler reads per-core MSRs every tick, so this must not create a
/// thread per call (it used to).
pub fn on_cpu<T>(cpu: usize, f: impl FnOnce() -> T) -> Option<T> {
    let _restore = Pinned::to(cpu)?;
    Some(f())
}

/// The calling thread pinned to one CPU; the previous affinity comes back on drop.
struct Pinned {
    #[cfg(windows)]
    prev: win::GroupAffinity,
    #[cfg(windows)]
    priority: i32,
    #[cfg(target_os = "linux")]
    prev: libc::cpu_set_t,
}

impl Pinned {
    #[cfg(windows)]
    fn to(cpu: usize) -> Option<Pinned> {
        let mut rest = cpu as u32;
        // SAFETY: GetCurrentThread is a pseudo-handle; both structs outlive the calls.
        unsafe {
            for grp in 0..win::GetActiveProcessorGroupCount() {
                let n = win::GetActiveProcessorCount(grp);
                if rest < n {
                    let ga = win::GroupAffinity { mask: 1usize << rest, group: grp, reserved: [0; 3] };
                    let mut prev = win::GroupAffinity { mask: 0, group: 0, reserved: [0; 3] };
                    // For the few microseconds we're pinned, run ahead of whatever
                    // keeps that core busy (a game): otherwise the move waits for
                    // its time slice to end, tens of ms per core under load.
                    let priority = win::GetThreadPriority(win::GetCurrentThread());
                    win::SetThreadPriority(win::GetCurrentThread(), win::THREAD_PRIORITY_TIME_CRITICAL);
                    if win::SetThreadGroupAffinity(win::GetCurrentThread(), &ga, &mut prev) == 0 {
                        win::SetThreadPriority(win::GetCurrentThread(), priority);
                        return None;
                    }
                    std::thread::yield_now(); // migrate now, before `f` runs
                    return Some(Pinned { prev, priority });
                }
                rest -= n;
            }
        }
        None
    }

    #[cfg(target_os = "linux")]
    fn to(cpu: usize) -> Option<Pinned> {
        // SAFETY: cpu_set_t is plain data; pid 0 targets the calling thread.
        unsafe {
            let size = std::mem::size_of::<libc::cpu_set_t>();
            let mut prev: libc::cpu_set_t = std::mem::zeroed();
            if libc::sched_getaffinity(0, size, &mut prev) != 0 {
                return None;
            }
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_SET(cpu, &mut set);
            if libc::sched_setaffinity(0, size, &set) != 0 {
                return None;
            }
            let p = Pinned { prev };
            (libc::sched_getcpu() == cpu as i32).then_some(p)
        }
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    fn to(_cpu: usize) -> Option<Pinned> {
        None // macOS has no hard thread affinity.
    }
}

#[cfg(any(windows, target_os = "linux"))]
impl Drop for Pinned {
    fn drop(&mut self) {
        // SAFETY: restores the affinity saved by `to` on the same thread.
        unsafe {
            #[cfg(windows)]
            {
                win::SetThreadGroupAffinity(win::GetCurrentThread(), &self.prev, std::ptr::null_mut());
                win::SetThreadPriority(win::GetCurrentThread(), self.priority);
            }
            #[cfg(target_os = "linux")]
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &self.prev);
        }
    }
}

#[cfg(target_os = "linux")]
pub fn pin(cpu: usize) -> bool {
    // SAFETY: cpu_set_t is plain data; sched_setaffinity(0) targets this thread.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(cpu, &mut set);
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set) == 0
            && libc::sched_getcpu() == cpu as i32
    }
}

#[cfg(windows)]
pub fn pin(cpu: usize) -> bool {
    // Map the global index to (group, bit) across processor groups.
    let mut rest = cpu as u32;
    unsafe {
        let groups = win::GetActiveProcessorGroupCount();
        for grp in 0..groups {
            let n = win::GetActiveProcessorCount(grp);
            if rest < n {
                let ga = win::GroupAffinity { mask: 1usize << rest, group: grp, reserved: [0; 3] };
                let ok = win::SetThreadGroupAffinity(win::GetCurrentThread(), &ga, std::ptr::null_mut()) != 0;
                std::thread::yield_now();
                return ok;
            }
            rest -= n;
        }
    }
    false
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn pin(_cpu: usize) -> bool {
    false // macOS has no hard thread affinity.
}

/// (group, mask) per Windows processor group; one group elsewhere.
#[cfg(windows)]
pub fn cpu_groups() -> Vec<(u16, u32)> {
    let n = unsafe { win::GetActiveProcessorGroupCount() };
    (0..n).map(|g| (g, unsafe { win::GetActiveProcessorCount(g) })).collect()
}

#[cfg(not(windows))]
pub fn cpu_groups() -> Vec<(u16, u32)> {
    vec![(0, logical_cpus() as u32)]
}

/// Double-clicked from Explorer, Windows gives the GUI its own console window;
/// drop it. Started from a terminal, the console is shared and stays.
pub fn hide_own_console() {
    #[cfg(windows)]
    unsafe {
        let mut pids = [0u32; 2];
        if win::GetConsoleProcessList(pids.as_mut_ptr(), 2) == 1 {
            win::FreeConsole();
            nul_std_handles();
        }
    }
}

/// After `FreeConsole` the standard handles are dead, and a child process that
/// inherits them fails to start with "The handle is invalid" (os error 6): that
/// broke the update download, "open folder" and the like when the app was
/// double-clicked. Point all three at `NUL` so every spawn works.
#[cfg(windows)]
fn nul_std_handles() {
    use std::os::windows::io::IntoRawHandle;
    // STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE
    for (which, write) in [(-10i32 as u32, false), (-11i32 as u32, true), (-12i32 as u32, true)] {
        if let Ok(f) = std::fs::OpenOptions::new().read(!write).write(write).open("NUL") {
            // SAFETY: SetStdHandle takes ownership of a valid handle that we hand over (not closed here).
            unsafe { win::SetStdHandle(which, f.into_raw_handle()) };
        }
    }
}

/// The effective user id.
#[cfg(unix)]
pub fn euid() -> u32 {
    // SAFETY: geteuid takes no arguments, has no preconditions and cannot fail.
    unsafe { libc::geteuid() }
}

/// The path a file handle resolves to now (GetFinalPathNameByHandleW, DOS
/// volume name, `\\?\` prefix kept).
#[cfg(windows)]
pub fn final_path(f: &std::fs::File) -> Option<String> {
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::io::AsRawHandle;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFinalPathNameByHandleW(
            hfile: *mut std::ffi::c_void,
            lpszfile: *mut u16,
            cchfile: u32,
            dwflags: u32,
        ) -> u32;
    }
    const CAP: u32 = 1024;
    let mut buf = vec![0u16; CAP as usize];
    // SAFETY: `f` owns a valid file handle; buf/CAP describe a writable
    // buffer of CAP wide chars; flags 0 = DOS volume name + normalized.
    let n = unsafe { GetFinalPathNameByHandleW(f.as_raw_handle(), buf.as_mut_ptr(), CAP, 0) };
    (n > 0 && n < CAP).then(|| std::ffi::OsString::from_wide(&buf[..n as usize]).to_string_lossy().into_owned())
}

/// A string sysctl (`machdep.cpu.brand_string`, …).
#[cfg(target_os = "macos")]
pub fn sysctl_str(name: &str) -> Option<String> {
    let c = std::ffi::CString::new(name).ok()?;
    let mut len = 0usize;
    // SAFETY: standard two-call sysctlbyname pattern with a sized buffer.
    unsafe {
        if libc::sysctlbyname(c.as_ptr(), std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) != 0 || len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len];
        if libc::sysctlbyname(c.as_ptr(), buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) != 0 {
            return None;
        }
        buf.truncate(len);
        let s = String::from_utf8_lossy(&buf).trim_end_matches('\0').trim().to_string();
        (!s.is_empty()).then_some(s)
    }
}

/// An integer sysctl (4 or 8 bytes).
#[cfg(target_os = "macos")]
pub fn sysctl_u64(name: &str) -> Option<u64> {
    let c = std::ffi::CString::new(name).ok()?;
    let mut v = 0u64;
    let mut len = 8usize;
    // SAFETY: integer sysctls are 4 or 8 bytes; the buffer is 8 and zeroed.
    if unsafe { libc::sysctlbyname(c.as_ptr(), (&mut v as *mut u64).cast(), &mut len, std::ptr::null_mut(), 0) } != 0 {
        return None;
    }
    Some(if len == 4 { v & 0xFFFF_FFFF } else { v })
}

/// host_processor_info(PROCESSOR_CPU_LOAD_INFO): user, system, idle and nice
/// ticks per CPU.
#[cfg(target_os = "macos")]
pub fn cpu_load_ticks() -> Option<Vec<[u64; 4]>> {
    unsafe extern "C" {
        fn mach_host_self() -> u32;
        static mach_task_self_: u32;
        fn host_processor_info(host: u32, flavor: i32, count: *mut u32, info: *mut *mut i32, n: *mut u32) -> i32;
        fn vm_deallocate(task: u32, addr: usize, size: usize) -> i32;
    }
    const PROCESSOR_CPU_LOAD_INFO: i32 = 2;
    let (mut cpus, mut info, mut n) = (0u32, std::ptr::null_mut::<i32>(), 0u32);
    // SAFETY: out-params per mach/host_info.h; the kernel-allocated array is
    // copied, then released with vm_deallocate.
    unsafe {
        if host_processor_info(mach_host_self(), PROCESSOR_CPU_LOAD_INFO, &mut cpus, &mut info, &mut n) != 0
            || info.is_null()
        {
            return None;
        }
        let per = std::slice::from_raw_parts(info, n as usize)
            .as_chunks::<4>()
            .0
            .iter()
            .take(cpus as usize)
            .map(|c| c.map(|x| x as u32 as u64))
            .collect();
        vm_deallocate(mach_task_self_, info as usize, n as usize * 4);
        Some(per)
    }
}

static CTRL_C: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set by Ctrl+C (Ctrl+Break on Windows) once `catch_ctrl_c` ran.
pub fn ctrl_c() -> bool {
    CTRL_C.load(std::sync::atomic::Ordering::SeqCst)
}

/// Turn Ctrl+C into a flag (`ctrl_c`) instead of ending the process.
#[cfg(unix)]
pub fn catch_ctrl_c() {
    extern "C" fn on_sigint(_: libc::c_int) {
        CTRL_C.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    // SAFETY: the handler only stores to an atomic (async-signal-safe).
    unsafe {
        libc::signal(libc::SIGINT, on_sigint as extern "C" fn(libc::c_int) as libc::sighandler_t);
    }
}

/// Turn Ctrl+C into a flag (`ctrl_c`) instead of ending the process.
#[cfg(windows)]
pub fn catch_ctrl_c() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(h: Option<unsafe extern "system" fn(u32) -> i32>, add: i32) -> i32;
    }
    unsafe extern "system" fn on_ctrl(ty: u32) -> i32 {
        // CTRL_C_EVENT / CTRL_BREAK_EVENT: handled; anything else: default action.
        if ty <= 1 {
            CTRL_C.store(true, std::sync::atomic::Ordering::SeqCst);
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

#[cfg(windows)]
pub mod win {
    #![allow(non_snake_case)]
    use std::ffi::c_void;

    pub const THREAD_PRIORITY_TIME_CRITICAL: i32 = 15;

    #[repr(C)]
    pub struct GroupAffinity {
        pub mask: usize,
        pub group: u16,
        pub reserved: [u16; 3],
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn SetStdHandle(which: u32, handle: *mut c_void) -> i32;
        pub fn GetActiveProcessorGroupCount() -> u16;
        pub fn GetActiveProcessorCount(group: u16) -> u32;
        pub fn GetCurrentThread() -> *mut c_void;
        pub fn SetThreadGroupAffinity(t: *mut c_void, a: *const GroupAffinity, prev: *mut GroupAffinity) -> i32;
        pub fn GetThreadPriority(t: *mut c_void) -> i32;
        pub fn SetThreadPriority(t: *mut c_void, p: i32) -> i32;
        pub fn GetSystemFirmwareTable(provider: u32, id: u32, buf: *mut u8, size: u32) -> u32;
        pub fn QueryPerformanceFrequency(f: *mut i64) -> i32;
        pub fn QueryPerformanceCounter(c: *mut i64) -> i32;
        pub fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
        pub fn FreeConsole() -> i32;
    }

    #[link(name = "advapi32")]
    unsafe extern "system" {
        pub fn RegGetValueW(
            hkey: isize,
            sub: *const u16,
            val: *const u16,
            flags: u32,
            ty: *mut u32,
            data: *mut c_void,
            len: *mut u32,
        ) -> i32;
    }

    pub const HKLM: isize = 0x8000_0002u32 as i32 as isize;
    const RRF_RT_ANY: u32 = 0xFFFF;

    fn w(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// Raw bytes of HKLM\sub\val.
    pub fn reg_raw(sub: &str, val: &str) -> Option<(u32, Vec<u8>)> {
        let (s, v) = (w(sub), w(val));
        let (mut ty, mut len) = (0u32, 0u32);
        unsafe {
            if RegGetValueW(HKLM, s.as_ptr(), v.as_ptr(), RRF_RT_ANY, &mut ty, std::ptr::null_mut(), &mut len) != 0 {
                return None;
            }
            let mut buf = vec![0u8; len as usize];
            if RegGetValueW(HKLM, s.as_ptr(), v.as_ptr(), RRF_RT_ANY, &mut ty, buf.as_mut_ptr().cast(), &mut len) != 0 {
                return None;
            }
            buf.truncate(len as usize);
            Some((ty, buf))
        }
    }

    /// Names of HKLM\sub's subkeys.
    pub fn reg_subkeys(sub: &str) -> Vec<String> {
        use windows_sys::Win32::System::Registry::{
            HKEY, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW,
        };
        let s = w(sub);
        let mut out = Vec::new();
        // SAFETY: our own key handle and name buffer; closed below.
        unsafe {
            let mut k: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, s.as_ptr(), 0, KEY_READ, &mut k) != 0 {
                return out;
            }
            for i in 0.. {
                let mut name = [0u16; 256];
                let mut n = name.len() as u32;
                let r = RegEnumKeyExW(
                    k,
                    i,
                    name.as_mut_ptr(),
                    &mut n,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                if r != 0 {
                    break;
                }
                out.push(String::from_utf16_lossy(&name[..n as usize]));
            }
            RegCloseKey(k);
        }
        out
    }

    pub fn reg_str(sub: &str, val: &str) -> Option<String> {
        let (_, b) = reg_raw(sub, val)?;
        let u: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let s = String::from_utf16_lossy(&u);
        let s = s.trim_end_matches('\0').trim();
        (!s.is_empty()).then(|| s.to_string())
    }

    pub fn reg_u64(sub: &str, val: &str) -> Option<u64> {
        let (_, b) = reg_raw(sub, val)?;
        let mut a = [0u8; 8];
        let n = b.len().min(8);
        a[..n].copy_from_slice(&b[..n]);
        Some(u64::from_le_bytes(a))
    }
}

// macOS has no pinning, so nothing to test there.
#[cfg(all(test, any(windows, target_os = "linux")))]
mod tests {
    use super::*;

    #[test]
    fn on_cpu_runs_on_the_calling_thread() {
        let me = std::thread::current().id();
        let last = logical_cpus() - 1;
        for cpu in [0, last] {
            assert_eq!(on_cpu(cpu, || std::thread::current().id()), Some(me), "cpu {cpu}");
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn on_cpu_restores_affinity() {
        let get = || unsafe {
            let mut s: libc::cpu_set_t = std::mem::zeroed();
            libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut s);
            (0..logical_cpus()).filter(|&c| libc::CPU_ISSET(c, &s)).collect::<Vec<_>>()
        };
        let before = get();
        on_cpu(0, || ());
        assert_eq!(get(), before);
    }
}

/// The local date and time as `YYYYMMDD-HHMMSS`.
pub fn local_stamp() -> String {
    #[cfg(unix)]
    {
        // SAFETY: `time` with a null pointer only returns the time; `localtime_r` writes only
        // into the zeroed `tm` we pass.
        let tm = unsafe {
            let t = libc::time(std::ptr::null_mut());
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&t, &mut tm);
            tm
        };
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::SYSTEMTIME;
        use windows_sys::Win32::System::SystemInformation::GetLocalTime;
        let mut t = SYSTEMTIME {
            wYear: 0,
            wMonth: 0,
            wDayOfWeek: 0,
            wDay: 0,
            wHour: 0,
            wMinute: 0,
            wSecond: 0,
            wMilliseconds: 0,
        };
        // SAFETY: GetLocalTime fills the struct we own.
        unsafe { GetLocalTime(&mut t) };
        format!("{:04}{:02}{:02}-{:02}{:02}{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
    }
}

/// The user's UI language tag from the OS where it keeps one (`pt-BR`);
/// None elsewhere (Unix reads the environment, macOS `defaults`).
pub fn user_locale() -> Option<String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
        let mut buf = [0u16; 85];
        // SAFETY: the buffer is ours and its length is passed.
        let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
        (n > 1).then(|| String::from_utf16_lossy(&buf[..n as usize - 1]))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod stamp_tests {
    #[test]
    fn the_stamp_has_the_documented_shape() {
        let s = super::local_stamp();
        assert_eq!(s.len(), 15, "{s}");
        assert_eq!(s.as_bytes()[8], b'-');
        assert!(s.bytes().enumerate().all(|(i, b)| i == 8 || b.is_ascii_digit()), "{s}");
        assert!(s.starts_with("20"), "{s}");
    }
}

#[cfg(all(test, windows))]
mod console_tests {
    use std::process::Command;

    /// The child half: only does anything when the parent started it with `CZ_CONSOLE_RESULT`.
    #[test]
    fn child_after_hiding_the_console() {
        let Some(out) = std::env::var_os("CZ_CONSOLE_RESULT") else { return };
        super::hide_own_console(); // the sole process on its own console: frees it
        // Everything inherited, like most of the app's spawns.
        let r = Command::new("cmd").args(["/C", "exit", "0"]).spawn().and_then(|mut c| c.wait());
        let _ = std::fs::write(
            out,
            match r {
                Ok(s) if s.success() => "ok".to_string(),
                Ok(s) => format!("exit {s}"),
                Err(e) => format!("spawn failed: {e}"),
            },
        );
    }

    /// Double-clicked from Explorer, the app owns a console it then frees; a child that
    /// inherits the (now dead) standard handles used to fail with os error 6.
    #[test]
    fn a_child_can_start_after_the_console_is_hidden() {
        if std::env::var_os("CZ_CONSOLE_RESULT").is_some() {
            return; // we are the child; the other test does the work
        }
        let exe = std::env::current_exe().unwrap();
        let out = std::env::temp_dir().join(format!("cz-console-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&out);
        // Start-Process gives the child a console of its own, with console (not pipe) handles.
        let st = Command::new("powershell")
            .args(["-NoProfile", "-Command"])
            .arg(format!(
                "Start-Process -FilePath '{}' -ArgumentList 'child_after_hiding_the_console','--test-threads=1' -WindowStyle Hidden -Wait",
                exe.display()
            ))
            .env("CZ_CONSOLE_RESULT", &out)
            .status();
        assert!(st.is_ok_and(|s| s.success()), "powershell ran");
        let got = std::fs::read_to_string(&out).unwrap_or_else(|e| format!("no result: {e}"));
        let _ = std::fs::remove_file(&out);
        assert_eq!(got, "ok", "a spawn after hide_own_console");
    }
}

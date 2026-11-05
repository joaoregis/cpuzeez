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
        }
    }
}

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

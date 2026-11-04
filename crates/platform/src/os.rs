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

/// Run `f` on a thread pinned to logical CPU `cpu` (global index).
/// Returns None if the OS can't pin (macOS) so callers can fall back.
pub fn on_cpu<T: Send>(cpu: usize, f: impl FnOnce() -> T + Send) -> Option<T> {
    std::thread::scope(|s| s.spawn(move || pin(cpu).then(f)).join().ok().flatten())
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

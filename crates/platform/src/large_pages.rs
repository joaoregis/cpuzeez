//! Page-size-controlled allocations for the memory benchmark's huge-page test
//! (S1-K04): the same pointer chase over 4 KiB pages and over large pages
//! (2 MiB) shows what TLB misses cost.
//!
//! - Windows: `VirtualAlloc(MEM_LARGE_PAGES)` needs the *Lock pages in memory*
//!   user right (`SeLockMemoryPrivilege`), which no account has by default,
//!   administrators included. Without it the test says so.
//! - Linux: transparent huge pages through `madvise(MADV_HUGEPAGE)` (or
//!   `MADV_NOHUGEPAGE` for the small-page run), unless THP is set to `never`.
//! - macOS: no user-controlled large pages.

/// Zeroed memory with a chosen page size, freed on drop.
pub struct Region {
    ptr: *mut u8,
    len: usize,
    #[cfg(unix)]
    map: (*mut u8, usize),
}

// SAFETY: the region owns its mapping exclusively.
unsafe impl Send for Region {}

impl Region {
    pub fn bytes(&mut self) -> &mut [u8] {
        // SAFETY: ptr..ptr+len is our committed mapping.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

/// `len` bytes on large pages (`large`) or forced small pages.
pub fn alloc(len: usize, large: bool) -> Result<Region, String> {
    imp::alloc(len, large)
}

#[cfg(windows)]
mod imp {
    use super::Region;
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, LUID};
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Memory::{
        GetLargePageMinimum, MEM_COMMIT, MEM_LARGE_PAGES, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc,
        VirtualFree,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    /// Turns on SeLockMemoryPrivilege in our token, if the account holds it.
    fn lock_memory_privilege() -> Result<(), String> {
        let name: Vec<u16> = "SeLockMemoryPrivilege\0".encode_utf16().collect();
        let mut token: HANDLE = std::ptr::null_mut();
        // SAFETY: plain Win32 calls on our own process token; buffers are locals.
        unsafe {
            if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let mut luid = LUID { LowPart: 0, HighPart: 0 };
            let ok = LookupPrivilegeValueW(std::ptr::null(), name.as_ptr(), &mut luid) != 0 && {
                let tp = TOKEN_PRIVILEGES {
                    PrivilegeCount: 1,
                    Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }],
                };
                AdjustTokenPrivileges(token, 0, &tp, 0, std::ptr::null_mut(), std::ptr::null_mut()) != 0
                    && GetLastError() != ERROR_NOT_ALL_ASSIGNED
            };
            CloseHandle(token);
            if ok { Ok(()) } else { Err("needs the \"Lock pages in memory\" user right".into()) }
        }
    }

    pub fn alloc(len: usize, large: bool) -> Result<Region, String> {
        let mut flags = MEM_RESERVE | MEM_COMMIT;
        let mut len = len.max(1);
        if large {
            lock_memory_privilege()?;
            // SAFETY: no arguments.
            let page = unsafe { GetLargePageMinimum() };
            if page == 0 {
                return Err("this CPU/OS has no large pages".into());
            }
            len = len.div_ceil(page) * page;
            flags |= MEM_LARGE_PAGES;
        }
        // SAFETY: a fresh private mapping; freed in Drop.
        let ptr = unsafe { VirtualAlloc(std::ptr::null(), len, flags, PAGE_READWRITE) };
        if ptr.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Region { ptr: ptr.cast(), len })
    }

    impl Drop for Region {
        fn drop(&mut self) {
            // SAFETY: allocated by VirtualAlloc above.
            unsafe { VirtualFree(self.ptr.cast(), 0, MEM_RELEASE) };
        }
    }
}

#[cfg(unix)]
mod imp {
    use super::Region;
    const HUGE: usize = 2 << 20;

    pub fn alloc(len: usize, large: bool) -> Result<Region, String> {
        #[cfg(not(target_os = "linux"))]
        if large {
            return Err("this OS has no user-controlled large pages".into());
        }
        #[cfg(target_os = "linux")]
        if large
            && std::fs::read_to_string("/sys/kernel/mm/transparent_hugepage/enabled")
                .is_ok_and(|s| s.contains("[never]"))
        {
            return Err("transparent huge pages are off (THP = never)".into());
        }
        let len = len.max(1).div_ceil(HUGE) * HUGE;
        let map_len = len + HUGE;
        // SAFETY: anonymous private mapping; unmapped in Drop.
        let map = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                map_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if map == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let map = map.cast::<u8>();
        let ptr = map.wrapping_add(map.align_offset(HUGE));
        #[cfg(target_os = "linux")]
        {
            let advice = if large { libc::MADV_HUGEPAGE } else { libc::MADV_NOHUGEPAGE };
            // SAFETY: the range lies inside our mapping.
            if unsafe { libc::madvise(ptr.cast(), len, advice) } != 0 && large {
                let e = std::io::Error::last_os_error().to_string();
                // SAFETY: our mapping.
                unsafe { libc::munmap(map.cast(), map_len) };
                return Err(e);
            }
        }
        Ok(Region { ptr, len, map: (map, map_len) })
    }

    impl Drop for Region {
        fn drop(&mut self) {
            // SAFETY: mapped by mmap above.
            unsafe { libc::munmap(self.map.0.cast(), self.map.1) };
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn small_pages_always_work() {
        let mut r = super::alloc(5 << 20, false).unwrap();
        let b = r.bytes();
        assert!(b.len() >= 5 << 20 && b.iter().all(|&x| x == 0));
        b[0] = 1;
        // Large pages: allowed to fail (no privilege / THP off), never to crash.
        if let Ok(mut l) = super::alloc(4 << 20, true) {
            l.bytes()[0] = 1;
        }
    }
}

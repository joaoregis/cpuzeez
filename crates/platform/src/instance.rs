//! One running copy per user session and data folder, and waiting for a
//! process to end (the elevated relaunch and the update helper).
//!
//! Windows only: a second launch asks the running copy to show its window
//! instead of starting another one that would hide in the tray next to it and
//! keep the installed files locked during an update. Elsewhere `claim` always
//! answers [`Claim::First`] and `wait_exit` returns at once.
//!
//! The objects are in the session's `Local\` namespace, their DACL grants the
//! current user (the same SID whether the copy is elevated or not) and SYSTEM,
//! and their label is Medium with no-write-up, so a normal and an elevated copy
//! can reach each other and a low-integrity process cannot.

use std::time::Duration;

pub enum Claim {
    /// No other copy holds the key: this one runs, and listens for others.
    First(Instance),
    /// Another copy holds it.
    Running,
}

/// Held for the life of the process.
pub struct Instance {
    #[cfg(windows)]
    imp: win::Objects,
}

impl Instance {
    /// Block until another launch asks this copy to show itself.
    pub fn wait_show(&self) {
        #[cfg(windows)]
        self.imp.wait_show();
        #[cfg(not(windows))]
        loop {
            std::thread::park();
        }
    }

    /// Tell the waiting launch that the window was shown.
    pub fn shown(&self) {
        #[cfg(windows)]
        self.imp.shown();
    }
}

/// Become the running copy for `key`, or learn that another one is.
pub fn claim(key: &str) -> Claim {
    #[cfg(windows)]
    return match win::Objects::open(key, true) {
        Some((o, true)) => Claim::First(Instance { imp: o }),
        Some((_, false)) => Claim::Running,
        // The objects could not be made at all: run unguarded rather than not at all.
        None => Claim::First(Instance { imp: win::Objects::none() }),
    };
    #[cfg(not(windows))]
    {
        let _ = key;
        Claim::First(Instance {})
    }
}

/// Ask the running copy for `key` to show its window. True when it answered
/// within `timeout`: a copy that does not answer (its interface is stuck, or it
/// is an older version) must not keep the user from opening the app.
pub fn show_running(key: &str, timeout: Duration) -> bool {
    #[cfg(windows)]
    return win::Objects::open(key, false).is_some_and(|(o, _)| o.ask_show(timeout));
    #[cfg(not(windows))]
    {
        let _ = (key, timeout);
        false
    }
}

/// The current user's SID as a string (`S-1-5-21-…`); the same in an
/// elevated copy. Windows only.
#[cfg(windows)]
pub fn user_sid() -> Option<String> {
    win::user_sid()
}

/// Wait up to `timeout` for process `pid` to end. True when it has ended (or
/// cannot be found), false on timeout.
pub fn wait_exit(pid: u32, timeout: Duration) -> bool {
    #[cfg(windows)]
    return win::wait_exit(pid, timeout);
    #[cfg(not(windows))]
    {
        let _ = (pid, timeout);
        true
    }
}

#[cfg(windows)]
mod win {
    use std::time::Duration;
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, LocalFree, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
    };
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, GetCurrentProcess, INFINITE, OpenProcess, OpenProcessToken, PROCESS_SYNCHRONIZE,
        SetEvent, WaitForSingleObject,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// The current user's SID as a string (`S-1-5-21-…`).
    pub fn user_sid() -> Option<String> {
        // SAFETY: the token is closed below; the buffer is sized by the first
        // call and outlives the SID pointer read from it; the string from
        // ConvertSidToStringSidW is read up to its terminator, then freed.
        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return None;
            }
            let mut len = 0u32;
            GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut len);
            let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
            let ok = GetTokenInformation(token, TokenUser, buf.as_mut_ptr().cast(), len, &mut len) != 0;
            CloseHandle(token);
            if !ok {
                return None;
            }
            let user = &*(buf.as_ptr() as *const TOKEN_USER);
            let mut s: *mut u16 = std::ptr::null_mut();
            if ConvertSidToStringSidW(user.User.Sid, &mut s) == 0 {
                return None;
            }
            let n = (0..).take_while(|&i| *s.add(i) != 0).count();
            let out = String::from_utf16_lossy(std::slice::from_raw_parts(s, n));
            LocalFree(s.cast());
            Some(out)
        }
    }

    /// The user and SYSTEM may use the objects; Medium label, no write-up.
    pub(super) fn sddl(sid: &str) -> String {
        format!("D:P(A;;GA;;;{sid})(A;;GA;;;SY)S:(ML;;NW;;;ME)")
    }

    /// A handle as a number, so `Objects` can cross threads (kernel handles are
    /// process-wide and these are only waited on and signalled).
    #[derive(Clone, Copy)]
    struct H(usize);

    impl H {
        fn get(self) -> HANDLE {
            self.0 as HANDLE
        }
    }

    pub struct Objects {
        mutex: Option<H>,
        show: Option<H>,
        ack: Option<H>,
    }

    impl Objects {
        pub fn none() -> Objects {
            Objects { mutex: None, show: None, ack: None }
        }

        /// Open (or create) the key's objects. With `own`, also the mutex: the
        /// bool says whether this process is the first to hold it.
        pub fn open(key: &str, own: bool) -> Option<(Objects, bool)> {
            let sddl = wide(&sddl(&user_sid()?));
            let name = |part: &str| wide(&format!("Local\\CpuZeeZ-{key}-{part}"));
            // SAFETY: the descriptor is freed after the objects are created; the
            // names and the attributes outlive the calls; handles are kept in
            // `Objects` for the life of the process.
            unsafe {
                let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
                if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    sddl.as_ptr(),
                    SDDL_REVISION_1,
                    &mut sd,
                    std::ptr::null_mut(),
                ) == 0
                {
                    return None;
                }
                let sa = SECURITY_ATTRIBUTES {
                    nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                    lpSecurityDescriptor: sd,
                    bInheritHandle: 0,
                };
                let some = |h: HANDLE| (!h.is_null()).then_some(H(h as usize));
                let mut first = false;
                let mutex = if own {
                    let h = CreateMutexW(&sa, 0, name("instance").as_ptr());
                    let err = GetLastError();
                    // Access denied: it exists, made by a copy we may not open.
                    first = !h.is_null() && err != ERROR_ALREADY_EXISTS && err != ERROR_ACCESS_DENIED;
                    some(h)
                } else {
                    None
                };
                // Auto-reset events: one request, one answer.
                let show = some(CreateEventW(&sa, 0, 0, name("show").as_ptr()));
                let ack = some(CreateEventW(&sa, 0, 0, name("shown").as_ptr()));
                LocalFree(sd);
                if own && !first {
                    for h in [mutex, show, ack].into_iter().flatten() {
                        CloseHandle(h.get());
                    }
                    return Some((Objects::none(), false));
                }
                Some((Objects { mutex, show, ack }, first))
            }
        }

        pub fn wait_show(&self) {
            match self.show {
                // SAFETY: a valid event handle held by `self`.
                Some(h) => unsafe {
                    WaitForSingleObject(h.get(), INFINITE);
                },
                None => loop {
                    std::thread::park();
                },
            }
        }

        pub fn shown(&self) {
            if let Some(h) = self.ack {
                // SAFETY: a valid event handle held by `self`.
                unsafe { SetEvent(h.get()) };
            }
        }

        pub fn ask_show(&self, timeout: Duration) -> bool {
            let (Some(show), Some(ack)) = (self.show, self.ack) else { return false };
            // SAFETY: valid event handles held by `self`.
            unsafe {
                SetEvent(show.get());
                WaitForSingleObject(ack.get(), timeout.as_millis().min(u32::MAX as u128 - 1) as u32) == WAIT_OBJECT_0
            }
        }
    }

    impl Drop for Objects {
        fn drop(&mut self) {
            for h in [self.mutex, self.show, self.ack].into_iter().flatten() {
                // SAFETY: handles opened by `open` and closed once, here.
                unsafe { CloseHandle(h.get()) };
            }
        }
    }

    pub fn wait_exit(pid: u32, timeout: Duration) -> bool {
        // SAFETY: the handle is closed after the wait.
        unsafe {
            let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if h.is_null() {
                return true; // gone (or never ours to see)
            }
            let r = WaitForSingleObject(h, timeout.as_millis().min(u32::MAX as u128 - 1) as u32);
            CloseHandle(h);
            r == WAIT_OBJECT_0
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn key() -> String {
        format!("test-{}-{:x}", std::process::id(), std::time::UNIX_EPOCH.elapsed().unwrap().as_nanos())
    }

    #[test]
    fn second_claim_sees_the_first_and_is_shown() {
        let k = key();
        let Claim::First(first) = claim(&k) else { panic!("the first claim must win") };
        assert!(matches!(claim(&k), Claim::Running));
        let first = std::sync::Arc::new(first);
        let f = first.clone();
        let t = std::thread::spawn(move || {
            f.wait_show();
            f.shown();
        });
        assert!(show_running(&k, Duration::from_secs(10)));
        t.join().unwrap();
        // Nobody listening any more: the request times out instead of hanging.
        assert!(!show_running(&k, Duration::from_millis(50)));
        drop(first);
        assert!(matches!(claim(&k), Claim::First(_)), "the key is free once the holder is gone");
    }

    #[test]
    fn waits_for_a_process_to_end() {
        let mut c = std::process::Command::new("cmd").args(["/c", "exit"]).spawn().unwrap();
        let pid = c.id();
        assert!(wait_exit(pid, Duration::from_secs(10)));
        c.wait().unwrap();
        let mut s = std::process::Command::new("cmd").args(["/c", "ping -n 30 127.0.0.1 >nul"]).spawn().unwrap();
        assert!(!wait_exit(s.id(), Duration::from_millis(100)), "a running process times out");
        let _ = s.kill();
        let _ = s.wait();
    }

    #[test]
    fn descriptor_names_only_the_user_and_system() {
        assert_eq!(win::sddl("S-1-5-21-1"), "D:P(A;;GA;;;S-1-5-21-1)(A;;GA;;;SY)S:(ML;;NW;;;ME)");
    }
}

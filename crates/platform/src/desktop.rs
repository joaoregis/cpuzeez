//! Top-level windows per process (S2a): what makes a process an "App" in the
//! Task Manager's sense, whether it's "Not responding", and the window
//! actions (switch to, minimize, maximize, close). Windows only; Linux/macOS have
//! no portable way to see other apps' windows (Wayland forbids it).

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub pid: u32,
    /// The window handle, for the window actions.
    pub handle: isize,
    pub title: String,
    /// The window stopped processing messages for 5 s (`IsHungAppWindow`).
    pub hung: bool,
}

/// Visible, titled, unowned top-level windows.
pub fn windows() -> Vec<Window> {
    imp::windows()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowOp {
    SwitchTo,
    Minimize,
    Maximize,
    /// Ask the window to close (`WM_CLOSE`), like its X button: the app may
    /// ask to save first.
    Close,
}

/// The process that owns the foreground window (the game, when one is played).
pub fn foreground_pid() -> Option<u32> {
    imp::foreground_pid()
}

/// A window action (no privilege: windows of this desktop).
pub fn window_op(handle: isize, op: WindowOp) -> bool {
    imp::window_op(handle, op)
}

#[cfg(windows)]
mod imp {
    use super::{Window, WindowOp};
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GW_OWNER, GetWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsHungAppWindow, IsWindowVisible, PostMessageW, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SetForegroundWindow,
        ShowWindow, WM_CLOSE,
    };

    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> i32 {
        // SAFETY: `l` is the &mut Vec passed by `windows` below, alive for the call.
        let out = unsafe { &mut *(l as *mut Vec<Window>) };
        // SAFETY: plain queries on a window handle the OS gave us.
        unsafe {
            if IsWindowVisible(h) == 0 || !GetWindow(h, GW_OWNER).is_null() {
                return 1;
            }
            let n = GetWindowTextLengthW(h);
            if n <= 0 {
                return 1;
            }
            let mut buf = vec![0u16; n as usize + 1];
            let got = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, &mut pid);
            out.push(Window {
                pid,
                handle: h as isize,
                title: String::from_utf16_lossy(&buf[..got.max(0) as usize]),
                hung: IsHungAppWindow(h) != 0,
            });
        }
        1
    }

    pub fn windows() -> Vec<Window> {
        let mut v: Vec<Window> = Vec::new();
        // SAFETY: the callback only touches `v` through the pointer we pass.
        unsafe { EnumWindows(Some(each), &mut v as *mut Vec<Window> as LPARAM) };
        v
    }

    pub fn foreground_pid() -> Option<u32> {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        // SAFETY: plain queries; a null window gives pid 0.
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
            (pid != 0).then_some(pid)
        }
    }

    pub fn window_op(handle: isize, op: WindowOp) -> bool {
        let h = handle as HWND;
        // SAFETY: plain window calls; a stale handle just fails.
        unsafe {
            match op {
                WindowOp::SwitchTo => {
                    ShowWindow(h, SW_RESTORE);
                    SetForegroundWindow(h) != 0
                }
                WindowOp::Minimize => ShowWindow(h, SW_MINIMIZE) != 0,
                WindowOp::Maximize => ShowWindow(h, SW_MAXIMIZE) != 0,
                WindowOp::Close => PostMessageW(h, WM_CLOSE, 0, 0) != 0,
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{Window, WindowOp};
    pub fn windows() -> Vec<Window> {
        Vec::new()
    }
    pub fn window_op(_: isize, _: WindowOp) -> bool {
        false
    }
    pub fn foreground_pid() -> Option<u32> {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lists_windows_without_panicking() {
        // A CI desktop may have none; a dev machine has some.
        for w in super::windows() {
            assert!(!w.title.is_empty() && w.pid > 0);
        }
    }
}

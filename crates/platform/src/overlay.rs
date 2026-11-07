//! Per-OS tweaks for CPU ZeeZ's own HUD windows (ZeeZ Live overlay, mini
//! monitors), docs/v1/zeez-live.md §3.2. They touch only windows of this
//! process, found by their title.
//!
//! - Windows: `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` (never takes focus, not in
//!   Alt-Tab), `WS_EX_TRANSPARENT` for click-through, the layered-window
//!   alpha that glow needs to be visible when click-through (ADR 0013),
//!   `HWND_TOPMOST` re-asserted, and `SetWindowDisplayAffinity` to keep the
//!   HUD out of captures and streams when asked.
//! - Linux/macOS: winit's always-on-top and mouse passthrough already do what
//!   these platforms allow; macOS fullscreen Spaces need `objc2` (spike X04).

/// What to apply to a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub click_through: bool,
    /// Never activated by a click, not in Alt-Tab.
    pub no_activate: bool,
    pub exclude_from_capture: bool,
}

/// Apply `s` to this process's window titled `title`. False when no such
/// window exists yet (it opens a frame later) or on platforms with nothing to do.
pub fn apply(title: &str, s: Style) -> bool {
    imp::apply(title, s)
}

#[cfg(windows)]
mod imp {
    use super::Style;
    use windows_sys::Win32::Foundation::{HWND, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId, HWND_TOPMOST, LWA_ALPHA,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetLayeredWindowAttributes, SetWindowDisplayAffinity,
        SetWindowLongPtrW, SetWindowPos, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    };

    struct Find {
        title: Vec<u16>,
        pid: u32,
        found: HWND,
    }

    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> i32 {
        // SAFETY: `l` is the &mut Find passed below, alive for the enumeration.
        let f = unsafe { &mut *(l as *mut Find) };
        let mut pid = 0u32;
        // SAFETY: plain queries on a handle the OS gave us.
        unsafe {
            GetWindowThreadProcessId(h, &mut pid);
            if pid != f.pid {
                return 1;
            }
            let mut buf = [0u16; 256];
            let n = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
            if buf[..n] == f.title[..] {
                f.found = h;
                return 0;
            }
        }
        1
    }

    pub fn apply(title: &str, s: Style) -> bool {
        let mut f =
            Find { title: title.encode_utf16().collect(), pid: std::process::id(), found: std::ptr::null_mut() };
        // SAFETY: the callback only touches `f` through the pointer we pass.
        unsafe { EnumWindows(Some(each), &mut f as *mut Find as LPARAM) };
        let h = f.found;
        if h.is_null() {
            return false;
        }
        // SAFETY: our own window.
        unsafe {
            let old = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
            let mut ex = old | WS_EX_LAYERED | WS_EX_TOPMOST;
            if s.no_activate {
                ex |= WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW;
            } else {
                ex &= !(WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW);
            }
            if s.click_through {
                ex |= WS_EX_TRANSPARENT;
            } else {
                ex &= !WS_EX_TRANSPARENT;
            }
            if ex != old {
                SetWindowLongPtrW(h, GWL_EXSTYLE, ex as isize);
            }
            // Without it, a layered glow window draws nothing (ADR 0013).
            SetLayeredWindowAttributes(h, 0, 255, LWA_ALPHA);
            SetWindowDisplayAffinity(h, if s.exclude_from_capture { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE });
            // Games that go fullscreen can push other topmost windows down.
            SetWindowPos(h, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
        true
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn apply(_: &str, _: super::Style) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_missing_window_is_not_found() {
        assert!(!super::apply("CPU ZeeZ test window that does not exist", super::Style::default()));
    }
}

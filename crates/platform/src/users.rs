//! User sessions enumeration and session actions (S2a-09, docs/v1/task-manager.md §7).
//!
//! - **Windows:** WTS (`WTSEnumerateSessionsW`, `WTSQuerySessionInformationW`,
//!   `WTSDisconnectSession`, `WTSLogoffSession`, `WTSSendMessageW`).
//! - **Linux:** `loginctl` (`org.freedesktop.login1`) / `who`.
//! - **macOS:** `who` / `launchctl`.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserSession {
    pub session_id: u32,
    pub user: String,
    pub domain: String,
    pub state: String,
    pub client_name: String,
}

impl UserSession {
    pub fn full_account(&self) -> String {
        if self.domain.is_empty() { self.user.clone() } else { format!("{}\\{}", self.domain, self.user) }
    }
}

pub fn sessions() -> Vec<UserSession> {
    imp::sessions()
}

pub fn disconnect(session_id: u32) -> Result<String, String> {
    imp::disconnect(session_id)
}

pub fn sign_out(session_id: u32) -> Result<String, String> {
    imp::sign_out(session_id)
}

pub fn send_message(session_id: u32, title: &str, body: &str) -> Result<String, String> {
    imp::send_message(session_id, title, body)
}

pub fn manage_accounts() -> Result<(), String> {
    imp::manage_accounts()
}

// ---------------------------------------------------------------------------
// Windows (WTS)
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::RemoteDesktop::{
        WTS_CONNECTSTATE_CLASS, WTS_CURRENT_SERVER_HANDLE, WTS_SESSION_INFOW, WTSActive, WTSClientName, WTSConnected,
        WTSDisconnectSession, WTSDisconnected, WTSDomainName, WTSEnumerateSessionsW, WTSFreeMemory, WTSIdle,
        WTSLogoffSession, WTSQuerySessionInformationW, WTSSendMessageW, WTSUserName,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::MB_OK;

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn query_wts_str(session_id: u32, info: i32) -> String {
        let mut buf: *mut u16 = null_mut();
        let mut bytes = 0u32;
        unsafe {
            if WTSQuerySessionInformationW(WTS_CURRENT_SERVER_HANDLE, session_id, info, &mut buf, &mut bytes) == 0
                || buf.is_null()
            {
                return String::new();
            }
            let len = (bytes as usize / 2).saturating_sub(1);
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(buf, len))
                .trim_end_matches('\0')
                .trim()
                .to_string();
            WTSFreeMemory(buf.cast());
            s
        }
    }

    fn state_label(st: WTS_CONNECTSTATE_CLASS) -> &'static str {
        if st == WTSActive {
            "Active"
        } else if st == WTSConnected {
            "Connected"
        } else if st == WTSDisconnected {
            "Disconnected"
        } else if st == WTSIdle {
            "Idle"
        } else {
            "Other"
        }
    }

    pub fn sessions() -> Vec<UserSession> {
        unsafe {
            let mut ptr: *mut WTS_SESSION_INFOW = null_mut();
            let mut count = 0u32;
            if WTSEnumerateSessionsW(WTS_CURRENT_SERVER_HANDLE, 0, 1, &mut ptr, &mut count) == 0 || ptr.is_null() {
                return Vec::new();
            }
            let slice = std::slice::from_raw_parts(ptr, count as usize);
            let mut out = Vec::new();
            for s in slice {
                let user = query_wts_str(s.SessionId, WTSUserName);
                if user.is_empty() {
                    continue;
                }
                let domain = query_wts_str(s.SessionId, WTSDomainName);
                let client = query_wts_str(s.SessionId, WTSClientName);
                out.push(UserSession {
                    session_id: s.SessionId,
                    user,
                    domain,
                    state: state_label(s.State).to_string(),
                    client_name: if client.is_empty() { "Console".into() } else { client },
                });
            }
            WTSFreeMemory(ptr.cast());
            out
        }
    }

    pub fn disconnect(session_id: u32) -> Result<String, String> {
        unsafe {
            if WTSDisconnectSession(WTS_CURRENT_SERVER_HANDLE, session_id, 0) == 0 {
                Err(format!("WTSDisconnectSession({session_id}) failed (error {})", GetLastError()))
            } else {
                Ok(format!("Disconnected session {session_id}"))
            }
        }
    }

    pub fn sign_out(session_id: u32) -> Result<String, String> {
        unsafe {
            if WTSLogoffSession(WTS_CURRENT_SERVER_HANDLE, session_id, 0) == 0 {
                Err(format!("WTSLogoffSession({session_id}) failed (error {})", GetLastError()))
            } else {
                Ok(format!("Signed out session {session_id}"))
            }
        }
    }

    pub fn send_message(session_id: u32, title: &str, body: &str) -> Result<String, String> {
        unsafe {
            let mut t_wide = to_wide(title);
            let mut b_wide = to_wide(body);
            let t_bytes = ((t_wide.len().saturating_sub(1)) * 2) as u32;
            let b_bytes = ((b_wide.len().saturating_sub(1)) * 2) as u32;
            let mut resp = 0i32;
            if WTSSendMessageW(
                WTS_CURRENT_SERVER_HANDLE,
                session_id,
                t_wide.as_mut_ptr(),
                t_bytes,
                b_wide.as_mut_ptr(),
                b_bytes,
                MB_OK,
                0,
                &mut resp,
                0,
            ) == 0
            {
                Err(format!("WTSSendMessageW({session_id}) failed (error {})", GetLastError()))
            } else {
                Ok(format!("Message sent to session {session_id}"))
            }
        }
    }

    pub fn manage_accounts() -> Result<(), String> {
        crate::proc_ops::run_task("ms-settings:otherusers", false)
    }
}

// ---------------------------------------------------------------------------
// Linux (logind / who)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod imp {
    use super::*;

    pub fn sessions() -> Vec<UserSession> {
        if let Some(text) = crate::os::cmd("loginctl", &["list-sessions", "--no-legend", "--no-pager"]) {
            let mut out = Vec::new();
            for line in text.lines() {
                let mut parts = line.split_whitespace();
                let Some(sid_s) = parts.next() else { continue };
                let _uid = parts.next();
                let Some(user) = parts.next() else { continue };
                let seat = parts.next().unwrap_or("seat0");
                let session_id = sid_s.parse::<u32>().unwrap_or(0);
                out.push(UserSession {
                    session_id,
                    user: user.to_string(),
                    domain: String::new(),
                    state: "Active".into(),
                    client_name: seat.to_string(),
                });
            }
            if !out.is_empty() {
                return out;
            }
        }
        Vec::new()
    }

    pub fn disconnect(session_id: u32) -> Result<String, String> {
        let sid = session_id.to_string();
        let out =
            std::process::Command::new("loginctl").args(["lock-session", &sid]).output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(format!("Locked session {session_id}"))
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }

    pub fn sign_out(session_id: u32) -> Result<String, String> {
        let sid = session_id.to_string();
        let out = std::process::Command::new("loginctl")
            .args(["terminate-session", &sid])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(format!("Terminated session {session_id}"))
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }

    pub fn send_message(_session_id: u32, title: &str, body: &str) -> Result<String, String> {
        let out =
            std::process::Command::new("notify-send").args(["--", title, body]).output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok("Sent desktop notification".into())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }

    pub fn manage_accounts() -> Result<(), String> {
        crate::proc_ops::run_task("gnome-control-center user-accounts", false)
    }
}

// ---------------------------------------------------------------------------
// macOS
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod imp {
    use super::*;

    pub fn sessions() -> Vec<UserSession> {
        let Some(text) = crate::os::cmd("who", &[]) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (idx, line) in text.lines().enumerate() {
            let mut parts = line.split_whitespace();
            let Some(user) = parts.next() else { continue };
            let tty = parts.next().unwrap_or("console");
            if out.iter().any(|u: &UserSession| u.user == user) {
                continue;
            }
            out.push(UserSession {
                session_id: idx as u32,
                user: user.to_string(),
                domain: String::new(),
                state: "Active".into(),
                client_name: tty.to_string(),
            });
        }
        out
    }

    pub fn disconnect(_session_id: u32) -> Result<String, String> {
        Err("Session disconnect is not supported on macOS".into())
    }

    pub fn sign_out(session_id: u32) -> Result<String, String> {
        Err(format!("Sign out for session {session_id} requires macOS System Settings"))
    }

    pub fn send_message(_session_id: u32, _title: &str, _body: &str) -> Result<String, String> {
        Err("WTSSendMessage is not available on macOS".into())
    }

    pub fn manage_accounts() -> Result<(), String> {
        crate::proc_ops::run_task("open x-apple.systempreferences:com.apple.Users-Groups-Settings.extension", false)
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    use super::*;
    pub fn sessions() -> Vec<UserSession> {
        Vec::new()
    }
    pub fn disconnect(_session_id: u32) -> Result<String, String> {
        Err("Not supported".into())
    }
    pub fn sign_out(_session_id: u32) -> Result<String, String> {
        Err("Not supported".into())
    }
    pub fn send_message(_session_id: u32, _title: &str, _body: &str) -> Result<String, String> {
        Err("Not supported".into())
    }
    pub fn manage_accounts() -> Result<(), String> {
        Err("Not supported".into())
    }
}

#[cfg(test)]
mod tests {
    /// Contract: sessions answer without privileges; on Windows there is at
    /// least the session this test runs in (or the services session on CI).
    #[test]
    fn sessions_answer() {
        let s = super::sessions();
        if cfg!(windows) {
            assert!(!s.is_empty());
        }
        for x in &s {
            assert!(x.session_id < 1 << 20, "{x:?}");
        }
    }
}

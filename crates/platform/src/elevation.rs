//! Administrator / root rights (docs/adr/0002): whether this process has
//! them, and "Restart as administrator", which starts an elevated copy of the
//! app with the same arguments.
//!
//! Windows asks through UAC (`ShellExecuteExW` with the `runas` verb), which
//! returns once the user has answered. Linux asks through polkit (`pkexec`),
//! which gives no such answer: the elevated copy writes a marker file when it
//! is up (`--elevated-handoff=PATH`), and [`Pending::poll`] watches for it, so
//! the original window only closes once the new one runs, and stays open with
//! a message when the user cancels. macOS gives administrators no extra
//! hardware access, so there is nothing to restart for.

use std::path::PathBuf;

/// Is this process running with administrator (Windows) or root (Unix) rights?
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    return win::is_elevated();
    #[cfg(unix)]
    return unsafe { libc::geteuid() } == 0;
    #[allow(unreachable_code)]
    false
}

/// Can "Restart as administrator" work on this OS?
pub fn can_relaunch() -> bool {
    cfg!(windows) || (cfg!(target_os = "linux") && which("pkexec").is_some())
}

/// Outcome of asking the OS for an elevated copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The elevated copy is running; the caller should exit.
    Started,
    /// The user declined the prompt.
    Cancelled,
    /// Not possible on this OS / setup.
    Unsupported,
    Failed(String),
}

/// A relaunch waiting for the user to answer the prompt (Linux).
pub struct Pending {
    #[cfg(target_os = "linux")]
    child: std::process::Child,
    marker: PathBuf,
    done: Option<Outcome>,
}

impl Pending {
    /// None while the prompt is still open; then the outcome, once.
    pub fn poll(&mut self) -> Option<Outcome> {
        if let Some(o) = self.done.take() {
            return Some(o);
        }
        if self.marker.exists() {
            let _ = std::fs::remove_file(&self.marker);
            return Some(Outcome::Started);
        }
        #[cfg(target_os = "linux")]
        if let Ok(Some(st)) = self.child.try_wait() {
            return Some(match st.code() {
                // pkexec: 126 = dialog dismissed or not authorized, 127 = authentication failed.
                Some(126 | 127) => Outcome::Cancelled,
                Some(c) => {
                    Outcome::Failed(format!("the elevated copy exited with code {c} (no display access as root?)"))
                }
                None => Outcome::Failed("the elevated copy was killed".into()),
            });
        }
        None
    }
}

/// Start an elevated copy of this program with `args` (the caller's own
/// arguments, minus the program name).
pub fn relaunch(args: &[String]) -> Pending {
    let marker = std::env::temp_dir().join(format!("cpuzeez-handoff-{}", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let fail = |o: Outcome, marker: PathBuf| Pending {
        #[cfg(target_os = "linux")]
        child: std::process::Command::new("true").spawn().expect("`true` runs"),
        marker,
        done: Some(o),
    };
    let Ok(exe) = std::env::current_exe() else {
        return fail(Outcome::Failed("cannot find this program's path".into()), marker);
    };
    #[cfg(windows)]
    {
        let done = win::runas(&exe, args);
        Pending { marker, done: Some(done) }
    }
    #[cfg(target_os = "linux")]
    {
        let Some(pkexec) = which("pkexec") else { return fail(Outcome::Unsupported, marker) };
        // pkexec clears the environment; pass what a GUI needs to reach this session's display.
        let mut c = std::process::Command::new(pkexec);
        c.arg("env");
        for k in ["DISPLAY", "XAUTHORITY", "WAYLAND_DISPLAY", "XDG_RUNTIME_DIR", "XDG_SESSION_TYPE", "LANG"] {
            if let Ok(v) = std::env::var(k) {
                c.arg(format!("{k}={v}"));
            }
        }
        c.arg(exe).args(args).arg(format!("--elevated-handoff={}", marker.display()));
        match c.spawn() {
            Ok(child) => Pending { child, marker, done: None },
            Err(e) => fail(Outcome::Failed(format!("pkexec: {e}")), marker),
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (exe, args);
        Pending { marker, done: Some(Outcome::Unsupported) }
    }
}

/// Called by the elevated copy once it is up: tells the waiting original.
pub fn signal_handoff(marker: &str) {
    let _ = std::fs::write(marker, b"ok");
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn which(prog: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?.to_str()?.split(':').map(|d| PathBuf::from(d).join(prog)).find(|p| p.is_file())
}

/// One argument quoted for a Windows command line (CommandLineToArgvW rules).
#[cfg_attr(not(windows), allow(dead_code))]
fn quote_arg(a: &str) -> String {
    if !a.is_empty() && !a.contains([' ', '\t', '"']) {
        return a.to_string();
    }
    let mut out = String::from('"');
    let mut slashes = 0;
    for ch in a.chars() {
        match ch {
            '\\' => slashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', slashes * 2 + 1));
                out.push('"');
                slashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', slashes));
                out.push(ch);
                slashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', slashes * 2));
    out.push('"');
    out
}

#[cfg(windows)]
mod win {
    use super::{Outcome, quote_arg};
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, GetLastError, HANDLE};
    use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    use windows_sys::Win32::UI::Shell::{SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
        s.as_ref().encode_wide().chain(Some(0)).collect()
    }

    pub fn is_elevated() -> bool {
        // SAFETY: the token handle is closed below; `e` is sized for TokenElevation.
        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return false;
            }
            let mut e = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut len = 0u32;
            let ok = GetTokenInformation(
                token,
                TokenElevation,
                (&mut e as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut len,
            ) != 0;
            CloseHandle(token);
            ok && e.TokenIsElevated != 0
        }
    }

    /// UAC prompt, then start `exe args` elevated. Returns once the user answered.
    pub fn runas(exe: &std::path::Path, args: &[String]) -> Outcome {
        let (verb, file) = (wide("runas"), wide(exe));
        let params = wide(args.iter().map(|a| quote_arg(a)).collect::<Vec<_>>().join(" "));
        // SAFETY: every pointer refers to a NUL-terminated buffer alive for the call.
        unsafe {
            let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            info.fMask = SEE_MASK_NOASYNC;
            info.lpVerb = verb.as_ptr();
            info.lpFile = file.as_ptr();
            info.lpParameters = params.as_ptr();
            info.nShow = SW_SHOWNORMAL;
            if ShellExecuteExW(&mut info) != 0 {
                Outcome::Started
            } else if GetLastError() == ERROR_CANCELLED {
                Outcome::Cancelled
            } else {
                Outcome::Failed(std::io::Error::last_os_error().to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_argument_quoting() {
        assert_eq!(quote_arg("-txt=report"), "-txt=report");
        assert_eq!(quote_arg("C:\\Program Files\\x"), "\"C:\\Program Files\\x\"");
        assert_eq!(quote_arg(""), "\"\"");
        assert_eq!(quote_arg("say \"hi\""), "\"say \\\"hi\\\"\"");
        // Trailing backslashes are doubled before the closing quote.
        assert_eq!(quote_arg("a dir\\"), "\"a dir\\\\\"");
    }

    #[test]
    fn handoff_marker_reports_started() {
        let marker = std::env::temp_dir().join(format!("cpuzeez-handoff-test-{}", std::process::id()));
        #[cfg(target_os = "linux")]
        let child = std::process::Command::new("sleep").arg("5").spawn().unwrap();
        let mut p = Pending {
            #[cfg(target_os = "linux")]
            child,
            marker: marker.clone(),
            done: None,
        };
        #[cfg(not(target_os = "linux"))]
        assert_eq!(p.poll(), None, "no marker yet");
        signal_handoff(marker.to_str().unwrap());
        assert_eq!(p.poll(), Some(Outcome::Started));
        assert!(!marker.exists(), "marker cleaned up");
        #[cfg(target_os = "linux")]
        let _ = p.child.kill();
    }

    #[test]
    fn elevation_query_does_not_crash() {
        let _ = is_elevated();
        let _ = can_relaunch();
    }
}

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
    /// The private marker file the elevated copy signals with (None when the
    /// relaunch already has its answer, or no marker could be created).
    marker: Option<PathBuf>,
    done: Option<Outcome>,
}

impl Pending {
    /// None while the prompt is still open; then the outcome, once.
    pub fn poll(&mut self) -> Option<Outcome> {
        if let Some(o) = self.done.take() {
            return Some(o);
        }
        // Only a plain file at the marker name is "up": the directory is
        // exclusively ours and 0700, and the elevated copy refuses to follow
        // symlinks, so nobody else can put any entry there — anything that is
        // not a regular file is not a signal (symlink_metadata does not follow).
        if self.marker.as_ref().is_some_and(|m| m.symlink_metadata().is_ok_and(|md| md.is_file())) {
            return self.finish(Outcome::Started);
        }
        #[cfg(target_os = "linux")]
        if let Ok(Some(st)) = self.child.try_wait() {
            return self.finish(match st.code() {
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

    /// Report `o` and reclaim the private handoff directory, which held only
    /// the marker: every "Restart as root" used to leave one behind in /tmp.
    fn finish(&mut self, o: Outcome) -> Option<Outcome> {
        if let Some(m) = self.marker.take() {
            let _ = std::fs::remove_dir_all(m.parent().unwrap_or(&m));
        }
        Some(o)
    }
}

/// Start an elevated copy of this program with `args` (the caller's own
/// arguments, minus the program name).
pub fn relaunch(args: &[String]) -> Pending {
    let fail = |o: Outcome| Pending {
        #[cfg(target_os = "linux")]
        child: std::process::Command::new("true").spawn().expect("`true` runs"),
        marker: None,
        done: Some(o),
    };
    let Ok(exe) = std::env::current_exe() else {
        return fail(Outcome::Failed("cannot find this program's path".into()));
    };
    #[cfg(windows)]
    {
        let done = win::runas(&exe, args);
        Pending { marker: None, done: Some(done) }
    }
    #[cfg(target_os = "linux")]
    {
        let Some(pkexec) = which("pkexec") else { return fail(Outcome::Unsupported) };
        // A predictable marker name in shared /tmp would let any local user
        // pre-plant a symlink and have the ROOT copy truncate a file of their
        // choosing when it signals (CWE-59): only a private, unpredictable
        // directory we just created is acceptable.
        let Some(marker) = handoff_marker() else {
            return fail(Outcome::Failed("could not create a private handoff directory".into()));
        };
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
            Ok(child) => Pending { child, marker: Some(marker), done: None },
            Err(e) => {
                // The directory was created for this relaunch only; a spawn
                // failure must not leave it behind in /tmp.
                let _ = std::fs::remove_dir_all(marker.parent().unwrap_or(&marker));
                fail(Outcome::Failed(format!("pkexec: {e}")))
            }
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (exe, args);
        Pending { marker: None, done: Some(Outcome::Unsupported) }
    }
}

/// Run this program elevated with `args` and wait for it: per-action
/// elevation (one prompt per action). Returns the elevated copy's exit code.
pub fn run(args: &[String]) -> Result<i32, Outcome> {
    let exe = std::env::current_exe().map_err(|e| Outcome::Failed(e.to_string()))?;
    #[cfg(windows)]
    return win::runas_wait(&exe, args);
    #[cfg(target_os = "linux")]
    {
        let pkexec = which("pkexec").ok_or(Outcome::Unsupported)?;
        let st = std::process::Command::new(pkexec)
            .arg(exe)
            .args(args)
            .status()
            .map_err(|e| Outcome::Failed(e.to_string()))?;
        return match st.code() {
            Some(126 | 127) => Err(Outcome::Cancelled),
            Some(c) => Ok(c),
            None => Err(Outcome::Failed("killed".into())),
        };
    }
    #[cfg(target_os = "macos")]
    {
        // The standard authorization prompt; -128 is the user's Cancel.
        let cmd = std::iter::once(exe.display().to_string())
            .chain(args.iter().cloned())
            .map(|a| sh_quote(&a))
            .collect::<Vec<_>>()
            .join(" ");
        // AppleScript string literal: escape backslashes, then double quotes.
        let lit = cmd.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!("do shell script \"{lit}\" with administrator privileges");
        let o = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| Outcome::Failed(e.to_string()))?;
        if o.status.success() {
            return Ok(0);
        }
        let err = String::from_utf8_lossy(&o.stderr);
        return Err(if err.contains("-128") { Outcome::Cancelled } else { Outcome::Failed(err.trim().to_string()) });
    }
    #[allow(unreachable_code)]
    {
        let _ = (exe, args);
        Err(Outcome::Unsupported)
    }
}

/// POSIX shell single-quoting.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn sh_quote(a: &str) -> String {
    format!("'{}'", a.replace('\'', r"'\''"))
}

/// Called by the elevated copy once it is up: tells the waiting original.
/// This runs as ROOT: only ever inside a fresh 0700 `cpuzeez-handoff-*`
/// directory the unprivileged original created ([`handoff_marker`]), and only
/// through a new, non-symlink file — a plain `fs::write` to a path from argv
/// would follow a planted symlink and truncate a file of an attacker's
/// choosing (CWE-59).
pub fn signal_handoff(marker: &str) {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
        let p = std::path::Path::new(marker);
        // pkexec tells us which user asked for elevation: the marker directory
        // must be theirs as well (absent when not launched through pkexec,
        // e.g. in tests). Defense in depth on top of the mode/name checks.
        let invoking = std::env::var("PKEXEC_UID").ok().and_then(|u| u.parse::<u32>().ok());
        let dir_ok = |d: &std::path::Path| {
            std::fs::symlink_metadata(d).is_ok_and(|md| {
                md.is_dir()
                    && md.permissions().mode() & 0o777 == 0o700
                    && d.file_name().is_some_and(|n| n.to_string_lossy().starts_with("cpuzeez-handoff-"))
                    && invoking.is_none_or(|uid| md.uid() == uid)
            })
        };
        if !p.parent().is_some_and(dir_ok) {
            return;
        }
        let _ = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(p)
            .and_then(|mut f| f.write_all(b"ok"));
    }
    #[cfg(not(unix))]
    {
        // No handoff flow exists outside Linux (Windows `runas` answers
        // synchronously; nothing relaunches with a marker). In particular this
        // must NEVER write a path from argv: the argument parser is
        // case-insensitive while the relaunch argument filter historically
        // matched case-sensitively, so a `--ELEVATED-HANDOFF=<file>` variant
        // could ride along into an administrator copy and truncate any
        // admin-writable file. A no-op cannot write anything.
        let _ = marker;
    }
}

/// A fresh private directory for the handoff marker (see
/// [`crate::tempdir::create`]): the root copy will write into whatever this
/// returns, so it must be a directory no other local user can reach.
#[cfg(target_os = "linux")]
fn handoff_marker() -> Option<PathBuf> {
    crate::tempdir::sweep("cpuzeez-handoff-", std::time::Duration::from_secs(86_400));
    crate::tempdir::create("cpuzeez-handoff").ok().map(|d| d.join("up"))
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
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetExitCodeProcess, INFINITE, OpenProcessToken, WaitForSingleObject,
    };
    use windows_sys::Win32::UI::Shell::{
        SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, SW_SHOWNORMAL};

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

    /// UAC prompt, run `exe args` elevated and hidden, wait for its exit code.
    pub fn runas_wait(exe: &std::path::Path, args: &[String]) -> Result<i32, Outcome> {
        let (verb, file) = (wide("runas"), wide(exe));
        let params = wide(args.iter().map(|a| quote_arg(a)).collect::<Vec<_>>().join(" "));
        // SAFETY: buffers outlive the call; the process handle is waited on and closed.
        unsafe {
            let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            info.fMask = SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS;
            info.lpVerb = verb.as_ptr();
            info.lpFile = file.as_ptr();
            info.lpParameters = params.as_ptr();
            info.nShow = SW_HIDE;
            if ShellExecuteExW(&mut info) == 0 {
                return Err(if GetLastError() == ERROR_CANCELLED {
                    Outcome::Cancelled
                } else {
                    Outcome::Failed(std::io::Error::last_os_error().to_string())
                });
            }
            if info.hProcess.is_null() {
                return Err(Outcome::Failed("no process handle".into()));
            }
            WaitForSingleObject(info.hProcess, INFINITE);
            let mut code = 1u32;
            GetExitCodeProcess(info.hProcess, &mut code);
            CloseHandle(info.hProcess);
            Ok(code as i32)
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
    fn posix_quoting() {
        assert_eq!(sh_quote("/Applications/CpuZeeZ.app/x"), "'/Applications/CpuZeeZ.app/x'");
        assert_eq!(sh_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn handoff_marker_reports_started() {
        #[cfg(target_os = "linux")]
        let marker = handoff_marker().expect("a private handoff directory");
        // macOS/BSD: signal_handoff's disciplined write only accepts a marker
        // inside a fresh 0700 `cpuzeez-handoff-*` directory — build the test
        // one by hand, so this platform exercises the validation, not skip it.
        #[cfg(all(unix, not(target_os = "linux")))]
        let marker = {
            use std::os::unix::fs::DirBuilderExt;
            let d = std::env::temp_dir().join(format!("cpuzeez-handoff-test-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::DirBuilder::new().mode(0o700).create(&d).unwrap();
            d.join("up")
        };
        // Windows: no handoff flow (runas answers synchronously); a scratch
        // directory stands in so the cleanup below cannot touch %TEMP% itself.
        #[cfg(not(unix))]
        let marker = {
            let d = std::env::temp_dir().join(format!("cpuzeez-handoff-test-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            d.join("up")
        };
        #[cfg(target_os = "linux")]
        let child = std::process::Command::new("sleep").arg("5").spawn().unwrap();
        let mut p = Pending {
            #[cfg(target_os = "linux")]
            child,
            marker: Some(marker.clone()),
            done: None,
        };
        #[cfg(not(target_os = "linux"))]
        assert_eq!(p.poll(), None, "no marker yet");
        #[cfg(unix)]
        signal_handoff(marker.to_str().unwrap());
        #[cfg(not(unix))]
        // Windows has no handoff flow (signal_handoff is a no-op there by
        // design); write the marker directly to exercise the waiting side.
        std::fs::write(&marker, b"ok").unwrap();
        assert_eq!(p.poll(), Some(Outcome::Started));
        assert!(!marker.exists(), "marker cleaned up");
        assert!(!marker.parent().unwrap().exists(), "the handoff directory is reclaimed");
        #[cfg(target_os = "linux")]
        let _ = p.child.kill();
    }

    /// The root-side write is disciplined: nothing outside a fresh private
    /// `cpuzeez-handoff-*` directory is ever written, and a symlink planted
    /// at the marker name is not followed (CWE-59).
    #[test]
    #[cfg(target_os = "linux")]
    fn signal_handoff_refuses_foreign_and_planted_paths() {
        // A path in plain /tmp (or anywhere non-private) must not be written.
        let foreign = std::env::temp_dir().join(format!("cpuzeez-handoff-evil-{}", std::process::id()));
        signal_handoff(foreign.to_str().unwrap());
        assert!(!foreign.exists(), "must not write outside a private handoff directory");

        // A valid private directory, but a symlink planted at the marker name.
        let marker = handoff_marker().expect("a private handoff directory");
        let dir = marker.parent().unwrap().to_path_buf();
        let bait = dir.parent().unwrap().join(format!("cpuzeez-handoff-bait-{}", std::process::id()));
        std::fs::write(&bait, b"precious").unwrap();
        std::os::unix::fs::symlink(&bait, &marker).unwrap();
        signal_handoff(marker.to_str().unwrap());
        assert_eq!(std::fs::read_to_string(&bait).unwrap(), "precious", "a planted symlink must not be followed");
        assert!(
            std::fs::symlink_metadata(&marker).is_ok_and(|m| m.file_type().is_symlink()),
            "the entry at the marker name must still be the planted symlink, not a written file"
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&bait);
    }

    /// The waiting side only accepts a plain file as the signal: a symlink at
    /// the marker name (even a dangling one) must not report `Started`.
    #[test]
    #[cfg(target_os = "linux")]
    fn poll_ignores_a_planted_or_dangling_symlink_marker() {
        let marker = handoff_marker().expect("a private handoff directory");
        let dir = marker.parent().unwrap();
        let bait = dir.parent().unwrap().join(format!("cpuzeez-handoff-bait-{}", std::process::id())); // deliberately absent
        std::os::unix::fs::symlink(&bait, &marker).unwrap();
        let child = std::process::Command::new("sleep").arg("5").spawn().unwrap();
        let mut p = Pending { child, marker: Some(marker.clone()), done: None };
        assert_eq!(p.poll(), None, "a symlink at the marker name is not a signal");
        assert!(std::fs::symlink_metadata(&marker).is_ok_and(|m| m.file_type().is_symlink()), "untouched");
        let _ = p.child.kill();
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Two handoff directories never share a name, and they are private.
    #[test]
    #[cfg(target_os = "linux")]
    fn handoff_dirs_are_fresh_private_and_unique() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let (a, b) = (handoff_marker().unwrap(), handoff_marker().unwrap());
        let (ok_a, ok_b) = (a.parent().unwrap(), b.parent().unwrap());
        let check = |d: &std::path::Path| {
            let md = std::fs::symlink_metadata(d).unwrap();
            // SAFETY: geteuid takes no arguments, has no preconditions and cannot fail.
            md.is_dir() && md.uid() == unsafe { libc::geteuid() } && md.permissions().mode() & 0o777 == 0o700
        };
        let (good, distinct) = (check(ok_a) && check(ok_b), ok_a != ok_b);
        let _ = std::fs::remove_dir_all(ok_a);
        let _ = std::fs::remove_dir_all(ok_b);
        assert!(good, "handoff directories must be 0700 and owned by us");
        assert!(distinct, "each relaunch must get its own unpredictable directory");
    }

    #[test]
    fn elevation_query_does_not_crash() {
        let _ = is_elevated();
        let _ = can_relaunch();
    }
}

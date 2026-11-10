//! Runs `cpuzeez_core::actions::Action`s (docs/v1/engineering.md §4.2):
//! in-process when the app already has the privilege, otherwise in an elevated
//! one-shot copy (`--elevated-op=`), one prompt per action. Every action that
//! ran (or failed) goes to the local audit log; nothing leaves the machine.

use cpuzeez_core::actions::{self, Action, AuditEntry, Privilege};
use cpuzeez_platform::elevation::{self, Outcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Done {
    Ok(String),
    /// The user declined the elevation prompt (not logged: nothing ran).
    Cancelled,
    Failed(String),
}

/// Run `a`, elevating for this one action if needed. Blocks while the OS
/// prompt is open, so call it off the UI thread.
pub fn run(a: &Action) -> Done {
    // The same checks the elevated entry point gets through `decode`.
    if let Err(e) = a.validate() {
        return Done::Failed(e);
    }
    let elevate = a.privilege() == Privilege::Admin && !elevation::is_elevated();
    let done = if elevate {
        match elevation::run(&[format!("{}{}", actions::ELEVATED_FLAG, actions::encode(a))]) {
            Ok(0) => Done::Ok(String::new()),
            Ok(code) if code > OS_ERROR => Done::Failed(format!("{} failed (error {})", a.describe(), code - OS_ERROR)),
            Ok(code) => Done::Failed(format!("the elevated copy exited with code {code}")),
            Err(Outcome::Cancelled) => Done::Cancelled,
            Err(Outcome::Unsupported) => Done::Failed("not possible on this OS".into()),
            Err(Outcome::Failed(e)) => Done::Failed(e),
            Err(Outcome::Started) => Done::Failed("unexpected elevation result".into()),
        }
    } else {
        match perform(a) {
            Ok(s) => Done::Ok(s),
            Err(e) => Done::Failed(e),
        }
    };
    if done != Done::Cancelled {
        audit(a, elevate, &done);
    }
    done
}

/// `--elevated-op=<hex>`: the elevated one-shot entry point. It accepts only
/// what `actions::decode` returns and only runs when actually elevated.
/// Exit code: 0 done, 1 failed, 2 refused (not an action), 3 not elevated,
/// `OS_ERROR + n` failed with the OS error `n` (so the launching copy, which
/// sees only the exit code, can still say why).
pub const OS_ERROR: i32 = 1_000_000;
pub fn elevated_main(arg: &str) -> i32 {
    let a = match actions::decode(arg) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("refused: {e}");
            return 2;
        }
    };
    if !elevation::is_elevated() {
        eprintln!("refused: {} needs administrator rights", a.describe());
        return 3;
    }
    match perform(&a) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{}: {e}", a.describe());
            os_error(&e).map_or(1, |n| OS_ERROR + n as i32)
        }
    }
}

/// The OS error code in a message built as "… failed (error N)".
pub fn os_error(msg: &str) -> Option<u32> {
    let tail = msg.rsplit_once("(error ")?.1;
    tail.strip_suffix(')')?.parse().ok().filter(|&n: &u32| n < 1_000_000)
}

/// The work itself, already holding whatever privilege it needs.
fn perform(a: &Action) -> Result<String, String> {
    match a {
        Action::LoadKernelModule { module } => load_kernel_module(module.name()),
        Action::InstallPawnIo => install_pawnio(),
        Action::Process { target, identity, .. } => {
            verify_identity(target, identity)?;
            perform_process(target)
        }
        Action::Service { target, .. } => cpuzeez_platform::services::apply(target),
        Action::Startup { target, .. } => cpuzeez_platform::startup::apply(target),
    }
}

/// Each target PID must still belong to the process the user picked: same
/// start time. A PID freed and reused while a prompt was open is refused.
fn verify_identity(op: &actions::ProcessOp, identity: &[(u32, u64)]) -> Result<(), String> {
    let now = cpuzeez_platform::procs::snapshot(false);
    for pid in op.pids() {
        let want = identity.iter().find(|i| i.0 == pid).map(|i| i.1).ok_or("missing process identity")?;
        match now.iter().find(|p| p.pid == pid) {
            Some(p) if p.start_ms.abs_diff(want) <= 1 => {
                if matches!(op, actions::ProcessOp::Dump { .. }) && cpuzeez_core::procs::anti_cheat(&p.name) {
                    return Err(format!("{} is protected by an anti-cheat: CPU ZeeZ never reads its memory", p.name));
                }
            }
            Some(_) => return Err(format!("process {pid} has exited; its PID now belongs to another process")),
            None => return Err(format!("process {pid} has exited")),
        }
    }
    Ok(())
}

fn perform_process(op: &actions::ProcessOp) -> Result<String, String> {
    use actions::ProcessOp;
    use cpuzeez_platform::proc_ops;
    match op {
        ProcessOp::End { pids, force } => {
            if pids.is_empty() {
                return Err("no process selected".into());
            }
            for &pid in pids {
                if *force {
                    proc_ops::kill(pid)?;
                } else {
                    proc_ops::end(pid)?;
                }
            }
            Ok(format!("ended {} process(es)", pids.len()))
        }
        ProcessOp::Suspend { pid, on } => {
            proc_ops::suspend(*pid, *on)?;
            Ok(if *on { format!("suspended {pid}") } else { format!("resumed {pid}") })
        }
        ProcessOp::Priority { pid, priority } => {
            proc_ops::set_priority(*pid, *priority)?;
            Ok(format!("priority of {pid} set to {}", priority.label()))
        }
        ProcessOp::Affinity { pid, mask } => {
            proc_ops::set_affinity(*pid, *mask)?;
            Ok(format!("affinity of {pid} set to {mask:#x}"))
        }
        ProcessOp::Eco { pid, on } => {
            proc_ops::eco(*pid, *on)?;
            Ok(if *on { format!("Efficiency mode on for {pid}") } else { format!("Efficiency mode off for {pid}") })
        }
        ProcessOp::Virtualization { pid, on } => {
            proc_ops::set_virtualization(*pid, *on)?;
            Ok(format!("UAC virtualization of {pid} set to {on}"))
        }
        ProcessOp::RestartShell { pid } => {
            let name = cpuzeez_platform::procs::snapshot(false).into_iter().find(|p| p.pid == *pid).map(|p| p.name);
            if !name.is_some_and(|n| n.eq_ignore_ascii_case("explorer.exe")) {
                return Err(format!("process {pid} is not Explorer"));
            }
            proc_ops::restart_shell(*pid)?;
            Ok("Explorer restarted".into())
        }
        ProcessOp::Dump { pid } => {
            let dir = std::env::temp_dir();
            let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
            // The elevated copy writes here, in the user's own temp folder: an
            // unpredictable name and an exclusive create (in proc_ops::dump) so
            // nothing planted there beforehand is overwritten or followed.
            let path = dir.join(format!("cpuzeez-pid{pid}-{ts}-{}.dmp", cpuzeez_platform::tempdir::token()));
            proc_ops::dump(*pid, &path)?;
            Ok(path.display().to_string())
        }
    }
}

/// `KEY=value` from packaging/pins.txt, compiled in so a swapped file next
/// to the exe can't change what we trust.
pub fn pin(key: &str) -> &'static str {
    include_str!("../packaging/pins.txt")
        .lines()
        .find_map(|l| l.strip_prefix(key)?.strip_prefix('='))
        .map(str::trim)
        .unwrap_or_default()
}

/// The PawnIO installer the MSI and the portable zip put next to the exe.
pub fn bundled_pawnio() -> Option<std::path::PathBuf> {
    let p = std::env::current_exe().ok()?.parent()?.join("PawnIO_setup.exe");
    p.is_file().then_some(p)
}

#[cfg(windows)]
fn install_pawnio() -> Result<String, String> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;
    let path = bundled_pawnio().ok_or("PawnIO_setup.exe is not next to cpuzeez.exe")?;
    // Held open without write/delete sharing from the hash check until the
    // installer exits: in a user-writable (portable) folder nobody can swap
    // the file in between.
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let got = crate::update::sha256_of(&mut f).map_err(|e| format!("{}: {e}", path.display()))?;
    let want = pin("PAWNIO_SETUP_SHA256");
    if !got.eq_ignore_ascii_case(want) {
        return Err(format!(
            "PawnIO_setup.exe doesn't match the pinned SHA-256 (expected {want}, got {got}); not running it"
        ));
    }
    let st = std::process::Command::new(&path)
        .args(["-install", "-silent"])
        .status()
        .map_err(|e| format!("PawnIO_setup.exe: {e}"))?;
    drop(f);
    if st.success() {
        Ok(format!("PawnIO {} installed", pin("PAWNIO_SETUP_VERSION")))
    } else {
        Err(format!("PawnIO_setup.exe exited with {st}"))
    }
}

#[cfg(not(windows))]
fn install_pawnio() -> Result<String, String> {
    Err("PawnIO is a Windows driver".into())
}

#[cfg(target_os = "linux")]
fn load_kernel_module(name: &str) -> Result<String, String> {
    // pkexec resets PATH; modprobe lives in sbin.
    let exe = ["/usr/sbin/modprobe", "/sbin/modprobe"]
        .into_iter()
        .find(|p| std::path::Path::new(p).exists())
        .unwrap_or("modprobe");
    let o = std::process::Command::new(exe).arg(name).output().map_err(|e| format!("{exe}: {e}"))?;
    if o.status.success() {
        Ok(format!("loaded {name}"))
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

#[cfg(not(target_os = "linux"))]
fn load_kernel_module(_name: &str) -> Result<String, String> {
    Err("kernel modules are a Linux feature".into())
}

fn audit_path() -> Option<std::path::PathBuf> {
    crate::settings::dir().map(|d| d.join("audit.log"))
}

/// Append one line to the local audit log (JSON Lines).
fn audit(a: &Action, elevated: bool, done: &Done) {
    use std::io::Write;
    let (ok, detail) = match done {
        Done::Ok(s) => (true, s.clone()),
        Done::Failed(e) => (false, e.clone()),
        Done::Cancelled => return,
    };
    let e = AuditEntry {
        time: cpuzeez_core::metrics::iso8601(std::time::SystemTime::now()),
        action: a.clone(),
        description: a.describe(),
        elevated,
        ok,
        detail,
    };
    let Some(path) = audit_path() else { return };
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(e.line().as_bytes());
    }
}

/// The local audit log, oldest first (for Settings › Privacy & data).
pub fn audit_log() -> Vec<AuditEntry> {
    let text = audit_path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn os_error_codes_survive_the_elevated_copy() {
        assert_eq!(os_error("OpenServiceW(CodexSandboxService.OpenAI.Codex) failed (error 5)"), Some(5));
        assert_eq!(os_error("ControlService(STOP, x) failed (error 1062)"), Some(1062));
        assert_eq!(os_error("no code here"), None);
        assert_eq!(os_error("odd (error x)"), None);
        assert_eq!(os_error("too big (error 1000000)"), None, "would overflow the exit code range");
    }

    use super::*;

    #[test]
    fn the_elevated_entry_point_refuses_what_isnt_an_action() {
        assert_eq!(elevated_main("not hex"), 2);
        let shell = r#"{"op":"run","cmd":"sh"}"#.bytes().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(elevated_main(&shell), 2);
        // A real action, but this test isn't elevated (CI runs unprivileged).
        if !elevation::is_elevated() {
            let a = Action::LoadKernelModule { module: actions::KernelModule::Msr };
            assert_eq!(elevated_main(&actions::encode(&a)), 3);
        }
    }

    #[test]
    fn pins_are_compiled_in() {
        for k in ["PAWNIO_SETUP_SHA256", "PAWNIO_MODULES_SHA256"] {
            let h = pin(k);
            assert!(h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()), "{k}={h}");
        }
        assert!(pin("PAWNIO_SETUP_URL").starts_with("https://github.com/namazso/"));
        assert_eq!(pin("NOPE"), "");
    }
}

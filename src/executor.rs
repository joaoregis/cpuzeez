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
    let elevate = a.privilege() == Privilege::Admin && !elevation::is_elevated();
    let done = if elevate {
        match elevation::run(&[format!("{}{}", actions::ELEVATED_FLAG, actions::encode(a))]) {
            Ok(0) => Done::Ok(String::new()),
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
/// Exit code: 0 done, 1 failed, 2 refused (not an action), 3 not elevated.
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
            1
        }
    }
}

/// The work itself, already holding whatever privilege it needs.
fn perform(a: &Action) -> Result<String, String> {
    match a {
        Action::LoadKernelModule { module } => load_kernel_module(module.name()),
    }
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
}

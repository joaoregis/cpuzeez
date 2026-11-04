//! Actions as data (docs/v1/engineering.md §4.2).
//!
//! The GUI describes what it wants done as an [`Action`]; the executor runs
//! it in-process when the app already has the privilege, or in an elevated
//! one-shot copy of the app otherwise (`cpuzeez --elevated-op=<hex>`, per-
//! action elevation, docs/adr/0002). The elevated entry point accepts only
//! what [`decode`] returns: a closed enum whose fields are enums too, so it
//! can't be asked to run an arbitrary program, path or command. Every executed
//! action is written to a local audit log ([`AuditEntry`], P1: never uploaded).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KernelModule {
    /// `/dev/cpu/N/msr`: model-specific registers.
    Msr,
    /// `/dev/i2c-N`: SMBus access for SPD.
    I2cDev,
}

impl KernelModule {
    pub fn name(self) -> &'static str {
        match self {
            KernelModule::Msr => "msr",
            KernelModule::I2cDev => "i2c-dev",
        }
    }
}

/// Everything the app can be asked to do with side effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Action {
    /// Linux: load a kernel module that exposes hardware to CpuZeeZ.
    LoadKernelModule { module: KernelModule },
    /// Windows: run the PawnIO installer shipped next to the exe, after
    /// checking it against the SHA-256 pinned at build time.
    InstallPawnIo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Privilege {
    User,
    /// Administrator (Windows) or root (Unix).
    Admin,
}

/// How strongly the GUI must confirm before running an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    Normal,
    /// Affects the whole system (e.g. a critical process or service).
    Critical,
    /// Can't be undone.
    Irreversible,
}

impl Action {
    pub fn privilege(&self) -> Privilege {
        match self {
            Action::LoadKernelModule { .. } | Action::InstallPawnIo => Privilege::Admin,
        }
    }

    pub fn risk(&self) -> Risk {
        match self {
            Action::LoadKernelModule { .. } => Risk::Normal,
            // Installs a kernel driver for the whole machine.
            Action::InstallPawnIo => Risk::Critical,
        }
    }

    /// One line for confirmations and the audit log (English).
    pub fn describe(&self) -> String {
        match self {
            Action::LoadKernelModule { module } => format!("Load the {} kernel module", module.name()),
            Action::InstallPawnIo => "Install the PawnIO driver".into(),
        }
    }
}

/// The command-line flag of the elevated entry point.
pub const ELEVATED_FLAG: &str = "--elevated-op=";

/// Anything bigger than this is not an action we wrote.
const MAX_ENCODED: usize = 4096;

/// Hex of the action's JSON: survives every OS's command-line quoting.
pub fn encode(a: &Action) -> String {
    serde_json::to_vec(a).unwrap_or_default().iter().map(|b| format!("{b:02x}")).collect()
}

/// The only way into the elevated entry point. Rejects anything that isn't
/// exactly an [`Action`] we know.
pub fn decode(s: &str) -> Result<Action, String> {
    if s.len() > MAX_ENCODED {
        return Err(format!("argument too long ({} bytes)", s.len()));
    }
    if !s.len().is_multiple_of(2) || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("argument is not hex".into());
    }
    let bytes: Vec<u8> = (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0)).collect();
    serde_json::from_slice(&bytes).map_err(|e| format!("not a known action: {e}"))
}

/// One line of the local audit log (JSON Lines).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// ISO-8601 UTC.
    pub time: String,
    pub action: Action,
    pub description: String,
    /// Ran in an elevated copy.
    pub elevated: bool,
    pub ok: bool,
    /// What happened, or the error.
    pub detail: String,
}

impl AuditEntry {
    pub fn line(&self) -> String {
        serde_json::to_string(self).unwrap_or_default() + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> String {
        s.bytes().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn round_trip() {
        let a = Action::LoadKernelModule { module: KernelModule::Msr };
        assert_eq!(decode(&encode(&a)), Ok(a.clone()));
        assert_eq!(a.privilege(), Privilege::Admin);
        assert_eq!(a.describe(), "Load the msr kernel module");
        assert_eq!(decode(&encode(&Action::InstallPawnIo)), Ok(Action::InstallPawnIo));
    }

    #[test]
    fn rejects_anything_else() {
        // Not hex, odd length, too long.
        assert!(decode("zz").is_err());
        assert!(decode("abc").is_err());
        assert!(decode(&"00".repeat(MAX_ENCODED)).is_err());
        // Unknown op, unknown module, extra fields, a free-form command.
        for json in [
            r#"{"op":"run-program","path":"/bin/sh"}"#,
            r#"{"op":"load-kernel-module","module":"evil"}"#,
            r#"{"op":"load-kernel-module","module":"msr","args":"-f"}"#,
            r#"{"op":"load-kernel-module"}"#,
            r#"{"op":"install-paw-nio","path":"C:\x.exe"}"#,
            r#"{"op":"install-pawn-io","path":"C:\x.exe"}"#,
            r#""load-kernel-module""#,
            "",
        ] {
            assert!(decode(&hex(json)).is_err(), "accepted {json}");
        }
    }

    #[test]
    fn audit_lines_are_json() {
        let e = AuditEntry {
            time: "2026-11-04T12:00:00.000Z".into(),
            action: Action::LoadKernelModule { module: KernelModule::I2cDev },
            description: "Load the i2c-dev kernel module".into(),
            elevated: true,
            ok: true,
            detail: "loaded".into(),
        };
        let l = e.line();
        assert!(l.ends_with('\n') && !l[..l.len() - 1].contains('\n'));
        let back: AuditEntry = serde_json::from_str(&l).unwrap();
        assert_eq!(back.action, e.action);
    }
}

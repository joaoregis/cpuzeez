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

/// Windows priority classes (Unix maps them to nice values).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
}

impl Priority {
    pub const ALL: [Priority; 6] = [
        Priority::Idle,
        Priority::BelowNormal,
        Priority::Normal,
        Priority::AboveNormal,
        Priority::High,
        Priority::Realtime,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Priority::Idle => "Low",
            Priority::BelowNormal => "Below normal",
            Priority::Normal => "Normal",
            Priority::AboveNormal => "Above normal",
            Priority::High => "High",
            Priority::Realtime => "Realtime",
        }
    }

    /// Unix nice value.
    pub fn nice(self) -> i32 {
        match self {
            Priority::Idle => 19,
            Priority::BelowNormal => 10,
            Priority::Normal => 0,
            Priority::AboveNormal => -5,
            Priority::High => -10,
            Priority::Realtime => -20,
        }
    }

    /// From a Windows base priority (4, 6, 8, 10, 13, 24) or a Unix nice.
    pub fn from_base(base: i32, unix: bool) -> Priority {
        if unix {
            return match base {
                n if n >= 15 => Priority::Idle,
                n if n >= 5 => Priority::BelowNormal,
                n if n > -5 => Priority::Normal,
                n if n > -10 => Priority::AboveNormal,
                n if n > -20 => Priority::High,
                _ => Priority::Realtime,
            };
        }
        match base {
            ..=4 => Priority::Idle,
            5..=6 => Priority::BelowNormal,
            7..=8 => Priority::Normal,
            9..=10 => Priority::AboveNormal,
            11..=15 => Priority::High,
            _ => Priority::Realtime,
        }
    }
}

/// Process operations (S2a-03): closed enum of typed operations, never a free-form command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProcessOp {
    End { pids: Vec<u32>, force: bool },
    Suspend { pid: u32, on: bool },
    Priority { pid: u32, priority: Priority },
    Affinity { pid: u32, mask: u64 },
    Eco { pid: u32, on: bool },
    Virtualization { pid: u32, on: bool },
    Dump { pid: u32 },
}

/// Service operations (S2a-07): closed enum of typed operations, never a free-form command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ServiceOp {
    Start { name: String },
    Stop { name: String },
    Pause { name: String, resume: bool },
    Restart { name: String },
    SetStartup { name: String, startup: crate::services::StartupType },
}

/// Startup entry operations (S2a-08): reversible enable/disable via `StartupApproved` / XDG / launchctl.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum StartupOp {
    SetEnabled { id: String, source: crate::startup::StartupSource, enabled: bool },
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
    /// Act on one or more processes (S2a-03). `elevate` is set when the
    /// target belongs to another user or an in-process attempt returned access denied.
    Process { target: ProcessOp, elevate: bool },
    /// Act on a system service (S2a-07).
    Service { target: ServiceOp, elevate: bool },
    /// Enable or disable a startup entry (S2a-08).
    Startup { target: StartupOp, elevate: bool },
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
            Action::Process { elevate: true, .. }
            | Action::Service { elevate: true, .. }
            | Action::Startup { elevate: true, .. } => Privilege::Admin,
            Action::Process { elevate: false, .. }
            | Action::Service { elevate: false, .. }
            | Action::Startup { elevate: false, .. } => Privilege::User,
        }
    }

    pub fn risk(&self) -> Risk {
        match self {
            Action::LoadKernelModule { .. } => Risk::Normal,
            // Installs a kernel driver for the whole machine.
            Action::InstallPawnIo => Risk::Critical,
            Action::Process { target: ProcessOp::End { pids, .. }, .. } => {
                if pids.iter().any(|&p| crate::procs::critical("", p)) { Risk::Critical } else { Risk::Irreversible }
            }
            Action::Process { target: ProcessOp::Priority { priority: Priority::Realtime, .. }, .. } => Risk::Critical,
            Action::Process { .. } | Action::Startup { .. } => Risk::Normal,
            Action::Service { target, .. } => {
                let name = match target {
                    ServiceOp::Start { name }
                    | ServiceOp::Stop { name }
                    | ServiceOp::Pause { name, .. }
                    | ServiceOp::Restart { name }
                    | ServiceOp::SetStartup { name, .. } => name.as_str(),
                };
                if crate::services::is_protected(name) { Risk::Critical } else { Risk::Normal }
            }
        }
    }

    /// One line for confirmations and the audit log (English).
    pub fn describe(&self) -> String {
        match self {
            Action::LoadKernelModule { module } => format!("Load the {} kernel module", module.name()),
            Action::InstallPawnIo => "Install the PawnIO driver".into(),
            Action::Process { target, .. } => match target {
                ProcessOp::End { pids, force } => {
                    let verb = if *force { "Force-kill" } else { "End" };
                    if pids.len() == 1 {
                        format!("{verb} process {}", pids[0])
                    } else {
                        format!("{verb} {} processes", pids.len())
                    }
                }
                ProcessOp::Suspend { pid, on: true } => format!("Suspend process {pid}"),
                ProcessOp::Suspend { pid, on: false } => format!("Resume process {pid}"),
                ProcessOp::Priority { pid, priority } => format!("Set process {pid} priority to {}", priority.label()),
                ProcessOp::Affinity { pid, mask } => format!("Set process {pid} CPU affinity to {mask:#x}"),
                ProcessOp::Eco { pid, on: true } => format!("Enable Efficiency mode for process {pid}"),
                ProcessOp::Eco { pid, on: false } => format!("Disable Efficiency mode for process {pid}"),
                ProcessOp::Virtualization { pid, on } => format!("Set UAC virtualization for process {pid} to {on}"),
                ProcessOp::Dump { pid } => format!("Create memory dump for process {pid}"),
            },
            Action::Service { target, .. } => match target {
                ServiceOp::Start { name } => format!("Start service {name}"),
                ServiceOp::Stop { name } => format!("Stop service {name}"),
                ServiceOp::Pause { name, resume: false } => format!("Pause service {name}"),
                ServiceOp::Pause { name, resume: true } => format!("Resume service {name}"),
                ServiceOp::Restart { name } => format!("Restart service {name}"),
                ServiceOp::SetStartup { name, startup } => {
                    format!("Set service {name} startup type to {}", startup.label())
                }
            },
            Action::Startup { target: StartupOp::SetEnabled { id, enabled, .. }, .. } => {
                if *enabled {
                    format!("Enable startup entry {id}")
                } else {
                    format!("Disable startup entry {id}")
                }
            }
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
        let p = Action::Process { target: ProcessOp::End { pids: vec![1234, 5678], force: false }, elevate: true };
        assert_eq!(decode(&encode(&p)), Ok(p.clone()));
        assert_eq!(p.privilege(), Privilege::Admin);
        assert_eq!(p.risk(), Risk::Irreversible);
        assert_eq!(p.describe(), "End 2 processes");
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
            r#"{"op":"process","target":{"kind":"run","cmd":"sh"},"elevate":true}"#,
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

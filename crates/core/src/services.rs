//! Services model, classification, protected-service list, and dependency graph (S2a-07, docs/v1/task-manager.md §8).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceState {
    Running,
    Stopped,
    StartPending,
    StopPending,
    Paused,
    #[default]
    Unknown,
}

impl ServiceState {
    pub fn label(self) -> &'static str {
        match self {
            ServiceState::Running => "Running",
            ServiceState::Stopped => "Stopped",
            ServiceState::StartPending => "Starting",
            ServiceState::StopPending => "Stopping",
            ServiceState::Paused => "Paused",
            ServiceState::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum StartupType {
    Automatic,
    AutomaticDelayed,
    Manual,
    Disabled,
    Static,
    Masked,
    #[default]
    Unknown,
}

impl StartupType {
    pub const EDITABLE: [StartupType; 4] =
        [StartupType::Automatic, StartupType::AutomaticDelayed, StartupType::Manual, StartupType::Disabled];

    pub fn label(self) -> &'static str {
        match self {
            StartupType::Automatic => "Automatic",
            StartupType::AutomaticDelayed => "Automatic (Delayed)",
            StartupType::Manual => "Manual",
            StartupType::Disabled => "Disabled",
            StartupType::Static => "Static",
            StartupType::Masked => "Masked",
            StartupType::Unknown => "Unknown",
        }
    }
}

/// What the service manager does when a service fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryAction {
    #[default]
    None,
    Restart,
    Reboot,
    /// Set in the OS's own tool (it needs a program path); shown, not edited here.
    RunProgram,
}

impl RecoveryAction {
    pub const EDITABLE: [RecoveryAction; 3] = [RecoveryAction::None, RecoveryAction::Restart, RecoveryAction::Reboot];

    pub fn label(self) -> &'static str {
        match self {
            RecoveryAction::None => "Take no action",
            RecoveryAction::Restart => "Restart the service",
            RecoveryAction::Reboot => "Restart the computer",
            RecoveryAction::RunProgram => "Run a program",
        }
    }
}

/// Failure recovery. Windows keeps three steps; systemd one policy (`first`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Recovery {
    pub first: RecoveryAction,
    pub second: RecoveryAction,
    pub subsequent: RecoveryAction,
    /// Reset the failure count after this many days without failures (Windows).
    pub reset_days: u32,
    /// Wait before restarting, seconds.
    pub restart_delay_s: u32,
}

/// The built-in accounts a service can run as without a password. Accounts
/// with a password are set in the OS's own tool: CPU ZeeZ never handles
/// passwords (they would cross the elevation boundary on a command line).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceAccount {
    LocalSystem,
    LocalService,
    NetworkService,
}

impl ServiceAccount {
    pub const ALL: [ServiceAccount; 3] =
        [ServiceAccount::LocalSystem, ServiceAccount::LocalService, ServiceAccount::NetworkService];

    /// The name the Windows SCM stores.
    pub fn windows_name(self) -> &'static str {
        match self {
            ServiceAccount::LocalSystem => "LocalSystem",
            ServiceAccount::LocalService => r"NT AUTHORITY\LocalService",
            ServiceAccount::NetworkService => r"NT AUTHORITY\NetworkService",
        }
    }

    /// The account a stored name means (case-insensitive, with or without the domain).
    pub fn from_name(n: &str) -> Option<ServiceAccount> {
        let short = n.rsplit('\\').next().unwrap_or(n);
        ServiceAccount::ALL.into_iter().find(|a| {
            let w = a.windows_name();
            w.eq_ignore_ascii_case(n) || w.rsplit('\\').next().is_some_and(|x| x.eq_ignore_ascii_case(short))
        })
    }
}

/// What the inspector shows beyond the list, read on demand for one service.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServiceDetails {
    /// "Start: Group policy change", "TriggeredBy: foo.socket"…
    pub triggers: Vec<String>,
    /// Windows service SID type ("None", "Unrestricted", "Restricted").
    pub sid_type: String,
    /// Windows required privileges; Linux capability bounding set.
    pub privileges: Vec<String>,
    pub recovery: Option<Recovery>,
    /// Windows "run a program" command, when set.
    pub recovery_command: String,
}

/// `systemctl show -p TriggeredBy,Restart,RestartUSec,FailureAction,CapabilityBoundingSet` → details.
pub fn parse_systemd_show(text: &str) -> ServiceDetails {
    let get = |k: &str| text.lines().find_map(|l| l.strip_prefix(k)?.strip_prefix('=')).unwrap_or("").trim();
    let restart = !matches!(get("Restart"), "" | "no");
    let reboot = get("FailureAction").contains("reboot");
    let action = if reboot {
        RecoveryAction::Reboot
    } else if restart {
        RecoveryAction::Restart
    } else {
        RecoveryAction::None
    };
    let delay = parse_systemd_secs(get("RestartUSec"));
    let caps: Vec<String> = get("CapabilityBoundingSet").split_whitespace().map(str::to_string).collect();
    ServiceDetails {
        triggers: get("TriggeredBy").split_whitespace().map(|t| format!("TriggeredBy: {t}")).collect(),
        privileges: if caps.len() >= 38 { vec!["all capabilities".into()] } else { caps },
        recovery: Some(Recovery {
            first: action,
            second: action,
            subsequent: action,
            reset_days: 0,
            restart_delay_s: delay,
        }),
        ..Default::default()
    }
}

/// "100ms", "5s", "1min 30s" → whole seconds (rounded up).
pub fn parse_systemd_secs(v: &str) -> u32 {
    let mut ms = 0f64;
    for part in v.split_whitespace() {
        let n: String = part.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
        let unit = &part[n.len()..];
        let x: f64 = n.parse().unwrap_or(0.0);
        ms += x * match unit {
            "us" => 0.001,
            "ms" => 1.0,
            "min" => 60_000.0,
            "h" => 3_600_000.0,
            _ => 1000.0,
        };
    }
    (ms / 1000.0).ceil() as u32
}

/// The systemd drop-in CPU ZeeZ writes for a recovery policy.
pub fn systemd_recovery_dropin(r: &Recovery) -> String {
    let restart = if r.first == RecoveryAction::Restart { "on-failure" } else { "no" };
    let failure = if r.first == RecoveryAction::Reboot { "reboot" } else { "none" };
    format!(
        "# Written by CPU ZeeZ (System > Services > Recovery). Delete this file to undo.\n[Unit]\nFailureAction={failure}\n\n[Service]\nRestart={restart}\nRestartSec={}s\n",
        r.restart_delay_s
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceRow {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub state: ServiceState,
    pub startup: StartupType,
    pub pid: Option<u32>,
    pub account: String,
    pub group: String,
    pub binary_path: String,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub failure_recovery: String,
    pub failed_recently: bool,
    pub last_error: Option<String>,
    pub third_party: bool,
    pub protected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServiceFilter {
    #[default]
    All,
    Running,
    StoppedAuto,
    Disabled,
    ThirdParty,
    FailedRecently,
}

impl ServiceFilter {
    pub const ALL: [ServiceFilter; 6] = [
        ServiceFilter::All,
        ServiceFilter::Running,
        ServiceFilter::StoppedAuto,
        ServiceFilter::Disabled,
        ServiceFilter::ThirdParty,
        ServiceFilter::FailedRecently,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ServiceFilter::All => "All",
            ServiceFilter::Running => "Running",
            ServiceFilter::StoppedAuto => "Stopped (Auto)",
            ServiceFilter::Disabled => "Disabled",
            ServiceFilter::ThirdParty => "Third-party",
            ServiceFilter::FailedRecently => "Failed recently",
        }
    }
}

/// True if stopping or disabling this service can render the OS unbootable or unstable.
pub fn is_protected(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    let stem = lower.strip_suffix(".service").unwrap_or(&lower);
    matches!(
        stem,
        "rpcss"
            | "dcomlaunch"
            | "lsm"
            | "samss"
            | "windefend"
            | "wdnisvc"
            | "mpssvc"
            | "bfe"
            | "trustedinstaller"
            | "eventlog"
            | "plugplay"
            | "power"
            | "profsvc"
            | "gpsvc"
            | "nsi"
            | "dhcp"
            | "dnscache"
            | "cryptsvc"
            | "keyiso"
            | "coremessagingregistrar"
            | "brokerinfrastructure"
            | "systemeventsbroker"
            | "dbus"
            | "systemd-journald"
            | "systemd-udevd"
            | "systemd-logind"
            | "polkit"
            | "com.apple.windowserver"
            | "com.apple.securityd"
            | "com.apple.opendirectoryd"
            | "com.apple.diskarbitrationd"
    )
}

/// Heuristic: true when the service is not shipped by the OS vendor (Microsoft, Apple, or standard Linux system paths).
pub fn is_third_party(name: &str, binary_path: &str) -> bool {
    let p = binary_path.trim().to_ascii_lowercase();
    let n = name.trim().to_ascii_lowercase();
    if n.starts_with("com.apple.") || n.starts_with("systemd-") {
        return false;
    }
    if p.is_empty() {
        return false;
    }
    if p.contains("\\windows\\system32")
        || p.contains("\\windows\\syswow64")
        || p.contains("\\windows\\servicing")
        || p.contains("\\programdata\\microsoft\\windows defender")
    {
        return false;
    }
    if p.starts_with("/usr/lib/systemd")
        || p.starts_with("/lib/systemd")
        || p.starts_with("/usr/sbin/")
        || p.starts_with("/sbin/")
        || p.starts_with("/system/library/")
        || p.starts_with("/usr/libexec/")
    {
        return false;
    }
    true
}

/// Populates `dependents` on every row from the `dependencies` edges.
pub fn populate_dependents(rows: &mut [ServiceRow]) {
    let mut rev: HashMap<String, Vec<String>> = HashMap::new();
    for r in rows.iter() {
        for dep in &r.dependencies {
            rev.entry(dep.to_ascii_lowercase()).or_default().push(r.name.clone());
        }
    }
    for r in rows.iter_mut() {
        if let Some(mut list) = rev.remove(&r.name.to_ascii_lowercase()) {
            list.sort();
            list.dedup();
            r.dependents = list;
        }
    }
}

/// Computes the transitive list of services that depend on `root` (breadth-first, cycle-safe).
pub fn transitive_dependents(rows: &[ServiceRow], root: &str) -> Vec<String> {
    let by_name: HashMap<String, &ServiceRow> = rows.iter().map(|r| (r.name.to_ascii_lowercase(), r)).collect();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let root_key = root.to_ascii_lowercase();
    visited.insert(root_key.clone());
    if let Some(r) = by_name.get(&root_key) {
        for d in &r.dependents {
            queue.push_back(d.clone());
        }
    }
    let mut out = Vec::new();
    while let Some(cur) = queue.pop_front() {
        let key = cur.to_ascii_lowercase();
        if !visited.insert(key.clone()) {
            continue;
        }
        out.push(cur);
        if let Some(r) = by_name.get(&key) {
            for d in &r.dependents {
                queue.push_back(d.clone());
            }
        }
    }
    out
}

pub fn matches_filter(row: &ServiceRow, filter: ServiceFilter, query: &str) -> bool {
    let ok_filter = match filter {
        ServiceFilter::All => true,
        ServiceFilter::Running => row.state == ServiceState::Running,
        ServiceFilter::StoppedAuto => {
            row.state == ServiceState::Stopped
                && matches!(row.startup, StartupType::Automatic | StartupType::AutomaticDelayed)
        }
        ServiceFilter::Disabled => matches!(row.startup, StartupType::Disabled | StartupType::Masked),
        ServiceFilter::ThirdParty => row.third_party,
        ServiceFilter::FailedRecently => row.failed_recently,
    };
    if !ok_filter {
        return false;
    }
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return true;
    }
    row.name.to_ascii_lowercase().contains(&q)
        || row.display_name.to_ascii_lowercase().contains(&q)
        || row.description.to_ascii_lowercase().contains(&q)
        || row.group.to_ascii_lowercase().contains(&q)
        || row.binary_path.to_ascii_lowercase().contains(&q)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_and_third_party_classification() {
        assert!(is_protected("RpcSs"));
        assert!(is_protected("WinDefend"));
        assert!(is_protected("systemd-journald.service"));
        assert!(!is_protected("Spooler"));

        assert!(!is_third_party("Spooler", r"C:\Windows\System32\spoolsv.exe"));
        assert!(is_third_party("MyCustomSvc", r"C:\Program Files\Vendor\svc.exe"));
    }

    #[test]
    fn dependency_chain_is_cycle_safe_and_filters_work() {
        let mut rows = vec![
            ServiceRow {
                name: "A".into(),
                display_name: "Service A".into(),
                state: ServiceState::Running,
                startup: StartupType::Automatic,
                dependencies: vec!["C".into()],
                ..Default::default()
            },
            ServiceRow {
                name: "B".into(),
                display_name: "Service B".into(),
                state: ServiceState::Stopped,
                startup: StartupType::AutomaticDelayed,
                dependencies: vec!["A".into()],
                ..Default::default()
            },
            ServiceRow {
                name: "C".into(),
                display_name: "Service C".into(),
                state: ServiceState::Stopped,
                startup: StartupType::Disabled,
                dependencies: vec!["B".into()],
                ..Default::default()
            },
        ];
        populate_dependents(&mut rows);
        let deps = transitive_dependents(&rows, "A");
        assert_eq!(deps, vec!["B".to_string(), "C".to_string()]);
        assert!(matches_filter(&rows[1], ServiceFilter::StoppedAuto, ""));
        assert!(!matches_filter(&rows[0], ServiceFilter::StoppedAuto, ""));
        assert!(matches_filter(&rows[2], ServiceFilter::Disabled, "service c"));
    }

    #[test]
    fn recovery_and_accounts() {
        let d = parse_systemd_show(
            "TriggeredBy=foo.socket bar.timer\nRestart=on-failure\nRestartUSec=1min 30s\nFailureAction=none\nCapabilityBoundingSet=cap_net_bind_service cap_chown\n",
        );
        assert_eq!(d.triggers, ["TriggeredBy: foo.socket", "TriggeredBy: bar.timer"]);
        let r = d.recovery.unwrap();
        assert_eq!((r.first, r.restart_delay_s), (RecoveryAction::Restart, 90));
        assert_eq!(d.privileges, ["cap_net_bind_service", "cap_chown"]);
        assert_eq!(parse_systemd_secs("100ms"), 1);
        assert_eq!(parse_systemd_secs("5s"), 5);
        let none = parse_systemd_show("Restart=no\nFailureAction=reboot-force\n");
        assert_eq!(none.recovery.unwrap().first, RecoveryAction::Reboot);
        let drop = systemd_recovery_dropin(&Recovery {
            first: RecoveryAction::Restart,
            restart_delay_s: 10,
            ..Default::default()
        });
        assert!(
            drop.contains("Restart=on-failure")
                && drop.contains("RestartSec=10s")
                && drop.contains("FailureAction=none")
        );
        assert_eq!(ServiceAccount::from_name("NT AUTHORITY\\NetworkService"), Some(ServiceAccount::NetworkService));
        assert_eq!(ServiceAccount::from_name("localsystem"), Some(ServiceAccount::LocalSystem));
        assert_eq!(ServiceAccount::from_name(".\\bob"), None);
    }
}

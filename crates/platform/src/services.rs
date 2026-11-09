//! OS service enumeration, logs, and control (S2a-07, docs/v1/task-manager.md §8).
//!
//! - **Windows:** Service Control Manager (`OpenSCManagerW`, `EnumServicesStatusExW`,
//!   `QueryServiceConfigW`, `QueryServiceConfig2W`, `StartServiceW`, `ControlService`,
//!   `ChangeServiceConfigW`, `ChangeServiceConfig2W`).
//! - **Linux:** `systemd` (`systemctl list-units`, `systemctl show`, `journalctl -u`).
//! - **macOS:** `launchd` (`launchctl list`, LaunchDaemons / LaunchAgents plists).

use cpuzeez_core::actions::ServiceOp;
#[cfg(windows)]
use cpuzeez_core::services::{Recovery, RecoveryAction};
use cpuzeez_core::services::{
    ServiceDetails, ServiceRow, ServiceState, StartupType, is_protected, is_third_party, populate_dependents,
};

pub fn list() -> Vec<ServiceRow> {
    let mut rows = imp::list();
    for r in &mut rows {
        r.protected = is_protected(&r.name);
        r.third_party = is_third_party(&r.name, &r.binary_path);
    }
    populate_dependents(&mut rows);
    rows.sort_by_key(|a| a.name.to_ascii_lowercase());
    rows
}

pub fn logs(name: &str, max_lines: usize) -> Vec<String> {
    imp::logs(name, max_lines)
}

/// Triggers, SID type, required privileges and recovery of one service
/// (read when the inspector opens it, not for the whole list).
pub fn details(name: &str) -> ServiceDetails {
    if !cpuzeez_core::actions::valid_service_name(name) {
        return ServiceDetails::default();
    }
    imp::details(name)
}

pub fn apply(op: &ServiceOp) -> Result<String, String> {
    let name = op.name();
    if is_protected(name) {
        return Err(format!("Service '{name}' is protected by the operating system and cannot be modified"));
    }
    imp::apply(op)
}

// ---------------------------------------------------------------------------
// Windows (SCM)
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{ERROR_MORE_DATA, GetLastError};
    use windows_sys::Win32::System::Services::{
        ChangeServiceConfig2W, ChangeServiceConfigW, CloseServiceHandle, ControlService, ENUM_SERVICE_STATUS_PROCESSW,
        EnumServicesStatusExW, OpenSCManagerW, OpenServiceW, QUERY_SERVICE_CONFIGW, QueryServiceConfig2W,
        QueryServiceConfigW, SC_ACTION, SC_ACTION_NONE, SC_ACTION_REBOOT, SC_ACTION_RESTART, SC_ACTION_RUN_COMMAND,
        SC_ENUM_PROCESS_INFO, SC_HANDLE, SC_MANAGER_CONNECT, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_AUTO_START,
        SERVICE_CHANGE_CONFIG, SERVICE_CONFIG_DELAYED_AUTO_START_INFO, SERVICE_CONFIG_DESCRIPTION,
        SERVICE_CONFIG_FAILURE_ACTIONS, SERVICE_CONFIG_REQUIRED_PRIVILEGES_INFO, SERVICE_CONFIG_SERVICE_SID_INFO,
        SERVICE_CONFIG_TRIGGER_INFO, SERVICE_CONTINUE_PENDING, SERVICE_CONTROL_CONTINUE, SERVICE_CONTROL_PAUSE,
        SERVICE_CONTROL_STOP, SERVICE_DELAYED_AUTO_START_INFO, SERVICE_DEMAND_START, SERVICE_DESCRIPTIONW,
        SERVICE_DISABLED, SERVICE_FAILURE_ACTIONSW, SERVICE_NO_CHANGE, SERVICE_PAUSE_CONTINUE, SERVICE_PAUSE_PENDING,
        SERVICE_PAUSED, SERVICE_QUERY_CONFIG, SERVICE_QUERY_STATUS, SERVICE_REQUIRED_PRIVILEGES_INFOW, SERVICE_RUNNING,
        SERVICE_SID_INFO, SERVICE_START, SERVICE_START_PENDING, SERVICE_STATE_ALL, SERVICE_STATUS, SERVICE_STOP,
        SERVICE_STOP_PENDING, SERVICE_STOPPED, SERVICE_TRIGGER, SERVICE_TRIGGER_INFO, SERVICE_WIN32, StartServiceW,
    };

    /// `QueryServiceConfig2W` into a buffer of the size it asks for.
    unsafe fn config2(svc: SC_HANDLE, level: u32) -> Option<Vec<u64>> {
        let mut req = 0u32;
        unsafe {
            let _ = QueryServiceConfig2W(svc, level, null_mut(), 0, &mut req);
            if req == 0 || req > 1 << 20 {
                return None;
            }
            let mut buf = vec![0u64; (req as usize).div_ceil(8)];
            (QueryServiceConfig2W(svc, level, buf.as_mut_ptr().cast(), req, &mut req) != 0).then_some(buf)
        }
    }

    fn trigger_name(t: u32) -> &'static str {
        match t {
            1 => "Device arrival",
            2 => "IP address availability",
            3 => "Domain join",
            4 => "Firewall port event",
            5 => "Group policy change",
            6 => "Network endpoint",
            7 => "System state change",
            20 => "Custom event",
            30 => "Aggregate event",
            _ => "Other",
        }
    }

    fn action_of(t: i32) -> RecoveryAction {
        match t {
            SC_ACTION_RESTART => RecoveryAction::Restart,
            SC_ACTION_REBOOT => RecoveryAction::Reboot,
            SC_ACTION_RUN_COMMAND => RecoveryAction::RunProgram,
            _ => RecoveryAction::None,
        }
    }

    pub fn details(name: &str) -> ServiceDetails {
        let mut d = ServiceDetails::default();
        // SAFETY: SCM handles closed by ScHandle; each buffer is sized by the API.
        unsafe {
            let scm = ScHandle(OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT));
            if scm.0.is_null() {
                return d;
            }
            let w = to_wide(name);
            let svc = ScHandle(OpenServiceW(scm.0, w.as_ptr(), SERVICE_QUERY_CONFIG));
            if svc.0.is_null() {
                return d;
            }
            if let Some(b) = config2(svc.0, SERVICE_CONFIG_TRIGGER_INFO) {
                let info = &*(b.as_ptr() as *const SERVICE_TRIGGER_INFO);
                if !info.pTriggers.is_null() {
                    let ts = std::slice::from_raw_parts(
                        info.pTriggers as *const SERVICE_TRIGGER,
                        info.cTriggers.min(64) as usize,
                    );
                    d.triggers = ts
                        .iter()
                        .map(|t| {
                            let verb = if t.dwAction == 2 { "Stop" } else { "Start" };
                            format!("{verb}: {}", trigger_name(t.dwTriggerType))
                        })
                        .collect();
                }
            }
            if let Some(b) = config2(svc.0, SERVICE_CONFIG_SERVICE_SID_INFO) {
                d.sid_type = match (*(b.as_ptr() as *const SERVICE_SID_INFO)).dwServiceSidType {
                    1 => "Unrestricted",
                    3 => "Restricted",
                    _ => "None",
                }
                .into();
            }
            if let Some(b) = config2(svc.0, SERVICE_CONFIG_REQUIRED_PRIVILEGES_INFO) {
                d.privileges =
                    multi_sz_to_vec((*(b.as_ptr() as *const SERVICE_REQUIRED_PRIVILEGES_INFOW)).pmszRequiredPrivileges);
            }
            if let Some(b) = config2(svc.0, SERVICE_CONFIG_FAILURE_ACTIONS) {
                let fa = &*(b.as_ptr() as *const SERVICE_FAILURE_ACTIONSW);
                let acts: &[SC_ACTION] = if fa.lpsaActions.is_null() {
                    &[]
                } else {
                    std::slice::from_raw_parts(fa.lpsaActions, fa.cActions.min(16) as usize)
                };
                // Windows repeats the last action for later failures.
                let at = |i: usize| acts.get(i).or(acts.last()).map_or(RecoveryAction::None, |a| action_of(a.Type));
                let delay = acts.iter().find(|a| a.Type == SC_ACTION_RESTART).map_or(60_000, |a| a.Delay);
                d.recovery = Some(Recovery {
                    first: at(0),
                    second: at(1),
                    subsequent: at(2),
                    reset_days: fa.dwResetPeriod / 86_400,
                    restart_delay_s: delay / 1000,
                });
                d.recovery_command = pwstr_to_string(fa.lpCommand);
            }
        }
        d
    }

    /// SeShutdownPrivilege, which a reboot recovery action needs the caller to hold.
    fn enable_shutdown_privilege() {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LUID};
        use windows_sys::Win32::Security::{
            AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
            TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
        // SAFETY: our own token; closed below.
        unsafe {
            let mut tok: HANDLE = null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut tok) == 0 {
                return;
            }
            let mut luid = LUID { LowPart: 0, HighPart: 0 };
            let name = to_wide("SeShutdownPrivilege");
            if LookupPrivilegeValueW(null(), name.as_ptr(), &mut luid) != 0 {
                let tp = TOKEN_PRIVILEGES {
                    PrivilegeCount: 1,
                    Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }],
                };
                AdjustTokenPrivileges(tok, 0, &tp, 0, null_mut(), null_mut());
            }
            CloseHandle(tok);
        }
    }

    fn sc_type(a: RecoveryAction) -> i32 {
        match a {
            RecoveryAction::Restart => SC_ACTION_RESTART,
            RecoveryAction::Reboot => SC_ACTION_REBOOT,
            RecoveryAction::RunProgram => SC_ACTION_RUN_COMMAND,
            RecoveryAction::None => SC_ACTION_NONE,
        }
    }

    struct ScHandle(SC_HANDLE);
    impl Drop for ScHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CloseServiceHandle(self.0);
                }
            }
        }
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe fn pwstr_to_string(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut len = 0usize;
        unsafe {
            while *p.add(len) != 0 && len < 8192 {
                len += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(p, len))
        }
    }

    unsafe fn multi_sz_to_vec(p: *const u16) -> Vec<String> {
        if p.is_null() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut cur = p;
        unsafe {
            while *cur != 0 {
                let s = pwstr_to_string(cur);
                let adv = s.encode_utf16().count() + 1;
                if !s.is_empty() && !s.starts_with('+') {
                    out.push(s);
                }
                cur = cur.add(adv);
            }
        }
        out
    }

    pub fn list() -> Vec<ServiceRow> {
        unsafe {
            let scm = ScHandle(OpenSCManagerW(null(), null(), SC_MANAGER_ENUMERATE_SERVICE));
            if scm.0.is_null() {
                return Vec::new();
            }
            let mut needed = 0u32;
            let mut count = 0u32;
            let mut resume = 0u32;
            let _ = EnumServicesStatusExW(
                scm.0,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                null_mut(),
                0,
                &mut needed,
                &mut count,
                &mut resume,
                null(),
            );
            if needed == 0 {
                return Vec::new();
            }
            let mut buf = vec![0u8; needed as usize + 1024];
            resume = 0;
            let ok = EnumServicesStatusExW(
                scm.0,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
                SERVICE_STATE_ALL,
                buf.as_mut_ptr(),
                buf.len() as u32,
                &mut needed,
                &mut count,
                &mut resume,
                null(),
            );
            if ok == 0 && GetLastError() != ERROR_MORE_DATA {
                return Vec::new();
            }
            let entries =
                std::slice::from_raw_parts(buf.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW, count as usize);
            let mut out = Vec::with_capacity(entries.len());
            for e in entries {
                let name = pwstr_to_string(e.lpServiceName);
                if name.is_empty() {
                    continue;
                }
                let display_name = pwstr_to_string(e.lpDisplayName);
                let st = e.ServiceStatusProcess;
                let state = match st.dwCurrentState {
                    SERVICE_RUNNING => ServiceState::Running,
                    SERVICE_STOPPED => ServiceState::Stopped,
                    SERVICE_START_PENDING | SERVICE_CONTINUE_PENDING => ServiceState::StartPending,
                    SERVICE_STOP_PENDING | SERVICE_PAUSE_PENDING => ServiceState::StopPending,
                    SERVICE_PAUSED => ServiceState::Paused,
                    _ => ServiceState::Unknown,
                };
                let pid = (st.dwProcessId > 0 && state == ServiceState::Running).then_some(st.dwProcessId);
                let failed_recently = st.dwWin32ExitCode != 0 && st.dwWin32ExitCode != 1077;
                let last_error = failed_recently.then(|| format!("Win32 exit code {}", st.dwWin32ExitCode));

                let mut row = ServiceRow {
                    name: name.clone(),
                    display_name,
                    state,
                    pid,
                    failed_recently,
                    last_error,
                    ..Default::default()
                };
                enrich_config(scm.0, &name, &mut row);
                out.push(row);
            }
            out
        }
    }

    unsafe fn enrich_config(scm: SC_HANDLE, name: &str, row: &mut ServiceRow) {
        let wide = to_wide(name);
        let svc = unsafe { ScHandle(OpenServiceW(scm, wide.as_ptr(), SERVICE_QUERY_CONFIG)) };
        if svc.0.is_null() {
            return;
        }
        let mut needed = 0u32;
        unsafe {
            let _ = QueryServiceConfigW(svc.0, null_mut(), 0, &mut needed);
            if needed > 0 && needed <= 65536 {
                let mut buf = vec![0u8; needed as usize];
                if QueryServiceConfigW(
                    svc.0,
                    buf.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW,
                    buf.len() as u32,
                    &mut needed,
                ) != 0
                {
                    let cfg = &*(buf.as_ptr() as *const QUERY_SERVICE_CONFIGW);
                    row.binary_path = pwstr_to_string(cfg.lpBinaryPathName);
                    row.account = pwstr_to_string(cfg.lpServiceStartName);
                    row.group = pwstr_to_string(cfg.lpLoadOrderGroup);
                    row.dependencies = multi_sz_to_vec(cfg.lpDependencies);
                    row.startup = match cfg.dwStartType {
                        SERVICE_AUTO_START => StartupType::Automatic,
                        SERVICE_DEMAND_START => StartupType::Manual,
                        SERVICE_DISABLED => StartupType::Disabled,
                        _ => StartupType::Unknown,
                    };
                }
            }
            if row.startup == StartupType::Automatic {
                let mut delayed = SERVICE_DELAYED_AUTO_START_INFO { fDelayedAutostart: 0 };
                let mut req = 0u32;
                if QueryServiceConfig2W(
                    svc.0,
                    SERVICE_CONFIG_DELAYED_AUTO_START_INFO,
                    &mut delayed as *mut _ as *mut u8,
                    std::mem::size_of::<SERVICE_DELAYED_AUTO_START_INFO>() as u32,
                    &mut req,
                ) != 0
                    && delayed.fDelayedAutostart != 0
                {
                    row.startup = StartupType::AutomaticDelayed;
                }
            }
            let mut req = 0u32;
            let _ = QueryServiceConfig2W(svc.0, SERVICE_CONFIG_DESCRIPTION, null_mut(), 0, &mut req);
            if req > 0 && req <= 65536 {
                let mut buf = vec![0u8; req as usize];
                if QueryServiceConfig2W(svc.0, SERVICE_CONFIG_DESCRIPTION, buf.as_mut_ptr(), buf.len() as u32, &mut req)
                    != 0
                {
                    let desc = &*(buf.as_ptr() as *const SERVICE_DESCRIPTIONW);
                    row.description = pwstr_to_string(desc.lpDescription);
                }
            }
            let mut req_fa = 0u32;
            let _ = QueryServiceConfig2W(svc.0, SERVICE_CONFIG_FAILURE_ACTIONS, null_mut(), 0, &mut req_fa);
            if req_fa > 0 && req_fa <= 65536 {
                let mut buf = vec![0u8; req_fa as usize];
                if QueryServiceConfig2W(
                    svc.0,
                    SERVICE_CONFIG_FAILURE_ACTIONS,
                    buf.as_mut_ptr(),
                    buf.len() as u32,
                    &mut req_fa,
                ) != 0
                {
                    let fa = &*(buf.as_ptr() as *const SERVICE_FAILURE_ACTIONSW);
                    if fa.cActions > 0 && !fa.lpsaActions.is_null() {
                        let acts = std::slice::from_raw_parts(
                            fa.lpsaActions as *const SC_ACTION,
                            (fa.cActions as usize).min(3),
                        );
                        let labels: Vec<&str> = acts
                            .iter()
                            .map(|a| match a.Type {
                                SC_ACTION_RESTART => "Restart service",
                                SC_ACTION_REBOOT => "Restart computer",
                                SC_ACTION_RUN_COMMAND => "Run program",
                                _ => "Take no action",
                            })
                            .collect();
                        row.failure_recovery = labels.join(" / ");
                    }
                }
            }
        }
    }

    pub fn logs(name: &str, max_lines: usize) -> Vec<String> {
        let count_arg = format!("/c:{}", max_lines.clamp(1, 30));
        let Some(text) = crate::os::cmd(
            "wevtutil",
            &[
                "qe",
                "System",
                &count_arg,
                "/rd:true",
                "/f:text",
                "/q:*[System[Provider[@Name='Service Control Manager']]]",
            ],
        ) else {
            return Vec::new();
        };
        let needle = name.to_ascii_lowercase();
        text.lines()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty() && (t.to_ascii_lowercase().contains(&needle) || t.starts_with("Date:"))
            })
            .take(max_lines)
            .map(|s| s.trim().to_string())
            .collect()
    }

    pub fn apply(op: &ServiceOp) -> Result<String, String> {
        unsafe {
            let scm = ScHandle(OpenSCManagerW(null(), null(), SC_MANAGER_CONNECT));
            if scm.0.is_null() {
                return Err(format!("OpenSCManagerW failed (error {})", GetLastError()));
            }
            match op {
                ServiceOp::Start { name } => {
                    let wide = to_wide(name);
                    let svc = ScHandle(OpenServiceW(scm.0, wide.as_ptr(), SERVICE_START));
                    if svc.0.is_null() {
                        return Err(format!("OpenServiceW({name}) failed (error {})", GetLastError()));
                    }
                    if StartServiceW(svc.0, 0, null()) == 0 {
                        return Err(format!("StartServiceW({name}) failed (error {})", GetLastError()));
                    }
                    Ok(format!("Started service {name}"))
                }
                ServiceOp::Stop { name } => {
                    let wide = to_wide(name);
                    let svc = ScHandle(OpenServiceW(scm.0, wide.as_ptr(), SERVICE_STOP));
                    if svc.0.is_null() {
                        return Err(format!("OpenServiceW({name}) failed (error {})", GetLastError()));
                    }
                    let mut st: SERVICE_STATUS = std::mem::zeroed();
                    if ControlService(svc.0, SERVICE_CONTROL_STOP, &mut st) == 0 {
                        return Err(format!("ControlService(STOP, {name}) failed (error {})", GetLastError()));
                    }
                    Ok(format!("Stopped service {name}"))
                }
                ServiceOp::Pause { name, resume } => {
                    let wide = to_wide(name);
                    let svc = ScHandle(OpenServiceW(scm.0, wide.as_ptr(), SERVICE_PAUSE_CONTINUE));
                    if svc.0.is_null() {
                        return Err(format!("OpenServiceW({name}) failed (error {})", GetLastError()));
                    }
                    let mut st: SERVICE_STATUS = std::mem::zeroed();
                    let ctrl = if *resume { SERVICE_CONTROL_CONTINUE } else { SERVICE_CONTROL_PAUSE };
                    if ControlService(svc.0, ctrl, &mut st) == 0 {
                        return Err(format!("ControlService({name}) failed (error {})", GetLastError()));
                    }
                    Ok(format!("{} service {name}", if *resume { "Resumed" } else { "Paused" }))
                }
                ServiceOp::Restart { name } => {
                    let wide = to_wide(name);
                    let svc = ScHandle(OpenServiceW(
                        scm.0,
                        wide.as_ptr(),
                        SERVICE_STOP | SERVICE_START | SERVICE_QUERY_STATUS,
                    ));
                    if svc.0.is_null() {
                        return Err(format!("OpenServiceW({name}) failed (error {})", GetLastError()));
                    }
                    let mut st: SERVICE_STATUS = std::mem::zeroed();
                    let _ = ControlService(svc.0, SERVICE_CONTROL_STOP, &mut st);
                    std::thread::sleep(std::time::Duration::from_millis(400));
                    if StartServiceW(svc.0, 0, null()) == 0 {
                        return Err(format!("StartServiceW({name}) failed (error {})", GetLastError()));
                    }
                    Ok(format!("Restarted service {name}"))
                }
                ServiceOp::StartWithParams { name, params } => {
                    let wide = to_wide(name);
                    let svc = ScHandle(OpenServiceW(scm.0, wide.as_ptr(), SERVICE_START));
                    if svc.0.is_null() {
                        return Err(format!("OpenServiceW({name}) failed (error {})", GetLastError()));
                    }
                    let owned: Vec<Vec<u16>> = params.iter().map(|a| to_wide(a)).collect();
                    let argv: Vec<*const u16> = owned.iter().map(|a| a.as_ptr()).collect();
                    if StartServiceW(svc.0, argv.len() as u32, argv.as_ptr()) == 0 {
                        return Err(format!("StartServiceW({name}) failed (error {})", GetLastError()));
                    }
                    Ok(format!("Started service {name} with {} parameter(s)", params.len()))
                }
                ServiceOp::SetAccount { name, account } => {
                    let wide = to_wide(name);
                    let svc = ScHandle(OpenServiceW(scm.0, wide.as_ptr(), SERVICE_CHANGE_CONFIG));
                    if svc.0.is_null() {
                        return Err(format!("OpenServiceW({name}) failed (error {})", GetLastError()));
                    }
                    let acc = to_wide(account.windows_name());
                    // The built-in service accounts take an empty password.
                    let empty = to_wide("");
                    if ChangeServiceConfigW(
                        svc.0,
                        SERVICE_NO_CHANGE,
                        SERVICE_NO_CHANGE,
                        SERVICE_NO_CHANGE,
                        null(),
                        null(),
                        null_mut(),
                        null(),
                        acc.as_ptr(),
                        empty.as_ptr(),
                        null(),
                    ) == 0
                    {
                        return Err(format!("ChangeServiceConfigW({name}) failed (error {})", GetLastError()));
                    }
                    Ok(format!("{name} now runs as {} (takes effect on the next start)", account.windows_name()))
                }
                ServiceOp::SetRecovery { name, recovery: r } => {
                    let wide = to_wide(name);
                    // Restart actions need SERVICE_START on the handle.
                    let svc = ScHandle(OpenServiceW(scm.0, wide.as_ptr(), SERVICE_CHANGE_CONFIG | SERVICE_START));
                    if svc.0.is_null() {
                        return Err(format!("OpenServiceW({name}) failed (error {})", GetLastError()));
                    }
                    if [r.first, r.second, r.subsequent].contains(&RecoveryAction::Reboot) {
                        enable_shutdown_privilege();
                    }
                    let delay = r.restart_delay_s.saturating_mul(1000);
                    let mut acts = [r.first, r.second, r.subsequent].map(|a| SC_ACTION {
                        Type: sc_type(a),
                        Delay: if a == RecoveryAction::None { 0 } else { delay },
                    });
                    let mut fa = SERVICE_FAILURE_ACTIONSW {
                        dwResetPeriod: r.reset_days.saturating_mul(86_400),
                        lpRebootMsg: null_mut(),
                        lpCommand: null_mut(),
                        cActions: 3,
                        lpsaActions: acts.as_mut_ptr(),
                    };
                    if ChangeServiceConfig2W(
                        svc.0,
                        SERVICE_CONFIG_FAILURE_ACTIONS,
                        (&mut fa as *mut SERVICE_FAILURE_ACTIONSW).cast(),
                    ) == 0
                    {
                        return Err(format!("ChangeServiceConfig2W({name}) failed (error {})", GetLastError()));
                    }
                    Ok(format!("Recovery of {name} updated"))
                }
                ServiceOp::SetStartup { name, startup } => {
                    let wide = to_wide(name);
                    let svc = ScHandle(OpenServiceW(scm.0, wide.as_ptr(), SERVICE_CHANGE_CONFIG));
                    if svc.0.is_null() {
                        return Err(format!("OpenServiceW({name}) failed (error {})", GetLastError()));
                    }
                    let (dw_start, delayed) = match startup {
                        StartupType::Automatic => (SERVICE_AUTO_START, false),
                        StartupType::AutomaticDelayed => (SERVICE_AUTO_START, true),
                        StartupType::Manual => (SERVICE_DEMAND_START, false),
                        StartupType::Disabled => (SERVICE_DISABLED, false),
                        other => return Err(format!("Unsupported startup type {:?}", other)),
                    };
                    if ChangeServiceConfigW(
                        svc.0,
                        SERVICE_NO_CHANGE,
                        dw_start,
                        SERVICE_NO_CHANGE,
                        null(),
                        null(),
                        null_mut(),
                        null(),
                        null(),
                        null(),
                        null(),
                    ) == 0
                    {
                        return Err(format!("ChangeServiceConfigW({name}) failed (error {})", GetLastError()));
                    }
                    if dw_start == SERVICE_AUTO_START {
                        let mut info = SERVICE_DELAYED_AUTO_START_INFO { fDelayedAutostart: i32::from(delayed) };
                        let _ = ChangeServiceConfig2W(
                            svc.0,
                            SERVICE_CONFIG_DELAYED_AUTO_START_INFO,
                            &mut info as *mut _ as *mut _,
                        );
                    }
                    Ok(format!("Set {name} startup type to {}", startup.label()))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Linux (systemd)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::collections::HashMap;

    pub fn list() -> Vec<ServiceRow> {
        let mut states: HashMap<String, StartupType> = HashMap::new();
        if let Some(text) =
            crate::os::cmd("systemctl", &["list-unit-files", "--type=service", "--no-pager", "--no-legend"])
        {
            for line in text.lines() {
                let mut parts = line.split_whitespace();
                if let (Some(unit), Some(st)) = (parts.next(), parts.next()) {
                    let startup = match st {
                        "enabled" | "enabled-runtime" => StartupType::Automatic,
                        "disabled" => StartupType::Disabled,
                        "static" | "indirect" => StartupType::Static,
                        "masked" | "masked-runtime" => StartupType::Masked,
                        _ => StartupType::Manual,
                    };
                    states.insert(unit.to_string(), startup);
                }
            }
        }

        let Some(text) = crate::os::cmd(
            "systemctl",
            &["list-units", "--type=service", "--all", "--no-pager", "--no-legend", "--plain"],
        ) else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let Some(unit) = parts.next() else { continue };
            let _load = parts.next();
            let active = parts.next().unwrap_or("");
            let sub = parts.next().unwrap_or("");
            let desc: Vec<&str> = parts.collect();
            let state = match (active, sub) {
                ("active", "running") | ("active", "exited") => ServiceState::Running,
                ("activating", _) => ServiceState::StartPending,
                ("deactivating", _) => ServiceState::StopPending,
                _ => ServiceState::Stopped,
            };
            let startup = states.get(unit).copied().unwrap_or(StartupType::Manual);
            rows.push(ServiceRow {
                name: unit.to_string(),
                display_name: desc.join(" "),
                description: desc.join(" "),
                state,
                startup,
                failed_recently: active == "failed" || sub == "failed",
                last_error: (active == "failed").then(|| "Unit in failed state".to_string()),
                ..Default::default()
            });
        }
        rows
    }

    pub fn logs(name: &str, max_lines: usize) -> Vec<String> {
        let n_str = max_lines.to_string();
        let Some(text) = crate::os::cmd("journalctl", &["-u", name, "-n", &n_str, "--no-pager", "-o", "short-iso"])
        else {
            return Vec::new();
        };
        text.lines().map(|l| l.to_string()).collect()
    }

    pub fn details(name: &str) -> ServiceDetails {
        crate::os::cmd(
            "systemctl",
            &[
                "show",
                "--no-pager",
                "-p",
                "TriggeredBy,Restart,RestartUSec,FailureAction,CapabilityBoundingSet",
                "--",
                name,
            ],
        )
        .map(|t| cpuzeez_core::services::parse_systemd_show(&t))
        .unwrap_or_default()
    }

    pub fn apply(op: &ServiceOp) -> Result<String, String> {
        let (verb, unit): (&str, &str) = match op {
            ServiceOp::StartWithParams { .. } => return Err("systemd units take no start parameters".into()),
            ServiceOp::SetAccount { .. } => {
                return Err("The log-on account is a Windows service setting (systemd: User= in the unit)".into());
            }
            ServiceOp::SetRecovery { name, recovery } => {
                // A drop-in next to the unit: the vendor file stays untouched, deleting ours undoes it.
                let dir = std::path::Path::new("/etc/systemd/system").join(format!("{name}.d"));
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                std::fs::write(
                    dir.join("50-cpuzeez-recovery.conf"),
                    cpuzeez_core::services::systemd_recovery_dropin(recovery),
                )
                .map_err(|e| e.to_string())?;
                let out =
                    std::process::Command::new("systemctl").arg("daemon-reload").output().map_err(|e| e.to_string())?;
                return if out.status.success() {
                    Ok(format!("Recovery of {name} updated"))
                } else {
                    Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
                };
            }
            ServiceOp::Start { name } => ("start", name),
            ServiceOp::Stop { name } => ("stop", name),
            // systemd has no pause; "stop" here would look like a pause that worked.
            ServiceOp::Pause { .. } => return Err("systemd services can't be paused".into()),
            ServiceOp::Restart { name } => ("restart", name),
            ServiceOp::SetStartup { name, startup } => {
                let v = match startup {
                    StartupType::Automatic | StartupType::AutomaticDelayed => "enable",
                    StartupType::Disabled => "disable",
                    StartupType::Masked => "mask",
                    _ => return Err(format!("Unsupported systemd startup transition {:?}", startup)),
                };
                (v, name.as_str())
            }
        };
        // `--`: a unit name is never read as an option.
        let out =
            std::process::Command::new("systemctl").args([verb, "--", unit]).output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(format!("systemctl {verb} {unit}"))
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// macOS (launchd)
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod imp {
    use super::*;

    pub fn list() -> Vec<ServiceRow> {
        let Some(text) = crate::os::cmd("launchctl", &["list"]) else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        for (idx, line) in text.lines().enumerate() {
            if idx == 0 && line.contains("PID") {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(pid_s) = parts.next() else { continue };
            let Some(status_s) = parts.next() else { continue };
            let Some(label) = parts.next() else { continue };
            let pid = pid_s.parse::<u32>().ok();
            let exit_code = status_s.parse::<i32>().unwrap_or(0);
            rows.push(ServiceRow {
                name: label.to_string(),
                display_name: label.to_string(),
                state: if pid.is_some() { ServiceState::Running } else { ServiceState::Stopped },
                startup: StartupType::Automatic,
                pid,
                failed_recently: exit_code != 0,
                last_error: (exit_code != 0).then(|| format!("Last exit status {exit_code}")),
                ..Default::default()
            });
        }
        rows
    }

    pub fn logs(name: &str, max_lines: usize) -> Vec<String> {
        let target = format!("system/{name}");
        let Some(text) = crate::os::cmd("launchctl", &["print", &target]) else {
            return Vec::new();
        };
        text.lines().take(max_lines).map(|l| l.to_string()).collect()
    }

    pub fn details(_name: &str) -> ServiceDetails {
        ServiceDetails::default()
    }

    /// launchd's domain for what `launchctl list` showed: the system's as
    /// root, the logged-in user's GUI domain otherwise.
    fn target(label: &str) -> String {
        // SAFETY: getuid has no preconditions.
        let uid = unsafe { libc::getuid() };
        if uid == 0 { format!("system/{label}") } else { format!("gui/{uid}/{label}") }
    }

    pub fn apply(op: &ServiceOp) -> Result<String, String> {
        // launchctl(1): start/stop take a label; kickstart, enable and disable
        // take a service target (domain/label).
        let args: Vec<String> = match op {
            ServiceOp::Pause { .. } => return Err("launchd services can't be paused".into()),
            ServiceOp::StartWithParams { .. } | ServiceOp::SetAccount { .. } | ServiceOp::SetRecovery { .. } => {
                return Err("launchd keeps this in the job's plist; edit it there".into());
            }
            ServiceOp::Start { name } => vec!["start".into(), name.clone()],
            ServiceOp::Stop { name } => vec!["stop".into(), name.clone()],
            ServiceOp::Restart { name } => vec!["kickstart".into(), "-k".into(), target(name)],
            ServiceOp::SetStartup { name, startup } => {
                let v = if *startup == StartupType::Disabled { "disable" } else { "enable" };
                vec![v.into(), target(name)]
            }
        };
        let out = std::process::Command::new("launchctl").args(&args).output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(format!("launchctl {}", args.join(" ")))
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    use super::*;
    pub fn list() -> Vec<ServiceRow> {
        Vec::new()
    }
    pub fn logs(_name: &str, _max_lines: usize) -> Vec<String> {
        Vec::new()
    }
    pub fn apply(_op: &ServiceOp) -> Result<String, String> {
        Err("Not supported on this platform".into())
    }
    pub fn details(_name: &str) -> ServiceDetails {
        ServiceDetails::default()
    }
}

#[cfg(test)]
mod tests {
    /// Contract: every listed service has a valid name, and the OS's own
    /// service manager answers (CI runners have one on every OS).
    #[test]
    fn lists_services_with_valid_names() {
        let all = super::list();
        if cfg!(any(windows, target_os = "macos")) {
            assert!(all.len() > 10, "{} services", all.len());
        }
        for s in &all {
            assert!(!s.name.is_empty(), "{s:?}");
        }
        if cfg!(windows) {
            assert!(all.iter().any(|s| s.name.eq_ignore_ascii_case("EventLog")), "the event log service");
        }
        // Protected services are refused before the OS is asked.
        if let Some(p) = all.iter().find(|s| super::is_protected(&s.name)) {
            let op = cpuzeez_core::actions::ServiceOp::Stop { name: p.name.clone() };
            assert!(super::apply(&op).is_err());
        }
    }

    /// Contract: the inspector's details answer for a real service; on
    /// Windows the event log has a SID type and a recovery policy.
    #[test]
    fn details_of_a_real_service() {
        assert_eq!(super::details("--bad"), Default::default(), "names are checked first");
        if cfg!(windows) {
            let d = super::details("EventLog");
            assert!(!d.sid_type.is_empty() && d.recovery.is_some(), "{d:?}");
            assert!(!d.privileges.is_empty(), "the event log requires privileges: {d:?}");
        }
    }
}

//! Process actions (S2a-03). Each opens the process with the one right the
//! action needs, only when the user asks for it (the reading side, `procs`,
//! never asks for more than the limited query right). Every call here is
//! reached through the Executor, which confirms, elevates per action when
//! needed and writes the audit log.

pub use cpuzeez_core::actions::Priority;

pub fn end(pid: u32) -> Result<(), String> {
    imp::end(pid)
}

/// Unix: SIGKILL after SIGTERM was ignored.
pub fn kill(pid: u32) -> Result<(), String> {
    imp::kill(pid)
}

pub fn suspend(pid: u32, on: bool) -> Result<(), String> {
    imp::suspend(pid, on)
}

pub fn set_priority(pid: u32, p: Priority) -> Result<(), String> {
    imp::set_priority(pid, p)
}

/// One bit per logical CPU (group 0 on Windows).
pub fn set_affinity(pid: u32, mask: u64) -> Result<(), String> {
    if mask == 0 {
        return Err("pick at least one CPU".into());
    }
    imp::set_affinity(pid, mask)
}

/// Windows "Efficiency mode": EcoQoS + idle priority. Linux: SCHED_IDLE + nice 19.
/// macOS: background QoS (`taskpolicy -b`).
pub fn eco(pid: u32, on: bool) -> Result<(), String> {
    imp::eco(pid, on)
}

/// Writes a full memory dump to `path` (Windows; ❌ elsewhere).
pub fn dump(pid: u32, path: &std::path::Path) -> Result<(), String> {
    imp::dump(pid, path)
}

/// One line per blocked thread: what it waits on (Windows Wait Chain Traversal).
pub fn wait_chain(pid: u32, threads: &[u32]) -> Result<Vec<String>, String> {
    imp::wait_chain(pid, threads)
}

/// Windows UAC file/registry virtualization for the process's token.
pub fn set_virtualization(pid: u32, on: bool) -> Result<(), String> {
    imp::set_virtualization(pid, on)
}

/// "Open file location": the file manager with the file selected.
/// End Explorer and get the shell back: Windows restarts it by itself
/// (Winlogon's AutoRestartShell); if it hasn't within 4 s, start it.
pub fn restart_shell(pid: u32) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("Restart is for the Windows shell (Explorer)".into());
    }
    end(pid)?;
    let back =
        || crate::procs::snapshot(false).iter().any(|p| p.pid != pid && p.name.eq_ignore_ascii_case("explorer.exe"));
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if back() {
            return Ok(());
        }
    }
    run_task("explorer.exe", false)
}

pub fn reveal(path: &str) -> Result<(), String> {
    imp::reveal(path)
}

/// "Run new task": `command` through the shell, optionally as administrator
/// (the OS asks for the credentials itself; they never pass through us).
pub fn run_task(command: &str, admin: bool) -> Result<(), String> {
    if command.trim().is_empty() {
        return Err("nothing to run".into());
    }
    imp::run_task(command.trim(), admin)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetConn {
    pub proto: &'static str,
    pub local: String,
    pub remote: String,
    pub state: &'static str,
}

/// Active TCP/UDP sockets owned by `pid` (S2a-04).
pub fn connections(pid: u32) -> Vec<NetConn> {
    imp::connections(pid)
}

#[cfg(windows)]
mod imp {
    use super::Priority;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE};
    use windows_sys::Win32::System::Threading::*;

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtSuspendProcess(h: HANDLE) -> i32;
        fn NtResumeProcess(h: HANDLE) -> i32;
    }

    struct H(HANDLE);
    impl Drop for H {
        fn drop(&mut self) {
            // SAFETY: our handle, closed once.
            unsafe { CloseHandle(self.0) };
        }
    }

    fn err() -> String {
        // SAFETY: no arguments.
        match unsafe { GetLastError() } {
            5 => "access denied (try as administrator)".into(),
            87 => "the process has exited".into(),
            e => std::io::Error::from_raw_os_error(e as i32).to_string(),
        }
    }

    fn open(pid: u32, access: u32) -> Result<H, String> {
        // SAFETY: plain Win32 call.
        let h = unsafe { OpenProcess(access, 0, pid) };
        if h.is_null() { Err(err()) } else { Ok(H(h)) }
    }

    fn check(ok: bool) -> Result<(), String> {
        if ok { Ok(()) } else { Err(err()) }
    }

    pub fn end(pid: u32) -> Result<(), String> {
        let h = open(pid, PROCESS_TERMINATE)?;
        // SAFETY: our handle.
        check(unsafe { TerminateProcess(h.0, 1) } != 0)
    }

    pub fn kill(pid: u32) -> Result<(), String> {
        end(pid)
    }

    pub fn suspend(pid: u32, on: bool) -> Result<(), String> {
        let h = open(pid, PROCESS_SUSPEND_RESUME)?;
        // SAFETY: our handle.
        let st = unsafe { if on { NtSuspendProcess(h.0) } else { NtResumeProcess(h.0) } };
        if st >= 0 { Ok(()) } else { Err(format!("NTSTATUS {st:#x}")) }
    }

    fn class(p: Priority) -> PROCESS_CREATION_FLAGS {
        match p {
            Priority::Idle => IDLE_PRIORITY_CLASS,
            Priority::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
            Priority::Normal => NORMAL_PRIORITY_CLASS,
            Priority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
            Priority::High => HIGH_PRIORITY_CLASS,
            Priority::Realtime => REALTIME_PRIORITY_CLASS,
        }
    }

    pub fn set_priority(pid: u32, p: Priority) -> Result<(), String> {
        let h = open(pid, PROCESS_SET_INFORMATION)?;
        // SAFETY: our handle.
        check(unsafe { SetPriorityClass(h.0, class(p)) } != 0)
    }

    pub fn set_affinity(pid: u32, mask: u64) -> Result<(), String> {
        let h = open(pid, PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION)?;
        // SAFETY: our handle.
        check(unsafe { SetProcessAffinityMask(h.0, mask as usize) } != 0)
    }

    pub fn eco(pid: u32, on: bool) -> Result<(), String> {
        let h = open(pid, PROCESS_SET_INFORMATION)?;
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: if on { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
        };
        // SAFETY: our handle and a struct of the documented size.
        unsafe {
            check(
                SetProcessInformation(
                    h.0,
                    ProcessPowerThrottling,
                    (&state as *const PROCESS_POWER_THROTTLING_STATE).cast(),
                    size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
                ) != 0,
            )?;
            check(SetPriorityClass(h.0, if on { IDLE_PRIORITY_CLASS } else { NORMAL_PRIORITY_CLASS }) != 0)
        }
    }

    pub fn dump(pid: u32, path: &std::path::Path) -> Result<(), String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::Diagnostics::Debug::{
            MiniDumpWithFullMemory, MiniDumpWithHandleData, MiniDumpWriteDump,
        };
        let h = open(pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)?;
        let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
        // SAFETY: our process and file handles.
        let ok = unsafe {
            MiniDumpWriteDump(
                h.0,
                pid,
                f.as_raw_handle() as HANDLE,
                MiniDumpWithFullMemory | MiniDumpWithHandleData,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            )
        } != 0;
        if !ok {
            let e = err();
            drop(f);
            let _ = std::fs::remove_file(path);
            return Err(e);
        }
        Ok(())
    }

    pub fn wait_chain(_pid: u32, threads: &[u32]) -> Result<Vec<String>, String> {
        use windows_sys::Win32::System::Diagnostics::Debug::*;
        // SAFETY: synchronous WCT session; node buffers are local arrays of
        // the documented maximum size; the session is closed below.
        unsafe {
            let s = OpenThreadWaitChainSession(0, None);
            if s.is_null() {
                return Err(err());
            }
            let mut out = Vec::new();
            for &tid in threads {
                let mut nodes = [WAITCHAIN_NODE_INFO::default(); WCT_MAX_NODE_COUNT as usize];
                let (mut n, mut cycle) = (WCT_MAX_NODE_COUNT, 0);
                if GetThreadWaitChain(
                    s,
                    0,
                    WCT_OUT_OF_PROC_FLAG | WCT_OUT_OF_PROC_COM_FLAG | WCT_OUT_OF_PROC_CS_FLAG,
                    tid,
                    &mut n,
                    nodes.as_mut_ptr(),
                    &mut cycle,
                ) == 0
                {
                    continue;
                }
                let n = (n as usize).min(nodes.len());
                if n < 2 && cycle == 0 {
                    continue; // not waiting on anything another thread holds
                }
                let names: Vec<String> = nodes[..n]
                    .iter()
                    .map(|nd| {
                        if nd.ObjectType == WctThreadType {
                            let t = nd.Anonymous.ThreadObject;
                            format!("thread {} (process {})", t.ThreadId, t.ProcessId)
                        } else {
                            let kind = match nd.ObjectType {
                                1 => "critical section",
                                2 => "SendMessage",
                                3 => "mutex",
                                4 => "ALPC",
                                5 => "COM",
                                6 => "thread wait",
                                7 => "process wait",
                                9 => "COM activation",
                                11 => "socket I/O",
                                12 => "SMB I/O",
                                _ => "object",
                            };
                            let name = nd.Anonymous.LockObject.ObjectName;
                            let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
                            let nm = String::from_utf16_lossy(&name[..end]);
                            if nm.is_empty() { kind.to_string() } else { format!("{kind} \"{nm}\"") }
                        }
                    })
                    .collect();
                let dead = if cycle != 0 { " · DEADLOCK" } else { "" };
                out.push(format!("{}{dead}", names.join(" → ")));
            }
            CloseThreadWaitChainSession(s);
            Ok(out)
        }
    }

    pub fn set_virtualization(pid: u32, on: bool) -> Result<(), String> {
        use windows_sys::Win32::Security::{
            SetTokenInformation, TOKEN_ADJUST_DEFAULT, TOKEN_QUERY, TokenVirtualizationEnabled,
        };
        let h = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
        let mut tok: HANDLE = std::ptr::null_mut();
        // SAFETY: our handles; closed below.
        unsafe {
            if OpenProcessToken(h.0, TOKEN_ADJUST_DEFAULT | TOKEN_QUERY, &mut tok) == 0 {
                return Err(err());
            }
            let v: u32 = on as u32;
            let ok = SetTokenInformation(tok, TokenVirtualizationEnabled, (&v as *const u32).cast(), 4) != 0;
            CloseHandle(tok);
            check(ok)
        }
    }

    pub fn reveal(path: &str) -> Result<(), String> {
        std::process::Command::new("explorer")
            .arg(format!("/select,{path}"))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn run_task(command: &str, admin: bool) -> Result<(), String> {
        use windows_sys::Win32::UI::Shell::{SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW};
        // "prog.exe args": split at the first space outside quotes.
        let (file, args) = split_command(command);
        let w = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
        let (verb, file, args) = (w(if admin { "runas" } else { "open" }), w(&file), w(&args));
        let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOASYNC;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = args.as_ptr();
        info.nShow = 1; // SW_SHOWNORMAL
        // SAFETY: every pointer refers to a live local buffer.
        check(unsafe { ShellExecuteExW(&mut info) } != 0)
    }

    fn split_command(c: &str) -> (String, String) {
        if let Some(rest) = c.strip_prefix('"')
            && let Some(end) = rest.find('"')
        {
            return (rest[..end].to_string(), rest[end + 1..].trim().to_string());
        }
        match c.split_once(' ') {
            Some((a, b)) => (a.to_string(), b.trim().to_string()),
            None => (c.to_string(), String::new()),
        }
    }

    #[cfg(test)]
    #[test]
    fn splits_commands() {
        assert_eq!(
            split_command(r#""C:\Program Files\x.exe" -a b"#),
            (r"C:\Program Files\x.exe".into(), "-a b".into())
        );
        assert_eq!(split_command("notepad file.txt"), ("notepad".into(), "file.txt".into()));
        assert_eq!(split_command("calc"), ("calc".into(), String::new()));
    }

    pub fn connections(pid: u32) -> Vec<super::NetConn> {
        use std::net::Ipv4Addr;
        use windows_sys::Win32::NetworkManagement::IpHelper::{
            GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP_STATE_CLOSE_WAIT, MIB_TCP_STATE_CLOSED,
            MIB_TCP_STATE_CLOSING, MIB_TCP_STATE_DELETE_TCB, MIB_TCP_STATE_ESTAB, MIB_TCP_STATE_FIN_WAIT1,
            MIB_TCP_STATE_FIN_WAIT2, MIB_TCP_STATE_LAST_ACK, MIB_TCP_STATE_LISTEN, MIB_TCP_STATE_SYN_RCVD,
            MIB_TCP_STATE_SYN_SENT, MIB_TCP_STATE_TIME_WAIT, MIB_TCPROW_OWNER_PID, MIB_UDPROW_OWNER_PID,
            TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
        };
        let mut out = Vec::new();
        let addr = |ip: u32, port: u32| format!("{}:{}", Ipv4Addr::from(ip.to_be()), (port as u16).to_be());
        // SAFETY: two-call sizing pattern documented for GetExtendedTcpTable / GetExtendedUdpTable.
        unsafe {
            let mut size = 0u32;
            let _ = GetExtendedTcpTable(std::ptr::null_mut(), &mut size, 0, 2, TCP_TABLE_OWNER_PID_ALL, 0);
            if size >= 4 {
                let mut buf = vec![0u8; size as usize];
                if GetExtendedTcpTable(buf.as_mut_ptr().cast(), &mut size, 0, 2, TCP_TABLE_OWNER_PID_ALL, 0) == 0 {
                    let count = u32::from_ne_bytes(buf[..4].try_into().unwrap_or_default()) as usize;
                    let rows = std::slice::from_raw_parts(
                        buf.as_ptr().add(4).cast::<MIB_TCPROW_OWNER_PID>(),
                        count.min((buf.len() - 4) / size_of::<MIB_TCPROW_OWNER_PID>()),
                    );
                    for r in rows.iter().filter(|r| r.dwOwningPid == pid) {
                        let st = match r.dwState as i32 {
                            MIB_TCP_STATE_CLOSED => "CLOSED",
                            MIB_TCP_STATE_LISTEN => "LISTEN",
                            MIB_TCP_STATE_SYN_SENT => "SYN_SENT",
                            MIB_TCP_STATE_SYN_RCVD => "SYN_RCVD",
                            MIB_TCP_STATE_ESTAB => "ESTABLISHED",
                            MIB_TCP_STATE_FIN_WAIT1 => "FIN_WAIT1",
                            MIB_TCP_STATE_FIN_WAIT2 => "FIN_WAIT2",
                            MIB_TCP_STATE_CLOSE_WAIT => "CLOSE_WAIT",
                            MIB_TCP_STATE_CLOSING => "CLOSING",
                            MIB_TCP_STATE_LAST_ACK => "LAST_ACK",
                            MIB_TCP_STATE_TIME_WAIT => "TIME_WAIT",
                            MIB_TCP_STATE_DELETE_TCB => "DELETE_TCB",
                            _ => "UNKNOWN",
                        };
                        out.push(super::NetConn {
                            proto: "TCP",
                            local: addr(r.dwLocalAddr, r.dwLocalPort),
                            remote: addr(r.dwRemoteAddr, r.dwRemotePort),
                            state: st,
                        });
                    }
                }
            }
            let mut size = 0u32;
            let _ = GetExtendedUdpTable(std::ptr::null_mut(), &mut size, 0, 2, UDP_TABLE_OWNER_PID, 0);
            if size >= 4 {
                let mut buf = vec![0u8; size as usize];
                if GetExtendedUdpTable(buf.as_mut_ptr().cast(), &mut size, 0, 2, UDP_TABLE_OWNER_PID, 0) == 0 {
                    let count = u32::from_ne_bytes(buf[..4].try_into().unwrap_or_default()) as usize;
                    let rows = std::slice::from_raw_parts(
                        buf.as_ptr().add(4).cast::<MIB_UDPROW_OWNER_PID>(),
                        count.min((buf.len() - 4) / size_of::<MIB_UDPROW_OWNER_PID>()),
                    );
                    for r in rows.iter().filter(|r| r.dwOwningPid == pid) {
                        out.push(super::NetConn {
                            proto: "UDP",
                            local: addr(r.dwLocalAddr, r.dwLocalPort),
                            remote: "*:*".into(),
                            state: "OPEN",
                        });
                    }
                }
            }
        }
        out
    }
}

#[cfg(unix)]
mod imp {
    use super::Priority;

    fn sig(pid: u32, s: libc::c_int) -> Result<(), String> {
        // SAFETY: kill(2) on a PID; errors come back through errno.
        if unsafe { libc::kill(pid as libc::pid_t, s) } == 0 {
            Ok(())
        } else {
            Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EPERM) => "not allowed (try as administrator)".into(),
                Some(libc::ESRCH) => "the process has exited".into(),
                _ => std::io::Error::last_os_error().to_string(),
            })
        }
    }

    pub fn end(pid: u32) -> Result<(), String> {
        sig(pid, libc::SIGTERM)
    }

    pub fn kill(pid: u32) -> Result<(), String> {
        sig(pid, libc::SIGKILL)
    }

    pub fn suspend(pid: u32, on: bool) -> Result<(), String> {
        sig(pid, if on { libc::SIGSTOP } else { libc::SIGCONT })
    }

    pub fn set_priority(pid: u32, p: Priority) -> Result<(), String> {
        // SAFETY: setpriority(2) on a PID.
        if unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as _, p.nice()) } == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().to_string())
        }
    }

    #[cfg(target_os = "linux")]
    pub fn set_affinity(pid: u32, mask: u64) -> Result<(), String> {
        // SAFETY: a zeroed cpu_set_t filled with CPU_SET, then sched_setaffinity(2).
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            for i in 0..64 {
                if mask >> i & 1 == 1 {
                    libc::CPU_SET(i, &mut set);
                }
            }
            if libc::sched_setaffinity(pid as libc::pid_t, size_of::<libc::cpu_set_t>(), &set) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error().to_string())
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn set_affinity(_: u32, _: u64) -> Result<(), String> {
        Err("macOS offers apps no CPU affinity".into())
    }

    #[cfg(target_os = "linux")]
    pub fn eco(pid: u32, on: bool) -> Result<(), String> {
        let param = libc::sched_param { sched_priority: 0 };
        let policy = if on { libc::SCHED_IDLE } else { libc::SCHED_OTHER };
        // SAFETY: sched_setscheduler(2) with a valid param.
        if unsafe { libc::sched_setscheduler(pid as libc::pid_t, policy, &param) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        set_priority(pid, if on { Priority::Idle } else { Priority::Normal })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn eco(pid: u32, on: bool) -> Result<(), String> {
        let flag = if on { "-b" } else { "-B" };
        let ok = std::process::Command::new("taskpolicy")
            .args([flag, "-p", &pid.to_string()])
            .status()
            .is_ok_and(|s| s.success());
        if ok { Ok(()) } else { Err("taskpolicy failed".into()) }
    }

    pub fn dump(_: u32, _: &std::path::Path) -> Result<(), String> {
        Err("memory dumps need gcore (Linux) or a debugger entitlement (macOS)".into())
    }

    pub fn wait_chain(pid: u32, threads: &[u32]) -> Result<Vec<String>, String> {
        // Linux: threads in uninterruptible sleep and where (wchan).
        Ok(threads
            .iter()
            .filter_map(|t| {
                let stat = std::fs::read_to_string(format!("/proc/{pid}/task/{t}/stat")).ok()?;
                let state = stat.rsplit_once(')')?.1.split_whitespace().next()?.to_string();
                let wchan = std::fs::read_to_string(format!("/proc/{pid}/task/{t}/wchan")).unwrap_or_default();
                (state == "D").then(|| format!("thread {t}: waiting in the kernel ({})", wchan.trim()))
            })
            .collect())
    }

    pub fn set_virtualization(_: u32, _: bool) -> Result<(), String> {
        Err("UAC virtualization is Windows-only".into())
    }

    pub fn reveal(path: &str) -> Result<(), String> {
        let (prog, arg) = if cfg!(target_os = "macos") {
            ("open", vec!["-R".to_string(), path.to_string()])
        } else {
            let dir = std::path::Path::new(path).parent().map(|p| p.display().to_string()).unwrap_or_default();
            ("xdg-open", vec![dir])
        };
        std::process::Command::new(prog).args(arg).spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    pub fn run_task(command: &str, admin: bool) -> Result<(), String> {
        let mut c = if admin {
            let mut c = std::process::Command::new(if cfg!(target_os = "macos") { "osascript" } else { "pkexec" });
            if cfg!(target_os = "macos") {
                c.args([
                    "-e",
                    &format!("do shell script \"{}\" with administrator privileges", command.replace('"', "\\\"")),
                ]);
            } else {
                c.args(["sh", "-c", command]);
            }
            c
        } else {
            let mut c = std::process::Command::new("sh");
            c.args(["-c", command]);
            c
        };
        c.spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    pub fn connections(pid: u32) -> Vec<super::NetConn> {
        use std::collections::BTreeSet;
        use std::net::Ipv4Addr;
        let Ok(fds) = std::fs::read_dir(format!("/proc/{pid}/fd")) else {
            return Vec::new();
        };
        let inodes: BTreeSet<u64> = fds
            .flatten()
            .filter_map(|e| std::fs::read_link(e.path()).ok())
            .filter_map(|p| p.to_str()?.strip_prefix("socket:[")?.strip_suffix(']')?.parse().ok())
            .collect();
        if inodes.is_empty() {
            return Vec::new();
        }
        let parse_addr = |s: &str| -> String {
            let Some((h, p)) = s.split_once(':') else { return s.into() };
            let ip = u32::from_str_radix(h, 16).unwrap_or(0).swap_bytes();
            let port = u16::from_str_radix(p, 16).unwrap_or(0);
            format!("{}:{port}", Ipv4Addr::from(ip))
        };
        let mut out = Vec::new();
        for (proto, file) in [("TCP", "/proc/net/tcp"), ("UDP", "/proc/net/udp")] {
            let Ok(text) = std::fs::read_to_string(file) else { continue };
            for line in text.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() < 10 {
                    continue;
                }
                let Ok(ino) = cols[9].parse::<u64>() else { continue };
                if !inodes.contains(&ino) {
                    continue;
                }
                let state = match cols[3] {
                    "01" => "ESTABLISHED",
                    "02" => "SYN_SENT",
                    "03" => "SYN_RCVD",
                    "04" => "FIN_WAIT1",
                    "05" => "FIN_WAIT2",
                    "06" => "TIME_WAIT",
                    "07" => "CLOSED",
                    "08" => "CLOSE_WAIT",
                    "09" => "LAST_ACK",
                    "0A" => "LISTEN",
                    _ => "OPEN",
                };
                out.push(super::NetConn { proto, local: parse_addr(cols[1]), remote: parse_addr(cols[2]), state });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_maps_both_ways() {
        assert_eq!(Priority::from_base(8, false), Priority::Normal);
        assert_eq!(Priority::from_base(4, false), Priority::Idle);
        assert_eq!(Priority::from_base(13, false), Priority::High);
        assert_eq!(Priority::from_base(24, false), Priority::Realtime);
        for p in Priority::ALL {
            assert_eq!(Priority::from_base(p.nice(), true), p, "{p:?}");
        }
    }

    /// The actions work on a child we start (end, suspend/resume, priority).
    #[test]
    fn acts_on_a_child_process() {
        let mut child = if cfg!(windows) {
            std::process::Command::new("cmd").args(["/C", "ping -n 30 127.0.0.1 >NUL"]).spawn().unwrap()
        } else {
            std::process::Command::new("sleep").arg("30").spawn().unwrap()
        };
        let pid = child.id();
        set_priority(pid, Priority::BelowNormal).unwrap();
        suspend(pid, true).unwrap();
        suspend(pid, false).unwrap();
        if cfg!(any(windows, target_os = "linux")) {
            set_affinity(pid, 1).unwrap();
        }
        assert!(set_affinity(pid, 0).is_err(), "an empty mask is refused");
        end(pid).unwrap();
        let st = child.wait().unwrap();
        assert!(!st.success());
        assert!(end(pid).is_err(), "it's gone");
        assert!(run_task("  ", false).is_err());
    }
}

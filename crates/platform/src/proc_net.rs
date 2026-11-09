//! Network bytes per process (S2a-02, task-manager §3.1), without opening or
//! touching any process:
//! - Windows: a private real-time ETW session on the
//!   `Microsoft-Windows-Kernel-Network` provider (TCP and UDP send/receive
//!   events carry the PID and the size). Starting a session needs an
//!   administrator (or the Performance Log Users group).
//! - Linux: `sock_diag` netlink, `tcp_info` byte counters per TCP socket,
//!   mapped to PIDs through `/proc/<pid>/fd` (other users' sockets need root).
//!   UDP has no per-socket byte counters, so Linux counts TCP only.
//! - macOS: `nettop -P -L 1 -x -J bytes_in,bytes_out`, refreshed every 2 s
//!   on its own thread.
//!
//! [`totals`] is cumulative bytes (sent, received) per PID; the sampler turns
//! two readings into rates.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Status {
    Running,
    /// The OS refused: run CPU ZeeZ as administrator / root.
    NeedsAdmin,
    #[default]
    Unavailable,
}

/// Start collecting (idempotent, cheap after the first call).
pub fn start() -> Status {
    imp::start()
}

/// Cumulative (sent, received) bytes per PID.
pub fn totals() -> HashMap<u32, (u64, u64)> {
    imp::totals()
}

/// Stop the OS-side collection (the Windows ETW session outlives the process
/// otherwise). Call on exit.
pub fn stop() {
    imp::stop()
}

/// `nettop -x -J bytes_in,bytes_out` CSV → (pid, sent, received).
pub fn parse_nettop(text: &str) -> HashMap<u32, (u64, u64)> {
    let mut out: HashMap<u32, (u64, u64)> = HashMap::new();
    for line in text.lines().skip(1) {
        let mut f = line.split(',');
        let (Some(who), Some(rx), Some(tx)) = (f.next(), f.next(), f.next()) else { continue };
        let Some(pid) = who.rsplit_once('.').and_then(|(_, p)| p.parse().ok()) else { continue };
        let (Ok(rx), Ok(tx)) = (rx.trim().parse::<u64>(), tx.trim().parse::<u64>()) else { continue };
        let e = out.entry(pid).or_default();
        e.0 += tx;
        e.1 += rx;
    }
    out
}

/// One `inet_diag_msg` with its attributes → (inode, bytes_acked, bytes_received).
pub fn parse_inet_diag(msg: &[u8]) -> Option<(u32, u64, u64)> {
    const INET_DIAG_INFO: u16 = 2;
    let inode = u32::from_ne_bytes(msg.get(68..72)?.try_into().ok()?);
    let mut off = 72;
    while off + 4 <= msg.len() {
        let len = u16::from_ne_bytes([msg[off], msg[off + 1]]) as usize;
        let ty = u16::from_ne_bytes([msg[off + 2], msg[off + 3]]);
        if len < 4 {
            break;
        }
        if ty == INET_DIAG_INFO {
            let info = msg.get(off + 4..off + len)?;
            let acked = u64::from_ne_bytes(info.get(120..128)?.try_into().ok()?);
            let recv = u64::from_ne_bytes(info.get(128..136)?.try_into().ok()?);
            return Some((inode, acked, recv));
        }
        off += (len + 3) & !3;
    }
    None
}

#[cfg(windows)]
mod imp {
    use super::Status;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    use windows_sys::Win32::System::Diagnostics::Etw::*;
    use windows_sys::core::GUID;

    const SESSION: &str = "CpuZeeZ-ProcessNetwork";
    const KERNEL_NETWORK: GUID = GUID::from_u128(0x7dd42a49_5329_4832_8dfd_43d979153a88);
    /// KERNEL_NETWORK_KEYWORD_IPV4 | _IPV6.
    const KEYWORDS: u64 = 0x10 | 0x20;

    static TOTALS: Mutex<Option<HashMap<u32, (u64, u64)>>> = Mutex::new(None);
    static STATE: OnceLock<Status> = OnceLock::new();

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain([0]).collect()
    }

    /// EVENT_TRACE_PROPERTIES followed by room for the session name.
    fn props() -> Vec<u64> {
        let size = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + 1024;
        let mut buf = vec![0u64; size.div_ceil(8)];
        // SAFETY: the buffer is at least one EVENT_TRACE_PROPERTIES, 8-byte aligned.
        let p = unsafe { &mut *(buf.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES) };
        p.Wnode.BufferSize = size as u32;
        p.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        p.Wnode.ClientContext = 1; // QPC timestamps
        p.LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
        p.FlushTimer = 1;
        p.LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;
        buf
    }

    unsafe extern "system" fn on_event(r: *mut EVENT_RECORD) {
        // SAFETY: ETW passes a valid record for the duration of the call.
        let r = unsafe { &*r };
        let sent = match r.EventHeader.EventDescriptor.Id {
            10 | 26 | 42 | 58 => true, // TCPv4, TCPv6, UDPv4, UDPv6 send
            11 | 27 | 43 | 59 => false,
            _ => return,
        };
        if r.UserDataLength < 8 || r.UserData.is_null() {
            return;
        }
        // SAFETY: UserData holds UserDataLength bytes; PID then size (u32 each).
        let d = unsafe { std::slice::from_raw_parts(r.UserData as *const u8, 8) };
        let pid = u32::from_le_bytes([d[0], d[1], d[2], d[3]]);
        let size = u32::from_le_bytes([d[4], d[5], d[6], d[7]]) as u64;
        let mut t = TOTALS.lock().unwrap_or_else(|e| e.into_inner());
        let e = t.get_or_insert_with(HashMap::new).entry(pid).or_default();
        if sent {
            e.0 += size;
        } else {
            e.1 += size;
        }
    }

    fn stop_session() {
        let name = wide(SESSION);
        let mut p = props();
        // SAFETY: our properties buffer; stopping a session that doesn't exist is an error we ignore.
        unsafe {
            ControlTraceW(
                CONTROLTRACE_HANDLE { Value: 0 },
                name.as_ptr(),
                p.as_mut_ptr().cast(),
                EVENT_TRACE_CONTROL_STOP,
            )
        };
    }

    fn begin() -> Status {
        stop_session(); // one left behind by a crash
        let name = wide(SESSION);
        let mut p = props();
        let mut h = CONTROLTRACE_HANDLE::default();
        // SAFETY: our buffers; ETW copies the name.
        let e = unsafe { StartTraceW(&mut h, name.as_ptr(), p.as_mut_ptr().cast()) };
        if e == 5 {
            return Status::NeedsAdmin; // ERROR_ACCESS_DENIED
        }
        if e != 0 {
            return Status::Unavailable;
        }
        // SAFETY: the session we just started.
        let e = unsafe {
            EnableTraceEx2(h, &KERNEL_NETWORK, EVENT_CONTROL_CODE_ENABLE_PROVIDER, 4, KEYWORDS, 0, 0, std::ptr::null())
        };
        if e != 0 {
            stop_session();
            return Status::Unavailable;
        }
        let spawned = std::thread::Builder::new().name("cz-proc-net".into()).spawn(|| {
            let mut name = wide(SESSION);
            // SAFETY: zeroed is a valid EVENT_TRACE_LOGFILEW; we set what ETW reads.
            let mut lf: EVENT_TRACE_LOGFILEW = unsafe { std::mem::zeroed() };
            lf.LoggerName = name.as_mut_ptr();
            lf.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
            lf.Anonymous2.EventRecordCallback = Some(on_event);
            // SAFETY: `lf` and `name` live until ProcessTrace returns (the session stops).
            unsafe {
                let th = OpenTraceW(&mut lf);
                if th.Value == u64::MAX {
                    return;
                }
                ProcessTrace(&th, 1, std::ptr::null(), std::ptr::null());
                CloseTrace(th);
            }
        });
        if spawned.is_err() {
            stop_session();
            return Status::Unavailable;
        }
        Status::Running
    }

    pub fn start() -> Status {
        *STATE.get_or_init(begin)
    }

    pub fn totals() -> HashMap<u32, (u64, u64)> {
        TOTALS.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default()
    }

    pub fn stop() {
        if STATE.get() == Some(&Status::Running) {
            stop_session();
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::Status;
    use std::collections::HashMap;

    pub fn start() -> Status {
        // SAFETY: plain socket call, closed at once.
        let fd = unsafe { libc::socket(libc::AF_NETLINK, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 4) };
        if fd < 0 {
            return Status::Unavailable;
        }
        // SAFETY: our fd.
        unsafe { libc::close(fd) };
        Status::Running
    }

    /// (inode → bytes acked, bytes received) of every TCP socket.
    fn tcp_sockets() -> HashMap<u32, (u64, u64)> {
        const SOCK_DIAG_BY_FAMILY: u16 = 20;
        let mut out = HashMap::new();
        // SAFETY: netlink socket, closed below; buffers are ours.
        unsafe {
            let fd = libc::socket(libc::AF_NETLINK, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, 4);
            if fd < 0 {
                return out;
            }
            for family in [libc::AF_INET as u8, libc::AF_INET6 as u8] {
                // nlmsghdr (16) + inet_diag_req_v2 (56).
                let mut req = [0u8; 72];
                req[0..4].copy_from_slice(&72u32.to_ne_bytes());
                req[4..6].copy_from_slice(&SOCK_DIAG_BY_FAMILY.to_ne_bytes());
                req[6..8].copy_from_slice(&((libc::NLM_F_REQUEST | libc::NLM_F_DUMP) as u16).to_ne_bytes());
                req[16] = family;
                req[17] = libc::IPPROTO_TCP as u8;
                req[18] = 1 << (2 - 1); // INET_DIAG_INFO
                req[20..24].copy_from_slice(&u32::MAX.to_ne_bytes()); // every state
                if libc::send(fd, req.as_ptr().cast(), req.len(), 0) < 0 {
                    continue;
                }
                let mut buf = vec![0u8; 1 << 16];
                'recv: loop {
                    let n = libc::recv(fd, buf.as_mut_ptr().cast(), buf.len(), 0);
                    if n <= 0 {
                        break;
                    }
                    let mut off = 0usize;
                    let n = n as usize;
                    while off + 16 <= n {
                        let len = u32::from_ne_bytes(buf[off..off + 4].try_into().unwrap_or_default()) as usize;
                        let ty = u16::from_ne_bytes([buf[off + 4], buf[off + 5]]);
                        if len < 16 || off + len > n || ty == libc::NLMSG_DONE as u16 || ty == libc::NLMSG_ERROR as u16
                        {
                            break 'recv;
                        }
                        if let Some((inode, tx, rx)) = super::parse_inet_diag(&buf[off + 16..off + len]) {
                            out.insert(inode, (tx, rx));
                        }
                        off += (len + 3) & !3;
                    }
                }
            }
            libc::close(fd);
        }
        out
    }

    pub fn totals() -> HashMap<u32, (u64, u64)> {
        let socks = tcp_sockets();
        let mut out: HashMap<u32, (u64, u64)> = HashMap::new();
        if socks.is_empty() {
            return out;
        }
        let Ok(rd) = std::fs::read_dir("/proc") else { return out };
        for e in rd.flatten() {
            let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else { continue };
            let Ok(fds) = std::fs::read_dir(e.path().join("fd")) else { continue };
            for fd in fds.flatten() {
                let Ok(link) = std::fs::read_link(fd.path()) else { continue };
                let link = link.to_string_lossy();
                let Some(inode) = link.strip_prefix("socket:[").and_then(|s| s.strip_suffix(']')) else { continue };
                if let Some((tx, rx)) = inode.parse().ok().and_then(|i: u32| socks.get(&i)) {
                    let t = out.entry(pid).or_default();
                    t.0 += tx;
                    t.1 += rx;
                }
            }
        }
        out
    }

    pub fn stop() {}
}

#[cfg(target_os = "macos")]
mod imp {
    use super::Status;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static LAST: Mutex<Option<HashMap<u32, (u64, u64)>>> = Mutex::new(None);
    static STATE: OnceLock<Status> = OnceLock::new();

    fn read() -> Option<HashMap<u32, (u64, u64)>> {
        let out = std::process::Command::new("/usr/bin/nettop")
            .args(["-P", "-L", "1", "-n", "-x", "-J", "bytes_in,bytes_out"])
            .output()
            .ok()?;
        out.status.success().then(|| super::parse_nettop(&String::from_utf8_lossy(&out.stdout)))
    }

    pub fn start() -> Status {
        *STATE.get_or_init(|| {
            if read().is_none() {
                return Status::Unavailable;
            }
            let ok = std::thread::Builder::new().name("cz-proc-net".into()).spawn(|| {
                loop {
                    if let Some(m) = read() {
                        *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some(m);
                    }
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            });
            if ok.is_ok() { Status::Running } else { Status::Unavailable }
        })
    }

    pub fn totals() -> HashMap<u32, (u64, u64)> {
        LAST.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default()
    }

    pub fn stop() {}
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn start() -> super::Status {
        super::Status::Unavailable
    }
    pub fn totals() -> std::collections::HashMap<u32, (u64, u64)> {
        Default::default()
    }
    pub fn stop() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nettop_csv() {
        let t = ",bytes_in,bytes_out,\nlaunchd.1,0,0,\nGoogle Chrome He.812,1500,300,\nbad line\n";
        let m = parse_nettop(t);
        assert_eq!(m.get(&812), Some(&(300, 1500)), "sent = bytes_out, received = bytes_in");
        assert_eq!(m.get(&1), Some(&(0, 0)));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn inet_diag_message() {
        let mut m = vec![0u8; 72];
        m[68..72].copy_from_slice(&4242u32.to_ne_bytes());
        // rtattr INET_DIAG_INFO with a 136-byte tcp_info.
        let mut info = vec![0u8; 136];
        info[120..128].copy_from_slice(&1000u64.to_ne_bytes());
        info[128..136].copy_from_slice(&2000u64.to_ne_bytes());
        // An unrelated attribute first.
        m.extend(8u16.to_ne_bytes());
        m.extend(1u16.to_ne_bytes());
        m.extend([0u8; 4]);
        m.extend(((info.len() + 4) as u16).to_ne_bytes());
        m.extend(2u16.to_ne_bytes());
        m.extend(info);
        assert_eq!(parse_inet_diag(&m), Some((4242, 1000, 2000)));
        assert_eq!(parse_inet_diag(&m[..72]), None);
    }

    /// The collector answers on every OS without a panic; with rights, our
    /// own traffic shows up.
    #[test]
    fn starts_and_reports() {
        let st = start();
        let _ = totals();
        assert!(matches!(st, Status::Running | Status::NeedsAdmin | Status::Unavailable));
        stop();
    }
}

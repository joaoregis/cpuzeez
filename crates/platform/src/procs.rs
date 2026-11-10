//! Process snapshots (S2a-01), one kernel query per refresh, **without
//! opening the processes** (the anti-cheat rule, task-manager §3.1):
//! - Windows: `NtQuerySystemInformation(SystemProcessInformation)`: every
//!   process and thread with times, memory, I/O and handles in one call.
//!   Paths and users come from handles opened with
//!   `PROCESS_QUERY_LIMITED_INFORMATION` only (the right Task Manager uses,
//!   granted for protected and anti-cheat processes without touching them).
//! - Linux: `/proc/<pid>/{stat,status,io}` (other users' I/O needs root).
//! - macOS: `libproc` (`proc_pidinfo` task-all-info, `proc_pid_rusage`).
//!
//! Nothing here reads another process's memory.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Thread {
    pub tid: u32,
    /// User + kernel time, ns.
    pub cpu_ns: u64,
    /// Waiting because it was suspended (Windows).
    pub suspended: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Proc {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    /// Unix ms; with the PID it identifies the process (PIDs get reused).
    pub start_ms: u64,
    pub threads: u32,
    pub handles: u32,
    pub user_ns: u64,
    pub kernel_ns: u64,
    pub working_set: u64,
    pub peak_working_set: u64,
    /// Private bytes (Windows commit charge; Linux RssAnon; macOS footprint).
    pub private: u64,
    pub paged_pool: u64,
    pub nonpaged_pool: u64,
    pub page_faults: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub session: Option<u32>,
    pub priority: i32,
    /// Every thread suspended (Windows), or stopped (`T`, Unix).
    pub suspended: bool,
    /// Linux/macOS process state letter (`R`, `S`, `D`, `Z`, `T`…).
    pub state: Option<char>,
    pub uid: Option<u32>,
    pub thread_list: Vec<Thread>,
}

/// Every process. `threads`: include the per-thread list (Windows and Linux).
pub fn snapshot(threads: bool) -> Vec<Proc> {
    imp::snapshot(threads)
}

/// Full image path.
pub fn path(pid: u32) -> Option<String> {
    imp::path(pid)
}

/// The account a process runs as ("DOMAIN\\user" on Windows, the user name on Unix).
pub fn user(pid: u32) -> Option<String> {
    imp::user(pid)
}

/// The command line (Windows: through the kernel, `ProcessCommandLineInformation`,
/// with the limited right; never by reading the process's memory).
pub fn command_line(pid: u32) -> Option<String> {
    imp::command_line(pid)
}

/// (company, description) from an executable's version resource (what the
/// Task Manager shows as Publisher and Name): reads the file, not the process.
pub fn version_info(path: &str) -> Option<(String, String)> {
    imp::version_info(path)
}

/// What a process runs as, beyond the account: read once per process.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Token {
    /// "x64", "x86", "ARM64", "x86_64", "aarch64"… (empty when unknown).
    pub arch: String,
    /// Windows: the token is elevated. Unix: runs as root.
    pub elevated: Option<bool>,
    /// Windows integrity level ("Low", "Medium", "High", "System", "AppContainer"…).
    pub integrity: String,
    /// Windows UAC file/registry virtualization is on.
    pub virtualization: Option<bool>,
}

/// Architecture, elevation, integrity and UAC virtualization, from a handle
/// with the limited right (Windows), the executable's ELF header (Linux) or
/// the kernel's process flags (macOS). `uid`: the snapshot's, for Unix.
pub fn token(pid: u32, uid: Option<u32>) -> Token {
    imp::token(pid, uid)
}

/// (GDI objects, USER objects) a process holds (Windows only).
pub fn gui_objects(pid: u32) -> Option<(u32, u32)> {
    imp::gui_objects(pid)
}

/// Architecture name of a Windows `IMAGE_FILE_MACHINE_*` value.
pub fn machine_name(m: u16) -> &'static str {
    match m {
        0x8664 => "x64",
        0x014c => "x86",
        0xAA64 => "ARM64",
        0x01c4 => "ARM",
        _ => "",
    }
}

/// Architecture from an ELF header (Linux executables).
pub fn elf_arch(h: &[u8]) -> &'static str {
    if h.len() < 20 || &h[..4] != b"\x7fELF" {
        return "";
    }
    match u16::from_le_bytes([h[18], h[19]]) {
        0x3E => "x86_64",
        0x03 => "x86",
        0xB7 => "aarch64",
        0x28 => "arm",
        0xF3 => "riscv64",
        0x15 => "ppc64",
        _ => "",
    }
}

/// Windows mandatory label RID → name.
pub fn integrity_name(rid: u32) -> &'static str {
    match rid {
        0..0x1000 => "Untrusted",
        0x1000..0x2000 => "Low",
        0x2000..0x2100 => "Medium",
        0x2100..0x3000 => "Medium Plus",
        0x3000..0x4000 => "High",
        0x4000..0x5000 => "System",
        _ => "Protected",
    }
}

/// The arguments in a macOS `KERN_PROCARGS2` buffer: argc, the exec path,
/// padding, then argc NUL-terminated strings.
pub fn parse_procargs2(b: &[u8]) -> Option<String> {
    let argc = u32::from_ne_bytes(b.get(..4)?.try_into().ok()?) as usize;
    let mut rest = &b[4..];
    let path_end = rest.iter().position(|&c| c == 0)?;
    rest = &rest[path_end..];
    let start = rest.iter().position(|&c| c != 0)?;
    let args: Vec<String> =
        rest[start..].split(|&c| c == 0).take(argc).map(|a| String::from_utf8_lossy(a).into_owned()).collect();
    (!args.is_empty()).then(|| args.join(" "))
}

/// Unix ms from a Windows FILETIME (100 ns since 1601).
pub fn filetime_ms(ft: u64) -> u64 {
    (ft / 10_000).saturating_sub(11_644_473_600_000)
}

#[cfg(windows)]
mod imp {
    use super::{Proc, Thread, filetime_ms};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{GetTokenInformation, LookupAccountSidW, TOKEN_QUERY, TOKEN_USER, TokenUser};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQuerySystemInformation(class: u32, info: *mut u8, len: u32, ret: *mut u32) -> i32;
        fn NtQueryInformationProcess(h: HANDLE, class: u32, info: *mut u8, len: u32, ret: *mut u32) -> i32;
    }

    const SYSTEM_PROCESS_INFORMATION: u32 = 5;
    const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004u32 as i32;

    fn u32_at(b: &[u8], o: usize) -> u32 {
        b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes(s.try_into().unwrap_or_default()))
    }
    fn u64_at(b: &[u8], o: usize) -> u64 {
        b.get(o..o + 8).map_or(0, |s| u64::from_le_bytes(s.try_into().unwrap_or_default()))
    }

    /// Decodes a SYSTEM_PROCESS_INFORMATION list (64-bit layout). `base` is
    /// the buffer's address: the image-name pointers point into it.
    pub(super) fn parse(buf: &[u8], base: usize, threads: bool) -> Vec<Proc> {
        let mut out = Vec::new();
        let mut off = 0usize;
        while let Some(e) = buf.get(off..) {
            if e.len() < 256 {
                break;
            }
            let next = u32_at(e, 0) as usize;
            let nthreads = u32_at(e, 4);
            // ImageName: UNICODE_STRING { Length, MaximumLength, Buffer } at 56.
            let len = u16::from_le_bytes([e[56], e[57]]) as usize;
            let ptr = u64_at(e, 64) as usize;
            let name = if ptr >= base && ptr + len <= base + buf.len() && len > 0 {
                let s = &buf[ptr - base..ptr - base + len];
                String::from_utf16_lossy(&s.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>())
            } else {
                String::new()
            };
            let pid = u64_at(e, 80) as u32;
            let mut thread_list = Vec::new();
            let mut all_suspended = nthreads > 0;
            for t in 0..nthreads as usize {
                let Some(th) = e.get(256 + 80 * t..256 + 80 * (t + 1)) else { break };
                // SYSTEM_THREAD_INFORMATION: KernelTime, UserTime, …, ClientId at 40,
                // ThreadState at 68 (5 = Waiting), WaitReason at 72 (5 = Suspended).
                let suspended = u32_at(th, 68) == 5 && u32_at(th, 72) == 5;
                all_suspended &= suspended;
                if threads {
                    thread_list.push(Thread {
                        tid: u64_at(th, 48) as u32,
                        cpu_ns: (u64_at(th, 0) + u64_at(th, 8)) * 100,
                        suspended,
                    });
                }
            }
            out.push(Proc {
                pid,
                ppid: u64_at(e, 88) as u32,
                name: if pid == 0 { "System Idle Process".into() } else { name },
                start_ms: filetime_ms(u64_at(e, 32)),
                threads: nthreads,
                handles: u32_at(e, 96),
                user_ns: u64_at(e, 40) * 100,
                kernel_ns: u64_at(e, 48) * 100,
                working_set: u64_at(e, 144),
                peak_working_set: u64_at(e, 136),
                private: u64_at(e, 184),
                paged_pool: u64_at(e, 160),
                nonpaged_pool: u64_at(e, 176),
                page_faults: u32_at(e, 128) as u64,
                read_ops: u64_at(e, 208),
                write_ops: u64_at(e, 216),
                read_bytes: u64_at(e, 232),
                write_bytes: u64_at(e, 240),
                session: Some(u32_at(e, 100)),
                priority: u32_at(e, 72) as i32,
                suspended: all_suspended && pid > 4,
                state: None,
                uid: None,
                thread_list,
            });
            if next == 0 {
                break;
            }
            off += next;
        }
        out
    }

    pub fn snapshot(threads: bool) -> Vec<Proc> {
        let mut size = 1usize << 20;
        loop {
            // u64 elements keep the buffer 8-byte aligned, as the kernel requires.
            let mut buf = vec![0u64; size / 8];
            let mut ret = 0u32;
            // SAFETY: the buffer is `size` bytes; the kernel writes at most that.
            let st = unsafe {
                NtQuerySystemInformation(SYSTEM_PROCESS_INFORMATION, buf.as_mut_ptr().cast(), size as u32, &mut ret)
            };
            if st == STATUS_INFO_LENGTH_MISMATCH && size < 1 << 28 {
                size = (ret as usize + (64 << 10)).max(size * 2);
                continue;
            }
            if st < 0 {
                return Vec::new();
            }
            // SAFETY: reinterpreting our own u64 buffer as bytes.
            let bytes = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), size) };
            return parse(bytes, buf.as_ptr() as usize, threads);
        }
    }

    /// The limited right only (it never lets us read memory or change anything).
    fn open(pid: u32) -> Option<HANDLE> {
        // SAFETY: plain Win32 call; the handle is closed by the caller.
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        (!h.is_null()).then_some(h)
    }

    pub fn path(pid: u32) -> Option<String> {
        let h = open(pid)?;
        let mut buf = [0u16; 1024];
        let mut n = buf.len() as u32;
        // SAFETY: our handle and buffer.
        let ok = unsafe { QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut n) } != 0;
        // SAFETY: our handle.
        unsafe { CloseHandle(h) };
        ok.then(|| String::from_utf16_lossy(&buf[..n as usize]))
    }

    pub fn user(pid: u32) -> Option<String> {
        let h = open(pid)?;
        let mut tok: HANDLE = std::ptr::null_mut();
        // SAFETY: our handles and buffers; closed below.
        unsafe {
            let ok = OpenProcessToken(h, TOKEN_QUERY, &mut tok) != 0;
            CloseHandle(h);
            if !ok {
                return None;
            }
            let mut buf = [0u64; 64];
            let mut n = 0u32;
            let got = GetTokenInformation(tok, TokenUser, buf.as_mut_ptr().cast(), 512, &mut n) != 0;
            CloseHandle(tok);
            if !got {
                return None;
            }
            let sid = (*(buf.as_ptr() as *const TOKEN_USER)).User.Sid;
            let (mut name, mut dom) = ([0u16; 256], [0u16; 256]);
            let (mut nl, mut dl, mut use_) = (256u32, 256u32, 0);
            if LookupAccountSidW(
                std::ptr::null(),
                sid,
                name.as_mut_ptr(),
                &mut nl,
                dom.as_mut_ptr(),
                &mut dl,
                &mut use_,
            ) == 0
            {
                return None;
            }
            let (n, d) =
                (String::from_utf16_lossy(&name[..nl as usize]), String::from_utf16_lossy(&dom[..dl as usize]));
            Some(if d.is_empty() { n } else { format!("{d}\\{n}") })
        }
    }

    pub fn version_info(path: &str) -> Option<(String, String)> {
        use windows_sys::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
        let w: Vec<u16> = path.encode_utf16().chain([0]).collect();
        // SAFETY: sizes come from the API; VerQueryValue returns pointers into `data`.
        unsafe {
            let size = GetFileVersionInfoSizeW(w.as_ptr(), std::ptr::null_mut());
            if size == 0 {
                return None;
            }
            let mut data = vec![0u8; size as usize];
            if GetFileVersionInfoW(w.as_ptr(), 0, size, data.as_mut_ptr().cast()) == 0 {
                return None;
            }
            let query = |sub: &str| -> Option<Vec<u8>> {
                let q: Vec<u16> = sub.encode_utf16().chain([0]).collect();
                let (mut p, mut n) = (std::ptr::null_mut(), 0u32);
                (VerQueryValueW(data.as_ptr().cast(), q.as_ptr(), &mut p, &mut n) != 0 && n > 0)
                    .then(|| std::slice::from_raw_parts(p as *const u8, n as usize * 2).to_vec())
            };
            // The first language/codepage pair names the string table.
            let tr = query(r"\VarFileInfo\Translation").filter(|t| t.len() >= 4);
            let table = tr.map_or("040904B0".to_string(), |t| {
                format!("{:04X}{:04X}", u16::from_le_bytes([t[0], t[1]]), u16::from_le_bytes([t[2], t[3]]))
            });
            let text = |key: &str| -> String {
                query(&format!(r"\StringFileInfo\{table}\{key}"))
                    .map(|b| {
                        let u: Vec<u16> =
                            b.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
                        String::from_utf16_lossy(&u).trim().to_string()
                    })
                    .unwrap_or_default()
            };
            Some((text("CompanyName"), text("FileDescription")))
        }
    }

    pub fn token(pid: u32, _uid: Option<u32>) -> super::Token {
        use windows_sys::Win32::Security::{
            GetSidSubAuthority, GetSidSubAuthorityCount, TOKEN_MANDATORY_LABEL, TokenElevation, TokenIntegrityLevel,
            TokenIsAppContainer, TokenVirtualizationEnabled,
        };
        use windows_sys::Win32::System::Threading::{
            GetProcessInformation, IsWow64Process2, PROCESS_MACHINE_INFORMATION, ProcessMachineTypeInfo,
        };
        let mut t = super::Token::default();
        let Some(h) = open(pid) else { return t };
        // SAFETY: our handle, tokens and buffers; every handle is closed here.
        unsafe {
            let mut info: PROCESS_MACHINE_INFORMATION = std::mem::zeroed();
            let size = std::mem::size_of::<PROCESS_MACHINE_INFORMATION>() as u32;
            if GetProcessInformation(
                h,
                ProcessMachineTypeInfo,
                (&mut info as *mut PROCESS_MACHINE_INFORMATION).cast(),
                size,
            ) != 0
            {
                t.arch = super::machine_name(info.ProcessMachine).into();
            } else {
                // Windows 10: WOW64 tells x86 on x64; otherwise the native machine.
                let (mut process, mut native) = (0u16, 0u16);
                if IsWow64Process2(h, &mut process, &mut native) != 0 {
                    t.arch = super::machine_name(if process != 0 { process } else { native }).into();
                }
            }
            let mut tok: HANDLE = std::ptr::null_mut();
            let ok = OpenProcessToken(h, TOKEN_QUERY, &mut tok) != 0;
            CloseHandle(h);
            if !ok {
                return t;
            }
            let dword = |class| {
                let (mut v, mut n) = (0u32, 0u32);
                (GetTokenInformation(tok, class, (&mut v as *mut u32).cast(), 4, &mut n) != 0).then_some(v)
            };
            t.elevated = dword(TokenElevation).map(|v| v != 0);
            t.virtualization = dword(TokenVirtualizationEnabled).map(|v| v != 0);
            let app_container = dword(TokenIsAppContainer).is_some_and(|v| v != 0);
            let mut buf = [0u64; 16];
            let mut n = 0u32;
            if app_container {
                t.integrity = "AppContainer".into();
            } else if GetTokenInformation(tok, TokenIntegrityLevel, buf.as_mut_ptr().cast(), 128, &mut n) != 0 {
                let sid = (*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL)).Label.Sid;
                let count = *GetSidSubAuthorityCount(sid);
                if count > 0 {
                    t.integrity = super::integrity_name(*GetSidSubAuthority(sid, count as u32 - 1)).into();
                }
            }
            CloseHandle(tok);
        }
        t
    }

    pub fn gui_objects(pid: u32) -> Option<(u32, u32)> {
        use windows_sys::Win32::System::Threading::{GR_GDIOBJECTS, GR_USEROBJECTS, GetGuiResources};
        let h = open(pid)?;
        // SAFETY: our handle, closed below.
        let (g, u) = unsafe { (GetGuiResources(h, GR_GDIOBJECTS), GetGuiResources(h, GR_USEROBJECTS)) };
        // SAFETY: our handle.
        unsafe { CloseHandle(h) };
        Some((g, u))
    }

    pub fn command_line(pid: u32) -> Option<String> {
        const PROCESS_COMMAND_LINE_INFORMATION: u32 = 60;
        let h = open(pid)?;
        let mut buf = vec![0u64; 8192];
        let mut ret = 0u32;
        // SAFETY: our handle and buffer; the result is a UNICODE_STRING whose
        // buffer points into ours (filled by the kernel).
        let st = unsafe {
            NtQueryInformationProcess(h, PROCESS_COMMAND_LINE_INFORMATION, buf.as_mut_ptr().cast(), 65536, &mut ret)
        };
        // SAFETY: our handle.
        unsafe { CloseHandle(h) };
        if st < 0 {
            return None;
        }
        // SAFETY: our buffer as bytes.
        let b = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<u8>(), 65536) };
        let len = u16::from_le_bytes([b[0], b[1]]) as usize;
        let ptr = u64_at(b, 8) as usize;
        let base = buf.as_ptr() as usize;
        (ptr >= base && ptr + len <= base + 65536).then(|| {
            let s = &b[ptr - base..ptr - base + len];
            String::from_utf16_lossy(&s.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>())
        })
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{Proc, Thread};

    fn ticks() -> u64 {
        // SAFETY: sysconf has no preconditions.
        (unsafe { libc::sysconf(libc::_SC_CLK_TCK) }).max(1) as u64
    }

    /// Fields of `/proc/<pid>/stat` after the `(comm)`: index 0 = state.
    pub(super) fn stat_fields(s: &str) -> Option<(String, Vec<&str>)> {
        let (a, b) = (s.find('(')?, s.rfind(')')?);
        Some((s[a + 1..b].to_string(), s[b + 1..].split_whitespace().collect()))
    }

    fn boot_ms() -> u64 {
        let now =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
        let up =
            std::fs::read_to_string("/proc/uptime").ok().and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok());
        now.saturating_sub((up.unwrap_or(0.0) * 1000.0) as u64)
    }

    fn kv(text: &str, key: &str) -> Option<u64> {
        text.lines().find_map(|l| l.strip_prefix(key)?.split_whitespace().next()?.parse().ok())
    }

    pub fn snapshot(threads: bool) -> Vec<Proc> {
        let (tck, boot) = (ticks(), boot_ms());
        let page = 4096u64;
        let Ok(rd) = std::fs::read_dir("/proc") else { return Vec::new() };
        let mut out = Vec::new();
        for e in rd.flatten() {
            let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else { continue };
            let dir = e.path();
            let Ok(stat) = std::fs::read_to_string(dir.join("stat")) else { continue };
            let Some((name, f)) = stat_fields(&stat) else { continue };
            let n = |i: usize| f.get(i).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0);
            let status = std::fs::read_to_string(dir.join("status")).unwrap_or_default();
            let io = std::fs::read_to_string(dir.join("io")).unwrap_or_default();
            let state = f.first().and_then(|s| s.chars().next());
            let thread_list = if threads {
                std::fs::read_dir(dir.join("task"))
                    .map(|r| {
                        r.flatten()
                            .filter_map(|t| {
                                let tid = t.file_name().to_string_lossy().parse().ok()?;
                                let s = std::fs::read_to_string(t.path().join("stat")).ok()?;
                                let (_, tf) = stat_fields(&s)?;
                                let tn = |i: usize| tf.get(i).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0);
                                Some(Thread { tid, cpu_ns: (tn(11) + tn(12)) * 1_000_000_000 / tck, suspended: false })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            out.push(Proc {
                pid,
                ppid: n(1) as u32,
                name,
                start_ms: boot + n(19) * 1000 / tck,
                threads: n(17) as u32,
                handles: std::fs::read_dir(dir.join("fd")).map_or(0, |r| r.count() as u32),
                user_ns: n(11) * 1_000_000_000 / tck,
                kernel_ns: n(12) * 1_000_000_000 / tck,
                working_set: n(21) * page,
                peak_working_set: kv(&status, "VmHWM:").unwrap_or(0) * 1024,
                private: kv(&status, "RssAnon:").unwrap_or(0) * 1024,
                paged_pool: 0,
                nonpaged_pool: 0,
                page_faults: n(7) + n(9),
                read_bytes: kv(&io, "read_bytes:").unwrap_or(0),
                write_bytes: kv(&io, "write_bytes:").unwrap_or(0),
                read_ops: kv(&io, "syscr:").unwrap_or(0),
                write_ops: kv(&io, "syscw:").unwrap_or(0),
                session: Some(n(4) as u32),
                priority: f.get(16).and_then(|x| x.parse().ok()).unwrap_or(0),
                suspended: state == Some('T'),
                state,
                uid: kv(&status, "Uid:").map(|u| u as u32),
                thread_list,
            });
        }
        out
    }

    pub fn path(pid: u32) -> Option<String> {
        std::fs::read_link(format!("/proc/{pid}/exe")).ok().map(|p| p.to_string_lossy().into_owned())
    }

    pub fn user(pid: u32) -> Option<String> {
        let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        let uid = kv(&status, "Uid:")?;
        let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
        Some(
            passwd
                .lines()
                .find_map(|l| {
                    let f: Vec<&str> = l.split(':').collect();
                    (f.get(2) == Some(&uid.to_string().as_str())).then(|| f[0].to_string())
                })
                .unwrap_or_else(|| uid.to_string()),
        )
    }

    pub fn version_info(_: &str) -> Option<(String, String)> {
        None
    }

    pub fn token(pid: u32, uid: Option<u32>) -> super::Token {
        use std::io::Read;
        let mut head = [0u8; 20];
        let arch = std::fs::File::open(format!("/proc/{pid}/exe"))
            .and_then(|mut f| f.read_exact(&mut head))
            .map_or("", |_| super::elf_arch(&head));
        super::Token { arch: arch.into(), elevated: uid.map(|u| u == 0), ..Default::default() }
    }

    pub fn gui_objects(_: u32) -> Option<(u32, u32)> {
        None
    }

    pub fn command_line(pid: u32) -> Option<String> {
        let b = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
        let s: Vec<String> =
            b.split(|&c| c == 0).filter(|x| !x.is_empty()).map(|x| String::from_utf8_lossy(x).into_owned()).collect();
        (!s.is_empty()).then(|| s.join(" "))
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::Proc;

    #[repr(C)]
    struct Timebase {
        numer: u32,
        denom: u32,
    }

    unsafe extern "C" {
        fn mach_timebase_info(tb: *mut Timebase) -> i32;
    }

    /// Mach absolute time → ns.
    fn timebase() -> (u64, u64) {
        let mut tb = Timebase { numer: 1, denom: 1 };
        // SAFETY: fills our struct.
        unsafe { mach_timebase_info(&mut tb) };
        (tb.numer.max(1) as u64, tb.denom.max(1) as u64)
    }

    pub fn snapshot(_threads: bool) -> Vec<Proc> {
        let (numer, denom) = timebase();
        let ns = |t: u64| t * numer / denom;
        let mut pids = vec![0i32; 8192];
        // SAFETY: the buffer holds the stated number of bytes.
        let n = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), (pids.len() * 4) as i32) };
        let mut out = Vec::new();
        for &pid in &pids[..n.max(0) as usize] {
            // SAFETY: plain struct filled by the kernel.
            let mut info: libc::proc_taskallinfo = unsafe { std::mem::zeroed() };
            let size = std::mem::size_of::<libc::proc_taskallinfo>() as i32;
            // SAFETY: the buffer is exactly `size` bytes.
            let got = unsafe {
                libc::proc_pidinfo(
                    pid,
                    libc::PROC_PIDTASKALLINFO,
                    0,
                    (&mut info as *mut libc::proc_taskallinfo).cast(),
                    size,
                )
            };
            if got != size {
                continue;
            }
            let b = &info.pbsd;
            let t = &info.ptinfo;
            // SAFETY: plain struct filled by the kernel.
            let mut ru: libc::rusage_info_v2 = unsafe { std::mem::zeroed() };
            // SAFETY: the flavour matches the struct.
            let has_ru = unsafe {
                libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V2, (&mut ru as *mut libc::rusage_info_v2).cast())
            } == 0;
            let cstr = |s: &[libc::c_char]| {
                let bytes: Vec<u8> = s.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
                String::from_utf8_lossy(&bytes).into_owned()
            };
            let name = if b.pbi_name[0] != 0 { cstr(&b.pbi_name) } else { cstr(&b.pbi_comm) };
            out.push(Proc {
                pid: pid as u32,
                ppid: b.pbi_ppid,
                name,
                start_ms: b.pbi_start_tvsec * 1000 + b.pbi_start_tvusec / 1000,
                threads: t.pti_threadnum.max(0) as u32,
                handles: b.pbi_nfiles,
                user_ns: ns(t.pti_total_user),
                kernel_ns: ns(t.pti_total_system),
                working_set: t.pti_resident_size,
                peak_working_set: 0,
                private: if has_ru { ru.ri_phys_footprint } else { 0 },
                paged_pool: 0,
                nonpaged_pool: 0,
                page_faults: t.pti_faults.max(0) as u64,
                read_bytes: if has_ru { ru.ri_diskio_bytesread } else { 0 },
                write_bytes: if has_ru { ru.ri_diskio_byteswritten } else { 0 },
                read_ops: 0,
                write_ops: 0,
                session: None,
                priority: t.pti_priority,
                suspended: b.pbi_status == 4, // SSTOP
                state: None,
                uid: Some(b.pbi_uid),
                thread_list: Vec::new(),
            });
        }
        out
    }

    pub fn path(pid: u32) -> Option<String> {
        let mut buf = vec![0u8; 4096];
        // SAFETY: the buffer is 4096 bytes.
        let n = unsafe { libc::proc_pidpath(pid as i32, buf.as_mut_ptr().cast(), 4096) };
        (n > 0).then(|| String::from_utf8_lossy(&buf[..n as usize]).into_owned())
    }

    pub fn user(pid: u32) -> Option<String> {
        let p = snapshot(false).into_iter().find(|p| p.pid == pid)?;
        let uid = p.uid?;
        // SAFETY: getpwuid returns static storage or null.
        let pw = unsafe { libc::getpwuid(uid) };
        if pw.is_null() {
            return Some(uid.to_string());
        }
        // SAFETY: pw_name is a C string owned by libc.
        Some(unsafe { std::ffi::CStr::from_ptr((*pw).pw_name) }.to_string_lossy().into_owned())
    }

    /// Through `sysctl(KERN_PROCARGS2)`: the kernel copies the arguments out;
    /// nothing reads the process's memory. Other users' processes need root.
    pub fn command_line(pid: u32) -> Option<String> {
        let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as i32];
        let mut size: libc::size_t = 0;
        // SAFETY: a size query, then a fill of a buffer that size.
        unsafe {
            if libc::sysctl(mib.as_mut_ptr(), 3, std::ptr::null_mut(), &mut size, std::ptr::null_mut(), 0) != 0 {
                return None;
            }
            let mut buf = vec![0u8; size];
            if libc::sysctl(mib.as_mut_ptr(), 3, buf.as_mut_ptr().cast(), &mut size, std::ptr::null_mut(), 0) != 0 {
                return None;
            }
            buf.truncate(size);
            super::parse_procargs2(&buf)
        }
    }

    pub fn token(pid: u32, uid: Option<u32>) -> super::Token {
        // SAFETY: plain struct filled by the kernel.
        let mut b: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
        // SAFETY: the buffer is exactly `size` bytes.
        let got = unsafe {
            libc::proc_pidinfo(pid as i32, libc::PROC_PIDTBSDINFO, 0, (&mut b as *mut libc::proc_bsdinfo).cast(), size)
        };
        const PROC_FLAG_LP64: u32 = 0x10;
        let arch = if got != size {
            ""
        } else if b.pbi_flags & PROC_FLAG_LP64 == 0 {
            "32-bit"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x86_64"
        };
        super::Token { arch: arch.into(), elevated: uid.map(|u| u == 0), ..Default::default() }
    }

    pub fn gui_objects(_: u32) -> Option<(u32, u32)> {
        None
    }

    /// The app bundle's name ("Safari" from …/Safari.app/Contents/MacOS/Safari).
    pub fn version_info(path: &str) -> Option<(String, String)> {
        let app = path.split('/').find(|c| c.ends_with(".app"))?;
        Some((String::new(), app.trim_end_matches(".app").to_string()))
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn snapshot(_: bool) -> Vec<super::Proc> {
        Vec::new()
    }
    pub fn path(_: u32) -> Option<String> {
        None
    }
    pub fn user(_: u32) -> Option<String> {
        None
    }
    pub fn command_line(_: u32) -> Option<String> {
        None
    }
    pub fn version_info(_: &str) -> Option<(String, String)> {
        None
    }
    pub fn token(_: u32, _: Option<u32>) -> super::Token {
        super::Token::default()
    }
    pub fn gui_objects(_: u32) -> Option<(u32, u32)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn our_own_process_is_in_the_snapshot() {
        let me = std::process::id();
        // Some CPU time first: Linux counts it in 10 ms ticks, and under
        // nextest this is a fresh process that may not have used one yet.
        let t0 = std::time::Instant::now();
        let mut x = 0u64;
        while t0.elapsed() < std::time::Duration::from_millis(50) {
            x = std::hint::black_box(x.wrapping_add(1));
        }
        let all = snapshot(true);
        let p = all.iter().find(|p| p.pid == me).expect("this test's process");
        assert!(p.threads >= 1 && p.working_set > 0, "{p:?}");
        assert!(p.user_ns + p.kernel_ns > 0, "some CPU time");
        assert!(!p.name.is_empty());
        if cfg!(any(windows, target_os = "linux")) {
            assert!(!p.thread_list.is_empty());
        }
        assert!(all.len() > 3, "more than a few processes");
        assert!(path(me).is_some_and(|x| !x.is_empty()));
        assert!(user(me).is_some());
        if cfg!(any(windows, target_os = "linux")) {
            assert!(command_line(me).is_some());
        }
        if cfg!(windows) {
            let explorer = std::env::var("WINDIR").unwrap_or_default() + r"\explorer.exe";
            let (company, desc) = version_info(&explorer).expect("explorer's version info");
            assert!(company.contains("Microsoft") && !desc.is_empty(), "{company} / {desc}");
        }
    }

    /// The anti-cheat rule: reading processes never asks for more than the
    /// limited query right, and never reads their memory.
    #[test]
    fn reading_uses_only_the_limited_right() {
        let src = include_str!("procs.rs");
        let body = &src[..src.find("#[cfg(test)]").unwrap_or(src.len())];
        for bad in [
            "PROCESS_VM_READ",
            "PROCESS_ALL_ACCESS",
            "ReadProcessMemory",
            "PROCESS_QUERY_INFORMATION,",
            "PROCESS_TERMINATE",
        ] {
            assert!(!body.contains(bad), "procs.rs mentions {bad}");
        }
        assert_eq!(body.matches("OpenProcess(").count(), 1, "one OpenProcess call, with the limited right");
        assert!(body.contains("OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION"));
    }

    #[test]
    fn our_token_and_gui_objects() {
        let me = std::process::id();
        let t = token(me, None);
        assert!(!t.arch.is_empty(), "the test binary's architecture: {t:?}");
        if cfg!(windows) {
            assert!(t.elevated.is_some() && !t.integrity.is_empty(), "{t:?}");
            assert!(gui_objects(me).is_some());
        }
    }

    #[test]
    fn decoders() {
        assert_eq!(machine_name(0x8664), "x64");
        assert_eq!(machine_name(0xAA64), "ARM64");
        let mut elf = vec![0x7f, b'E', b'L', b'F'];
        elf.resize(20, 0);
        elf[18] = 0xB7;
        assert_eq!(elf_arch(&elf), "aarch64");
        assert_eq!(elf_arch(b"MZ not an elf header"), "");
        assert_eq!(integrity_name(0x2000), "Medium");
        assert_eq!(integrity_name(0x3000), "High");
        assert_eq!(integrity_name(0x4000), "System");
        let mut a = 2u32.to_ne_bytes().to_vec();
        a.extend(b"/bin/sleep\0\0\0sleep\x0010\0FOO=bar\0");
        assert_eq!(parse_procargs2(&a).as_deref(), Some("sleep 10"));
    }

    #[test]
    fn filetime_conversion() {
        assert_eq!(filetime_ms(116_444_736_000_000_000), 0);
        assert_eq!(filetime_ms(116_444_736_000_000_000 + 10_000_000), 1000);
    }
}

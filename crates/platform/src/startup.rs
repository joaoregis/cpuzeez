//! Startup applications, `StartupApproved` registry state, WDI impact, and Last BIOS time
//! (S2a-08, docs/v1/task-manager.md §1, §6).

use cpuzeez_core::actions::StartupOp;
use cpuzeez_core::startup::{BootSummary, Impact, StartupRow, StartupSource};

pub fn query() -> BootSummary {
    let mut sum = imp::query();
    sum.items.sort_by_key(|r| r.name.to_ascii_lowercase());
    sum
}

pub fn apply(op: &StartupOp) -> Result<String, String> {
    imp::apply(op)
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod imp {
    use super::*;
    use cpuzeez_core::startup::{
        decode_startup_approved, encode_startup_approved, extract_exe_path, filetime_to_ymd, parse_wdi_startup_xml,
    };
    use std::collections::HashMap;
    use std::path::Path;
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, REG_BINARY, REG_DWORD, REG_EXPAND_SZ,
        REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW,
    };

    struct RegKey(HKEY);
    impl Drop for RegKey {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    RegCloseKey(self.0);
                }
            }
        }
    }

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn query() -> BootSummary {
        let bios_time_ms = read_fw_post_time();
        let (wdi_map, impact_source_note) = read_wdi_impacts();
        let mut items = Vec::new();

        // Registry Run / RunOnce keys
        let reg_sources = [
            (
                HKEY_CURRENT_USER,
                r"Software\Microsoft\Windows\CurrentVersion\Run",
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
                StartupSource::HkcuRun,
            ),
            (
                HKEY_CURRENT_USER,
                r"Software\Microsoft\Windows\CurrentVersion\RunOnce",
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
                StartupSource::HkcuRunOnce,
            ),
            (
                HKEY_LOCAL_MACHINE,
                r"Software\Microsoft\Windows\CurrentVersion\Run",
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
                StartupSource::HklmRun,
            ),
            (
                HKEY_LOCAL_MACHINE,
                r"Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Run",
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32",
                StartupSource::HklmRun32,
            ),
            (
                HKEY_LOCAL_MACHINE,
                r"Software\Microsoft\Windows\CurrentVersion\RunOnce",
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
                StartupSource::HklmRunOnce,
            ),
        ];

        for (hive, subkey, approved_subkey, source) in reg_sources {
            let approved = read_approved_map(hive, approved_subkey);
            for (val_name, cmd) in enum_reg_strings(hive, subkey) {
                let (enabled, disabled_date) =
                    approved.get(&val_name.to_ascii_lowercase()).cloned().unwrap_or((true, None));
                items.push(build_row(val_name, cmd, source, enabled, disabled_date, &wdi_map));
            }
        }

        // Startup folders (User and Common)
        if let Ok(appdata) = std::env::var("APPDATA") {
            let folder = Path::new(&appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup");
            let approved = read_approved_map(
                HKEY_CURRENT_USER,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
            );
            scan_startup_folder(&folder, StartupSource::UserFolder, &approved, &wdi_map, &mut items);
        }
        if let Ok(progdata) = std::env::var("PROGRAMDATA") {
            let folder = Path::new(&progdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup");
            let approved = read_approved_map(
                HKEY_LOCAL_MACHINE,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
            );
            scan_startup_folder(&folder, StartupSource::CommonFolder, &approved, &wdi_map, &mut items);
        }

        BootSummary { bios_time_ms, os_boot_ms: None, impact_source_note, items }
    }

    fn build_row(
        id: String,
        command: String,
        source: StartupSource,
        enabled: bool,
        disabled_date: Option<String>,
        wdi_map: &HashMap<String, (u64, u64)>,
    ) -> StartupRow {
        let exe_path = extract_exe_path(&command);
        let (publisher, desc) = if !exe_path.is_empty() {
            crate::procs::version_info(&exe_path).unwrap_or_default()
        } else {
            (String::new(), String::new())
        };
        let display_name = if !desc.is_empty() { desc } else { id.strip_suffix(".lnk").unwrap_or(&id).to_string() };
        let exe_base = Path::new(&exe_path).file_name().and_then(|s| s.to_str()).unwrap_or(&id).to_ascii_lowercase();
        let (cpu_ms, disk_bytes, impact) = match wdi_map.get(&exe_base) {
            Some(&(c, d)) => (Some(c), Some(d), Impact::from_metrics(c, d)),
            None => (None, None, Impact::NotMeasured),
        };
        StartupRow {
            id,
            name: display_name,
            publisher,
            enabled,
            disabled_date,
            impact,
            cpu_ms,
            disk_bytes,
            source,
            command,
            path: exe_path,
        }
    }

    fn scan_startup_folder(
        dir: &Path,
        source: StartupSource,
        approved: &HashMap<String, (bool, Option<String>)>,
        wdi_map: &HashMap<String, (u64, u64)>,
        out: &mut Vec<StartupRow>,
    ) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for entry in rd.flatten() {
            let path = entry.path();
            let Some(fname) = path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()) else {
                continue;
            };
            if fname.eq_ignore_ascii_case("desktop.ini") {
                continue;
            }
            let (enabled, disabled_date) = approved.get(&fname.to_ascii_lowercase()).cloned().unwrap_or((true, None));
            let full = path.display().to_string();
            out.push(build_row(fname, full, source, enabled, disabled_date, wdi_map));
        }
    }

    fn read_fw_post_time() -> Option<u64> {
        unsafe {
            let sub = to_wide(r"SYSTEM\CurrentControlSet\Control\Session Manager\Power");
            let mut hkey: HKEY = null_mut();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, sub.as_ptr(), 0, KEY_READ, &mut hkey) != ERROR_SUCCESS {
                return None;
            }
            let _guard = RegKey(hkey);
            let val_name = to_wide("FwPOSTTime");
            let mut val = 0u32;
            let mut len = std::mem::size_of::<u32>() as u32;
            let mut ty = 0u32;
            if RegQueryValueExW(hkey, val_name.as_ptr(), null(), &mut ty, &mut val as *mut u32 as *mut u8, &mut len)
                == ERROR_SUCCESS
                && ty == REG_DWORD
                && val > 0
            {
                Some(u64::from(val))
            } else {
                None
            }
        }
    }

    fn enum_reg_strings(hive: HKEY, subkey: &str) -> Vec<(String, String)> {
        unsafe {
            let sub = to_wide(subkey);
            let mut hkey: HKEY = null_mut();
            if RegOpenKeyExW(hive, sub.as_ptr(), 0, KEY_READ, &mut hkey) != ERROR_SUCCESS {
                return Vec::new();
            }
            let _guard = RegKey(hkey);
            let mut out = Vec::new();
            let mut idx = 0u32;
            loop {
                let mut name_buf = vec![0u16; 512];
                let mut name_len = name_buf.len() as u32;
                let mut ty = 0u32;
                let mut data_buf = vec![0u8; 4096];
                let mut data_len = data_buf.len() as u32;
                let st = RegEnumValueW(
                    hkey,
                    idx,
                    name_buf.as_mut_ptr(),
                    &mut name_len,
                    null(),
                    &mut ty,
                    data_buf.as_mut_ptr(),
                    &mut data_len,
                );
                if st != ERROR_SUCCESS {
                    break;
                }
                idx += 1;
                if (ty == REG_SZ || ty == REG_EXPAND_SZ) && data_len >= 2 {
                    let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);
                    let u16_slice =
                        std::slice::from_raw_parts(data_buf.as_ptr() as *const u16, (data_len as usize) / 2);
                    let val = String::from_utf16_lossy(u16_slice).trim_end_matches('\0').trim().to_string();
                    if !name.is_empty() && !val.is_empty() {
                        out.push((name, val));
                    }
                }
            }
            out
        }
    }

    fn read_approved_map(hive: HKEY, subkey: &str) -> HashMap<String, (bool, Option<String>)> {
        unsafe {
            let sub = to_wide(subkey);
            let mut hkey: HKEY = null_mut();
            if RegOpenKeyExW(hive, sub.as_ptr(), 0, KEY_READ, &mut hkey) != ERROR_SUCCESS {
                return HashMap::new();
            }
            let _guard = RegKey(hkey);
            let mut out = HashMap::new();
            let mut idx = 0u32;
            loop {
                let mut name_buf = vec![0u16; 512];
                let mut name_len = name_buf.len() as u32;
                let mut ty = 0u32;
                let mut data_buf = [0u8; 64];
                let mut data_len = data_buf.len() as u32;
                let st = RegEnumValueW(
                    hkey,
                    idx,
                    name_buf.as_mut_ptr(),
                    &mut name_len,
                    null(),
                    &mut ty,
                    data_buf.as_mut_ptr(),
                    &mut data_len,
                );
                if st != ERROR_SUCCESS {
                    break;
                }
                idx += 1;
                if ty == REG_BINARY && data_len >= 4 {
                    let name = String::from_utf16_lossy(&name_buf[..name_len as usize]).to_ascii_lowercase();
                    let decoded = decode_startup_approved(&data_buf[..data_len as usize]);
                    out.insert(name, decoded);
                }
            }
            out
        }
    }

    fn read_wdi_impacts() -> (HashMap<String, (u64, u64)>, Option<String>) {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let dir = Path::new(&windir).join(r"System32\WDI\LogFiles\StartupInfo");
        let Ok(rd) = std::fs::read_dir(&dir) else {
            return (HashMap::new(), None);
        };
        let mut newest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()).is_some_and(|x| x.eq_ignore_ascii_case("xml"))
                && let Ok(meta) = e.metadata()
                && let Ok(mtime) = meta.modified()
                && newest.as_ref().is_none_or(|(prev, _)| mtime > *prev)
            {
                newest = Some((mtime, p));
            }
        }
        let Some((mtime, path)) = newest else {
            return (HashMap::new(), None);
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return (HashMap::new(), None);
        };
        let xml = if bytes.starts_with(&[0xFF, 0xFE]) {
            let u16s: Vec<u16> = bytes[2..].as_chunks::<2>().0.iter().copied().map(u16::from_le_bytes).collect();
            String::from_utf16_lossy(&u16s)
        } else {
            String::from_utf8_lossy(&bytes).into_owned()
        };
        let mut map = HashMap::new();
        for (name, cpu_ms, disk_b) in parse_wdi_startup_xml(&xml) {
            map.insert(name.trim().to_string(), (cpu_ms, disk_b));
        }
        let secs = mtime.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let ft = (secs + 11_644_473_600) * 10_000_000;
        let note = filetime_to_ymd(ft).map(|d| format!("WDI StartupInfo ({d})"));
        (map, note)
    }

    pub fn apply(op: &StartupOp) -> Result<String, String> {
        let StartupOp::SetEnabled { id, source, enabled } = op;
        let (hive, approved_subkey) = match source {
            StartupSource::HkcuRun | StartupSource::HkcuRunOnce => {
                (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run")
            }
            StartupSource::UserFolder => {
                (HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder")
            }
            StartupSource::HklmRun | StartupSource::HklmRunOnce => {
                (HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run")
            }
            StartupSource::HklmRun32 => {
                (HKEY_LOCAL_MACHINE, r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32")
            }
            StartupSource::CommonFolder => (
                HKEY_LOCAL_MACHINE,
                r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder",
            ),
            other => return Err(format!("Unsupported Windows startup source {:?}", other)),
        };

        let now_secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let bytes = encode_startup_approved(*enabled, now_secs);

        unsafe {
            let sub = to_wide(approved_subkey);
            let mut hkey: HKEY = null_mut();
            let st = RegCreateKeyExW(
                hive,
                sub.as_ptr(),
                0,
                null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                null(),
                &mut hkey,
                null_mut(),
            );
            if st != ERROR_SUCCESS {
                return Err(format!("RegCreateKeyExW failed (status {st})"));
            }
            let _guard = RegKey(hkey);
            let val_name = to_wide(id);
            let st = RegSetValueExW(hkey, val_name.as_ptr(), 0, REG_BINARY, bytes.as_ptr(), bytes.len() as u32);
            if st != ERROR_SUCCESS {
                return Err(format!("RegSetValueExW({id}) failed (status {st})"));
            }
            Ok(format!("{} startup entry {id}", if *enabled { "Enabled" } else { "Disabled" }))
        }
    }
}

// ---------------------------------------------------------------------------
// Linux (XDG Autostart + ACPI FPDT / systemd-analyze)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::path::{Path, PathBuf};

    pub fn query() -> BootSummary {
        let bios_time_ms = crate::os::read_u64("/sys/firmware/acpi/fpdt/boot/startup_raw").map(|ns| ns / 1_000_000);
        let mut items = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            let user_dir = Path::new(&home).join(".config/autostart");
            scan_xdg_dir(&user_dir, StartupSource::XdgUser, &mut items);
        }
        scan_xdg_dir(Path::new("/etc/xdg/autostart"), StartupSource::XdgSystem, &mut items);
        BootSummary { bios_time_ms, os_boot_ms: None, impact_source_note: None, items }
    }

    fn scan_xdg_dir(dir: &Path, source: StartupSource, out: &mut Vec<StartupRow>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("desktop") {
                continue;
            }
            let Some(fname) = p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()) else {
                continue;
            };
            if out.iter().any(|r| r.id == fname) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&p) else { continue };
            let mut name = fname.trim_end_matches(".desktop").to_string();
            let mut exec = String::new();
            let mut hidden = false;
            for line in text.lines() {
                if let Some(v) = line.strip_prefix("Name=") {
                    name = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("Exec=") {
                    exec = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("Hidden=") {
                    hidden = v.trim().eq_ignore_ascii_case("true");
                }
            }
            out.push(StartupRow {
                id: fname,
                name,
                publisher: String::new(),
                enabled: !hidden,
                disabled_date: None,
                impact: Impact::NotMeasured,
                cpu_ms: None,
                disk_bytes: None,
                source,
                command: exec,
                path: p.display().to_string(),
            });
        }
    }

    pub fn apply(op: &StartupOp) -> Result<String, String> {
        let StartupOp::SetEnabled { id, enabled, .. } = op;
        let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
        let user_dir = PathBuf::from(home).join(".config/autostart");
        std::fs::create_dir_all(&user_dir).map_err(|e| e.to_string())?;
        let user_file = user_dir.join(id);
        let sys_file = Path::new("/etc/xdg/autostart").join(id);
        let base = if user_file.exists() {
            std::fs::read_to_string(&user_file).unwrap_or_default()
        } else if sys_file.exists() {
            std::fs::read_to_string(&sys_file).unwrap_or_default()
        } else {
            String::from("[Desktop Entry]\nType=Application\n")
        };
        let mut lines: Vec<String> =
            base.lines().filter(|l| !l.starts_with("Hidden=")).map(|l| l.to_string()).collect();
        lines.push(format!("Hidden={}", !enabled));
        std::fs::write(&user_file, lines.join("\n") + "\n").map_err(|e| e.to_string())?;
        Ok(format!("Updated {}", user_file.display()))
    }
}

// ---------------------------------------------------------------------------
// macOS (LaunchAgents)
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::path::Path;

    pub fn query() -> BootSummary {
        let mut items = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            scan_agents(&Path::new(&home).join("Library/LaunchAgents"), StartupSource::LaunchAgentUser, &mut items);
        }
        scan_agents(Path::new("/Library/LaunchAgents"), StartupSource::LaunchAgentSystem, &mut items);
        BootSummary { bios_time_ms: None, os_boot_ms: None, impact_source_note: None, items }
    }

    fn scan_agents(dir: &Path, source: StartupSource, out: &mut Vec<StartupRow>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("plist") {
                continue;
            }
            let Some(fname) = p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()) else {
                continue;
            };
            let label = fname.trim_end_matches(".plist").to_string();
            out.push(StartupRow {
                id: label.clone(),
                name: label,
                publisher: String::new(),
                enabled: true,
                disabled_date: None,
                impact: Impact::NotMeasured,
                cpu_ms: None,
                disk_bytes: None,
                source,
                command: p.display().to_string(),
                path: p.display().to_string(),
            });
        }
    }

    pub fn apply(op: &StartupOp) -> Result<String, String> {
        let StartupOp::SetEnabled { id, enabled, .. } = op;
        let uid = unsafe { libc::getuid() };
        let verb = if *enabled { "enable" } else { "disable" };
        let target = format!("gui/{uid}/{id}");
        let out = std::process::Command::new("launchctl").args([verb, &target]).output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(format!("launchctl {verb} {target}"))
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    use super::*;
    pub fn query() -> BootSummary {
        BootSummary::default()
    }
    pub fn apply(_op: &StartupOp) -> Result<String, String> {
        Err("Not supported on this platform".into())
    }
}

//! Volume encryption (S1-G06), without admin:
//! - Windows: BitLocker protection as Explorer shows it (the shell property
//!   `System.Volume.BitLockerProtection`, read through PowerShell once, in
//!   the background: it takes a moment).
//! - Linux: dm-crypt / LUKS under the mounted device (through LVM too).
//! - macOS: FileVault (`fdesetup status`) for the system volume.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// volume → "BitLocker on", "Off", "LUKS"… once known.
pub type Status = HashMap<String, String>;

static DONE: OnceLock<Status> = OnceLock::new();
static STARTED: Mutex<bool> = Mutex::new(false);

/// The status of `volumes` (drive letters or mount points), or None while
/// it's still being read (the first call starts that).
pub fn status(volumes: &[String]) -> Option<&'static Status> {
    if let Some(s) = DONE.get() {
        return Some(s);
    }
    let mut started = STARTED.lock().unwrap_or_else(|e| e.into_inner());
    if !*started {
        *started = true;
        let v = volumes.to_vec();
        std::thread::spawn(move || {
            let _ = DONE.set(imp::read(&v));
        });
    }
    None
}

/// Windows' `System.Volume.BitLockerProtection` values.
pub fn bitlocker_text(v: u32) -> Option<&'static str> {
    match v {
        1 => Some("BitLocker on"),
        2 => Some("Off"),
        3 => Some("BitLocker (encrypting)"),
        4 => Some("BitLocker (decrypting)"),
        5 => Some("BitLocker suspended"),
        6 => Some("BitLocker on (locked)"),
        _ => None, // 0: the volume can't be encrypted
    }
}

#[cfg(windows)]
mod imp {
    pub fn read(volumes: &[String]) -> super::Status {
        let letters: Vec<String> =
            volumes.iter().filter_map(|v| v.get(..2).filter(|l| l.ends_with(':')).map(str::to_string)).collect();
        if letters.is_empty() {
            return Default::default();
        }
        let list = letters.iter().map(|l| format!("'{l}'")).collect::<Vec<_>>().join(",");
        let script = format!(
            "$s=(New-Object -ComObject Shell.Application).NameSpace(17);foreach($d in {list}){{$i=$s.ParseName($d);if($i){{$d+'='+$i.ExtendedProperty('System.Volume.BitLockerProtection')}}}}"
        );
        let out =
            crate::os::cmd("powershell", &["-NoProfile", "-NonInteractive", "-Command", &script]).unwrap_or_default();
        out.lines()
            .filter_map(|l| {
                let (d, v) = l.trim().split_once('=')?;
                Some((format!("{d}\\"), super::bitlocker_text(v.trim().parse().ok()?)?.to_string()))
            })
            .collect()
    }
}

#[cfg(target_os = "linux")]
mod imp {
    /// The dm device's crypt kind, looking through stacked dm devices (LVM on LUKS).
    fn crypt(dm: &str, depth: u32) -> Option<String> {
        let uuid = crate::os::read_trim(format!("/sys/block/{dm}/dm/uuid")).unwrap_or_default();
        if uuid.starts_with("CRYPT-LUKS") {
            return Some("LUKS".into());
        }
        if uuid.starts_with("CRYPT-") {
            return Some("dm-crypt".into());
        }
        if depth > 3 {
            return None;
        }
        std::fs::read_dir(format!("/sys/block/{dm}/slaves"))
            .ok()?
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.starts_with("dm-"))
            .find_map(|n| crypt(&n, depth + 1))
    }

    pub fn read(volumes: &[String]) -> super::Status {
        let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
        volumes
            .iter()
            .filter_map(|v| {
                let dev = mounts.lines().find_map(|l| {
                    let mut f = l.split_whitespace();
                    let (dev, at) = (f.next()?, f.next()?);
                    (at == v).then(|| dev.to_string())
                })?;
                let real = std::fs::canonicalize(&dev).ok()?;
                let name = real.file_name()?.to_string_lossy().into_owned();
                let kind = if name.starts_with("dm-") { crypt(&name, 0) } else { None };
                Some((v.clone(), kind.unwrap_or_else(|| "Off".into())))
            })
            .collect()
    }
}

#[cfg(target_os = "macos")]
mod imp {
    pub fn read(volumes: &[String]) -> super::Status {
        let out = crate::os::cmd("fdesetup", &["status"]).unwrap_or_default();
        let on = if out.contains("FileVault is On") {
            "FileVault on"
        } else if out.contains("FileVault is Off") {
            "Off"
        } else {
            return Default::default();
        };
        volumes.iter().filter(|v| v.as_str() == "/").map(|v| (v.clone(), on.to_string())).collect()
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn read(_: &[String]) -> super::Status {
        Default::default()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bitlocker_values() {
        assert_eq!(super::bitlocker_text(1), Some("BitLocker on"));
        assert_eq!(super::bitlocker_text(2), Some("Off"));
        assert_eq!(super::bitlocker_text(0), None);
        // Reading never blocks: the first call answers None and starts the read.
        let _ = super::status(&["C:\\".into(), "/".into()]);
    }
}

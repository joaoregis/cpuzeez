//! Firmware facts (S1-G04): UEFI or legacy BIOS, Secure Boot, the TPM and
//! the ACPI table list. No privileges needed (Linux needs root to read ACPI
//! table contents; their names are listed regardless).

pub use cpuzeez_core::acpi::Header;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Firmware {
    /// Booted through UEFI (false: legacy BIOS / CSM).
    pub uefi: Option<bool>,
    pub secure_boot: Option<bool>,
    /// "2.0", "1.2"; None: no TPM visible.
    pub tpm: Option<String>,
    /// One header per table signature (Linux without root: names only).
    pub acpi: Vec<Header>,
}

pub fn read() -> Firmware {
    imp::read()
}

#[cfg(windows)]
mod imp {
    use super::{Firmware, Header};
    use windows_sys::Win32::System::SystemInformation::{EnumSystemFirmwareTables, GetFirmwareType};
    use windows_sys::Win32::System::TpmBaseServices::{TPM_DEVICE_INFO, Tbsi_GetDeviceInfo};

    pub fn read() -> Firmware {
        let mut ft = 0;
        // SAFETY: plain out-param; FirmwareTypeUefi = 2.
        let uefi = (unsafe { GetFirmwareType(&mut ft) } != 0).then_some(ft == 2);
        let secure_boot =
            crate::os::win::reg_u64(r"SYSTEM\CurrentControlSet\Control\SecureBoot\State", "UEFISecureBootEnabled")
                .map(|v| v & 0xFF == 1);
        let mut info: TPM_DEVICE_INFO = unsafe { std::mem::zeroed() };
        // SAFETY: the struct is sized as documented; version 1 of the struct.
        let tpm = (unsafe {
            Tbsi_GetDeviceInfo(size_of::<TPM_DEVICE_INFO>() as u32, (&mut info as *mut TPM_DEVICE_INFO).cast())
        } == 0)
            .then(|| match info.tpmVersion {
                1 => "1.2".to_string(),
                2 => "2.0".to_string(),
                v => v.to_string(),
            });
        Firmware { uefi, secure_boot, tpm, acpi: acpi() }
    }

    fn acpi() -> Vec<Header> {
        let prov = u32::from_be_bytes(*b"ACPI");
        // SAFETY: size query then fill; the buffer holds 4-byte signatures.
        let sigs = unsafe {
            let n = EnumSystemFirmwareTables(prov, std::ptr::null_mut(), 0);
            let mut b = vec![0u8; n as usize];
            if n == 0 || EnumSystemFirmwareTables(prov, b.as_mut_ptr(), n) != n {
                return Vec::new();
            }
            b
        };
        let mut seen = Vec::new();
        sigs.as_chunks::<4>()
            .0
            .iter()
            .filter(|s| {
                let new = !seen.contains(*s);
                seen.push(**s);
                new
            })
            .filter_map(|s| {
                let id = u32::from_le_bytes(*s);
                // SAFETY: size query then fill, as documented.
                let b = unsafe {
                    let n = crate::os::win::GetSystemFirmwareTable(prov, id, std::ptr::null_mut(), 0);
                    let mut b = vec![0u8; n as usize];
                    (n > 0 && crate::os::win::GetSystemFirmwareTable(prov, id, b.as_mut_ptr(), n) == n).then_some(b)
                }?;
                Header::parse(&b)
            })
            .collect()
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{Firmware, Header};

    /// EFI_GLOBAL_VARIABLE GUID.
    const SECURE_BOOT: &str = "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c";

    pub fn read() -> Firmware {
        let uefi = Some(std::path::Path::new("/sys/firmware/efi").exists());
        // 4 bytes of attributes, then the value.
        let secure_boot = std::fs::read(SECURE_BOOT).ok().and_then(|b| b.get(4).map(|v| *v == 1));
        let tpm = std::fs::read_to_string("/sys/class/tpm/tpm0/tpm_version_major")
            .ok()
            .map(|v| if v.trim() == "2" { "2.0".to_string() } else { "1.2".to_string() })
            .or_else(|| std::path::Path::new("/sys/class/tpm/tpm0").exists().then(|| "1.2".to_string()));
        let mut acpi: Vec<Header> = std::fs::read_dir("/sys/firmware/acpi/tables")
            .map(|d| {
                d.flatten()
                    .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                    .map(|e| {
                        let name = e.file_name().to_string_lossy().into_owned();
                        // Unreadable without root: the name is the signature, numbered
                        // when a table repeats (SSDT1, SSDT2…).
                        std::fs::read(e.path()).ok().and_then(|b| Header::parse(&b)).unwrap_or(Header {
                            signature: name.chars().take(4).collect(),
                            length: 0,
                            revision: 0,
                            oem_id: String::new(),
                            oem_table_id: String::new(),
                            oem_revision: 0,
                            creator_id: String::new(),
                            creator_revision: 0,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        acpi.sort_by(|a, b| a.signature.cmp(&b.signature));
        Firmware { uefi, secure_boot, tpm, acpi }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn read() -> super::Firmware {
        super::Firmware::default()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads() {
        let f = super::read();
        for t in &f.acpi {
            assert_eq!(t.signature.len(), 4, "{t:?}");
        }
    }
}

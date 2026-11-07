//! What the OS says about the CPU (S1-G03): speculative-execution
//! vulnerabilities and their mitigations, virtualization (VT-x/AMD-V, a
//! hypervisor, VBS/HVCI) and the power policy (plan, governor, EPP). No
//! privileges needed.

/// (issue, status) as the OS reports it, e.g. ("Spectre v2", "Mitigated").
pub fn vulnerabilities() -> Vec<(String, String)> {
    imp::vulnerabilities()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Virtualization {
    /// VT-x / AMD-V switched on in the firmware (None: the OS can't tell,
    /// e.g. a hypervisor already owns it).
    pub firmware_enabled: Option<bool>,
    /// Windows Virtualization-Based Security running.
    pub vbs: Option<bool>,
    /// Hypervisor-enforced code integrity (Memory integrity) running.
    pub hvci: Option<bool>,
    /// Linux: /dev/kvm is there.
    pub kvm: Option<bool>,
}

pub fn virtualization() -> Virtualization {
    imp::virtualization()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Power {
    /// Windows power plan's name.
    pub plan: Option<String>,
    /// Linux cpufreq driver, governor and energy-performance preference.
    pub driver: Option<String>,
    pub governor: Option<String>,
    pub epp: Option<String>,
}

pub fn power() -> Power {
    imp::power()
}

#[cfg(windows)]
mod imp {
    use super::{Power, Virtualization};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::System::Power::{PowerGetActiveScheme, PowerReadFriendlyName};
    use windows_sys::Win32::System::Threading::{IsProcessorFeaturePresent, PF_VIRT_FIRMWARE_ENABLED};

    #[link(name = "ntdll")]
    unsafe extern "system" {
        fn NtQuerySystemInformation(class: u32, info: *mut u8, len: u32, ret: *mut u32) -> i32;
    }

    /// One documented `SYSTEM_INFORMATION_CLASS` read into `N` bytes.
    fn query<const N: usize>(class: u32) -> Option<[u8; N]> {
        let mut b = [0u8; N];
        let mut ret = 0;
        // SAFETY: the buffer is N bytes; the kernel writes at most that.
        (unsafe { NtQuerySystemInformation(class, b.as_mut_ptr(), N as u32, &mut ret) } >= 0).then_some(b)
    }

    const SPECULATION_CONTROL: u32 = 201;
    const KVA_SHADOW: u32 = 196;
    const CODE_INTEGRITY: u32 = 103;

    pub fn vulnerabilities() -> Vec<(String, String)> {
        let mut v = Vec::new();
        // Flag values as Microsoft's SpeculationControl module documents them.
        if let Some(b) = query::<8>(SPECULATION_CONTROL) {
            let f = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            let bti = if f & 0x01 != 0 {
                if f & 0x1_0000 != 0 { "Mitigated (enhanced IBRS)" } else { "Mitigated" }
            } else if f & 0x04 != 0 {
                "Not mitigated: no hardware support"
            } else if f & 0x02 != 0 {
                "Not mitigated: turned off by policy"
            } else {
                "Not mitigated"
            };
            v.push(("Spectre v2 (branch target injection)".to_string(), bti.to_string()));
            let ssb = if f & 0x1000 == 0 {
                "Not affected"
            } else if f & 0x400 != 0 {
                "Mitigated (system-wide)"
            } else {
                "Mitigated per process, where requested"
            };
            v.push(("Speculative Store Bypass (Spectre v4)".to_string(), ssb.to_string()));
        }
        if let Some(b) = query::<4>(KVA_SHADOW) {
            let f = u32::from_le_bytes(b);
            let meltdown = if f & 0x20 != 0 && f & 0x10 == 0 {
                "Not affected"
            } else if f & 0x01 != 0 {
                "Mitigated (KVA shadow)"
            } else {
                "Not mitigated"
            };
            v.push(("Meltdown (rogue data cache load)".to_string(), meltdown.to_string()));
        }
        v
    }

    pub fn virtualization() -> Virtualization {
        // SYSTEM_CODEINTEGRITY_INFORMATION { Length, CodeIntegrityOptions }; Length must be set.
        let ci = {
            let mut b = [0u8; 8];
            b[..4].copy_from_slice(&8u32.to_le_bytes());
            let mut ret = 0;
            // SAFETY: 8-byte buffer with its length field set, as the class requires.
            (unsafe { NtQuerySystemInformation(CODE_INTEGRITY, b.as_mut_ptr(), 8, &mut ret) } >= 0)
                .then(|| u32::from_le_bytes([b[4], b[5], b[6], b[7]]))
        };
        Virtualization {
            // SAFETY: plain query.
            firmware_enabled: Some(unsafe { IsProcessorFeaturePresent(PF_VIRT_FIRMWARE_ENABLED) } != 0),
            vbs: ci.map(|o| o & 0x2000 != 0), // CODEINTEGRITY_OPTION_HVCI_IUM_ENABLED
            hvci: ci.map(|o| o & 0x400 != 0), // CODEINTEGRITY_OPTION_HVCI_KMCI_ENABLED
            kvm: None,
        }
    }

    pub fn power() -> Power {
        let mut g = std::ptr::null_mut();
        // SAFETY: the API allocates the GUID (freed with LocalFree); the name
        // is read in two calls, sizing then filling.
        let plan = unsafe {
            if PowerGetActiveScheme(std::ptr::null_mut(), &mut g) != 0 || g.is_null() {
                None
            } else {
                let mut size = 0u32;
                PowerReadFriendlyName(
                    std::ptr::null_mut(),
                    g,
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    &mut size,
                );
                let mut buf = vec![0u8; size as usize];
                let ok = size > 0
                    && PowerReadFriendlyName(
                        std::ptr::null_mut(),
                        g,
                        std::ptr::null(),
                        std::ptr::null(),
                        buf.as_mut_ptr(),
                        &mut size,
                    ) == 0;
                LocalFree(g.cast());
                ok.then(|| {
                    let u: Vec<u16> = buf.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
                    String::from_utf16_lossy(&u).trim_end_matches('\0').trim().to_string()
                })
            }
        };
        Power { plan, ..Default::default() }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{Power, Virtualization};

    fn read(p: &str) -> Option<String> {
        std::fs::read_to_string(p).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }

    pub fn vulnerabilities() -> Vec<(String, String)> {
        let Ok(dir) = std::fs::read_dir("/sys/devices/system/cpu/vulnerabilities") else { return Vec::new() };
        let mut v: Vec<(String, String)> = dir
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().replace('_', " ");
                Some((name, read(e.path().to_str()?)?))
            })
            .collect();
        v.sort();
        v
    }

    pub fn virtualization() -> Virtualization {
        Virtualization { kvm: Some(std::path::Path::new("/dev/kvm").exists()), ..Default::default() }
    }

    pub fn power() -> Power {
        let f = |n: &str| read(&format!("/sys/devices/system/cpu/cpu0/cpufreq/{n}"));
        Power {
            plan: None,
            driver: f("scaling_driver"),
            governor: f("scaling_governor"),
            epp: f("energy_performance_preference"),
        }
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    use super::{Power, Virtualization};

    pub fn vulnerabilities() -> Vec<(String, String)> {
        Vec::new()
    }
    pub fn virtualization() -> Virtualization {
        Virtualization::default()
    }
    pub fn power() -> Power {
        Power::default()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_without_privileges() {
        for (name, status) in super::vulnerabilities() {
            assert!(!name.is_empty() && !status.is_empty());
        }
        let _ = (super::virtualization(), super::power());
    }
}

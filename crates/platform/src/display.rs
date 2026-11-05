//! Displays and the power plan, for build health (S1-G02). No privileges.
//! Windows only for displays (current vs best refresh at the current
//! resolution, and which adapter drives each); the power plan on Windows and
//! the CPU governor on Linux.

#[derive(Debug, Clone, PartialEq)]
pub struct Mode {
    /// The monitor as the OS names it.
    pub monitor: String,
    /// The display adapter driving it (DXGI/driver name).
    pub adapter: String,
    pub hz: u32,
    /// Best refresh rate the driver offers at the current resolution.
    pub max_hz: u32,
    /// The monitor's own name (from its EDID), when the OS has it.
    pub name: Option<String>,
    /// (HDR supported, HDR on), Windows 10 1709+.
    pub hdr: Option<(bool, bool)>,
    /// Bits per colour channel of the current signal.
    pub bits: Option<u32>,
}

/// Every display attached to the desktop.
pub fn modes() -> Vec<Mode> {
    imp::modes()
}

/// The OS is set to save power at the CPU's expense: Windows' "Power saver"
/// plan, or Linux's `powersave` governor under acpi-cpufreq (under
/// intel_pstate/amd-pstate "powersave" is the normal, dynamic default).
pub fn power_saver() -> Option<bool> {
    imp::power_saver()
}

#[cfg(windows)]
mod imp {
    use super::Mode;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Graphics::Gdi::{
        DEVMODEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, EnumDisplayDevicesW,
        EnumDisplaySettingsW,
    };
    use windows_sys::Win32::System::Power::PowerGetActiveScheme;

    fn wstr(b: &[u16]) -> String {
        String::from_utf16_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]).trim().to_string()
    }

    fn device() -> DISPLAY_DEVICEW {
        let mut d: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
        d.cb = size_of::<DISPLAY_DEVICEW>() as u32;
        d
    }

    fn mode() -> DEVMODEW {
        let mut m: DEVMODEW = unsafe { std::mem::zeroed() };
        m.dmSize = size_of::<DEVMODEW>() as u16;
        m
    }

    pub fn modes() -> Vec<Mode> {
        let mut v = Vec::new();
        let colour = advanced_colour();
        // SAFETY: documented enumeration; every struct has its size field set.
        unsafe {
            let mut i = 0;
            loop {
                let mut a = device();
                if EnumDisplayDevicesW(std::ptr::null(), i, &mut a, 0) == 0 {
                    break;
                }
                i += 1;
                if a.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0 {
                    continue;
                }
                let mut cur = mode();
                if EnumDisplaySettingsW(a.DeviceName.as_ptr(), ENUM_CURRENT_SETTINGS, &mut cur) == 0 {
                    continue;
                }
                let (w, h) = (cur.dmPelsWidth, cur.dmPelsHeight);
                let mut max_hz = cur.dmDisplayFrequency;
                let mut k = 0;
                let mut m = mode();
                while EnumDisplaySettingsW(a.DeviceName.as_ptr(), k, &mut m) != 0 {
                    if m.dmPelsWidth == w && m.dmPelsHeight == h {
                        max_hz = max_hz.max(m.dmDisplayFrequency);
                    }
                    k += 1;
                    m = mode();
                }
                let mut mon = device();
                let monitor = if EnumDisplayDevicesW(a.DeviceName.as_ptr(), 0, &mut mon, 0) != 0 {
                    wstr(&mon.DeviceString)
                } else {
                    wstr(&a.DeviceName)
                };
                let gdi = wstr(&a.DeviceName);
                let color = colour.iter().find(|c| c.0 == gdi);
                v.push(Mode {
                    monitor,
                    adapter: wstr(&a.DeviceString),
                    hz: cur.dmDisplayFrequency,
                    max_hz,
                    name: color.and_then(|c| c.1.clone()),
                    hdr: color.and_then(|c| c.2),
                    bits: color.and_then(|c| c.3),
                });
            }
        }
        v
    }

    /// (GDI name "\\.\DISPLAY1", EDID name, (HDR supported, on), bits per
    /// channel) for every active path, from the DisplayConfig API.
    /// (GDI name, EDID name, (HDR supported, on), bits per channel).
    type Colour = (String, Option<String>, Option<(bool, bool)>, Option<u32>);

    fn advanced_colour() -> Vec<Colour> {
        use windows_sys::Win32::Devices::Display::*;
        let (mut np, mut nm) = (0u32, 0u32);
        // SAFETY: sizes come from GetDisplayConfigBufferSizes; every request
        // packet has its header type, size and ids set.
        unsafe {
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != 0 {
                return Vec::new();
            }
            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
            if QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut np,
                paths.as_mut_ptr(),
                &mut nm,
                modes.as_mut_ptr(),
                std::ptr::null_mut(),
            ) != 0
            {
                return Vec::new();
            }
            paths.truncate(np as usize);
            paths
                .iter()
                .map(|p| {
                    let mut src = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
                    src.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
                    src.header.size = size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
                    src.header.adapterId = p.sourceInfo.adapterId;
                    src.header.id = p.sourceInfo.id;
                    let gdi = if DisplayConfigGetDeviceInfo(&mut src.header) == 0 {
                        wstr(&src.viewGdiDeviceName)
                    } else {
                        String::new()
                    };
                    let mut tgt = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
                    tgt.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
                    tgt.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
                    tgt.header.adapterId = p.targetInfo.adapterId;
                    tgt.header.id = p.targetInfo.id;
                    let name = (DisplayConfigGetDeviceInfo(&mut tgt.header) == 0)
                        .then(|| wstr(&tgt.monitorFriendlyDeviceName))
                        .filter(|n| !n.is_empty());
                    let mut col = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
                    col.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
                    col.header.size = size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
                    col.header.adapterId = p.targetInfo.adapterId;
                    col.header.id = p.targetInfo.id;
                    let (hdr, bits) = if DisplayConfigGetDeviceInfo(&mut col.header) == 0 {
                        let v = col.Anonymous.value;
                        (Some((v & 1 != 0, v & 2 != 0)), Some(col.bitsPerColorChannel).filter(|&b| b > 0))
                    } else {
                        (None, None)
                    };
                    (gdi, name, hdr, bits)
                })
                .collect()
        }
    }

    /// GUID_MAX_POWER_SAVINGS ("Power saver").
    const POWER_SAVER: [u8; 16] = guid(0xa1841308, 0x3541, 0x4fab, [0xbc, 0x81, 0xf7, 0x15, 0x56, 0xf2, 0x0b, 0x4a]);

    const fn guid(a: u32, b: u16, c: u16, d: [u8; 8]) -> [u8; 16] {
        let a = a.to_le_bytes();
        let (b, c) = (b.to_le_bytes(), c.to_le_bytes());
        [a[0], a[1], a[2], a[3], b[0], b[1], c[0], c[1], d[0], d[1], d[2], d[3], d[4], d[5], d[6], d[7]]
    }

    pub fn power_saver() -> Option<bool> {
        let mut g = std::ptr::null_mut();
        // SAFETY: the API allocates the GUID; it's freed with LocalFree.
        unsafe {
            if PowerGetActiveScheme(std::ptr::null_mut(), &mut g) != 0 || g.is_null() {
                return None;
            }
            let bytes = std::ptr::read_unaligned(g.cast::<[u8; 16]>());
            LocalFree(g.cast());
            Some(bytes == POWER_SAVER)
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    pub fn modes() -> Vec<super::Mode> {
        Vec::new() // ponytail: DRM modes need the KMS ioctls; add with the Linux display page.
    }

    pub fn power_saver() -> Option<bool> {
        let read = |f: &str| std::fs::read_to_string(format!("/sys/devices/system/cpu/cpu0/cpufreq/{f}")).ok();
        let (gov, driver) = (read("scaling_governor")?, read("scaling_driver")?);
        Some(gov.trim() == "powersave" && driver.trim() == "acpi-cpufreq")
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn modes() -> Vec<super::Mode> {
        Vec::new()
    }

    pub fn power_saver() -> Option<bool> {
        None
    }
}

/// The shell's small-icon size in pixels (the tray's), DPI-aware on Windows:
/// 16 at 100 %, 20 at 125 %, 24 at 150 %, 32 at 200 %.
pub fn small_icon_px() -> Option<usize> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};
        // SAFETY: no arguments.
        let px = unsafe { GetSystemMetrics(SM_CXSMICON) };
        (px >= 16).then_some(px as usize)
    }
    #[cfg(not(windows))]
    None
}

/// Whether a full-screen app (a game, a video, a presentation) has the
/// foreground, so desktop widgets can step aside. Windows only; elsewhere false.
pub fn fullscreen_app() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::SHQueryUserNotificationState;
        let mut state = 0;
        // SAFETY: writes one enum value into `state`.
        // QUNS_BUSY (2), QUNS_RUNNING_D3D_FULL_SCREEN (3), QUNS_PRESENTATION_MODE (4).
        (unsafe { SHQueryUserNotificationState(&mut state) }) >= 0 && matches!(state, 2..=4)
    }
    #[cfg(not(windows))]
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn modes_are_sane() {
        for m in super::modes() {
            assert!(m.max_hz >= m.hz, "{m:?}");
        }
        let _ = super::power_saver();
    }
}

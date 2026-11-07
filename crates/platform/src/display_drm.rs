//! Linux displays through KMS (the DRM modeset query ioctls), for the same
//! `display::Mode` list Windows has: the refresh rate each connected monitor
//! runs at now, and the best it offers at that resolution.
//!
//! The query ioctls (`GETRESOURCES`, `GETCONNECTOR`, `GETENCODER`, `GETCRTC`)
//! need no root and no DRM master, only access to `/dev/dri/card*` (the
//! logged-in user has it on a normal desktop). The byte decoding is plain
//! code, tested everywhere on fixtures; only the ioctls are Linux-only.

/// `struct drm_mode_modeinfo`: one video mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeInfo {
    pub clock_khz: u32,
    pub hdisplay: u16,
    pub htotal: u16,
    pub vdisplay: u16,
    pub vtotal: u16,
    pub vscan: u16,
    pub flags: u32,
}

/// Size of `struct drm_mode_modeinfo`.
pub const MODEINFO_SIZE: usize = 68;

const FLAG_INTERLACE: u32 = 1 << 4;
const FLAG_DBLSCAN: u32 = 1 << 5;

/// Decode a `drm_mode_modeinfo` (little-endian: every Linux target we ship).
pub fn parse_modeinfo(b: &[u8]) -> Option<ModeInfo> {
    let b = b.get(..MODEINFO_SIZE)?;
    let u16at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32at = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    // clock u32 | hdisplay hsync_start hsync_end htotal hskew | vdisplay vsync_start vsync_end vtotal vscan |
    // vrefresh u32 | flags u32 | type u32 | name[32]
    Some(ModeInfo {
        clock_khz: u32at(0),
        hdisplay: u16at(4),
        htotal: u16at(10),
        vdisplay: u16at(14),
        vtotal: u16at(20),
        vscan: u16at(22),
        flags: u32at(28),
    })
}

impl ModeInfo {
    /// The refresh rate the mode really runs at (59.94 for a "60 Hz" NTSC
    /// timing), the way the kernel computes `vrefresh`.
    pub fn hz(&self) -> f64 {
        let (h, mut v) = (f64::from(self.htotal), f64::from(self.vtotal));
        if h < 1.0 || v < 1.0 || self.clock_khz == 0 {
            return 0.0;
        }
        if self.flags & FLAG_DBLSCAN != 0 {
            v *= 2.0;
        }
        if self.flags & FLAG_INTERLACE != 0 {
            v /= 2.0;
        }
        if self.vscan > 1 {
            v *= f64::from(self.vscan);
        }
        f64::from(self.clock_khz) * 1000.0 / (h * v)
    }

    pub fn rounded_hz(&self) -> u32 {
        self.hz().round() as u32
    }
}

/// The best refresh `modes` offer at the resolution of `current`.
pub fn best_hz(current: &ModeInfo, modes: &[ModeInfo]) -> u32 {
    modes
        .iter()
        .filter(|m| (m.hdisplay, m.vdisplay) == (current.hdisplay, current.vdisplay))
        .map(ModeInfo::rounded_hz)
        .chain([current.rounded_hz()])
        .max()
        .unwrap_or(0)
}

/// The kernel's connector name: `HDMI-A-1`, `DP-2`, `eDP-1`.
pub fn connector_name(kind: u32, index: u32) -> String {
    const NAMES: [&str; 21] = [
        "Unknown",
        "VGA",
        "DVI-I",
        "DVI-D",
        "DVI-A",
        "Composite",
        "SVIDEO",
        "LVDS",
        "Component",
        "DIN",
        "DP",
        "HDMI-A",
        "HDMI-B",
        "TV",
        "eDP",
        "Virtual",
        "DSI",
        "DPI",
        "Writeback",
        "SPI",
        "USB",
    ];
    format!("{}-{index}", NAMES.get(kind as usize).unwrap_or(&"Unknown"))
}

#[cfg(target_os = "linux")]
pub use linux::modes;

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use crate::display::Mode;
    use std::os::fd::AsRawFd;

    #[repr(C)]
    #[derive(Default)]
    struct CardRes {
        fb_id_ptr: u64,
        crtc_id_ptr: u64,
        connector_id_ptr: u64,
        encoder_id_ptr: u64,
        count_fbs: u32,
        count_crtcs: u32,
        count_connectors: u32,
        count_encoders: u32,
        min_width: u32,
        max_width: u32,
        min_height: u32,
        max_height: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct GetConnector {
        encoders_ptr: u64,
        modes_ptr: u64,
        props_ptr: u64,
        prop_values_ptr: u64,
        count_modes: u32,
        count_props: u32,
        count_encoders: u32,
        encoder_id: u32,
        connector_id: u32,
        connector_type: u32,
        connector_type_id: u32,
        connection: u32,
        mm_width: u32,
        mm_height: u32,
        subpixel: u32,
        pad: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct GetEncoder {
        encoder_id: u32,
        encoder_type: u32,
        crtc_id: u32,
        possible_crtcs: u32,
        possible_clones: u32,
    }

    #[repr(C)]
    struct Crtc {
        set_connectors_ptr: u64,
        count_connectors: u32,
        crtc_id: u32,
        fb_id: u32,
        x: u32,
        y: u32,
        gamma_size: u32,
        mode_valid: u32,
        mode: [u8; MODEINFO_SIZE],
    }

    const _: () = assert!(size_of::<CardRes>() == 64);
    const _: () = assert!(size_of::<GetConnector>() == 80);
    const _: () = assert!(size_of::<GetEncoder>() == 20);
    const _: () = assert!(size_of::<Crtc>() == 104);

    /// `_IOWR('d', nr, size)`.
    const fn iowr(nr: u32, size: usize) -> u32 {
        (3 << 30) | ((size as u32) << 16) | ((b'd' as u32) << 8) | nr
    }
    const GETRESOURCES: u32 = iowr(0xA0, size_of::<CardRes>());
    const GETCRTC: u32 = iowr(0xA1, size_of::<Crtc>());
    const GETENCODER: u32 = iowr(0xA6, size_of::<GetEncoder>());
    const GETCONNECTOR: u32 = iowr(0xA7, size_of::<GetConnector>());
    const CONNECTED: u32 = 1;

    fn ioctl<T>(fd: i32, req: u32, arg: &mut T) -> bool {
        // SAFETY: `arg` is a live, exclusively borrowed `repr(C)` struct of exactly the size
        // encoded in `req`; the kernel only reads and writes within it (and within the
        // buffers whose pointers we put in it, which outlive the call).
        unsafe { libc::ioctl(fd, req as _, std::ptr::from_mut(arg)) == 0 }
    }

    fn driver_of(card: &str) -> String {
        std::fs::read_link(format!("/sys/class/drm/{card}/device/driver"))
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| card.to_string())
    }

    fn monitor_name(card: &str, connector: &str) -> Option<String> {
        let edid = std::fs::read(format!("/sys/class/drm/{card}-{connector}/edid")).ok()?;
        cpuzeez_core::gpu::edid::parse(&edid)?.model
    }

    fn card_modes(card: &str, out: &mut Vec<Mode>) -> Option<()> {
        let f = std::fs::OpenOptions::new().read(true).write(true).open(format!("/dev/dri/{card}")).ok()?;
        let fd = f.as_raw_fd();
        let mut res = CardRes::default();
        if !ioctl(fd, GETRESOURCES, &mut res) || res.count_connectors == 0 || res.count_connectors > 64 {
            return None;
        }
        let mut ids = vec![0u32; res.count_connectors as usize];
        let mut res = CardRes {
            connector_id_ptr: ids.as_mut_ptr() as u64,
            count_connectors: ids.len() as u32,
            ..Default::default()
        };
        if !ioctl(fd, GETRESOURCES, &mut res) {
            return None;
        }
        for id in ids {
            let mut c = GetConnector { connector_id: id, ..Default::default() };
            if !ioctl(fd, GETCONNECTOR, &mut c)
                || c.connection != CONNECTED
                || c.count_modes == 0
                || c.count_modes > 4096
            {
                continue;
            }
            let mut raw = vec![0u8; c.count_modes as usize * MODEINFO_SIZE];
            let mut c2 = GetConnector {
                connector_id: id,
                modes_ptr: raw.as_mut_ptr() as u64,
                count_modes: c.count_modes,
                ..Default::default()
            };
            if !ioctl(fd, GETCONNECTOR, &mut c2) {
                continue;
            }
            let list: Vec<ModeInfo> =
                raw.as_chunks::<MODEINFO_SIZE>().0.iter().filter_map(|c| parse_modeinfo(c)).collect();
            // The mode now on screen: connector -> encoder -> CRTC.
            let mut enc = GetEncoder { encoder_id: c2.encoder_id, ..Default::default() };
            if c2.encoder_id == 0 || !ioctl(fd, GETENCODER, &mut enc) || enc.crtc_id == 0 {
                continue; // connected but switched off
            }
            let mut crtc = Crtc {
                set_connectors_ptr: 0,
                count_connectors: 0,
                crtc_id: enc.crtc_id,
                fb_id: 0,
                x: 0,
                y: 0,
                gamma_size: 0,
                mode_valid: 0,
                mode: [0; MODEINFO_SIZE],
            };
            if !ioctl(fd, GETCRTC, &mut crtc) || crtc.mode_valid != 1 {
                continue;
            }
            let Some(now) = parse_modeinfo(&crtc.mode) else { continue };
            let name = connector_name(c2.connector_type, c2.connector_type_id);
            out.push(Mode {
                name: monitor_name(card, &name),
                monitor: name,
                adapter: driver_of(card),
                hz: now.rounded_hz(),
                max_hz: best_hz(&now, &list),
                hdr: None,
                bits: None,
            });
        }
        Some(())
    }

    /// Every connected, enabled display of every DRM card; empty when
    /// `/dev/dri` cannot be opened (a server, a container, no permission).
    pub fn modes() -> Vec<Mode> {
        let mut cards: Vec<String> = std::fs::read_dir("/dev/dri")
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.starts_with("card") && n[4..].chars().all(|c| c.is_ascii_digit()))
            .collect();
        cards.sort();
        let mut out = Vec::new();
        for card in cards {
            let _ = card_modes(&card, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `drm_mode_modeinfo` for 1920x1080 as the kernel lays it out.
    fn fixture(clock: u32, h: u16, htotal: u16, v: u16, vtotal: u16, flags: u32) -> Vec<u8> {
        let mut b = vec![0u8; MODEINFO_SIZE];
        b[0..4].copy_from_slice(&clock.to_le_bytes());
        b[4..6].copy_from_slice(&h.to_le_bytes());
        b[10..12].copy_from_slice(&htotal.to_le_bytes());
        b[14..16].copy_from_slice(&v.to_le_bytes());
        b[20..22].copy_from_slice(&vtotal.to_le_bytes());
        b[28..32].copy_from_slice(&flags.to_le_bytes());
        b
    }

    fn mode(clock: u32, h: u16, htotal: u16, v: u16, vtotal: u16, flags: u32) -> ModeInfo {
        parse_modeinfo(&fixture(clock, h, htotal, v, vtotal, flags)).unwrap()
    }

    #[test]
    fn refresh_from_the_timing() {
        // CEA 1080p60: 148.5 MHz, 2200 x 1125 -> exactly 60 Hz; the NTSC one is 59.94.
        assert_eq!(mode(148_500, 1920, 2200, 1080, 1125, 0).rounded_hz(), 60);
        let ntsc = mode(148_352, 1920, 2200, 1080, 1125, 0);
        assert!((ntsc.hz() - 59.94).abs() < 0.01, "{}", ntsc.hz());
        assert_eq!(ntsc.rounded_hz(), 60);
        // 1080p144 (CVT-RB): 325.08 MHz, 2080 x 1086.
        assert_eq!(mode(325_080, 1920, 2080, 1080, 1086, 0).rounded_hz(), 144);
        // Interlaced 1080i: the field rate is twice the frame rate of the timing.
        assert_eq!(mode(74_250, 1920, 2200, 1080, 1125, FLAG_INTERLACE).rounded_hz(), 60);
        // The classic VGA timing: 640x480 at 25.175 MHz is 60 Hz.
        assert_eq!(mode(25_175, 640, 800, 480, 525, 0).rounded_hz(), 60);
        assert_eq!(mode(0, 1920, 2200, 1080, 1125, 0).hz(), 0.0, "no clock, no rate");
        assert_eq!(mode(1, 0, 0, 0, 0, 0).hz(), 0.0, "no division by zero");
    }

    #[test]
    fn the_best_mode_is_at_the_current_resolution() {
        let now = mode(148_500, 1920, 2200, 1080, 1125, 0);
        let list = [
            now,
            mode(325_080, 1920, 2080, 1080, 1086, 0),   // 144 Hz, same size
            mode(594_000, 3840, 4400, 2160, 2250, 0),   // 60 Hz at 4K
            mode(1_000_000, 3840, 4000, 2160, 2000, 0), // 125 Hz at 4K: another size
        ];
        assert_eq!(best_hz(&now, &list), 144, "only modes of this resolution count");
        assert_eq!(best_hz(&now, &[]), 60, "the current mode is its own best");
    }

    #[test]
    fn short_or_garbled_input_never_panics() {
        assert!(parse_modeinfo(&[0; 10]).is_none());
        assert!(parse_modeinfo(&[]).is_none());
        let junk = vec![0xFFu8; MODEINFO_SIZE];
        let m = parse_modeinfo(&junk).unwrap();
        let _ = (m.hz(), m.rounded_hz(), best_hz(&m, &[m]));
    }

    #[test]
    fn connector_names_match_the_kernels() {
        assert_eq!(connector_name(11, 1), "HDMI-A-1");
        assert_eq!(connector_name(10, 2), "DP-2");
        assert_eq!(connector_name(14, 1), "eDP-1");
        assert_eq!(connector_name(999, 3), "Unknown-3");
    }
}

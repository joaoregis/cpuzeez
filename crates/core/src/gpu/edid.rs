//! VESA E-EDID 1.3/1.4 decoder (base block, CTA-861 and DisplayID extension
//! timings). Pure code, no OS access.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Monitor {
    pub model: Option<String>,
    pub vendor: String,
    /// PNP ID + product code, e.g. "AOC2702".
    pub id: String,
    pub serial: Option<String>,
    pub week: u8,
    pub year: u16,
    pub size_in: Option<f64>,
    /// (width, height, refresh Hz)
    pub max_res: Option<(u32, u32, u32)>,
    pub h_range_khz: Option<(u32, u32)>,
    pub v_range_hz: Option<(u32, u32)>,
    pub max_pclk_mhz: Option<u32>,
    pub gamma: Option<f64>,
    pub input: String,
    pub raw: Vec<u8>,
}

/// Plug and Play manufacturer IDs (UEFI PNP ID registry), common monitor makers.
const PNP: &[(&str, &str)] = &[
    ("ACI", "Asus"),
    ("ACR", "Acer"),
    ("AOC", "AOC International"),
    ("APP", "Apple"),
    ("AUO", "AU Optronics"),
    ("AUS", "Asus"),
    ("BNQ", "BenQ"),
    ("BOE", "BOE"),
    ("CMN", "Chimei Innolux"),
    ("DEL", "Dell"),
    ("ENC", "Eizo"),
    ("GBT", "Gigabyte"),
    ("GSM", "LG Electronics"),
    ("HPN", "HP"),
    ("HSD", "HannStar"),
    ("HWP", "HP"),
    ("IVM", "Iiyama"),
    ("LEN", "Lenovo"),
    ("LGD", "LG Display"),
    ("MEI", "Panasonic"),
    ("MSI", "MSI"),
    ("MSF", "Microsoft"),
    ("NEC", "NEC"),
    ("PHL", "Philips"),
    ("SAM", "Samsung"),
    ("SDC", "Samsung Display"),
    ("SEC", "Seiko Epson"),
    ("SHP", "Sharp"),
    ("SNY", "Sony"),
    ("TOS", "Toshiba"),
    ("VSC", "ViewSonic"),
    ("XMI", "Xiaomi"),
    ("HKC", "HKC"),
    ("KTC", "KTC"),
    ("PCK", "Sun Electronics"),
    ("FUS", "Fujitsu Siemens"),
    ("RHT", "Red Hat (virtual)"),
    ("MS_", "Microsoft (virtual)"),
];

pub fn pnp_name(id: &str) -> Option<&'static str> {
    PNP.iter().find(|(k, _)| *k == id).map(|(_, v)| *v)
}

fn text(d: &[u8]) -> Option<String> {
    let s: String = d.iter().take_while(|&&b| b != 0x0A && b != 0).map(|&b| b as char).collect();
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// 18-byte detailed timing descriptor → (h, v, Hz).
fn dtd(d: &[u8]) -> Option<(u32, u32, u32)> {
    let pclk = u16::from_le_bytes([d[0], d[1]]) as u64 * 10_000;
    if pclk == 0 {
        return None;
    }
    let ha = d[2] as u32 | ((d[4] as u32 >> 4) << 8);
    let hb = d[3] as u32 | ((d[4] as u32 & 15) << 8);
    let va = d[5] as u32 | ((d[7] as u32 >> 4) << 8);
    let vb = d[6] as u32 | ((d[7] as u32 & 15) << 8);
    let tot = (ha + hb) as u64 * (va + vb) as u64;
    (tot > 0).then(|| (ha, va, ((pclk as f64 / tot as f64).round()) as u32))
}

/// DisplayID type I (tag 0x03, 10 kHz) / type VII (tag 0x22, 1 kHz) timings.
fn displayid(b: &[u8], out: &mut Vec<(u32, u32, u32)>) {
    let le = |x: &[u8]| x.iter().rev().fold(0u64, |a, &v| a << 8 | v as u64);
    let mut i = 5; // ext tag, version, section length, product type, ext count
    let end = (5 + *b.get(2).unwrap_or(&0) as usize).min(b.len());
    while i + 3 <= end {
        let (tag, len) = (b[i], b[i + 2] as usize);
        let body = &b[(i + 3).min(end)..(i + 3 + len).min(end)];
        if tag == 0x03 || tag == 0x22 {
            let unit = if tag == 0x03 { 10_000.0 } else { 1_000.0 };
            for t in body.as_chunks::<20>().0 {
                let pclk = (le(&t[0..3]) + 1) as f64 * unit;
                let (ha, hb) = (le(&t[4..6]) + 1, le(&t[6..8]) + 1);
                let (va, vb) = (le(&t[12..14]) + 1, le(&t[14..16]) + 1);
                out.push((ha as u32, va as u32, (pclk / ((ha + hb) * (va + vb)) as f64).round() as u32));
            }
        }
        i += 3 + len;
    }
}

pub fn parse(e: &[u8]) -> Option<Monitor> {
    if e.len() < 128 || e[..8] != [0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0] {
        return None;
    }
    let m = u16::from_be_bytes([e[8], e[9]]);
    let pnp: String = [(m >> 10) & 31, (m >> 5) & 31, m & 31].iter().map(|&c| (b'@' + c as u8) as char).collect();
    let product = u16::from_le_bytes([e[10], e[11]]);
    let serial_num = u32::from_le_bytes([e[12], e[13], e[14], e[15]]);
    let mut mon = Monitor {
        vendor: pnp_name(&pnp).map_or(pnp.clone(), str::to_string),
        id: format!("{pnp}{product:04X}"),
        week: e[16],
        year: 1990 + e[17] as u16,
        gamma: (e[23] != 0xFF).then(|| (e[23] as f64 + 100.0) / 100.0),
        raw: e.to_vec(),
        ..Default::default()
    };
    mon.input = if e[20] & 0x80 == 0 {
        "Analog".into()
    } else {
        match e[20] & 15 {
            1 => "DVI",
            2 => "HDMI-a",
            3 => "HDMI-b",
            4 => "MDDI",
            5 => "Display Port",
            _ => "Digital",
        }
        .into()
    };
    let (w, h) = (e[21] as f64, e[22] as f64);
    if w > 0.0 && h > 0.0 {
        mon.size_in = Some((w * w + h * h).sqrt() / 2.54);
    }
    let mut timings = Vec::new();
    for d in e[54..126].as_chunks::<18>().0 {
        if d[0] != 0 || d[1] != 0 {
            timings.extend(dtd(d));
            continue;
        }
        match d[3] {
            0xFC => mon.model = text(&d[5..]),
            0xFF => mon.serial = text(&d[5..]),
            0xFD => {
                // EDID 1.4 offset flags: +255 on max/min for V (bits 1,0) and H (bits 3,2).
                let o = d[4];
                let add = |v: u8, bit: u8| v as u32 + if o & bit != 0 { 255 } else { 0 };
                let vmin = add(d[5], if o & 3 == 3 { 1 } else { 0 });
                let hmin = add(d[7], if o & 12 == 12 { 4 } else { 0 });
                mon.v_range_hz = Some((vmin, add(d[6], 2)));
                mon.h_range_khz = Some((hmin, add(d[8], 8)));
                mon.max_pclk_mhz = (d[9] != 0).then(|| d[9] as u32 * 10);
            }
            _ => {}
        }
    }
    for b in e[128..].as_chunks::<128>().0 {
        match b[0] {
            0x02 => {
                let start = (b[2] as usize).clamp(4, 127);
                for d in b[start..127].as_chunks::<18>().0 {
                    timings.extend(dtd(d));
                }
            }
            0x70 => displayid(b, &mut timings),
            _ => {}
        }
    }
    if mon.serial.is_none() && serial_num != 0 && serial_num != 0x01010101 {
        mon.serial = Some(serial_num.to_string());
    }
    // Largest area wins; the first (preferred) timing wins ties.
    mon.max_res = timings.into_iter().fold(None, |best: Option<(u32, u32, u32)>, t| match best {
        Some(b) if b.0 * b.1 >= t.0 * t.1 => Some(b),
        _ => Some(t),
    });
    Some(mon)
}

#[cfg(test)]
mod tests {
    /// Hand-built EDID 1.4: "ABC" 0x1234, 52x29 cm, DisplayPort, gamma 2.2,
    /// 2560x1440 @ 60 preferred, range 48-165 Hz / 30-250 kHz / 600 MHz.
    fn sample() -> Vec<u8> {
        let mut e = vec![0u8; 128];
        e[..8].copy_from_slice(&[0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0]);
        let m: u16 = (1 << 10) | (2 << 5) | 3; // "ABC"
        e[8..10].copy_from_slice(&m.to_be_bytes());
        e[10..12].copy_from_slice(&0x1234u16.to_le_bytes());
        e[16] = 12;
        e[17] = 34; // 2024
        e[18..21].copy_from_slice(&[1, 4, 0xA5]); // 1.4, digital, 8 bpc, DP
        e[21] = 52;
        e[22] = 29;
        e[23] = 120;
        // 2560x1440: htotal 2720, vtotal 1481, 241.5 MHz -> 60 Hz.
        let d = [0x56, 0x5E, 0x00, 0xA0, 0xA0, 0xA0, 0x29, 0x50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x1A];
        e[54..72].copy_from_slice(&d);
        let mut name =
            [0u8, 0, 0, 0xFC, 0, b'T', b'E', b'S', b'T', b'2', b'7', 0x0A, b' ', b' ', b' ', b' ', b' ', b' '];
        e[72..90].copy_from_slice(&name);
        name[3] = 0xFF;
        name[5..11].copy_from_slice(b"SN0042");
        e[90..108].copy_from_slice(&name);
        e[108..126]
            .copy_from_slice(&[0, 0, 0, 0xFD, 0, 48, 165, 30, 250, 60, 1, 0x0A, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20]);
        let s: u8 = e[..127].iter().fold(0u8, |a, &b| a.wrapping_add(b));
        e[127] = 0u8.wrapping_sub(s);
        e
    }

    #[test]
    fn decode() {
        let m = super::parse(&sample()).unwrap();
        assert_eq!(m.id, "ABC1234");
        assert_eq!(m.vendor, "ABC");
        assert_eq!(m.model.as_deref(), Some("TEST27"));
        assert_eq!(m.serial.as_deref(), Some("SN0042"));
        assert_eq!((m.week, m.year), (12, 2024));
        assert_eq!(format!("{:.1}", m.size_in.unwrap()), "23.4"); // sqrt(52²+29²)/2.54
        assert_eq!(m.max_res, Some((2560, 1440, 60)));
        assert_eq!(m.v_range_hz, Some((48, 165)));
        assert_eq!(m.h_range_khz, Some((30, 250)));
        assert_eq!(m.max_pclk_mhz, Some(600));
        assert_eq!(m.gamma, Some(2.2));
        assert_eq!(m.input, "Display Port");
        assert!(super::parse(&[0; 128]).is_none());
        assert_eq!(super::pnp_name("AOC"), Some("AOC International"));
    }

    #[test]
    fn displayid_ext() {
        let mut e = sample();
        let mut x = vec![0u8; 128];
        x[0] = 0x70;
        x[1] = 0x20;
        x[2] = 23; // section bytes: one 3-byte header + 20-byte timing
        x[5..8].copy_from_slice(&[0x22, 0, 20]);
        // 3840x2160, htotal 4000, vtotal 2222, 533.28 MHz (1 kHz units) -> 60 Hz.
        let le3 = (533_280u32 - 1).to_le_bytes();
        x[8..11].copy_from_slice(&le3[..3]);
        x[12..14].copy_from_slice(&(3840u16 - 1).to_le_bytes());
        x[14..16].copy_from_slice(&(160u16 - 1).to_le_bytes());
        x[20..22].copy_from_slice(&(2160u16 - 1).to_le_bytes());
        x[22..24].copy_from_slice(&(62u16 - 1).to_le_bytes());
        e[126] = 1;
        e.extend(x);
        assert_eq!(super::parse(&e).unwrap().max_res, Some((3840, 2160, 60)));
    }
}

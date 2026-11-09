//! Identify data decoded for people (S1-G06, Expert level): NVMe Identify
//! Controller (CNS 01h, 4096 bytes) and ATA IDENTIFY DEVICE (512 bytes),
//! plus a hex dump of the whole block.

fn le16(b: &[u8], o: usize) -> u16 {
    b.get(o..o + 2).map_or(0, |s| u16::from_le_bytes([s[0], s[1]]))
}
fn le32(b: &[u8], o: usize) -> u32 {
    b.get(o..o + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn ascii(b: &[u8]) -> String {
    String::from_utf8_lossy(b).trim_matches(|c: char| c == '\0' || c.is_whitespace()).to_string()
}

/// "1.4" from the NVMe VER field.
fn nvme_version(v: u32) -> String {
    match (v >> 16, (v >> 8) & 0xFF, v & 0xFF) {
        (0, ..) => "1.0 / 1.1".into(),
        (maj, min, 0) => format!("{maj}.{min}"),
        (maj, min, ter) => format!("{maj}.{min}.{ter}"),
    }
}

fn kelvin(k: u16) -> String {
    if k == 0 { "–".into() } else { format!("{} °C", k as i32 - 273) }
}

/// Key NVMe Identify Controller fields, in spec order.
pub fn nvme(id: &[u8]) -> Vec<(String, String)> {
    if id.len() < 4096 {
        return Vec::new();
    }
    let mut v: Vec<(&str, String)> = vec![
        ("PCI Vendor ID", format!("{:04X}", le16(id, 0))),
        ("Subsystem Vendor ID", format!("{:04X}", le16(id, 2))),
        ("Serial Number", ascii(&id[4..24])),
        ("Model Number", ascii(&id[24..64])),
        ("Firmware Revision", ascii(&id[64..72])),
        ("IEEE OUI", format!("{:02X}{:02X}{:02X}", id[75], id[74], id[73])),
        ("NVMe version", nvme_version(le32(id, 80))),
        (
            "Max data transfer",
            match id[77] {
                0 => "No limit".into(),
                m => format!("2^{m} × the minimum page"),
            },
        ),
        ("Controller ID", le16(id, 78).to_string()),
    ];
    let oacs = le16(id, 256);
    let flags = |bits: &[(u16, &str)], word: u16| -> String {
        let on: Vec<&str> = bits.iter().filter(|(b, _)| word & b != 0).map(|(_, n)| *n).collect();
        if on.is_empty() { "–".into() } else { on.join(", ") }
    };
    v.push((
        "Admin commands",
        flags(
            &[
                (1 << 0, "Security"),
                (1 << 1, "Format NVM"),
                (1 << 2, "Firmware download"),
                (1 << 3, "Namespace management"),
                (1 << 4, "Device self-test"),
                (1 << 5, "Directives"),
                (1 << 8, "Doorbell buffer config"),
                (1 << 9, "Get LBA status"),
            ],
            oacs,
        ),
    ));
    v.push(("Firmware slots", ((id[260] >> 1) & 0x7).to_string()));
    v.push(("Power states", (id[263] as u32 + 1).to_string()));
    v.push(("Autonomous power state transitions", if id[265] & 1 != 0 { "Yes" } else { "No" }.into()));
    v.push(("Warning temperature", kelvin(le16(id, 266))));
    v.push(("Critical temperature", kelvin(le16(id, 268))));
    let cap = u128::from_le_bytes(id[280..296].try_into().unwrap_or_default());
    if cap > 0 {
        v.push(("Total NVM capacity", format!("{:.1} GB", cap as f64 / 1e9)));
    }
    let hmb = le32(id, 272);
    if hmb > 0 {
        v.push(("Host memory buffer (preferred)", format!("{} MiB", hmb as u64 * 4096 / (1 << 20))));
    }
    v.push(("Namespaces", le32(id, 516).to_string()));
    let oncs = le16(id, 520);
    v.push((
        "NVM commands",
        flags(
            &[
                (1 << 0, "Compare"),
                (1 << 1, "Write uncorrectable"),
                (1 << 2, "Dataset management (TRIM)"),
                (1 << 3, "Write zeroes"),
                (1 << 4, "Save/select features"),
                (1 << 5, "Reservations"),
                (1 << 6, "Timestamp"),
                (1 << 7, "Verify"),
            ],
            oncs,
        ),
    ));
    v.push(("Volatile write cache", if id[525] & 1 != 0 { "Present" } else { "None" }.into()));
    // Power state descriptors: 32 bytes each from 2048; max power in centiwatts (×0.0001 W if MXPS).
    for ps in 0..=(id[263] as usize).min(31) {
        let d = &id[2048 + 32 * ps..2048 + 32 * (ps + 1)];
        let mp = le16(d, 0) as f64 * if d[3] & 1 != 0 { 0.0001 } else { 0.01 };
        let non_op = d[3] & 2 != 0;
        v.push((
            "",
            format!(
                "PS{ps}: {mp:.2} W{}, entry {} µs, exit {} µs",
                if non_op { " (idle)" } else { "" },
                le32(d, 4),
                le32(d, 8)
            ),
        ));
    }
    v.into_iter().map(|(k, s)| (if k.is_empty() { "Power state".into() } else { k.into() }, s)).collect()
}

/// ATA strings: two characters per word, high byte first.
fn ata_text(id: &[u8], words: std::ops::Range<usize>) -> String {
    let b: Vec<u8> = words.flat_map(|w| [id[2 * w + 1], id[2 * w]]).collect();
    ascii(&b)
}

/// Key ATA IDENTIFY DEVICE fields.
pub fn ata(id: &[u8]) -> Vec<(String, String)> {
    if id.len() < 512 {
        return Vec::new();
    }
    let w = |n: usize| le16(id, 2 * n);
    let sata = w(76);
    let generation = if sata & (1 << 3) != 0 {
        "SATA 3 (6 Gb/s)"
    } else if sata & (1 << 2) != 0 {
        "SATA 2 (3 Gb/s)"
    } else if sata & (1 << 1) != 0 {
        "SATA 1 (1.5 Gb/s)"
    } else {
        "–"
    };
    let lba48 = (0..4).map(|i| (w(100 + i) as u64) << (16 * i)).sum::<u64>();
    let sector =
        if w(106) & 0x4000 != 0 && w(106) & 0x1000 != 0 { (w(117) as u32 | (w(118) as u32) << 16) * 2 } else { 512 };
    let physical = if w(106) & 0x4000 != 0 && w(106) & 0x2000 != 0 { sector << (w(106) & 0xF) } else { sector };
    let major = (5..=12).rev().find(|b| w(80) & (1 << b) != 0).map_or("–".into(), |b| format!("ATA/ATAPI-{b}"));
    let mut v: Vec<(&str, String)> = vec![
        ("Serial Number", ata_text(id, 10..20)),
        ("Firmware Revision", ata_text(id, 23..27)),
        ("Model Number", ata_text(id, 27..47)),
        ("ATA version", major),
        ("SATA generation", generation.into()),
        ("Sectors (48-bit)", lba48.to_string()),
        ("Logical / physical sector", format!("{sector} / {physical} bytes")),
        (
            "Rotation",
            match w(217) {
                1 => "Solid state".into(),
                0 | 0xFFFF => "–".into(),
                r => format!("{r} RPM"),
            },
        ),
        ("Queue depth (NCQ)", if sata & (1 << 8) != 0 { (w(75) & 0x1F) as u32 + 1 } else { 1 }.to_string()),
    ];
    let yes = |b: bool| if b { "Yes" } else { "No" }.to_string();
    v.push(("SMART", yes(w(82) & 1 != 0)));
    v.push(("Security", yes(w(82) & 2 != 0)));
    v.push(("Write cache", yes(w(85) & (1 << 5) != 0)));
    v.push(("TRIM", yes(w(169) & 1 != 0)));
    v.push(("Advanced power management", yes(w(83) & (1 << 3) != 0)));
    v.into_iter().map(|(k, s)| (k.into(), s)).collect()
}

/// Classic hex dump: offset, 16 bytes, ASCII.
pub fn hex_dump(b: &[u8]) -> String {
    b.chunks(16)
        .enumerate()
        .map(|(i, c)| {
            let hex: Vec<String> = c.iter().map(|x| format!("{x:02X}")).collect();
            let text: String = c.iter().map(|&x| if (0x20..0x7F).contains(&x) { x as char } else { '.' }).collect();
            format!("{:04X}  {:<47}  {text}", i * 16, hex.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvme_fields() {
        let mut id = vec![0u8; 4096];
        id[0..2].copy_from_slice(&0x144Du16.to_le_bytes());
        id[4..24].copy_from_slice(b"S5XXNX0T123456      ");
        id[24..64].copy_from_slice(&[b"Samsung SSD 980 PRO 1TB".as_slice(), &[b' '; 17]].concat());
        id[64..72].copy_from_slice(b"5B2QGXA7");
        id[80..84].copy_from_slice(&0x0001_0400u32.to_le_bytes());
        id[256] = 0x17; // security, format, firmware, self-test
        id[263] = 1; // 2 power states
        id[266..268].copy_from_slice(&355u16.to_le_bytes());
        id[520] = 0x04;
        id[525] = 1;
        id[2048..2050].copy_from_slice(&850u16.to_le_bytes()); // PS0 8.50 W
        let f = nvme(&id);
        let get = |k: &str| f.iter().find(|x| x.0 == k).map(|x| x.1.clone()).unwrap_or_default();
        assert_eq!(get("PCI Vendor ID"), "144D");
        assert_eq!(get("Model Number"), "Samsung SSD 980 PRO 1TB");
        assert_eq!(get("NVMe version"), "1.4");
        assert_eq!(get("Admin commands"), "Security, Format NVM, Firmware download, Device self-test");
        assert_eq!(get("Warning temperature"), "82 °C");
        assert_eq!(get("NVM commands"), "Dataset management (TRIM)");
        assert!(f.iter().any(|x| x.0 == "Power state" && x.1.starts_with("PS0: 8.50 W")));
        assert!(nvme(&[0; 10]).is_empty());
    }

    #[test]
    fn ata_fields_and_dump() {
        let mut id = vec![0u8; 512];
        let mut put = |w: usize, v: u16| id[2 * w..2 * w + 2].copy_from_slice(&v.to_le_bytes());
        put(76, 1 << 3 | 1 << 8);
        put(75, 31);
        put(80, 1 << 10);
        put(82, 1);
        put(85, 1 << 5);
        put(100, 0x6DB0);
        put(101, 0x7470); // 1 953 525 168 sectors (1 TB)
        put(106, 0x6003); // 512 logical, 4096 physical
        put(169, 1);
        put(217, 7200);
        // Strings are stored byte-swapped within each word.
        let model = format!("{:<40}", "WDC WD10EZEX-00WN4A0");
        for (k, pair) in model.as_bytes().chunks(2).enumerate() {
            (id[54 + 2 * k], id[55 + 2 * k]) = (pair[1], pair[0]);
        }
        let f = ata(&id);
        let get = |k: &str| f.iter().find(|x| x.0 == k).map(|x| x.1.clone()).unwrap_or_default();
        assert_eq!(get("SATA generation"), "SATA 3 (6 Gb/s)");
        assert_eq!(get("Queue depth (NCQ)"), "32");
        assert_eq!(get("Sectors (48-bit)"), "1953525168");
        assert_eq!(get("Logical / physical sector"), "512 / 4096 bytes");
        assert_eq!(
            (get("Rotation"), get("TRIM"), get("ATA version")),
            ("7200 RPM".into(), "Yes".into(), "ATA/ATAPI-10".into())
        );
        assert_eq!(get("Model Number"), "WDC WD10EZEX-00WN4A0");
        let d = hex_dump(&[0x41, 0x42, 0, 0xFF]);
        assert_eq!(d, format!("0000  {:<47}  AB..", "41 42 00 FF"));
    }
}

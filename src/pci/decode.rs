//! Pure PCI configuration-space decoder (PCI Local Bus 3.0, PCIe Base Spec).
//! Input is whatever bytes the OS let us read; anything past the end is skipped.

use super::{Device, Link, vendor_label};
use crate::report::{Node, f, g, hex};

pub fn u8_at(c: &[u8], o: usize) -> Option<u8> {
    c.get(o).copied()
}
pub fn u16_at(c: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*c.get(o)?, *c.get(o + 1)?]))
}
pub fn u32_at(c: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes([*c.get(o)?, *c.get(o + 1)?, *c.get(o + 2)?, *c.get(o + 3)?]))
}

/// Standard capability list: (id, offset).
pub fn caps(c: &[u8]) -> Vec<(u8, usize)> {
    let mut v = Vec::new();
    if u16_at(c, 6).is_none_or(|s| s & 0x10 == 0) {
        return v;
    }
    let start = if u8_at(c, 0x0E).unwrap_or(0) & 0x7F == 2 { 0x14 } else { 0x34 };
    let mut p = u8_at(c, start).unwrap_or(0) as usize & 0xFC;
    for _ in 0..48 {
        if p < 0x40 || p + 1 >= c.len() {
            break;
        }
        v.push((c[p], p));
        p = c[p + 1] as usize & 0xFC;
    }
    v
}

/// Extended capability list (needs the 4 KB space): (id, offset).
pub fn ext_caps(c: &[u8]) -> Vec<(u16, usize)> {
    let mut v = Vec::new();
    let mut p = 0x100;
    for _ in 0..128 {
        let Some(h) = u32_at(c, p) else { break };
        if h == 0 || h == 0xFFFF_FFFF {
            break;
        }
        v.push((h as u16, p));
        p = (h >> 20) as usize & 0xFFC;
        if p < 0x100 {
            break;
        }
    }
    v
}

pub fn cap(c: &[u8], id: u8) -> Option<usize> {
    caps(c).into_iter().find(|x| x.0 == id).map(|x| x.1)
}

/// PCIe capability: (device/port type, slot implemented).
pub fn pcie_port(c: &[u8]) -> Option<(u8, bool)> {
    let o = cap(c, 0x10)?;
    let r = u16_at(c, o + 2)?;
    Some((((r >> 4) & 0xF) as u8, r & 0x100 != 0))
}

pub fn pcie_link(c: &[u8]) -> Option<Link> {
    let o = cap(c, 0x10)?;
    let lc = u32_at(c, o + 0x0C)?;
    let ls = u16_at(c, o + 0x12)?;
    if lc == 0 {
        return None;
    }
    Some(Link {
        speed: (ls & 0xF) as u8,
        width: ((ls >> 4) & 0x3F) as u8,
        max_speed: (lc & 0xF) as u8,
        max_width: ((lc >> 4) & 0x3F) as u8,
    })
}

/// Link speed code (Link Status / Link Capabilities encoding) → GT/s text.
pub fn speed_text(code: u8) -> String {
    match code {
        1 => "2.5 GT/s".into(),
        2 => "5.0 GT/s".into(),
        3 => "8.0 GT/s".into(),
        4 => "16.0 GT/s".into(),
        5 => "32.0 GT/s".into(),
        6 => "64.0 GT/s".into(),
        n => format!("unknown ({n})"),
    }
}

pub fn port_type_text(t: u8) -> String {
    match t {
        0 => "PCI-E Endpoint Device".into(),
        1 => "Legacy PCI-E Endpoint Device".into(),
        4 => "Root Port of PCI-E Root Complex".into(),
        5 => "Upstream Port of PCI-E Switch".into(),
        6 => "Downstream Port of PCI-E Switch".into(),
        7 => "PCI-E to PCI/PCI-X Bridge".into(),
        8 => "PCI/PCI-X to PCI-E Bridge".into(),
        9 => "Root Complex Integrated Endpoint".into(),
        0xA => "Root Complex Event Collector".into(),
        n => format!("0x{n:X}"),
    }
}

fn cap_name(id: u8) -> String {
    match id {
        0x01 => "Power Management",
        0x02 => "AGP",
        0x03 => "Virtual Product Data",
        0x04 => "Slot Identification",
        0x05 => "Message Signalled Interrupts",
        0x06 => "CompactPCI Hot Swap",
        0x07 => "PCI-X",
        0x08 => "HyperTransport",
        0x09 => "Vendor Dependant",
        0x0A => "Debug Port",
        0x0D => "Subsystem Vendor",
        0x0E => "AGP 8x",
        0x10 => "PCI Express",
        0x11 => "MSI-X",
        0x12 => "Serial ATA",
        _ => return format!("0x{id:X}"),
    }
    .into()
}

fn ext_name(id: u16) -> String {
    match id {
        0x01 => "Advanced Error Reporting",
        0x02 => "Virtual Channel",
        0x03 => "Device Serial Number",
        0x04 => "Power Budgeting",
        0x05 => "Root Complex Link Declaration",
        0x0B => "Vendor Specific",
        0x15 => "Resizable BAR",
        0x19 => "Secondary PCI Express",
        _ => return format!("0x{id:X}"),
    }
    .into()
}

fn ht_type(b3: u8) -> String {
    // Command register bits 15:11 (byte 3 of the capability header).
    match b3 >> 5 {
        0 => return "Slave/Primary Interface".into(),
        1 => return "Host/Secondary Interface".into(),
        _ => {}
    }
    match b3 >> 3 {
        0x08 => "Switch",
        0x10 => "Interrupt Discovery and Configuration",
        0x11 => "Revision ID",
        0x12 => "UnitID Clumping",
        0x13 => "Extended Configuration Space Access",
        0x14 => "Address Mapping",
        0x15 => "MSI Mapping",
        0x16 => "DirectRoute",
        0x17 => "VCSet",
        0x18 => "Retry Mode",
        0x19 => "X86 Encoding",
        0x1A => "Gen3",
        0x1B => "Function-Level Extension",
        0x1C => "Power Management",
        n => return format!("0x{n:X}"),
    }
    .into()
}

fn bytes_text(b: u64) -> String {
    if b >= 1 << 30 && b.is_multiple_of(1 << 30) {
        format!("{} GB", b >> 30)
    } else if b >= 1 << 20 {
        format!("{} MB", b >> 20)
    } else {
        format!("{} KB", b >> 10)
    }
}

fn cap_node(c: &[u8], id: u8, o: usize) -> Node {
    let mut n = vec![f("Caps class", cap_name(id)), f("Caps offset", format!("0x{o:X}"))];
    match id {
        0x01 => {
            if let Some(pmc) = u16_at(c, o + 2) {
                let v = match pmc & 7 {
                    1 => "1.0".to_string(),
                    2 => "1.1".into(),
                    3 => "1.2".into(),
                    x => x.to_string(),
                };
                n.push(f("Caps version", v));
            }
        }
        0x02 | 0x0E => {
            if let Some(b) = u8_at(c, o + 2) {
                n.push(f("Caps version", format!("{}.{}", b >> 4, b & 0xF)));
            }
        }
        0x08 => {
            if let Some(b) = u8_at(c, o + 3) {
                n.push(f("Interface type", ht_type(b)));
            }
        }
        0x0D => {
            if let (Some(v), Some(d)) = (u16_at(c, o + 4), u16_at(c, o + 6)) {
                n.push(f("SubVendor ID", vendor_label(v)));
                n.push(f("SubSystem ID", format!("0x{d:04X}")));
            }
        }
        0x10 => n.extend(pcie_fields(c, o)),
        0x12 => {
            if let Some(b) = u8_at(c, o + 2) {
                n.push(f("Caps version", format!("{}.{}", b >> 4, b & 0xF)));
            }
        }
        _ => {}
    }
    g("PCI capability", n)
}

fn pcie_fields(c: &[u8], o: usize) -> Vec<Node> {
    let mut n = Vec::new();
    let Some(r) = u16_at(c, o + 2) else { return n };
    let ty = ((r >> 4) & 0xF) as u8;
    n.push(f("Device type", port_type_text(ty)));
    if let Some(lc) = u32_at(c, o + 0x0C) {
        n.push(f("Port", lc >> 24));
    }
    n.push(f("Caps version", format!("{}.0", r & 0xF)));
    if ty == 4 || ty == 6 {
        if r & 0x100 != 0 {
            if let Some(sc) = u32_at(c, o + 0x14) {
                n.push(f("Physical slot", format!("#{}", sc >> 19)));
            }
            if let Some(ss) = u16_at(c, o + 0x1A) {
                n.push(f("Presence detect", if ss & 0x40 != 0 { "yes" } else { "no" }));
            }
        } else {
            n.push(f("Physical slot", "Integrated device"));
        }
    }
    if ty != 9
        && ty != 0xA
        && let Some(l) = pcie_link(c)
    {
        n.extend(link_fields(&l));
    }
    n
}

pub fn link_fields(l: &Link) -> Vec<Node> {
    vec![
        f("Link width", format!("{}x (max {}x)", l.width, l.max_width)),
        f("Link speed", format!("{} (max {})", speed_text(l.speed), speed_text(l.max_speed))),
    ]
}

fn ext_node(c: &[u8], id: u16, o: usize) -> Node {
    let mut n = vec![f("Caps class", ext_name(id)), f("Caps offset", format!("0x{o:X}"))];
    match id {
        0x03 => {
            if let (Some(lo), Some(hi)) = (u32_at(c, o + 4), u32_at(c, o + 8)) {
                n.push(f("Serial (EUI-64)", format!("{hi:X}-{lo:X}")));
            }
        }
        0x0B => {
            if let Some(h) = u32_at(c, o + 4) {
                n.push(f("Vendor", h & 0xFFFF));
                n.push(f("Size", h >> 20));
            }
        }
        0x15 => {
            let count = u32_at(c, o + 8).map_or(0, |r| (r >> 5) & 7) as usize;
            n.push(f("Resizable BARs", count));
            for i in 0..count {
                if let Some(ctl) = u32_at(c, o + 8 + i * 8) {
                    let sz = 1u64 << (20 + ((ctl >> 8) & 0x3F));
                    n.push(f(format!("BAR {i} size"), bytes_text(sz)));
                }
            }
        }
        _ => {}
    }
    g("PCI extended capability", n)
}

/// Type 0 BARs as CPU-Z prints them: raw value, 64-bit BARs merged.
fn bars(c: &[u8]) -> Vec<Node> {
    let mut n = Vec::new();
    let mut i = 0;
    while i < 6 {
        let Some(v) = u32_at(c, 0x10 + i * 4) else { break };
        if v & 1 == 1 {
            if v & !3 != 0 {
                n.push(f(format!("Address {i} (port)"), format!("0x{:08X}", v & !3)));
            }
        } else if (v >> 1) & 3 == 2 {
            let hi = u32_at(c, 0x14 + i * 4).unwrap_or(0);
            let full = (hi as u64) << 32 | v as u64;
            if full & !0xF != 0 {
                n.push(f(format!("Address {i} (memory)"), format!("0x{full:016X}")));
            }
            i += 1;
        } else if v & !0xF != 0 {
            n.push(f(format!("Address {i} (memory)"), format!("0x{v:08X}")));
        }
        i += 1;
    }
    n
}

/// Every node of one device in the PCI Devices section.
pub fn device_nodes(d: &Device) -> Vec<Node> {
    let c = &d.config[..];
    let b = d.bdf;
    let mut out = vec![
        f("Description", super::class_desc(d.class)),
        f(
            "Location",
            format!(
                "{}bus {} (0x{:02X}), device {} (0x{:02X}), function {} (0x{:02X})",
                if b.seg != 0 { format!("segment {}, ", b.seg) } else { String::new() },
                b.bus,
                b.bus,
                b.dev,
                b.dev,
                b.func,
                b.func
            ),
        ),
    ];
    let mut h = vec![
        f("Vendor ID", vendor_label(d.vendor)),
        f("Model ID", format!("0x{:04X}", d.device)),
        f("Revision ID", format!("0x{:02X}", d.revision)),
        f("PI", format!("0x{:02X}", d.class & 0xFF)),
        f("SubClass", format!("0x{:02X}", (d.class >> 8) & 0xFF)),
        f("BaseClass", format!("0x{:02X}", (d.class >> 16) & 0xFF)),
    ];
    for (k, o) in [("Cache Line", 0x0C), ("Latency", 0x0D), ("Header", 0x0E)] {
        if let Some(v) = u8_at(c, o) {
            h.push(f(k, format!("0x{v:02X}")));
        }
    }
    out.push(g("Common header", h));

    let hx = |o: usize| u8_at(c, o).map(|v| format!("0x{v:02X}"));
    let subsys =
        || vec![f("Subvendor ID", vendor_label(d.subvendor)), f("Subsystem ID", format!("0x{:04X}", d.subdevice))];
    let mut p = Vec::new();
    match (c.len() >= 0x40).then(|| c[0x0E] & 0x7F) {
        Some(0) => {
            p.extend(bars(c));
            p.extend(subsys());
        }
        Some(1) => {
            p.push(f("Primary bus", hx(0x18).unwrap_or_default()));
            p.push(f("Secondary bus", hx(0x19).unwrap_or_default()));
        }
        Some(2) => {
            p.push(f("Primary bus", hx(0x18).unwrap_or_default()));
            p.push(f("CardBus bus", hx(0x19).unwrap_or_default()));
            p.push(f("Subordinate bus", hx(0x1A).unwrap_or_default()));
        }
        Some(_) => {}
        None => p.extend(subsys()), // OS-reported IDs only, no config bytes
    }
    for (k, o) in [("Int. Line", 0x3C), ("Int. Pin", 0x3D)] {
        if let Some(v) = hx(o) {
            p.push(f(k, v));
        }
    }
    out.push(g("PCI header", p));

    let std = caps(c);
    for &(id, o) in &std {
        out.push(cap_node(c, id, o));
    }
    if !std.iter().any(|x| x.0 == 0x10) {
        // No config bytes: show what the OS told us about the link.
        if let Some(l) = &d.link {
            let mut n = vec![f("Caps class", "PCI Express")];
            if let Some(t) = d.port_type {
                n.push(f("Device type", port_type_text(t)));
            }
            n.extend(link_fields(l));
            out.push(g("PCI capability", n));
        }
    }
    let ext = if std.iter().any(|x| x.0 == 0x10) { ext_caps(c) } else { Vec::new() };
    for &(id, o) in &ext {
        out.push(ext_node(c, id, o));
    }
    if !c.is_empty() {
        let n = if ext.is_empty() { c.len().min(256) } else { c.len().min(512) };
        out.push(hex("PCI registers", c[..n].to_vec()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::Bdf;
    use crate::report::to_txt;

    /// Root port 00:01.1 of the reference machine (first 0x110 bytes of its dump).
    fn root_port() -> Vec<u8> {
        let rows = [
            "22 10 DB 14 07 04 10 00 00 00 04 06 10 00 81 00",
            "00 00 00 00 00 00 00 00 00 01 01 00 F1 F1 00 20",
            "00 DD 00 DE 01 00 F1 01 F8 00 00 00 FA 00 00 00",
            "00 00 00 00 50 00 00 00 00 00 00 00 00 00 00 00",
            "00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00",
            "01 58 03 C8 00 00 00 00 10 A0 42 01 22 80 00 00",
            "30 29 00 00 05 71 73 00 40 00 84 F0 80 25 04 00",
            "00 00 40 01 18 00 01 00 00 00 00 00 FF 19 73 00",
            "06 14 00 00 3E 00 80 01 44 00 1F 00 00 00 00 00",
            "00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00",
            "05 C0 81 00 04 00 E0 FE 00 00 00 00 00 00 00 00",
            "00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00",
            "0D C8 00 00 62 14 47 7E 08 00 03 A8 00 00 00 00",
            "00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00",
            "00 00 00 00 5C A3 0E 00 00 00 00 00 00 00 00 00",
            "00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00",
            "0B 00 01 00 01 00 01 01 00 00 00 00 00 00 00 00",
        ];
        let mut v: Vec<u8> =
            rows.iter().flat_map(|r| r.split(' ').map(|b| u8::from_str_radix(b, 16).unwrap())).collect();
        v.resize(4096, 0);
        v
    }

    fn dev(config: Vec<u8>) -> Device {
        Device {
            bdf: Bdf { seg: 0, bus: 0, dev: 1, func: 1 },
            vendor: 0x1022,
            device: 0x14DB,
            subvendor: 0x1462,
            subdevice: 0x7E47,
            revision: 0,
            class: 0x060400,
            config,
            name: None,
            link: None,
            port_type: None,
            parent: None,
        }
    }

    #[test]
    fn root_port_decode() {
        let c = root_port();
        assert_eq!(caps(&c), vec![(1, 0x50), (0x10, 0x58), (5, 0xA0), (0x0D, 0xC0), (8, 0xC8)]);
        assert_eq!(ext_caps(&c), vec![(0x0B, 0x100)]);
        let l = pcie_link(&c).unwrap();
        assert_eq!((l.speed, l.width, l.max_speed, l.max_width), (4, 8, 5, 16));
        let t = to_txt(&[crate::report::section("PCI Devices", device_nodes(&dev(c)))]);
        for want in [
            "Description\t\t\tPCI to PCI Bridge\n",
            "Location\t\t\tbus 0 (0x00), device 1 (0x01), function 1 (0x01)\n",
            "\tVendor ID\t\t0x1022 (AMD)\n",
            "\tHeader\t\t\t0x81\n",
            "\tSecondary bus\t\t0x01\n",
            "\tCaps version\t\t1.2\n",
            "\tDevice type\t\tRoot Port of PCI-E Root Complex\n",
            "\tPort\t\t\t0\n",
            "\tCaps version\t\t2.0\n",
            "\tPhysical slot\t\t#0\n",
            "\tPresence detect\t\tyes\n",
            "\tLink width\t\t8x (max 16x)\n",
            "\tLink speed\t\t16.0 GT/s (max 32.0 GT/s)\n",
            "\tSubVendor ID\t\t0x1462 (MSI)\n",
            "\tSubSystem ID\t\t0x7E47\n",
            "\tInterface type\t\tMSI Mapping\n",
            "\tCaps class\t\tVendor Specific\n",
            "\tVendor\t\t\t1\n",
            "\tSize\t\t\t16\n",
        ] {
            assert!(t.contains(want), "missing {want:?} in\n{t}");
        }
        assert!(t.contains("\t100\t0B 00 01 00"));
    }

    #[test]
    fn bars_and_ext() {
        // GPU-style BARs: 32-bit mem, two 64-bit prefetchable, one port.
        let mut c = vec![0u8; 0x40];
        c[0x10..0x28].copy_from_slice(&[
            0, 0, 0, 0xDD, 0x0C, 0, 0, 0, 0xF8, 0, 0, 0, 0x0C, 0, 0, 0, 0xFA, 0, 0, 0, 0x01, 0xF0, 0, 0,
        ]);
        let n: Vec<String> = bars(&c)
            .into_iter()
            .map(|x| match x {
                Node::Field { key, value } => format!("{key}={value}"),
                _ => String::new(),
            })
            .collect();
        assert_eq!(
            n,
            [
                "Address 0 (memory)=0xDD000000",
                "Address 1 (memory)=0x000000F80000000C",
                "Address 3 (memory)=0x000000FA0000000C",
                "Address 5 (port)=0x0000F000"
            ]
        );
        // DSN + resizable BAR.
        let mut e = vec![0u8; 0x200];
        e[0x100..0x10C].copy_from_slice(&[0x03, 0, 0x01, 0x18, 0x00, 0xE0, 0x4C, 0x68, 0, 0, 0, 1]);
        e[0x180..0x184].copy_from_slice(&[0x15, 0, 0x01, 0x00]);
        e[0x188..0x18C].copy_from_slice(&0x0000_0440u32.to_le_bytes()); // 2 BARs, 16 MB
        e[0x190..0x194].copy_from_slice(&0x0000_0D01u32.to_le_bytes()); // 8 GB
        let t = to_txt(&[crate::report::section(
            "x",
            ext_caps(&e).into_iter().map(|(id, o)| ext_node(&e, id, o)).collect(),
        )]);
        assert!(t.contains("Serial (EUI-64)\t\t1000000-684CE000"), "{t}");
        assert!(
            t.contains("Resizable BARs\t\t2") && t.contains("BAR 0 size\t\t16 MB") && t.contains("BAR 1 size\t\t8 GB"),
            "{t}"
        );
    }

    #[test]
    fn os_only_device() {
        let mut d = dev(Vec::new());
        d.link = Some(Link { speed: 4, width: 8, max_speed: 4, max_width: 8 });
        let t = to_txt(&[crate::report::section("x", device_nodes(&d))]);
        assert!(t.contains("\tSubvendor ID\t\t0x1462 (MSI)") && t.contains("Link width\t\t8x (max 8x)"));
        assert!(!t.contains("PCI registers"));
    }
}

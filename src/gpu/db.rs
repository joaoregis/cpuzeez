//! GPU configuration table keyed by PCI vendor/device (and revision when one
//! device ID covers several SKUs). Figures are the vendors' published specs
//! as collected on Wikipedia / vendor product pages. 0 = not published.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spec {
    pub vendor: u16,
    pub device: u16,
    pub rev: Option<u8>,
    pub codename: &'static str,
    pub cores: u32,
    pub rops: u32,
    pub tmus: u32,
    pub rt: u32,
    pub tensor: u32,
    pub nm: u32,
    pub mem: &'static str,
    pub bus: u32,
}

const NV: u16 = 0x10DE;
const AMD: u16 = 0x1002;
const INTEL: u16 = 0x8086;

#[allow(clippy::too_many_arguments)]
const fn s(
    vendor: u16,
    device: u16,
    rev: Option<u8>,
    codename: &'static str,
    cores: u32,
    rops: u32,
    tmus: u32,
    rt: u32,
    tensor: u32,
    nm: u32,
    mem: &'static str,
    bus: u32,
) -> Spec {
    Spec { vendor, device, rev, codename, cores, rops, tmus, rt, tensor, nm, mem, bus }
}

#[rustfmt::skip]
static TABLE: &[Spec] = &[
    // NVIDIA Turing (TSMC 12 nm)
    s(NV, 0x1E02, None, "TU102-400", 4608, 96, 288, 72, 576, 12, "GDDR6", 384),  // TITAN RTX
    s(NV, 0x1E04, None, "TU102-300", 4352, 88, 272, 68, 544, 12, "GDDR6", 352),  // RTX 2080 Ti
    s(NV, 0x1E07, None, "TU102-300A", 4352, 88, 272, 68, 544, 12, "GDDR6", 352),
    s(NV, 0x1E81, None, "TU104-450", 3072, 64, 192, 48, 384, 12, "GDDR6", 256),  // RTX 2080 SUPER
    s(NV, 0x1E82, None, "TU104-400", 2944, 64, 184, 46, 368, 12, "GDDR6", 256),  // RTX 2080
    s(NV, 0x1E87, None, "TU104-400A", 2944, 64, 184, 46, 368, 12, "GDDR6", 256),
    s(NV, 0x1E84, None, "TU104-410", 2560, 64, 160, 40, 320, 12, "GDDR6", 256),  // RTX 2070 SUPER
    s(NV, 0x1F02, None, "TU106-400", 2304, 64, 144, 36, 288, 12, "GDDR6", 256),  // RTX 2070
    s(NV, 0x1F07, None, "TU106-400A", 2304, 64, 144, 36, 288, 12, "GDDR6", 256),
    s(NV, 0x1F06, None, "TU106-410", 2176, 64, 136, 34, 272, 12, "GDDR6", 256),  // RTX 2060 SUPER
    s(NV, 0x1F08, None, "TU106-200", 1920, 48, 120, 30, 240, 12, "GDDR6", 192),  // RTX 2060
    s(NV, 0x1F03, None, "TU106-300", 2176, 48, 136, 34, 272, 12, "GDDR6", 192),  // RTX 2060 12GB
    s(NV, 0x2182, None, "TU116-400", 1536, 48, 96, 0, 0, 12, "GDDR6", 192),      // GTX 1660 Ti
    s(NV, 0x2184, None, "TU116-300", 1408, 48, 88, 0, 0, 12, "GDDR5", 192),      // GTX 1660
    s(NV, 0x21C4, None, "TU116-300", 1408, 48, 88, 0, 0, 12, "GDDR6", 192),      // GTX 1660 SUPER
    s(NV, 0x2187, None, "TU116-250", 1280, 32, 80, 0, 0, 12, "GDDR6", 128),      // GTX 1650 SUPER
    s(NV, 0x1F82, None, "TU117-300", 896, 32, 56, 0, 0, 12, "GDDR5", 128),       // GTX 1650
    // NVIDIA Ampere (Samsung 8 nm)
    s(NV, 0x2203, None, "GA102-350", 10752, 112, 336, 84, 336, 8, "GDDR6X", 384), // RTX 3090 Ti
    s(NV, 0x2204, None, "GA102-300", 10496, 112, 328, 82, 328, 8, "GDDR6X", 384), // RTX 3090
    s(NV, 0x2208, None, "GA102-225", 10240, 112, 320, 80, 320, 8, "GDDR6X", 384), // RTX 3080 Ti
    s(NV, 0x220A, None, "GA102-220", 8960, 96, 280, 70, 280, 8, "GDDR6X", 384),   // RTX 3080 12GB
    s(NV, 0x2206, None, "GA102-200", 8704, 96, 272, 68, 272, 8, "GDDR6X", 320),   // RTX 3080
    s(NV, 0x2216, None, "GA102-202", 8704, 96, 272, 68, 272, 8, "GDDR6X", 320),   // RTX 3080 LHR
    s(NV, 0x2482, None, "GA104-400", 6144, 96, 192, 48, 192, 8, "GDDR6X", 256),   // RTX 3070 Ti
    s(NV, 0x2484, None, "GA104-300", 5888, 96, 184, 46, 184, 8, "GDDR6", 256),    // RTX 3070
    s(NV, 0x2488, None, "GA104-302", 5888, 96, 184, 46, 184, 8, "GDDR6", 256),    // RTX 3070 LHR
    s(NV, 0x2486, None, "GA104-200", 4864, 80, 152, 38, 152, 8, "GDDR6", 256),    // RTX 3060 Ti
    s(NV, 0x2489, None, "GA104-202", 4864, 80, 152, 38, 152, 8, "GDDR6", 256),    // RTX 3060 Ti LHR
    s(NV, 0x2503, None, "GA106-300", 3584, 48, 112, 28, 112, 8, "GDDR6", 192),    // RTX 3060
    s(NV, 0x2504, None, "GA106-302", 3584, 48, 112, 28, 112, 8, "GDDR6", 192),    // RTX 3060 LHR
    s(NV, 0x2507, None, "GA106-150", 2560, 32, 80, 20, 80, 8, "GDDR6", 128),      // RTX 3050
    // NVIDIA Ada Lovelace (TSMC 4N, 5 nm class)
    s(NV, 0x2684, None, "AD102-300", 16384, 176, 512, 128, 512, 5, "GDDR6X", 384), // RTX 4090
    s(NV, 0x2702, None, "AD103-400", 10240, 112, 320, 80, 320, 5, "GDDR6X", 256),  // RTX 4080 SUPER
    s(NV, 0x2704, None, "AD103-300", 9728, 112, 304, 76, 304, 5, "GDDR6X", 256),   // RTX 4080
    s(NV, 0x2705, None, "AD103-275", 8448, 96, 264, 66, 264, 5, "GDDR6X", 256),    // RTX 4070 Ti SUPER
    s(NV, 0x2782, None, "AD104-400", 7680, 80, 240, 60, 240, 5, "GDDR6X", 192),    // RTX 4070 Ti
    s(NV, 0x2783, None, "AD104-350", 7168, 80, 224, 56, 224, 5, "GDDR6X", 192),    // RTX 4070 SUPER
    s(NV, 0x2786, None, "AD104-250", 5888, 64, 184, 46, 184, 5, "GDDR6X", 192),    // RTX 4070
    s(NV, 0x2803, None, "AD106-350", 4352, 48, 136, 34, 136, 5, "GDDR6", 128),     // RTX 4060 Ti 8GB
    s(NV, 0x2805, None, "AD106-351", 4352, 48, 136, 34, 136, 5, "GDDR6", 128),     // RTX 4060 Ti 16GB
    s(NV, 0x2882, None, "AD107-400", 3072, 48, 96, 24, 96, 5, "GDDR6", 128),       // RTX 4060
    // NVIDIA Blackwell (TSMC 4N, 5 nm class)
    s(NV, 0x2B85, None, "GB202-300", 21760, 176, 680, 170, 680, 5, "GDDR7", 512), // RTX 5090
    s(NV, 0x2C02, None, "GB203-400", 10752, 112, 336, 84, 336, 5, "GDDR7", 256),  // RTX 5080
    s(NV, 0x2C05, None, "GB203-300", 8960, 96, 280, 70, 280, 5, "GDDR7", 256),    // RTX 5070 Ti
    s(NV, 0x2F04, None, "GB205-300", 6144, 80, 192, 48, 192, 5, "GDDR7", 192),    // RTX 5070
    s(NV, 0x2D04, None, "GB206-300", 4608, 48, 144, 36, 144, 5, "GDDR7", 128),    // RTX 5060 Ti
    s(NV, 0x2D05, None, "GB206-250", 3840, 48, 120, 30, 120, 5, "GDDR7", 128),    // RTX 5060
    // AMD RDNA 1 (TSMC 7 nm)
    s(AMD, 0x731F, Some(0xC1), "Navi 10", 2560, 64, 160, 0, 0, 7, "GDDR6", 256), // RX 5700 XT
    s(AMD, 0x731F, Some(0xC4), "Navi 10", 2304, 64, 144, 0, 0, 7, "GDDR6", 256), // RX 5700
    s(AMD, 0x731F, Some(0xCA), "Navi 10", 2304, 64, 144, 0, 0, 7, "GDDR6", 192), // RX 5600 XT
    s(AMD, 0x7340, None, "Navi 14", 1408, 32, 88, 0, 0, 7, "GDDR6", 128),        // RX 5500 XT
    // AMD RDNA 2 (TSMC 7 nm; Navi 24 6 nm)
    s(AMD, 0x73A5, None, "Navi 21", 5120, 128, 320, 80, 0, 7, "GDDR6", 256),       // RX 6950 XT
    s(AMD, 0x73BF, Some(0xC0), "Navi 21", 5120, 128, 320, 80, 0, 7, "GDDR6", 256), // RX 6900 XT
    s(AMD, 0x73BF, Some(0xC1), "Navi 21", 4608, 128, 288, 72, 0, 7, "GDDR6", 256), // RX 6800 XT
    s(AMD, 0x73BF, Some(0xC3), "Navi 21", 3840, 96, 240, 60, 0, 7, "GDDR6", 256),  // RX 6800
    s(AMD, 0x73DF, Some(0xC0), "Navi 22", 2560, 64, 160, 40, 0, 7, "GDDR6", 192),  // RX 6750 XT
    s(AMD, 0x73DF, Some(0xC1), "Navi 22", 2560, 64, 160, 40, 0, 7, "GDDR6", 192),  // RX 6700 XT
    s(AMD, 0x73DF, Some(0xC5), "Navi 22", 2304, 64, 144, 36, 0, 7, "GDDR6", 160),  // RX 6700
    s(AMD, 0x73EF, None, "Navi 23", 2048, 64, 128, 32, 0, 7, "GDDR6", 128),        // RX 6650 XT
    s(AMD, 0x73FF, Some(0xC1), "Navi 23", 2048, 64, 128, 32, 0, 7, "GDDR6", 128),  // RX 6600 XT
    s(AMD, 0x73FF, Some(0xC7), "Navi 23", 1792, 64, 112, 28, 0, 7, "GDDR6", 128),  // RX 6600
    s(AMD, 0x743F, Some(0xC1), "Navi 24", 1024, 32, 64, 16, 0, 6, "GDDR6", 64),    // RX 6500 XT
    s(AMD, 0x743F, Some(0xC7), "Navi 24", 768, 32, 48, 12, 0, 6, "GDDR6", 64),     // RX 6400
    // AMD RDNA 3 (Navi 31/32: TSMC 5 nm GCD; Navi 33: 6 nm)
    s(AMD, 0x744C, Some(0xC8), "Navi 31", 6144, 192, 384, 96, 0, 5, "GDDR6", 384), // RX 7900 XTX
    s(AMD, 0x744C, Some(0xCC), "Navi 31", 5376, 192, 336, 84, 0, 5, "GDDR6", 320), // RX 7900 XT
    s(AMD, 0x744C, Some(0xCE), "Navi 31", 5120, 160, 320, 80, 0, 5, "GDDR6", 256), // RX 7900 GRE
    s(AMD, 0x747E, Some(0xC8), "Navi 32", 3840, 96, 240, 60, 0, 5, "GDDR6", 256),  // RX 7800 XT
    s(AMD, 0x747E, Some(0xFF), "Navi 32", 3456, 96, 216, 54, 0, 5, "GDDR6", 192),  // RX 7700 XT
    s(AMD, 0x7480, None, "Navi 33", 2048, 64, 128, 32, 0, 6, "GDDR6", 128),        // RX 7600 / 7600 XT
    // AMD RDNA 4 (TSMC 4 nm)
    s(AMD, 0x7550, Some(0xC0), "Navi 48", 4096, 128, 256, 64, 0, 4, "GDDR6", 256), // RX 9070 XT
    s(AMD, 0x7550, Some(0xC3), "Navi 48", 3584, 128, 224, 56, 0, 4, "GDDR6", 256), // RX 9070
    s(AMD, 0x7590, None, "Navi 44", 2048, 64, 128, 32, 0, 4, "GDDR6", 128),        // RX 9060 XT
    // AMD APU graphics (system memory)
    s(AMD, 0x1638, None, "Cezanne", 512, 8, 32, 0, 0, 7, "", 0),     // Radeon Vega 8
    s(AMD, 0x1681, None, "Rembrandt", 768, 32, 48, 12, 0, 6, "", 0), // Radeon 680M
    s(AMD, 0x164E, None, "Raphael", 128, 8, 8, 2, 0, 6, "", 0),      // Ryzen 7000 Radeon Graphics
    s(AMD, 0x13C0, None, "Granite Ridge", 128, 8, 8, 2, 0, 6, "", 0), // Ryzen 9000 Radeon Graphics
    s(AMD, 0x15BF, None, "Phoenix", 768, 32, 48, 12, 0, 4, "", 0),   // Radeon 780M
    s(AMD, 0x150E, None, "Strix Point", 1024, 32, 64, 16, 0, 4, "", 0), // Radeon 890M
    s(AMD, 0x1586, None, "Strix Halo", 2560, 64, 160, 40, 0, 4, "", 0), // Radeon 8060S
    // Intel Arc Alchemist (TSMC 6 nm) / Battlemage (TSMC 5 nm); "tensor" = XMX engines
    s(INTEL, 0x56A0, None, "DG2-512 (ACM-G10)", 4096, 128, 256, 32, 512, 6, "GDDR6", 256), // A770
    s(INTEL, 0x56A1, None, "DG2-512 (ACM-G10)", 3584, 112, 224, 28, 448, 6, "GDDR6", 256), // A750
    s(INTEL, 0x56A2, None, "DG2-512 (ACM-G10)", 3072, 96, 192, 24, 384, 6, "GDDR6", 256),  // A580
    s(INTEL, 0x56A5, None, "DG2-128 (ACM-G11)", 1024, 32, 64, 8, 128, 6, "GDDR6", 96),     // A380
    s(INTEL, 0x56A6, None, "DG2-128 (ACM-G11)", 768, 16, 32, 6, 96, 6, "GDDR6", 64),       // A310
    s(INTEL, 0xE20B, None, "BMG-G21", 2560, 80, 160, 20, 160, 5, "GDDR6", 192),            // B580
    s(INTEL, 0xE20C, None, "BMG-G21", 2304, 72, 144, 18, 144, 5, "GDDR6", 160),            // B570
    // Intel integrated (EUs x 8 ALUs)
    s(INTEL, 0x9A49, None, "Tiger Lake GT2", 768, 0, 0, 0, 0, 10, "", 0),   // Iris Xe 96 EU
    s(INTEL, 0x4680, None, "Alder Lake-S GT1", 256, 0, 0, 0, 0, 10, "", 0), // UHD 770
    s(INTEL, 0xA780, None, "Raptor Lake-S GT1", 256, 0, 0, 0, 0, 10, "", 0), // UHD 770
    s(INTEL, 0x7D55, None, "Meteor Lake (Xe-LPG)", 1024, 0, 0, 8, 0, 5, "", 0), // Arc Graphics
    s(INTEL, 0x64A0, None, "Lunar Lake (Xe2-LPG)", 1024, 0, 0, 8, 0, 3, "", 0), // Arc 140V
];

/// Exact SKU when known; for a multi-SKU device ID with an unknown revision,
/// only the die-level facts (codename, process, memory type).
pub fn lookup(vendor: u16, device: u16, rev: u8) -> Option<Spec> {
    let mut hits = TABLE.iter().filter(|e| e.vendor == vendor && e.device == device);
    let first = *hits.clone().next()?;
    if let Some(e) = hits.find(|e| e.rev.is_none_or(|r| r == rev)) {
        return Some(*e);
    }
    Some(Spec { cores: 0, rops: 0, tmus: 0, rt: 0, tensor: 0, bus: 0, ..first })
}

/// PCI subsystem vendor → board manufacturer (PCI-SIG vendor IDs).
pub fn board_vendor(id: u16) -> Option<&'static str> {
    Some(match id {
        0x1002 => "AMD",
        0x1025 => "Acer",
        0x1028 => "Dell",
        0x103C => "Hewlett-Packard Company",
        0x1043 => "ASUSTeK Computer Inc.",
        0x10B0 => "Gainward",
        0x10DE => "NVIDIA",
        0x1458 => "Gigabyte Technology",
        0x1462 => "Micro-Star International Co., Ltd. (MSI)",
        0x148C => "PowerColor",
        0x1569 => "Palit Microsystems",
        0x174B => "PC Partner / Sapphire",
        0x17AA => "Lenovo",
        0x1849 => "ASRock",
        0x196E => "PNY",
        0x19DA => "ZOTAC",
        0x1ACC => "Point of View",
        0x1B4C => "KFA2 / Galax",
        0x1DA2 => "Sapphire Technology",
        0x1682 => "XFX",
        0x3842 => "EVGA",
        0x7377 => "Colorful",
        0x8086 => "Intel",
        0x106B => "Apple",
        0x144D => "Samsung",
        0x1D05 => "Tongfang",
        _ => return None,
    })
}

pub fn vendor_name(id: u16) -> &'static str {
    match id {
        0x10DE => "NVIDIA",
        0x1002 | 0x1022 => "AMD",
        0x8086 => "Intel",
        0x106B => "Apple",
        0x5143 => "Qualcomm",
        0x1414 => "Microsoft",
        0x15AD => "VMware",
        0x1AF4 | 0x1B36 => "Red Hat (virtio)",
        0x80EE => "VirtualBox",
        0x1A03 => "ASPEED",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn lookup() {
        let s = super::lookup(0x10DE, 0x2882, 0xA1).unwrap();
        assert_eq!((s.codename, s.cores, s.bus, s.mem), ("AD107-400", 3072, 128, "GDDR6"));
        assert_eq!(super::lookup(0x1002, 0x744C, 0xCC).unwrap().cores, 5376);
        let die = super::lookup(0x1002, 0x744C, 0x01).unwrap();
        assert_eq!((die.codename, die.cores), ("Navi 31", 0));
        assert_eq!(super::lookup(0x1002, 0x164E, 0xC2).unwrap().cores, 128);
        assert!(super::lookup(0x1234, 1, 0).is_none());
    }
}

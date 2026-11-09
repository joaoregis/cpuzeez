//! Processor names database, built from public sources only (Intel ARK and
//! spec updates, AMD product pages and PPR titles, the Linux kernel's
//! family/model tables, InstLatx64, Wikipedia CPU lists). Our own tables,
//! keyed by vendor + family/model (+ stepping/brand where one model covers
//! several products). Values are best-effort; unknown fields stay None.

#[derive(Debug, Clone, Default)]
pub struct Info {
    pub codename: Option<String>,
    pub package: Option<String>,
    pub technology: Option<String>, // "5 nm"
    pub core_stepping: Option<String>,
    pub tdp_w: Option<f64>,
    pub tjmax_c: Option<f64>,
    /// Marketing name when the brand string needs cleaning ("AMD Ryzen 9 7900X").
    pub name: Option<String>,
}

/// One family/model block. First matching row wins, so specific rows
/// (brand or mobile filters) come before the generic one for that model.
struct Row {
    /// Vendor tags accepted: I Intel, A AMD, H Hygon, C Centaur/VIA, Z Zhaoxin.
    v: &'static str,
    fam: u32,
    m: (u32, u32),
    s: (u32, u32),
    /// "" = any brand, else '|'-separated substrings.
    brand: &'static str,
    /// None = any, Some(true) = mobile SKUs only, Some(false) = desktop only.
    mobile: Option<bool>,
    code: &'static str,
    pkg: &'static str,
    tech: &'static str,
    /// AMD revision prefix: "RPL" + revision -> "RPL-B2".
    abbr: &'static str,
    /// AMD revision letter; 0 = 'A' + low nibble of the model.
    rev: u8,
    tj: f64,
}

const ANY: (u32, u32) = (0, 0xF);
const D: Option<bool> = Some(false);
const M: Option<bool> = Some(true);

#[allow(clippy::too_many_arguments)]
const fn i(
    m: (u32, u32),
    s: (u32, u32),
    brand: &'static str,
    mobile: Option<bool>,
    code: &'static str,
    pkg: &'static str,
    tech: &'static str,
    tj: f64,
) -> Row {
    Row { v: "I", fam: 6, m, s, brand, mobile, code, pkg, tech, abbr: "", rev: 0, tj }
}
const fn i1(m: u32, code: &'static str, pkg: &'static str, tech: &'static str) -> Row {
    i((m, m), ANY, "", None, code, pkg, tech, 100.0)
}
#[allow(clippy::too_many_arguments)]
const fn a(
    fam: u32,
    m: (u32, u32),
    brand: &'static str,
    mobile: Option<bool>,
    code: &'static str,
    pkg: &'static str,
    tech: &'static str,
    abbr: &'static str,
    rev: u8,
    tj: f64,
) -> Row {
    Row { v: "A", fam, m, s: ANY, brand, mobile, code, pkg, tech, abbr, rev, tj }
}
const fn o(
    v: &'static str,
    fam: u32,
    m: (u32, u32),
    brand: &'static str,
    code: &'static str,
    pkg: &'static str,
    tech: &'static str,
) -> Row {
    Row { v, fam, m, s: ANY, brand, mobile: None, code, pkg, tech, abbr: "", rev: 0, tj: 0.0 }
}

const L775: &str = "Socket 775 LGA";
const L1366: &str = "Socket 1366 LGA";
const L1156: &str = "Socket 1156 LGA";
const L1155: &str = "Socket 1155 LGA";
const L2011: &str = "Socket 2011 LGA";
const L1150: &str = "Socket 1150 LGA";
const L2011_3: &str = "Socket 2011-3 LGA";
const L1151: &str = "Socket 1151 LGA";
const L2066: &str = "Socket 2066 LGA";
const L3647: &str = "Socket 3647 LGA";
const L1200: &str = "Socket 1200 LGA";
const L4189: &str = "Socket 4189 LGA";
const L1700: &str = "Socket 1700 LGA";
const L4677: &str = "Socket 4677 LGA";
const L1851: &str = "Socket 1851 LGA";
const L7529: &str = "Socket 7529 LGA";
const AM2P: &str = "Socket AM2+ (940)";
const AM3: &str = "Socket AM3 (938)";
const AM3P: &str = "Socket AM3+ (942)";
const AM4: &str = "Socket AM4 (1331)";
const AM5: &str = "Socket AM5 (LGA1718)";
const SP3: &str = "Socket SP3 (LGA4094)";
const SP5: &str = "Socket SP5 (LGA6096)";
const STR5: &str = "Socket sTR5 (LGA4844)";

#[rustfmt::skip]
static ROWS: &[Row] = &[
    // ---- Intel family 6: Core 2 ----
    i((0x0F, 0x0F), ANY, "Xeon", None, "Clovertown/Woodcrest", "Socket 771 LGA", "65 nm", 100.0),
    i((0x0F, 0x0F), ANY, "Quad", None, "Kentsfield", L775, "65 nm", 100.0),
    i((0x0F, 0x0F), ANY, "", M, "Merom", "Socket P (478)", "65 nm", 100.0),
    i((0x0F, 0x0F), ANY, "", None, "Conroe", L775, "65 nm", 100.0),
    i((0x17, 0x17), ANY, "Xeon", None, "Harpertown/Wolfdale-DP", "Socket 771 LGA", "45 nm", 100.0),
    i((0x17, 0x17), ANY, "Quad", None, "Yorkfield", L775, "45 nm", 100.0),
    i((0x17, 0x17), ANY, "", M, "Penryn", "Socket P (478)", "45 nm", 105.0),
    i((0x17, 0x17), ANY, "", None, "Wolfdale", L775, "45 nm", 100.0),
    i1(0x1D, "Dunnington", "Socket 604 mPGA", "45 nm"),
    // ---- Nehalem / Westmere ----
    i((0x1A, 0x1A), ANY, "Xeon", None, "Gainestown", L1366, "45 nm", 100.0),
    i1(0x1A, "Bloomfield", L1366, "45 nm"),
    i((0x1E, 0x1E), ANY, "", M, "Clarksfield", "Socket G1 (rPGA988A)", "45 nm", 100.0),
    i1(0x1E, "Lynnfield", L1156, "45 nm"),
    i((0x25, 0x25), ANY, "", M, "Arrandale", "Socket G1 (rPGA988A)", "32 nm", 105.0),
    i1(0x25, "Clarkdale", L1156, "32 nm"),
    i((0x2C, 0x2C), ANY, "Xeon", None, "Westmere-EP", L1366, "32 nm", 100.0),
    i1(0x2C, "Gulftown", L1366, "32 nm"),
    i1(0x2E, "Beckton", "Socket 1567 LGA", "45 nm"),
    i1(0x2F, "Westmere-EX", "Socket 1567 LGA", "32 nm"),
    // ---- Sandy Bridge .. Broadwell ----
    i((0x2A, 0x2A), ANY, "", M, "Sandy Bridge", "Socket G2 (rPGA988B)", "32 nm", 100.0),
    i((0x2A, 0x2A), ANY, "", None, "Sandy Bridge", L1155, "32 nm", 98.0),
    i((0x2D, 0x2D), ANY, "", None, "Sandy Bridge-E", L2011, "32 nm", 100.0),
    i((0x3A, 0x3A), ANY, "", M, "Ivy Bridge", "Socket G2 (rPGA988B)", "22 nm", 105.0),
    i((0x3A, 0x3A), ANY, "", None, "Ivy Bridge", L1155, "22 nm", 105.0),
    i((0x3E, 0x3E), ANY, "", None, "Ivy Bridge-E", L2011, "22 nm", 100.0),
    i((0x3C, 0x3C), ANY, "", M, "Haswell", "Socket G3 (rPGA946B)", "22 nm", 100.0),
    i1(0x3C, "Haswell", L1150, "22 nm"),
    i1(0x45, "Haswell-ULT", "BGA1168", "22 nm"),
    i1(0x46, "Crystal Well", "BGA1364", "22 nm"),
    i1(0x3F, "Haswell-E", L2011_3, "22 nm"),
    i1(0x3D, "Broadwell-U", "BGA1168", "14 nm"),
    i((0x47, 0x47), ANY, "", M, "Broadwell-H", "BGA1364", "14 nm", 100.0),
    i1(0x47, "Broadwell-H", L1150, "14 nm"),
    i1(0x4F, "Broadwell-E", L2011_3, "14 nm"),
    i1(0x56, "Broadwell-DE", "BGA1667", "14 nm"),
    // ---- Skylake family (14 nm) ----
    i((0x5E, 0x5E), ANY, "", M, "Skylake", "BGA1440", "14 nm", 100.0),
    i1(0x5E, "Skylake", L1151, "14 nm"),
    i1(0x4E, "Skylake-U/Y", "BGA1356", "14 nm"),
    i((0x55, 0x55), (0, 4), "Core", None, "Skylake-X", L2066, "14 nm", 100.0),
    i((0x55, 0x55), (0, 4), "", None, "Skylake-SP", L3647, "14 nm", 100.0),
    i((0x55, 0x55), (5, 7), "Core|W-22", None, "Cascade Lake-X", L2066, "14 nm", 100.0),
    i((0x55, 0x55), (5, 7), "", None, "Cascade Lake", L3647, "14 nm", 100.0),
    i((0x55, 0x55), (10, 11), "", None, "Cooper Lake", L4189, "14 nm", 100.0),
    i((0x8E, 0x8E), (12, 12), "-10", None, "Comet Lake-U", "BGA1528", "14 nm", 100.0),
    i((0x8E, 0x8E), (11, 12), "", None, "Whiskey Lake-U", "BGA1528", "14 nm", 100.0),
    i((0x8E, 0x8E), (10, 10), "", None, "Kaby Lake-R", "BGA1356", "14 nm", 100.0),
    i1(0x8E, "Kaby Lake", "BGA1356", "14 nm"),
    i((0x9E, 0x9E), (9, 9), "", M, "Kaby Lake", "BGA1440", "14 nm", 100.0),
    i((0x9E, 0x9E), (9, 9), "", None, "Kaby Lake", L1151, "14 nm", 100.0),
    i((0x9E, 0x9E), ANY, "", M, "Coffee Lake", "BGA1440", "14 nm", 100.0),
    i1(0x9E, "Coffee Lake", L1151, "14 nm"),
    i((0xA5, 0xA5), ANY, "", M, "Comet Lake", "BGA1440", "14 nm", 100.0),
    i1(0xA5, "Comet Lake", L1200, "14 nm"),
    i1(0xA6, "Comet Lake-U", "BGA1528", "14 nm"),
    i1(0xA7, "Rocket Lake", L1200, "14 nm"),
    // ---- 10 nm and later client ----
    i1(0x66, "Cannon Lake", "BGA1440", "10 nm"),
    i1(0x7E, "Ice Lake", "BGA1526", "10 nm"),
    i1(0x7D, "Ice Lake", "BGA1377", "10 nm"),
    i1(0x8C, "Tiger Lake", "BGA1449", "10 nm"),
    i1(0x8D, "Tiger Lake-H", "BGA1787", "10 nm"),
    i1(0x8A, "Lakefield", "BGA1016", "10 nm"),
    i((0x97, 0x97), ANY, "", M, "Alder Lake", "BGA1964", "10 nm", 100.0),
    i1(0x97, "Alder Lake", L1700, "10 nm"),
    i1(0x9A, "Alder Lake", "BGA1744", "10 nm"),
    i((0xBE, 0xBE), ANY, "N150|N250|N350|N355", None, "Twin Lake", "BGA1264", "10 nm", 105.0),
    i((0xBE, 0xBE), ANY, "", None, "Alder Lake-N", "BGA1264", "10 nm", 105.0),
    i((0xB7, 0xB7), ANY, "", M, "Raptor Lake", "BGA1964", "10 nm", 100.0),
    i1(0xB7, "Raptor Lake", L1700, "10 nm"),
    i((0xBF, 0xBF), ANY, "", M, "Raptor Lake", "BGA1964", "10 nm", 100.0),
    i1(0xBF, "Raptor Lake", L1700, "10 nm"),
    i1(0xBA, "Raptor Lake", "BGA1744", "10 nm"),
    i((0xAA, 0xAA), ANY, "", None, "Meteor Lake", "BGA2049", "Intel 4", 110.0),
    i((0xC6, 0xC6), ANY, "", M, "Arrow Lake", "BGA2114", "3 nm", 105.0),
    i((0xC6, 0xC6), ANY, "", None, "Arrow Lake", L1851, "3 nm", 105.0),
    i((0xC5, 0xC5), ANY, "", None, "Arrow Lake-H", "BGA2049", "3 nm", 110.0),
    i((0xB5, 0xB5), ANY, "", None, "Arrow Lake-U", "BGA2049", "Intel 3", 110.0),
    i1(0xBD, "Lunar Lake", "BGA2833", "3 nm"),
    i1(0xCC, "Panther Lake", "", "Intel 18A"),
    i1(0xD5, "Wildcat Lake", "", "Intel 18A"),
    o("I", 18, (0x00, 0x0F), "", "Nova Lake", "", ""),
    // ---- Xeon ----
    i1(0x6A, "Ice Lake-SP", L4189, "10 nm"),
    i1(0x6C, "Ice Lake-D", "BGA2227", "10 nm"),
    i1(0x8F, "Sapphire Rapids", L4677, "10 nm"),
    i1(0xCF, "Emerald Rapids", L4677, "10 nm"),
    i1(0xAD, "Granite Rapids", L7529, "Intel 3"),
    i1(0xAE, "Granite Rapids-D", "BGA4368", "Intel 3"),
    i1(0xAF, "Sierra Forest", L7529, "Intel 3"),
    i1(0xDD, "Clearwater Forest", L7529, "Intel 18A"),
    o("I", 19, (0x00, 0x0F), "", "Diamond Rapids", "", ""),
    i1(0x57, "Knights Landing", L3647, "14 nm"),
    i1(0x85, "Knights Mill", L3647, "14 nm"),
    // ---- Atom / E-core SoCs ----
    i1(0x1C, "Diamondville/Pineview", "BGA437", "45 nm"),
    i1(0x26, "Lincroft", "BGA518", "45 nm"),
    i1(0x36, "Cedarview", "BGA559", "32 nm"),
    i1(0x27, "Penwell", "", "32 nm"),
    i1(0x35, "Cloverview", "", "32 nm"),
    i1(0x37, "Bay Trail", "BGA1170", "22 nm"),
    i1(0x4D, "Avoton/Rangeley", "BGA1283", "22 nm"),
    i1(0x4A, "Merrifield", "", "22 nm"),
    i1(0x5A, "Moorefield", "", "22 nm"),
    i1(0x4C, "Cherry Trail/Braswell", "BGA1170", "14 nm"),
    i1(0x5C, "Apollo Lake", "BGA1296", "14 nm"),
    i1(0x5F, "Denverton", "BGA1310", "14 nm"),
    i1(0x7A, "Gemini Lake", "BGA1090", "14 nm"),
    i1(0x86, "Snow Ridge/Jacobsville", "BGA2518", "10 nm"),
    i1(0x96, "Elkhart Lake", "BGA1493", "10 nm"),
    i1(0x9C, "Jasper Lake", "BGA1338", "10 nm"),
    i1(0xB6, "Grand Ridge", "", "10 nm"),

    // ---- AMD K10 ----
    a(0x10, (0x02, 0x02), "Opteron", None, "Barcelona", "Socket F (1207)", "65 nm", "BA", b'B', 0.0),
    a(0x10, (0x02, 0x02), "", None, "Agena", AM2P, "65 nm", "DR", b'B', 0.0),
    a(0x10, (0x04, 0x04), "Opteron", None, "Shanghai", "Socket F (1207)", "45 nm", "RB", b'C', 0.0),
    a(0x10, (0x04, 0x04), "", None, "Deneb", AM3, "45 nm", "RB", b'C', 0.0),
    a(0x10, (0x05, 0x05), "", None, "Propus", AM3, "45 nm", "BL", b'C', 0.0),
    a(0x10, (0x06, 0x06), "", None, "Regor", AM3, "45 nm", "DA", b'C', 0.0),
    a(0x10, (0x08, 0x08), "", None, "Istanbul", "Socket F (1207)", "45 nm", "HY", b'D', 0.0),
    a(0x10, (0x09, 0x09), "", None, "Magny-Cours", "Socket G34 (LGA1944)", "45 nm", "HY", b'D', 0.0),
    a(0x10, (0x0A, 0x0A), "", None, "Thuban", AM3, "45 nm", "PH", b'E', 0.0),
    a(0x11, (0x03, 0x03), "", None, "Griffin", "Socket S1g2", "65 nm", "LG", b'B', 0.0),
    a(0x12, (0x01, 0x01), "", None, "Llano", "Socket FM1 (905)", "32 nm", "LN", 0, 0.0),
    a(0x14, (0x00, 0x0F), "", None, "Ontario/Zacate", "BGA413 (FT1)", "40 nm", "ON", 0, 0.0),
    // ---- Bulldozer family ----
    a(0x15, (0x01, 0x01), "Opteron", None, "Interlagos", "Socket G34 (LGA1944)", "32 nm", "OR", 0, 0.0),
    a(0x15, (0x01, 0x01), "", None, "Zambezi", AM3P, "32 nm", "OR", 0, 0.0),
    a(0x15, (0x02, 0x02), "Opteron", None, "Abu Dhabi", "Socket G34 (LGA1944)", "32 nm", "OR", 0, 0.0),
    a(0x15, (0x02, 0x02), "", None, "Vishera", AM3P, "32 nm", "OR", 0, 0.0),
    a(0x15, (0x10, 0x10), "", None, "Trinity", "Socket FM2 (904)", "32 nm", "TN", 0, 0.0),
    a(0x15, (0x13, 0x13), "", None, "Richland", "Socket FM2 (904)", "32 nm", "RL", b'A', 0.0),
    a(0x15, (0x30, 0x30), "", None, "Kaveri", "Socket FM2+ (906)", "28 nm", "KV", 0, 0.0),
    a(0x15, (0x38, 0x38), "", None, "Godavari", "Socket FM2+ (906)", "28 nm", "KV", b'A', 0.0),
    a(0x15, (0x60, 0x60), "", None, "Carrizo", "Socket FP4", "28 nm", "CZ", 0, 0.0),
    a(0x15, (0x65, 0x65), "", M, "Bristol Ridge", "Socket FP4", "28 nm", "BR", b'A', 0.0),
    a(0x15, (0x65, 0x65), "", None, "Bristol Ridge", AM4, "28 nm", "BR", b'A', 0.0),
    a(0x15, (0x70, 0x70), "", None, "Stoney Ridge", "Socket FP4", "28 nm", "ST", 0, 0.0),
    a(0x16, (0x00, 0x0F), "", None, "Kabini", "Socket AM1 (FS1b)", "28 nm", "KB", 0, 0.0),
    a(0x16, (0x30, 0x3F), "", None, "Beema/Mullins", "BGA769 (FT3b)", "28 nm", "ML", 0, 0.0),
    // ---- Zen / Zen+ / Zen 2 (family 17h) ----
    a(0x17, (0x01, 0x01), "EPYC", None, "Naples", SP3, "14 nm", "ZP", 0, 0.0),
    a(0x17, (0x01, 0x01), "Threadripper", None, "Whitehaven", "Socket TR4 (LGA4094)", "14 nm", "ZP", 0, 68.0),
    a(0x17, (0x01, 0x01), "", None, "Summit Ridge", AM4, "14 nm", "ZP", 0, 95.0),
    a(0x17, (0x08, 0x08), "Threadripper", None, "Colfax", "Socket TR4 (LGA4094)", "12 nm", "PiR", b'B', 68.0),
    a(0x17, (0x08, 0x08), "", None, "Pinnacle Ridge", AM4, "12 nm", "PiR", b'B', 95.0),
    a(0x17, (0x11, 0x11), "", M, "Raven Ridge", "Socket FP5", "14 nm", "RV", 0, 105.0),
    a(0x17, (0x11, 0x11), "", None, "Raven Ridge", AM4, "14 nm", "RV", 0, 95.0),
    a(0x17, (0x18, 0x18), "", M, "Picasso", "Socket FP5", "12 nm", "PCO", b'B', 105.0),
    a(0x17, (0x18, 0x18), "", None, "Picasso", AM4, "12 nm", "PCO", b'B', 95.0),
    a(0x17, (0x20, 0x20), "", M, "Dali", "Socket FP5", "14 nm", "RV2", 0, 105.0),
    a(0x17, (0x20, 0x20), "", None, "Dali", AM4, "14 nm", "RV2", 0, 95.0),
    a(0x17, (0x31, 0x31), "Threadripper PRO", None, "Castle Peak", "Socket sWRX8 (LGA4094)", "7 nm", "SSP", 0, 95.0),
    a(0x17, (0x31, 0x31), "Threadripper", None, "Castle Peak", "Socket sTRX4 (LGA4094)", "7 nm", "SSP", 0, 95.0),
    a(0x17, (0x31, 0x31), "", None, "Rome", SP3, "7 nm", "SSP", 0, 0.0),
    a(0x17, (0x60, 0x60), "", M, "Renoir", "Socket FP6", "7 nm", "RN", 0, 105.0),
    a(0x17, (0x60, 0x60), "", None, "Renoir", AM4, "7 nm", "RN", 0, 95.0),
    a(0x17, (0x68, 0x68), "", None, "Lucienne", "Socket FP6", "7 nm", "LCN", b'A', 105.0),
    a(0x17, (0x71, 0x71), "", None, "Matisse", AM4, "7 nm", "MTS", 0, 95.0),
    a(0x17, (0x90, 0x91), "", None, "Van Gogh", "Socket FF3", "7 nm", "VN", 0, 100.0),
    a(0x17, (0xA0, 0xA0), "", None, "Mendocino", "Socket FT6", "6 nm", "MDN", 0, 95.0),
    // ---- Zen 3 / Zen 4 (family 19h) ----
    a(0x19, (0x00, 0x01), "", None, "Milan", SP3, "7 nm", "GN", 0, 0.0),
    a(0x19, (0x08, 0x08), "", None, "Chagall", "Socket sWRX8 (LGA4094)", "7 nm", "CGL", 0, 95.0),
    a(0x19, (0x10, 0x11), "", None, "Genoa", SP5, "5 nm", "RS", 0, 0.0),
    a(0x19, (0x18, 0x18), "", None, "Storm Peak", STR5, "5 nm", "STP", b'B', 95.0),
    a(0x19, (0x21, 0x21), "", None, "Vermeer", AM4, "7 nm", "VMR", 0, 90.0),
    a(0x19, (0x40, 0x44), "", None, "Rembrandt", "Socket FP7", "6 nm", "RMB", b'A', 95.0),
    a(0x19, (0x50, 0x50), "25U|30U|5125C", None, "Barcelo", "Socket FP6", "7 nm", "CZN", 0, 95.0),
    a(0x19, (0x50, 0x50), "", M, "Cezanne", "Socket FP6", "7 nm", "CZN", 0, 105.0),
    a(0x19, (0x50, 0x50), "", None, "Cezanne", AM4, "7 nm", "CZN", 0, 95.0),
    a(0x19, (0x61, 0x61), "", M, "Dragon Range", "Socket FL1", "5 nm", "RPL", 0, 100.0),
    a(0x19, (0x61, 0x61), "", None, "Raphael", AM5, "5 nm", "RPL", 0, 95.0),
    a(0x19, (0x74, 0x74), "", M, "Phoenix", "Socket FP7/FP8", "4 nm", "PHX", b'A', 100.0),
    a(0x19, (0x74, 0x75), "", D, "Phoenix", AM5, "4 nm", "PHX", b'A', 95.0),
    a(0x19, (0x75, 0x75), "", None, "Hawk Point", "Socket FP7/FP8", "4 nm", "HPT", b'A', 100.0),
    a(0x19, (0x78, 0x78), "", M, "Phoenix 2", "Socket FP7/FP8", "4 nm", "PHX2", b'A', 100.0),
    a(0x19, (0x78, 0x78), "", None, "Phoenix 2", AM5, "4 nm", "PHX2", b'A', 95.0),
    a(0x19, (0xA0, 0xAF), "EPYC 8", None, "Siena", "Socket SP6 (LGA4844)", "5 nm", "RSDN", 0, 0.0),
    a(0x19, (0xA0, 0xAF), "", None, "Bergamo", SP5, "5 nm", "RSDN", 0, 0.0),
    // ---- Zen 5 (family 1Ah) ----
    a(0x1A, (0x08, 0x08), "Threadripper", None, "Shimada Peak", STR5, "4 nm", "SHP", 0, 95.0),
    a(0x1A, (0x00, 0x0F), "", None, "Turin", SP5, "4 nm", "BRH", 0, 0.0),
    a(0x1A, (0x10, 0x1F), "", None, "Turin Dense", SP5, "3 nm", "BRHD", 0, 0.0),
    a(0x1A, (0x20, 0x2F), "", None, "Strix Point", "Socket FP8", "4 nm", "STX", b'B', 100.0),
    a(0x1A, (0x40, 0x4F), "", M, "Fire Range", "Socket FL1", "4 nm", "GNR", b'B', 100.0),
    a(0x1A, (0x40, 0x4F), "", None, "Granite Ridge", AM5, "4 nm", "GNR", b'B', 95.0),
    a(0x1A, (0x60, 0x6F), "", None, "Krackan Point", "Socket FP8", "4 nm", "KRK", 0, 100.0),
    a(0x1A, (0x70, 0x7F), "", None, "Strix Halo", "Socket FP11", "4 nm", "STXH", 0, 100.0),

    // ---- Hygon, VIA, Zhaoxin ----
    o("H", 0x18, (0x00, 0x0F), "", "Dhyana", "", "14 nm"),
    o("ZC", 7, (0x1B, 0x1B), "", "WuDaoKou", "", "28 nm"),
    o("ZC", 7, (0x3B, 0x3B), "", "LuJiaZui", "", "16 nm"),
    o("ZC", 7, (0x5B, 0x5B), "", "Century Avenue", "", ""),
    o("ZC", 6, (0x0F, 0x0F), "ZHAOXIN", "ZhangJiang", "", "28 nm"),
    o("ZC", 6, (0x19, 0x19), "ZHAOXIN", "ZhangJiang", "", "28 nm"),
    o("C", 6, (0x0D, 0x0D), "", "Esther", "", "90 nm"),
    o("C", 6, (0x0F, 0x0F), "", "Isaiah", "", "65 nm"),
];

/// Intel core stepping names by (model, stepping), from Intel spec updates
/// and microcode release notes.
#[rustfmt::skip]
static INTEL_STEPPINGS: &[(u32, u32, &str)] = &[
    (0x17, 6, "C0"), (0x17, 10, "E0"), (0x1A, 4, "C0"), (0x1A, 5, "D0"), (0x1E, 5, "B1"),
    (0x25, 2, "C2"), (0x25, 5, "K0"), (0x2C, 2, "B1"), (0x2A, 7, "D2"), (0x2D, 6, "C1"),
    (0x2D, 7, "C2"), (0x3A, 9, "E1"), (0x3E, 4, "S1"), (0x3C, 3, "C0"), (0x3F, 2, "R2"),
    (0x45, 1, "C0"), (0x47, 1, "G0"), (0x4F, 1, "B0"), (0x5E, 3, "R0"), (0x4E, 3, "D1"),
    (0x55, 4, "U0"), (0x55, 6, "B0"), (0x55, 7, "B1"), (0x55, 11, "A1"),
    (0x8E, 9, "H0"), (0x8E, 10, "Y0"), (0x8E, 11, "W0"), (0x8E, 12, "V0"),
    (0x9E, 9, "B0"), (0x9E, 10, "U0"), (0x9E, 11, "B0"), (0x9E, 12, "P0"), (0x9E, 13, "R0"),
    (0xA5, 2, "R1"), (0xA5, 3, "G1"), (0xA5, 5, "Q0"), (0xA6, 0, "A0"), (0xA7, 1, "B0"),
    (0x7E, 5, "D1"), (0x8C, 1, "B1"), (0x8D, 1, "R0"), (0x6A, 6, "D0"),
    (0x97, 2, "C0"), (0x97, 5, "H0"), (0x9A, 3, "L0"), (0x9A, 4, "R0"), (0xBE, 0, "N0"),
    (0xB7, 1, "B0"), (0xBF, 2, "C0"), (0xBF, 5, "H0"), (0xBA, 2, "J0"), (0xBA, 3, "Q0"),
    (0xAA, 4, "C0"), (0xC6, 2, "B0"), (0xBD, 1, "B0"),
];

/// Per-SKU TDP / Tjmax for popular parts whose numbers don't follow the
/// suffix rules. (vendor tag, model token, TDP W, Tjmax °C or 0 = row default).
#[rustfmt::skip]
static SKUS: &[(char, &str, f64, f64)] = &[
    // Zen 5
    ('A', "9950X3D", 170.0, 0.0), ('A', "9900X3D", 120.0, 0.0), ('A', "9800X3D", 120.0, 0.0),
    ('A', "9950X", 170.0, 0.0), ('A', "9900X", 120.0, 0.0), ('A', "9700X", 65.0, 0.0), ('A', "9600X", 65.0, 0.0),
    // Zen 4
    ('A', "7950X3D", 120.0, 89.0), ('A', "7900X3D", 120.0, 89.0), ('A', "7800X3D", 120.0, 89.0), ('A', "7600X3D", 65.0, 89.0),
    ('A', "7950X", 170.0, 0.0), ('A', "7900X", 170.0, 0.0), ('A', "7700X", 105.0, 0.0), ('A', "7600X", 105.0, 0.0),
    // Zen 3
    ('A', "5950X", 105.0, 0.0), ('A', "5900X", 105.0, 0.0), ('A', "5900XT", 105.0, 0.0), ('A', "5800X", 105.0, 0.0),
    ('A', "5800XT", 105.0, 0.0), ('A', "5800X3D", 105.0, 0.0), ('A', "5700X3D", 105.0, 0.0), ('A', "5600X3D", 105.0, 0.0),
    ('A', "5700X", 65.0, 0.0), ('A', "5600X", 65.0, 95.0), ('A', "5600", 65.0, 0.0),
    // Zen 2 / Zen+ / Zen
    ('A', "3950X", 105.0, 0.0), ('A', "3900X", 105.0, 0.0), ('A', "3900XT", 105.0, 0.0), ('A', "3800X", 105.0, 0.0),
    ('A', "3800XT", 105.0, 0.0), ('A', "3700X", 65.0, 0.0), ('A', "3600X", 95.0, 0.0), ('A', "3600XT", 95.0, 0.0),
    ('A', "2700X", 105.0, 85.0), ('A', "2600X", 95.0, 0.0), ('A', "1800X", 95.0, 0.0), ('A', "1700X", 95.0, 0.0),
    ('A', "1600X", 95.0, 0.0), ('A', "1500X", 65.0, 0.0),
    // Threadripper
    ('A', "1950X", 180.0, 0.0), ('A', "1920X", 180.0, 0.0), ('A', "2990WX", 250.0, 0.0), ('A', "2950X", 180.0, 0.0),
    ('A', "3990X", 280.0, 0.0), ('A', "3970X", 280.0, 0.0), ('A', "3960X", 280.0, 0.0), ('A', "3995WX", 280.0, 0.0),
    ('A', "5995WX", 280.0, 0.0), ('A', "5975WX", 280.0, 0.0), ('A', "7980X", 350.0, 0.0), ('A', "7970X", 350.0, 0.0),
    ('A', "7960X", 350.0, 0.0), ('A', "7995WX", 350.0, 0.0), ('A', "7985WX", 350.0, 0.0), ('A', "9980X", 350.0, 0.0),
    ('A', "9970X", 350.0, 0.0), ('A', "9960X", 350.0, 0.0), ('A', "9995WX", 350.0, 0.0),
    // Older AMD desktop
    ('A', "9590", 220.0, 0.0), ('A', "9370", 220.0, 0.0), ('A', "8370", 125.0, 0.0), ('A', "8350", 125.0, 0.0),
    ('A', "8320", 125.0, 0.0), ('A', "8150", 125.0, 0.0), ('A', "6300", 95.0, 0.0), ('A', "1100T", 125.0, 0.0),
    ('A', "1090T", 125.0, 0.0), ('A', "965", 125.0, 0.0), ('A', "7850K", 95.0, 0.0), ('A', "5800K", 100.0, 0.0),
    // Intel exceptions
    ('I', "9900KS", 127.0, 0.0), ('I', "4790K", 88.0, 0.0), ('I', "4690K", 88.0, 0.0), ('I', "4770K", 84.0, 0.0),
    ('I', "4670K", 84.0, 0.0), ('I', "980X", 130.0, 0.0), ('I', "990X", 130.0, 0.0), ('I', "8086K", 95.0, 0.0),
];

/// TDP by codename + model-number suffix (prefix match, first hit wins, ""
/// = no suffix / default). Covers the regular desktop and mobile lines.
#[rustfmt::skip]
static SUFFIX_TDP: &[(&str, &str, f64)] = &[
    ("Arrow Lake", "K", 125.0), ("Arrow Lake", "HX", 55.0), ("Arrow Lake", "T", 35.0), ("Arrow Lake", "", 65.0),
    ("Arrow Lake-H", "", 28.0), ("Arrow Lake-U", "", 15.0), ("Meteor Lake", "H", 28.0), ("Meteor Lake", "U", 15.0),
    ("Lunar Lake", "", 17.0), ("Panther Lake", "H", 25.0), ("Panther Lake", "", 15.0),
    ("Raptor Lake", "KS", 150.0), ("Raptor Lake", "K", 125.0), ("Raptor Lake", "HX", 55.0), ("Raptor Lake", "H", 45.0),
    ("Raptor Lake", "P", 28.0), ("Raptor Lake", "U", 15.0), ("Raptor Lake", "T", 35.0), ("Raptor Lake", "", 65.0),
    ("Alder Lake", "KS", 150.0), ("Alder Lake", "K", 125.0), ("Alder Lake", "HX", 55.0), ("Alder Lake", "H", 45.0),
    ("Alder Lake", "P", 28.0), ("Alder Lake", "U", 15.0), ("Alder Lake", "T", 35.0), ("Alder Lake", "", 65.0),
    ("Alder Lake-N", "", 6.0), ("Twin Lake", "", 6.0),
    ("Rocket Lake", "K", 125.0), ("Rocket Lake", "T", 35.0), ("Rocket Lake", "", 65.0),
    ("Tiger Lake", "G", 28.0), ("Tiger Lake", "", 15.0), ("Tiger Lake-H", "", 45.0), ("Ice Lake", "", 15.0),
    ("Comet Lake", "K", 125.0), ("Comet Lake", "H", 45.0), ("Comet Lake", "T", 35.0), ("Comet Lake", "", 65.0),
    ("Comet Lake-U", "", 15.0), ("Whiskey Lake-U", "", 15.0), ("Kaby Lake-R", "", 15.0),
    ("Coffee Lake", "K", 95.0), ("Coffee Lake", "H", 45.0), ("Coffee Lake", "T", 35.0), ("Coffee Lake", "", 65.0),
    ("Kaby Lake", "K", 91.0), ("Kaby Lake", "H", 45.0), ("Kaby Lake", "U", 15.0), ("Kaby Lake", "Y", 4.5),
    ("Kaby Lake", "T", 35.0), ("Kaby Lake", "", 65.0),
    ("Skylake", "K", 91.0), ("Skylake", "H", 45.0), ("Skylake", "T", 35.0), ("Skylake", "", 65.0),
    ("Skylake-U/Y", "U", 15.0), ("Skylake-U/Y", "Y", 4.5),
    ("Skylake-X", "", 140.0), ("Cascade Lake-X", "", 165.0), ("Haswell-E", "", 140.0), ("Broadwell-E", "", 140.0),
    ("Haswell", "H", 47.0), ("Haswell", "M", 47.0), ("Haswell", "T", 35.0), ("Haswell", "S", 65.0), ("Haswell", "", 84.0),
    ("Haswell-ULT", "", 15.0), ("Broadwell-H", "", 65.0), ("Broadwell-U", "", 15.0),
    ("Ivy Bridge", "M", 35.0), ("Ivy Bridge", "T", 35.0), ("Ivy Bridge", "S", 65.0), ("Ivy Bridge", "", 77.0),
    ("Sandy Bridge", "M", 35.0), ("Sandy Bridge", "T", 35.0), ("Sandy Bridge", "S", 65.0), ("Sandy Bridge", "", 95.0),
    ("Sandy Bridge-E", "", 130.0), ("Ivy Bridge-E", "", 130.0),
    ("Bloomfield", "", 130.0), ("Gulftown", "", 130.0), ("Lynnfield", "S", 82.0), ("Lynnfield", "", 95.0),
    ("Clarkdale", "", 73.0), ("Wolfdale", "", 65.0), ("Yorkfield", "", 95.0), ("Conroe", "", 65.0),
    // AMD
    ("Granite Ridge", "", 65.0), ("Raphael", "", 65.0), ("Vermeer", "", 65.0), ("Matisse", "", 65.0),
    ("Pinnacle Ridge", "", 65.0), ("Summit Ridge", "", 65.0),
    ("Fire Range", "", 55.0), ("Dragon Range", "", 55.0), ("Strix Halo", "", 55.0),
    ("Strix Point", "", 28.0), ("Krackan Point", "", 28.0),
    ("Hawk Point", "HS", 35.0), ("Hawk Point", "H", 45.0), ("Hawk Point", "U", 28.0),
    ("Phoenix", "GE", 35.0), ("Phoenix", "G", 65.0), ("Phoenix", "HS", 35.0), ("Phoenix", "H", 45.0), ("Phoenix", "U", 28.0),
    ("Phoenix 2", "G", 65.0), ("Phoenix 2", "U", 15.0),
    ("Rembrandt", "HX", 45.0), ("Rembrandt", "HS", 35.0), ("Rembrandt", "H", 45.0), ("Rembrandt", "U", 28.0),
    ("Cezanne", "GE", 35.0), ("Cezanne", "G", 65.0), ("Cezanne", "HX", 45.0), ("Cezanne", "HS", 35.0),
    ("Cezanne", "H", 45.0), ("Cezanne", "U", 15.0), ("Cezanne", "", 65.0), ("Barcelo", "", 15.0),
    ("Renoir", "GE", 35.0), ("Renoir", "G", 65.0), ("Renoir", "HS", 35.0), ("Renoir", "H", 45.0), ("Renoir", "U", 15.0),
    ("Lucienne", "", 15.0), ("Picasso", "GE", 35.0), ("Picasso", "G", 65.0), ("Picasso", "H", 35.0), ("Picasso", "U", 15.0),
    ("Raven Ridge", "GE", 35.0), ("Raven Ridge", "G", 65.0), ("Raven Ridge", "H", 35.0), ("Raven Ridge", "U", 15.0),
    ("Dali", "", 15.0), ("Mendocino", "", 15.0), ("Van Gogh", "", 15.0),
];

/// Apple Silicon by marketing name (longest names first). SoC IDs from the
/// public device trees (Asahi Linux docs).
#[rustfmt::skip]
static APPLE: &[(&str, &str, &str)] = &[
    ("M1 Ultra", "T6002", "5 nm (TSMC N5)"), ("M1 Max", "T6001", "5 nm (TSMC N5)"),
    ("M1 Pro", "T6000", "5 nm (TSMC N5)"), ("M1", "T8103", "5 nm (TSMC N5)"),
    ("M2 Ultra", "T6022", "5 nm (TSMC N5P)"), ("M2 Max", "T6021", "5 nm (TSMC N5P)"),
    ("M2 Pro", "T6020", "5 nm (TSMC N5P)"), ("M2", "T8112", "5 nm (TSMC N5P)"),
    ("M3 Ultra", "T6032", "3 nm (TSMC N3B)"), ("M3 Max", "T6031", "3 nm (TSMC N3B)"),
    ("M3 Pro", "T6030", "3 nm (TSMC N3B)"), ("M3", "T8122", "3 nm (TSMC N3B)"),
    ("M4 Max", "T6041", "3 nm (TSMC N3E)"), ("M4 Pro", "T6040", "3 nm (TSMC N3E)"), ("M4", "T8132", "3 nm (TSMC N3E)"),
    ("M5 Max", "", "3 nm (TSMC N3P)"), ("M5 Pro", "", "3 nm (TSMC N3P)"), ("M5", "T8142", "3 nm (TSMC N3P)"),
];

fn tag(vendor: &str) -> char {
    match vendor {
        "GenuineIntel" => 'I',
        "AuthenticAMD" => 'A',
        "HygonGenuine" => 'H',
        "CentaurHauls" => 'C',
        "  Shanghai  " => 'Z',
        _ => '?',
    }
}

/// Model number and its letter suffix: "Intel Core i9-13900KF" -> ("13900KF",
/// "KF"), "AMD Ryzen 7 7840HS" -> ("7840HS", "HS"), "1065G7" -> suffix "G7".
fn model_token(clean: &str) -> (&str, &str) {
    let has_num = |t: &str| t.as_bytes().windows(3).any(|w| w.iter().all(u8::is_ascii_digit));
    let Some(t) = clean.split_whitespace().rev().find(|t| has_num(t)) else { return ("", "") };
    let t = t.rsplit('-').find(|p| has_num(p)).unwrap_or(t);
    let t = t.trim_start_matches(|c: char| c.is_ascii_alphabetic());
    let suffix = t.trim_start_matches(|c: char| c.is_ascii_digit());
    (t, suffix)
}

fn is_mobile(tag: char, clean: &str, suffix: &str) -> bool {
    if clean.contains("Mobile") || clean.contains("Ryzen AI") {
        return true;
    }
    let first = suffix.chars().next().unwrap_or(' ');
    match tag {
        'I' => "UYHPMV".contains(first) || (first == 'G' && suffix.len() == 2),
        'A' => first == 'U' || first == 'H',
        _ => false,
    }
}

fn some(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/// Core iN-GXXX brand generation vs. the client models that generation used.
/// Unknown brands or models are accepted.
fn intel_brand_matches_model(brand: &str, model: u32) -> bool {
    let Some(i) = brand.find('-').filter(|_| brand.contains("Core(TM) i") || brand.contains("Core i")) else {
        return true;
    };
    let digits: String = brand[i + 1..].chars().take_while(char::is_ascii_digit).collect();
    let generation = match digits.len() {
        4 => digits[..1].parse::<u32>().ok(),
        5 => digits[..2].parse::<u32>().ok(),
        _ => None,
    };
    let models: &[u32] = match generation {
        Some(2) => &[0x2A, 0x2D],
        Some(3) => &[0x3A, 0x3E],
        Some(4) => &[0x3C, 0x3F, 0x45, 0x46],
        Some(5) => &[0x3D, 0x47, 0x4F],
        Some(6) => &[0x4E, 0x5E, 0x55],
        Some(7..=9) => &[0x8E, 0x9E, 0x55],
        Some(10) => &[0x7D, 0x7E, 0x8E, 0xA5, 0xA6, 0x55, 0x66],
        Some(11) => &[0x8C, 0x8D, 0xA7],
        Some(12..=14) => &[0x97, 0x9A, 0xB7, 0xBA, 0xBF, 0xBE],
        _ => return true,
    };
    models.contains(&model)
}

pub fn lookup(vendor: &str, family: u32, model: u32, stepping: u32, brand: &str) -> Info {
    if vendor == "Apple" || brand.starts_with("Apple") {
        return APPLE
            .iter()
            .find(|(n, ..)| brand.contains(&format!("{n} ")) || brand.ends_with(n))
            .map(|&(_, code, tech)| Info { codename: some(code), technology: some(tech), ..Info::default() })
            .unwrap_or_default();
    }
    let t = tag(vendor);
    if t == 'I' && family == 6 && !intel_brand_matches_model(brand, model) {
        // Hypervisors (e.g. macOS CI VMs) can mask the CPUID signature while
        // keeping the real brand string; no data beats wrong data.
        return Info::default();
    }
    let clean = clean_brand(brand);
    let (token, suffix) = model_token(&clean);
    let mobile = is_mobile(t, &clean, suffix);
    let Some(r) = ROWS.iter().find(|r| {
        r.v.contains(t)
            && r.fam == family
            && (r.m.0..=r.m.1).contains(&model)
            && (r.s.0..=r.s.1).contains(&stepping)
            && (r.brand.is_empty() || r.brand.split('|').any(|p| brand.contains(p)))
            && r.mobile.is_none_or(|m| m == mobile)
    }) else {
        return Info::default();
    };
    let core_stepping = if !r.abbr.is_empty() {
        let rev = if r.rev != 0 { r.rev } else { b'A' + (model & 0xF) as u8 };
        Some(format!("{}-{}{stepping}", r.abbr, rev as char))
    } else if t == 'I' && family == 6 {
        INTEL_STEPPINGS.iter().find(|s| s.0 == model && s.1 == stepping).map(|s| s.2.to_string())
    } else {
        None
    };
    let sku = SKUS.iter().find(|s| s.0 == t && s.1 == token);
    let tdp =
        sku.map(|s| s.2).or_else(|| SUFFIX_TDP.iter().find(|s| s.0 == r.code && suffix.starts_with(s.1)).map(|s| s.2));
    let tj = sku.map(|s| s.3).filter(|&v| v > 0.0).unwrap_or(r.tj);
    Info {
        codename: some(r.code),
        package: some(r.pkg),
        technology: some(r.tech),
        core_stepping,
        tdp_w: tdp,
        tjmax_c: (tj > 0.0).then_some(tj),
        name: None,
    }
}

/// "AMD Ryzen 9 7900X 12-Core Processor" -> "AMD Ryzen 9 7900X".
pub fn clean_brand(brand: &str) -> String {
    let mut s = brand.split_whitespace().collect::<Vec<_>>().join(" ");
    for junk in ["(R)", "(TM)", "(tm)", "(r)", " CPU", " Processor"] {
        s = s.replace(junk, "");
    }
    if let Some(i) = s.find(" @ ") {
        s.truncate(i);
    }
    if let Some(i) = s.find(|c: char| c.is_ascii_digit()).and(s.rfind(" w/ ")) {
        s.truncate(i);
    }
    let words: Vec<&str> = s.split(' ').filter(|w| !w.ends_with("-Core") && !w.ends_with("-core")).collect();
    words.join(" ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_signature_is_ignored() {
        // macOS CI VM: i7-8700B brand string, Ivy Bridge signature.
        assert!(
            super::lookup("GenuineIntel", 6, 0x3A, 9, "Intel(R) Core(TM) i7-8700B CPU @ 3.20GHz").codename.is_none()
        );
        assert!(
            super::lookup("GenuineIntel", 6, 0x9E, 10, "Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz").codename.is_some()
        );
    }

    #[test]
    fn brand() {
        assert_eq!(clean_brand("AMD Ryzen 9 7900X 12-Core Processor            "), "AMD Ryzen 9 7900X");
        assert_eq!(clean_brand("Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz"), "Intel Core i7-8700K");
        assert_eq!(model_token("Intel Core i9-13900KF"), ("13900KF", "KF"));
        assert_eq!(model_token("Intel Core i7-1065G7"), ("1065G7", "G7"));
        assert_eq!(model_token("AMD A10-7850K Radeon R7, 12 Compute Cores 4C+8G"), ("7850K", "K"));
    }

    #[allow(clippy::too_many_arguments)]
    fn check(
        v: &str,
        fms: (u32, u32, u32),
        brand: &str,
        code: &str,
        pkg: &str,
        tech: &str,
        step: Option<&str>,
        tdp: Option<f64>,
        tj: Option<f64>,
    ) {
        let i = lookup(v, fms.0, fms.1, fms.2, brand);
        assert_eq!(i.codename.as_deref(), Some(code), "{brand}");
        assert_eq!(i.package.as_deref().unwrap_or(""), pkg, "{brand}");
        assert_eq!(i.technology.as_deref(), Some(tech), "{brand}");
        assert_eq!(i.core_stepping.as_deref(), step, "{brand}");
        assert_eq!(i.tdp_w, tdp, "{brand}");
        assert_eq!(i.tjmax_c, tj, "{brand}");
    }

    #[test]
    fn known_cpus() {
        // The machine in reference/cpuz_dump.txt.
        check(
            "AuthenticAMD",
            (0x19, 0x61, 2),
            "AMD Ryzen 9 7900X 12-Core Processor            ",
            "Raphael",
            AM5,
            "5 nm",
            Some("RPL-B2"),
            Some(170.0),
            Some(95.0),
        );
        check(
            "AuthenticAMD",
            (0x19, 0x61, 2),
            "AMD Ryzen 7 7800X3D 8-Core Processor",
            "Raphael",
            AM5,
            "5 nm",
            Some("RPL-B2"),
            Some(120.0),
            Some(89.0),
        );
        check(
            "AuthenticAMD",
            (0x19, 0x61, 2),
            "AMD Ryzen 9 7945HX with Radeon Graphics",
            "Dragon Range",
            "Socket FL1",
            "5 nm",
            Some("RPL-B2"),
            Some(55.0),
            Some(100.0),
        );
        check(
            "AuthenticAMD",
            (0x19, 0x21, 2),
            "AMD Ryzen 7 5800X3D 8-Core Processor",
            "Vermeer",
            AM4,
            "7 nm",
            Some("VMR-B2"),
            Some(105.0),
            Some(90.0),
        );
        check(
            "AuthenticAMD",
            (0x1A, 0x44, 0),
            "AMD Ryzen 7 9700X 8-Core Processor",
            "Granite Ridge",
            AM5,
            "4 nm",
            Some("GNR-B0"),
            Some(65.0),
            Some(95.0),
        );
        check(
            "AuthenticAMD",
            (0x1A, 0x24, 0),
            "AMD Ryzen AI 9 HX 370 w/ Radeon 890M",
            "Strix Point",
            "Socket FP8",
            "4 nm",
            Some("STX-B0"),
            Some(28.0),
            Some(100.0),
        );
        check(
            "AuthenticAMD",
            (0x19, 0x11, 1),
            "AMD EPYC 9654 96-Core Processor",
            "Genoa",
            SP5,
            "5 nm",
            Some("RS-B1"),
            None,
            None,
        );
        check(
            "AuthenticAMD",
            (0x17, 0x71, 0),
            "AMD Ryzen 7 3700X 8-Core Processor",
            "Matisse",
            AM4,
            "7 nm",
            Some("MTS-B0"),
            Some(65.0),
            Some(95.0),
        );
        check(
            "AuthenticAMD",
            (0x19, 0x74, 1),
            "AMD Ryzen 7 7840HS w/ Radeon 780M Graphics",
            "Phoenix",
            "Socket FP7/FP8",
            "4 nm",
            Some("PHX-A1"),
            Some(35.0),
            Some(100.0),
        );
        check(
            "GenuineIntel",
            (6, 0x9E, 10),
            "Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz",
            "Coffee Lake",
            L1151,
            "14 nm",
            Some("U0"),
            Some(95.0),
            Some(100.0),
        );
        check(
            "GenuineIntel",
            (6, 0xB7, 1),
            "13th Gen Intel(R) Core(TM) i9-13900K",
            "Raptor Lake",
            L1700,
            "10 nm",
            Some("B0"),
            Some(125.0),
            Some(100.0),
        );
        check(
            "GenuineIntel",
            (6, 0xB7, 1),
            "13th Gen Intel(R) Core(TM) i9-13980HX",
            "Raptor Lake",
            "BGA1964",
            "10 nm",
            Some("B0"),
            Some(55.0),
            Some(100.0),
        );
        check(
            "GenuineIntel",
            (6, 0xC6, 2),
            "Intel(R) Core(TM) Ultra 9 285K",
            "Arrow Lake",
            L1851,
            "3 nm",
            Some("B0"),
            Some(125.0),
            Some(105.0),
        );
        check(
            "GenuineIntel",
            (6, 0xBE, 0),
            "Intel(R) N100",
            "Alder Lake-N",
            "BGA1264",
            "10 nm",
            Some("N0"),
            Some(6.0),
            Some(105.0),
        );
        check(
            "GenuineIntel",
            (6, 0x3C, 3),
            "Intel(R) Core(TM) i7-4790K CPU @ 4.00GHz",
            "Haswell",
            L1150,
            "22 nm",
            Some("C0"),
            Some(88.0),
            Some(100.0),
        );
    }

    #[test]
    fn apple() {
        let i = lookup("Apple", 0, 0, 0, "Apple M2 Max");
        assert_eq!((i.codename.as_deref(), i.technology.as_deref()), (Some("T6021"), Some("5 nm (TSMC N5P)")));
        assert_eq!(lookup("Apple", 0, 0, 0, "Apple M1").codename.as_deref(), Some("T8103"));
        assert_eq!(lookup("Apple", 0, 0, 0, "Apple M4 Pro").codename.as_deref(), Some("T6040"));
        assert!(lookup("GenuineIntel", 6, 0x01, 0, "nothing").codename.is_none());
    }
}

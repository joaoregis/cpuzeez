//! SMBIOS/DMI parser and DMI section (FEATURES.md 3.14).
//! Layouts and enum values come from the DMTF SMBIOS 3.x specification (DSP0134).
use crate::Ctx;
use crate::report::{Node, Section, f, g};

#[derive(Debug, Clone)]
pub struct Structure {
    pub ty: u8,
    pub handle: u16,
    pub data: Vec<u8>, // formatted area, including the 4-byte header
    pub strings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Table {
    /// 0.0 for a table synthesized from OS data (no real SMBIOS available).
    pub major: u8,
    pub minor: u8,
    pub structs: Vec<Structure>,
}

impl Structure {
    fn le<const N: usize>(&self, o: usize) -> Option<[u8; N]> {
        self.data.get(o..o + N)?.try_into().ok()
    }
    pub fn byte(&self, o: usize) -> Option<u8> {
        self.data.get(o).copied()
    }
    pub fn word(&self, o: usize) -> Option<u16> {
        self.le(o).map(u16::from_le_bytes)
    }
    pub fn dword(&self, o: usize) -> Option<u32> {
        self.le(o).map(u32::from_le_bytes)
    }
    pub fn qword(&self, o: usize) -> Option<u64> {
        self.le(o).map(u64::from_le_bytes)
    }
    /// String referenced by the index byte at `o` (None if 0, missing or blank).
    pub fn str(&self, o: usize) -> Option<&str> {
        let i = (self.byte(o)? as usize).checked_sub(1)?;
        self.strings.get(i).map(String::as_str).filter(|s| !s.trim().is_empty())
    }
}

impl Table {
    /// Walk the structure table: header, formatted area, then a string set
    /// ended by a double NUL. Stops at type 127 or on a truncated structure.
    pub fn parse(major: u8, minor: u8, raw: &[u8]) -> Table {
        let mut structs = Vec::new();
        let mut i = 0;
        while i + 4 <= raw.len() {
            let (ty, len) = (raw[i], raw[i + 1] as usize);
            if len < 4 || i + len > raw.len() {
                break;
            }
            let mut j = i + len;
            let mut strings = Vec::new();
            if raw.get(j..j + 2) == Some(&[0, 0]) {
                j += 2;
            } else {
                while let Some(n) = raw.get(j..).and_then(|r| r.iter().position(|&b| b == 0)) {
                    strings.push(String::from_utf8_lossy(&raw[j..j + n]).into_owned());
                    j += n + 1;
                    if raw.get(j) == Some(&0) {
                        j += 1;
                        break;
                    }
                }
            }
            let handle = u16::from_le_bytes([raw[i + 2], raw[i + 3]]);
            structs.push(Structure { ty, handle, data: raw[i..i + len].to_vec(), strings });
            if ty == 127 {
                break;
            }
            i = j;
        }
        Table { major, minor, structs }
    }

    pub fn of(&self, ty: u8) -> impl Iterator<Item = &Structure> {
        self.structs.iter().filter(move |s| s.ty == ty)
    }

    pub fn first(&self, ty: u8) -> Option<&Structure> {
        self.of(ty).next()
    }
}

pub fn get(ctx: &Ctx) -> Option<&Table> {
    ctx.smbios.get_or_init(fetch).as_ref()
}

// ---------------------------------------------------------------- fetching

#[cfg(windows)]
fn fetch() -> Option<Table> {
    use crate::os::win::GetSystemFirmwareTable;
    let rsmb = u32::from_be_bytes(*b"RSMB");
    // RawSMBIOSData: Used20CallingMethod, Major, Minor, DmiRevision, u32 Length, table.
    let n = unsafe { GetSystemFirmwareTable(rsmb, 0, std::ptr::null_mut(), 0) };
    if n < 8 {
        return None;
    }
    let mut b = vec![0u8; n as usize];
    if unsafe { GetSystemFirmwareTable(rsmb, 0, b.as_mut_ptr(), n) } != n {
        return None;
    }
    let len = (u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize).min(b.len() - 8);
    Some(Table::parse(b[1], b[2], &b[8..8 + len]))
}

#[cfg(target_os = "linux")]
fn fetch() -> Option<Table> {
    let dir = "/sys/firmware/dmi/tables";
    if let (Ok(ep), Ok(raw)) = (std::fs::read(format!("{dir}/smbios_entry_point")), std::fs::read(format!("{dir}/DMI")))
        && let Some((major, minor)) = entry_point_version(&ep)
    {
        return Some(Table::parse(major, minor, &raw));
    }
    sysfs_table("/sys/class/dmi/id") // the raw tables are root-only; dmi/id is mostly world-readable
}

#[cfg(target_os = "macos")]
fn fetch() -> Option<Table> {
    // Intel Macs publish the real table; Apple Silicon has no SMBIOS at all.
    if let Some(out) = crate::os::cmd("ioreg", &["-rd1", "-c", "AppleSMBIOS"])
        && let Some(raw) = ioreg_hex(&out, "SMBIOS")
    {
        let ver = ioreg_hex(&out, "SMBIOS-EPS").and_then(|ep| entry_point_version(&ep));
        let (major, minor) = ver.unwrap_or((0, 0));
        return Some(Table::parse(major, minor, &raw));
    }
    apple_table()
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn fetch() -> Option<Table> {
    None
}

/// SMBIOS version from a 32-bit ("_SM_"), 64-bit ("_SM3_") or legacy ("_DMI_") entry point.
#[allow(dead_code)]
fn entry_point_version(ep: &[u8]) -> Option<(u8, u8)> {
    if ep.starts_with(b"_SM3_") {
        Some((*ep.get(7)?, *ep.get(8)?))
    } else if ep.starts_with(b"_SM_") {
        Some((*ep.get(6)?, *ep.get(7)?))
    } else if ep.starts_with(b"_DMI_") {
        let bcd = *ep.get(0x0E)?;
        Some((bcd >> 4, bcd & 0x0F))
    } else {
        None
    }
}

/// `"key" = <hex>` from ioreg output.
#[allow(dead_code)]
fn ioreg_hex(out: &str, key: &str) -> Option<Vec<u8>> {
    let pat = format!("\"{key}\" = <");
    let s = &out[out.find(&pat)? + pat.len()..];
    let hex = &s[..s.find('>')?];
    (0..hex.len() / 2).map(|i| u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok()).collect()
}

/// Build one structure from strings at given offsets plus raw bytes.
#[allow(dead_code)]
fn synth(ty: u8, len: usize, strs: Vec<(usize, Option<String>)>, bytes: &[(usize, u8)]) -> Structure {
    let mut data = vec![0u8; len];
    data[0] = ty;
    data[1] = len as u8;
    data[2] = ty; // unique handle per type
    let mut strings = Vec::new();
    for (o, s) in strs {
        if let Some(s) = s.filter(|s| !s.is_empty()) {
            strings.push(s);
            data[o] = strings.len() as u8;
        }
    }
    for &(o, b) in bytes {
        data[o] = b;
    }
    Structure { ty, handle: ty as u16, data, strings }
}

/// Raw UUID bytes in SMBIOS (>= 2.6, little-endian first fields) order from its text form.
#[allow(dead_code)]
fn uuid_bytes(s: &str) -> Option<Vec<u8>> {
    let h: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if h.len() != 32 {
        return None;
    }
    let mut b: Vec<u8> = (0..16).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap_or(0)).collect();
    b[0..4].reverse();
    b[4..6].reverse();
    b[6..8].reverse();
    Some(b)
}

#[cfg(target_os = "linux")]
fn sysfs_table(dir: &str) -> Option<Table> {
    let r = |n: &str| crate::os::read_trim(format!("{dir}/{n}"));
    let mut t1 = synth(
        1,
        0x1B,
        vec![
            (4, r("sys_vendor")),
            (5, r("product_name")),
            (6, r("product_version")),
            (7, r("product_serial")),
            (0x19, r("product_sku")),
            (0x1A, r("product_family")),
        ],
        &[],
    );
    if let Some(u) = r("product_uuid").and_then(|u| uuid_bytes(&u)) {
        t1.data[8..0x18].copy_from_slice(&u);
    }
    let chassis = r("chassis_type").and_then(|v| v.parse::<u8>().ok()).unwrap_or(2);
    let structs = vec![
        synth(0, 9, vec![(4, r("bios_vendor")), (5, r("bios_version")), (8, r("bios_date"))], &[]),
        t1,
        synth(
            2,
            9,
            vec![
                (4, r("board_vendor")),
                (5, r("board_name")),
                (6, r("board_version")),
                (7, r("board_serial")),
                (8, r("board_asset_tag")),
            ],
            &[],
        ),
        synth(
            3,
            9,
            vec![
                (4, r("chassis_vendor")),
                (6, r("chassis_version")),
                (7, r("chassis_serial")),
                (8, r("chassis_asset_tag")),
            ],
            &[(5, chassis)],
        ),
    ];
    structs.iter().any(|s| !s.strings.is_empty()).then_some(Table { major: 0, minor: 0, structs })
}

/// Apple Silicon: types 0/1/2/3/4 from `system_profiler` (model, serial, iBoot, chip).
#[cfg(target_os = "macos")]
fn apple_table() -> Option<Table> {
    let out = crate::os::cmd("system_profiler", &["SPHardwareDataType", "-json"])?;
    let v: serde_json::Value = serde_json::from_str(&out).ok()?;
    let hw = v.get("SPHardwareDataType")?.get(0)?;
    let s = |k: &str| hw.get(k).and_then(|x| x.as_str()).map(str::to_string);
    let apple = || Some("Apple Inc.".to_string());
    let name = s("machine_name").unwrap_or_default();
    let mut t1 = synth(
        1,
        0x1B,
        vec![
            (4, apple()),
            (5, s("machine_model")),
            (7, s("serial_number")),
            (0x19, s("model_number")),
            (0x1A, Some(name.clone())),
        ],
        &[],
    );
    if let Some(u) = s("platform_UUID").and_then(|u| uuid_bytes(&u)) {
        t1.data[8..0x18].copy_from_slice(&u);
    }
    let chassis = if name.contains("Book") { 0x0A } else { 0x03 }; // Notebook / Desktop
    let structs = vec![
        synth(0, 9, vec![(4, apple()), (5, s("boot_rom_version"))], &[]),
        t1,
        synth(2, 9, vec![(4, apple()), (5, s("machine_model")), (7, s("serial_number"))], &[]),
        synth(3, 9, vec![(4, apple()), (7, s("serial_number"))], &[(5, chassis)]),
        synth(4, 0x1A, vec![(7, apple()), (0x10, s("chip_type"))], &[(5, 3)]),
    ];
    Some(Table { major: 0, minor: 0, structs })
}

// ---------------------------------------------------------------- accessors

/// Every string of every structure of type `ty`.
pub fn strings_of(t: &Table, ty: u8) -> Vec<&str> {
    t.of(ty).flat_map(|s| s.strings.iter().map(String::as_str)).collect()
}

#[derive(Debug, Clone, Default)]
pub struct Bios {
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub date: Option<String>,
    pub rom_kb: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct System {
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub version: Option<String>,
    pub serial: Option<String>,
    pub uuid: Option<String>,
    pub sku: Option<String>,
    pub family: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Board {
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub revision: Option<String>,
    pub serial: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MemDevice {
    pub locator: String,
    pub bank: String,
    /// 0 = empty slot.
    pub size_mb: u64,
    pub mem_type: &'static str,
    pub form_factor: &'static str,
    pub total_width: Option<u16>,
    pub data_width: Option<u16>,
    /// Rated and configured speed in MT/s (0 = unknown).
    pub speed: u32,
    pub configured_speed: u32,
    /// String, or JEP106 decode of the module id when the string is "Unknown".
    pub manufacturer: String,
    pub part_number: String,
    pub serial: String,
    pub rank: Option<u8>,
    pub min_mv: Option<u16>,
    pub max_mv: Option<u16>,
    pub configured_mv: Option<u16>,
    pub module_mfr_id: Option<u16>,
    pub module_product_id: Option<u16>,
}

fn own(s: Option<&str>) -> Option<String> {
    s.map(str::to_string)
}

fn bios_of(s: &Structure) -> Bios {
    let rom_kb = s.byte(9).map(|n| match (n, s.word(0x18)) {
        // FFh: 16 MB or more, real size in the extended field (bits 15:14 unit MB/GB).
        (0xFF, Some(e)) => (e as u64 & 0x3FFF) << if e >> 14 == 1 { 20 } else { 10 },
        _ => (n as u64 + 1) * 64,
    });
    Bios { vendor: own(s.str(4)), version: own(s.str(5)), date: own(s.str(8)), rom_kb: rom_kb.filter(|&k| k > 0) }
}

fn uuid_text(t: &Table, s: &Structure) -> Option<String> {
    let b = s.data.get(8..0x18)?;
    if b.iter().all(|&x| x == 0) {
        return None; // "not present"
    }
    // Since 2.6 the first three fields are little-endian (synthetic tables use that too).
    let le = t.major == 0 || (t.major, t.minor) >= (2, 6);
    let mut b = b.to_vec();
    if le {
        b[0..4].reverse();
        b[4..6].reverse();
        b[6..8].reverse();
    }
    let h: Vec<String> = b.iter().map(|x| format!("{x:02X}")).collect();
    Some(format!(
        "{{{}-{}-{}-{}-{}}}",
        h[0..4].concat(),
        h[4..6].concat(),
        h[6..8].concat(),
        h[8..10].concat(),
        h[10..].concat()
    ))
}

fn system_of(t: &Table, s: &Structure) -> System {
    System {
        manufacturer: own(s.str(4)),
        product: own(s.str(5)),
        version: own(s.str(6)),
        serial: own(s.str(7)),
        uuid: uuid_text(t, s),
        sku: own(s.str(0x19)),
        family: own(s.str(0x1A)),
    }
}

fn board_of(s: &Structure) -> Board {
    Board { vendor: own(s.str(4)), model: own(s.str(5)), revision: own(s.str(6)), serial: own(s.str(7)) }
}

fn mem_size_mb(s: &Structure) -> u64 {
    match s.word(0x0C) {
        None | Some(0) | Some(0xFFFF) => 0,
        Some(0x7FFF) => s.dword(0x1C).map_or(0, |e| (e & 0x7FFF_FFFF) as u64),
        Some(w) if w & 0x8000 != 0 => (w & 0x7FFF) as u64 / 1024, // KB granularity
        Some(w) => w as u64,
    }
}

fn speed(s: &Structure, o: usize, ext: usize) -> u32 {
    match s.word(o) {
        Some(0xFFFF) => s.dword(ext).unwrap_or(0) & 0x7FFF_FFFF,
        w => w.unwrap_or(0) as u32,
    }
}

fn mem_device_of(s: &Structure) -> MemDevice {
    let width = |o| s.word(o).filter(|&w| w != 0 && w != 0xFFFF);
    let mv = |o| s.word(o).filter(|&v| v != 0);
    let mfr_id = s.word(0x2C);
    let mfr_str = s.str(0x17).unwrap_or("Unknown");
    let manufacturer = match mfr_id.and_then(|id| jep106((id & 0x7F) as u8, (id >> 8) as u8)) {
        Some(n) if mfr_str.trim().eq_ignore_ascii_case("unknown") => n.to_string(),
        _ => mfr_str.to_string(),
    };
    MemDevice {
        locator: s.str(0x10).unwrap_or_default().to_string(),
        bank: s.str(0x11).unwrap_or_default().to_string(),
        size_mb: mem_size_mb(s),
        mem_type: s.byte(0x12).map_or("unknown", mem_type),
        form_factor: s.byte(0x0E).map_or("unknown", form_factor),
        total_width: width(0x08),
        data_width: width(0x0A),
        speed: speed(s, 0x15, 0x54),
        configured_speed: speed(s, 0x20, 0x58),
        manufacturer,
        part_number: s.str(0x1A).unwrap_or("Unknown").to_string(),
        serial: s.str(0x18).unwrap_or("Unknown").to_string(),
        rank: s.byte(0x1B).map(|a| a & 0x0F).filter(|&r| r != 0),
        min_mv: mv(0x22),
        max_mv: mv(0x24),
        configured_mv: mv(0x26),
        module_mfr_id: mfr_id,
        module_product_id: s.word(0x2E),
    }
}

pub fn bios(ctx: &Ctx) -> Option<Bios> {
    get(ctx)?.first(0).map(bios_of)
}

pub fn system(ctx: &Ctx) -> Option<System> {
    let t = get(ctx)?;
    t.first(1).map(|s| system_of(t, s))
}

pub fn board(ctx: &Ctx) -> Option<Board> {
    get(ctx)?.first(2).map(board_of)
}

pub fn memory_devices(ctx: &Ctx) -> Vec<MemDevice> {
    get(ctx).map_or_else(Vec::new, |t| t.of(17).map(mem_device_of).collect())
}

/// First populated CPU socket (type 4, status bit 6), else the first one.
fn cpu_struct(t: &Table) -> Option<&Structure> {
    t.of(4).find(|s| s.byte(0x18).is_some_and(|b| b & 0x40 != 0)).or_else(|| t.first(4))
}

/// (external clock, max speed, current speed) in MHz from type 4; 0 = unknown.
pub fn processor_clocks(ctx: &Ctx) -> Option<(u32, u32, u32)> {
    let s = cpu_struct(get(ctx)?)?;
    let w = |o| s.word(o).unwrap_or(0) as u32;
    Some((w(0x12), w(0x14), w(0x16)))
}

/// Socket designation string of the CPU (e.g. "AM5", "LGA1700").
pub fn socket(ctx: &Ctx) -> Option<String> {
    own(cpu_struct(get(ctx)?)?.str(4))
}

/// AGESA version from any SMBIOS string ("AGESA!V9 ComboAm5PI 1.2.0.2b", usually type 40).
pub fn agesa(t: &Table) -> Option<String> {
    let s = t.structs.iter().flat_map(|s| &s.strings).find(|s| s.starts_with("AGESA!"))?;
    Some(s.split_once(char::is_whitespace)?.1.trim().to_string()).filter(|v| !v.is_empty())
}

/// JEP106 manufacturer for a (continuation count, id with parity) pair.
/// ponytail: only the usual DRAM module vendors; reuse the SPD module's full table once it has one.
pub fn jep106(bank: u8, id: u8) -> Option<&'static str> {
    Some(match (bank, id) {
        (0, 0x2C) => "Micron Technology",
        (0, 0xCE) => "Samsung",
        (0, 0xAD) => "SK Hynix",
        (1, 0x98) => "Kingston",
        (1, 0x4F) => "Transcend Information",
        (1, 0x7A) => "Apacer Technology",
        (2, 0x9E) => "Corsair",
        (3, 0x0B) => "Nanya Technology",
        (4, 0xCD) => "G.Skill Intl",
        (4, 0xCB) => "A-DATA Technology",
        (4, 0xEF) => "TEAMGROUP Inc.",
        (5, 0x02) => "Patriot Memory",
        (5, 0x9B) => "Crucial Technology",
        _ => return None,
    })
}

// ---------------------------------------------------------------- enum tables

fn mem_type(v: u8) -> &'static str {
    match v {
        0x01 => "Other",
        0x03 => "DRAM",
        0x04 => "EDRAM",
        0x05 => "VRAM",
        0x06 => "SRAM",
        0x07 => "RAM",
        0x08 => "ROM",
        0x09 => "FLASH",
        0x0A => "EEPROM",
        0x0B => "FEPROM",
        0x0C => "EPROM",
        0x0D => "CDRAM",
        0x0E => "3DRAM",
        0x0F => "SDRAM",
        0x10 => "SGRAM",
        0x11 => "RDRAM",
        0x12 => "DDR",
        0x13 => "DDR2",
        0x14 => "DDR2 FB-DIMM",
        0x18 => "DDR3",
        0x19 => "FBD2",
        0x1A => "DDR4",
        0x1B => "LPDDR",
        0x1C => "LPDDR2",
        0x1D => "LPDDR3",
        0x1E => "LPDDR4",
        0x1F => "Logical non-volatile device",
        0x20 => "HBM",
        0x21 => "HBM2",
        0x22 => "DDR5",
        0x23 => "LPDDR5",
        0x24 => "HBM3",
        _ => "unknown",
    }
}

fn form_factor(v: u8) -> &'static str {
    match v {
        0x01 => "Other",
        0x03 => "SIMM",
        0x04 => "SIP",
        0x05 => "Chip",
        0x06 => "DIP",
        0x07 => "ZIP",
        0x08 => "Proprietary Card",
        0x09 => "DIMM",
        0x0A => "TSOP",
        0x0B => "Row of chips",
        0x0C => "RIMM",
        0x0D => "SODIMM",
        0x0E => "SRIMM",
        0x0F => "FB-DIMM",
        0x10 => "Die",
        0x11 => "CAMM",
        _ => "unknown",
    }
}

fn chassis_type(v: u8) -> &'static str {
    const T: [&str; 0x25] = [
        "unknown",
        "Other",
        "unknown",
        "Desktop",
        "Low Profile Desktop",
        "Pizza Box",
        "Mini Tower",
        "Tower",
        "Portable",
        "Laptop",
        "Notebook",
        "Hand Held",
        "Docking Station",
        "All in One",
        "Sub Notebook",
        "Space-saving",
        "Lunch Box",
        "Main Server Chassis",
        "Expansion Chassis",
        "SubChassis",
        "Bus Expansion Chassis",
        "Peripheral Chassis",
        "RAID Chassis",
        "Rack Mount Chassis",
        "Sealed-case PC",
        "Multi-system chassis",
        "Compact PCI",
        "Advanced TCA",
        "Blade",
        "Blade Enclosure",
        "Tablet",
        "Convertible",
        "Detachable",
        "IoT Gateway",
        "Embedded PC",
        "Mini PC",
        "Stick PC",
    ];
    T.get((v & 0x7F) as usize).copied().unwrap_or("unknown")
}

fn connector_type(v: u8) -> Option<&'static str> {
    const T: [&str; 0x24] = [
        "None",
        "Centronics",
        "Mini Centronics",
        "Proprietary",
        "DB-25 pin male",
        "DB-25 pin female",
        "DB-15 pin male",
        "DB-15 pin female",
        "DB-9 pin male",
        "DB-9 pin female",
        "RJ-11",
        "RJ-45",
        "50-pin MiniSCSI",
        "Mini-DIN",
        "Micro-DIN",
        "PS/2",
        "Infrared",
        "HP-HIL",
        "Access Bus (USB)",
        "SSA SCSI",
        "Circular DIN-8 male",
        "Circular DIN-8 female",
        "On Board IDE",
        "On Board Floppy",
        "9-pin Dual Inline (pin 10 cut)",
        "25-pin Dual Inline (pin 26 cut)",
        "50-pin Dual Inline",
        "68-pin Dual Inline",
        "On Board Sound Input from CD-ROM",
        "Mini-Centronics Type-14",
        "Mini-Centronics Type-26",
        "Mini Jack (headphones)",
        "BNC",
        "1394",
        "SAS/SATA Plug Receptacle",
        "USB Type-C Receptacle",
    ];
    match v {
        0xA0 => Some("PC-98"),
        0xA1 => Some("PC-98Hireso"),
        0xA2 => Some("PC-H98"),
        0xA3 => Some("PC-98Note"),
        0xA4 => Some("PC-98Full"),
        0xFF => Some("Other"),
        _ => T.get(v as usize).copied(),
    }
}

fn port_type(v: u8) -> Option<&'static str> {
    const T: [&str; 0x24] = [
        "None",
        "Parallel Port XT/AT Compatible",
        "Parallel Port PS/2",
        "Parallel Port ECP",
        "Parallel Port EPP",
        "Parallel Port ECP/EPP",
        "Serial Port XT/AT Compatible",
        "Serial Port 16450 Compatible",
        "Serial Port 16550 Compatible",
        "Serial Port 16550A Compatible",
        "SCSI Port",
        "MIDI Port",
        "Joy Stick Port",
        "Keyboard Port",
        "Mouse Port",
        "SSA SCSI",
        "USB",
        "FireWire (IEEE P1394)",
        "PCMCIA Type I",
        "PCMCIA Type II",
        "PCMCIA Type III",
        "Card bus",
        "Access Bus Port",
        "SCSI II",
        "SCSI Wide",
        "PC-98",
        "PC-98-Hireso",
        "PC-H98",
        "Video Port",
        "Audio Port",
        "Modem Port",
        "Network Port",
        "SATA",
        "SAS",
        "MFDP (Multi-Function Display Port)",
        "Thunderbolt",
    ];
    match v {
        0xA0 => Some("8251 Compatible"),
        0xA1 => Some("8251 FIFO Compatible"),
        0xFF => Some("Other"),
        _ => T.get(v as usize).copied(),
    }
}

/// Slot family, as CPU-Z shows it (PCIe generations/widths fold into "PCI Express").
fn slot_type(v: u8) -> &'static str {
    match v {
        0x01 => "Other",
        0x03 => "ISA",
        0x04 => "MCA",
        0x05 => "EISA",
        0x06 => "PCI",
        0x07 => "PC Card (PCMCIA)",
        0x08 => "VL-VESA",
        0x09 => "Proprietary",
        0x0A => "Processor Card Slot",
        0x0B => "Proprietary Memory Card Slot",
        0x0C => "I/O Riser Card Slot",
        0x0D => "NuBus",
        0x0E => "PCI-66MHz",
        0x0F => "AGP",
        0x10 => "AGP 2X",
        0x11 => "AGP 4X",
        0x12 => "PCI-X",
        0x13 => "AGP 8X",
        0x14..=0x17 => "M.2",
        0x18..=0x1E => "MXM",
        0x1F | 0x20 | 0x24 | 0x25 => "U.2",
        0x21..=0x23 => "PCI Express Mini",
        0x26..=0x28 => "OCP NIC",
        0x30 => "CXL Flexbus 1.0",
        0xA0..=0xA4 => "PC-98",
        0xA5..=0xC5 => "PCI Express",
        0xC6 | 0xC7 => "EDSFF",
        _ => "unknown",
    }
}

fn array_location(v: u8) -> &'static str {
    match v {
        0x01 => "Other",
        0x03 => "Motherboard",
        0x04 => "ISA add-on card",
        0x05 => "EISA add-on card",
        0x06 => "PCI add-on card",
        0x07 => "MCA add-on card",
        0x08 => "PCMCIA add-on card",
        0x09 => "Proprietary add-on card",
        0x0A => "NuBus",
        0xA0..=0xA3 => "PC-98 add-on card",
        0xA4 => "CXL add-on card",
        _ => "unknown",
    }
}

fn array_use(v: u8) -> &'static str {
    match v {
        0x01 => "Other",
        0x03 => "System Memory",
        0x04 => "Video Memory",
        0x05 => "Flash Memory",
        0x06 => "Non-volatile RAM",
        0x07 => "Cache Memory",
        _ => "unknown",
    }
}

fn array_ecc(v: u8) -> &'static str {
    match v {
        0x01 => "Other",
        0x03 => "None",
        0x04 => "Parity",
        0x05 => "Single-bit ECC",
        0x06 => "Multi-bit ECC",
        0x07 => "CRC",
        _ => "unknown",
    }
}

fn error_detect(v: u8) -> &'static str {
    match v {
        0x01 => "Other",
        0x03 => "None",
        0x04 => "8-bit Parity",
        0x05 => "32-bit ECC",
        0x06 => "64-bit ECC",
        0x07 => "128-bit ECC",
        0x08 => "CRC",
        _ => "unknown",
    }
}

fn interleave(v: u8) -> &'static str {
    match v {
        0x01 => "Other",
        0x03 => "One-Way",
        0x04 => "Two-Way",
        0x05 => "Four-Way",
        0x06 => "Eight-Way",
        0x07 => "Sixteen-Way",
        _ => "unknown",
    }
}

/// Names of the set bits of `v`, joined.
fn bits(v: u32, names: &[&str]) -> String {
    let s: Vec<&str> = names.iter().enumerate().filter(|(i, _)| v >> i & 1 != 0).map(|(_, n)| *n).collect();
    if s.is_empty() { "None".into() } else { s.join(", ") }
}

const ECC_CAPS: [&str; 6] = ["Other", "unknown", "None", "Single-bit", "Double-bit", "Scrubbing"];
const MODULE_TYPES: [&str; 11] =
    ["Other", "unknown", "Standard", "FPM", "EDO", "Parity", "ECC", "SIMM", "DIMM", "Burst EDO", "SDRAM"];

// ---------------------------------------------------------------- DMI section

/// CPU-Z drops commas from DMI strings ("Micro-Star International Co. Ltd.").
fn ds(s: &str) -> String {
    s.replace(',', "")
}

fn push_str(v: &mut Vec<Node>, key: &str, s: Option<&str>) {
    if let Some(s) = s {
        v.push(f(key, ds(s)));
    }
}

/// "128 GB" / "512 MB" / "64 KB" from a size in KB.
fn size_kb(kb: u64) -> String {
    if kb >= 1 << 20 && kb.is_multiple_of(1 << 20) {
        format!("{} GB", kb >> 20)
    } else if kb >= 1024 && kb.is_multiple_of(1024) {
        format!("{} MB", kb >> 10)
    } else {
        format!("{kb} KB")
    }
}

/// Type 6 size byte: 2^n MB with special codes.
fn module_size(b: u8) -> String {
    let base = match b & 0x7F {
        0x7D => "not determinable".to_string(),
        0x7E => "disabled".to_string(),
        0x7F => return "not installed".into(),
        n if n < 32 => size_kb(1024u64 << n),
        n => format!("2^{n} MB"),
    };
    if b & 0x80 != 0 { format!("{base} (double-bank)") } else { base }
}

fn dmi_nodes(t: &Table, s: &Structure) -> Option<Node> {
    let mut v = Vec::new();
    let title = match s.ty {
        0 => {
            let b = bios_of(s);
            push_str(&mut v, "vendor", b.vendor.as_deref());
            push_str(&mut v, "version", b.version.as_deref());
            push_str(&mut v, "date", b.date.as_deref());
            if let Some(k) = b.rom_kb {
                v.push(f("ROM size", format!("{k} KB")));
            }
            "DMI BIOS"
        }
        1 => {
            let y = system_of(t, s);
            push_str(&mut v, "manufacturer", y.manufacturer.as_deref());
            push_str(&mut v, "product", y.product.as_deref());
            push_str(&mut v, "version", y.version.as_deref());
            push_str(&mut v, "serial", y.serial.as_deref());
            push_str(&mut v, "UUID", y.uuid.as_deref());
            push_str(&mut v, "SKU", y.sku.as_deref());
            push_str(&mut v, "family", y.family.as_deref());
            "DMI System Information"
        }
        2 => {
            let b = board_of(s);
            push_str(&mut v, "vendor", b.vendor.as_deref());
            push_str(&mut v, "model", b.model.as_deref());
            push_str(&mut v, "revision", b.revision.as_deref());
            push_str(&mut v, "serial", b.serial.as_deref());
            "DMI Baseboard"
        }
        3 => {
            push_str(&mut v, "manufacturer", s.str(4));
            if let Some(c) = s.byte(5) {
                v.push(f("chassis type", chassis_type(c)));
            }
            push_str(&mut v, "chassis serial", s.str(7));
            "DMI System Enclosure"
        }
        11 => {
            for (i, x) in s.strings.iter().enumerate() {
                v.push(f(format!("string[{i}]"), ds(x)));
            }
            "DMI OEM Strings"
        }
        4 => {
            push_str(&mut v, "manufacturer", s.str(7));
            push_str(&mut v, "model", s.str(0x10).map(str::trim));
            let w = |o| s.word(o).filter(|&x| x != 0).map(|x| x as f64);
            let (ext, max, cur) = (w(0x12), w(0x14), w(0x16));
            if let Some(c) = cur {
                v.push(f("clock speed", format!("{c:.1} MHz")));
            }
            if let Some(e) = ext {
                v.push(f("FSB speed", format!("{e:.1} MHz")));
            }
            if let (Some(c), Some(e)) = (cur, ext) {
                v.push(f("multiplier", format!("{:.1}x", c / e)));
            }
            if let Some(m) = max {
                v.push(f("max clock speed", format!("{m:.1} MHz")));
            }
            "DMI Processor"
        }
        5 => {
            if let Some(b) = s.byte(4) {
                v.push(f("error detection", error_detect(b)));
            }
            if let Some(b) = s.byte(5) {
                v.push(f("error correction", bits(b as u32, &ECC_CAPS)));
            }
            if let Some(b) = s.byte(6) {
                v.push(f("supported interleave", interleave(b)));
            }
            if let Some(b) = s.byte(7) {
                v.push(f("current interleave", interleave(b)));
            }
            if let Some(n) = s.byte(8).filter(|&n| n < 32) {
                v.push(f("max module size", size_kb(1024u64 << n)));
                if let Some(slots) = s.byte(0x0E).filter(|&x| x > 0) {
                    v.push(f("max memory size", size_kb((1024u64 << n) * slots as u64)));
                }
            }
            if let Some(w) = s.word(9) {
                v.push(f("supported speeds", bits(w as u32, &["Other", "unknown", "70ns", "60ns", "50ns"])));
            }
            if let Some(w) = s.word(0x0B) {
                v.push(f("supported types", bits(w as u32, &MODULE_TYPES)));
            }
            if let Some(b) = s.byte(0x0D) {
                v.push(f("module voltage", bits(b as u32, &["5V", "3.3V", "2.9V"])));
            }
            if let Some(b) = s.byte(0x0E) {
                v.push(f("slots", b));
            }
            "DMI Memory Controller"
        }
        6 => {
            push_str(&mut v, "designation", s.str(4));
            if let Some(b) = s.byte(5) {
                let banks: Vec<String> =
                    [b >> 4, b & 0x0F].iter().filter(|&&x| x != 0xF).map(|x| x.to_string()).collect();
                v.push(f("bank connections", if banks.is_empty() { "None".into() } else { banks.join(" ") }));
            }
            if let Some(ns) = s.byte(6).filter(|&x| x != 0) {
                v.push(f("speed", format!("{ns} ns")));
            }
            if let Some(w) = s.word(7) {
                v.push(f("type", bits(w as u32, &MODULE_TYPES)));
            }
            if let Some(b) = s.byte(9) {
                v.push(f("installed size", module_size(b)));
            }
            if let Some(b) = s.byte(0x0A) {
                v.push(f("enabled size", module_size(b)));
            }
            if let Some(b) = s.byte(0x0B).filter(|&b| b & 3 != 0) {
                v.push(f("error status", if b & 1 != 0 { "uncorrectable errors" } else { "correctable errors" }));
            }
            "DMI Memory Module"
        }
        16 => {
            if let Some(b) = s.byte(4) {
                v.push(f("location", array_location(b)));
            }
            if let Some(b) = s.byte(5) {
                v.push(f("usage", array_use(b)));
            }
            if let Some(b) = s.byte(6) {
                v.push(f("correction", array_ecc(b)));
            }
            let kb = match s.dword(7) {
                Some(0x8000_0000) => s.qword(0x0F).map(|b| b >> 10),
                d => d.map(u64::from),
            };
            if let Some(kb) = kb.filter(|&k| k > 0) {
                v.push(f("max capacity", size_kb(kb)));
            }
            if let Some(n) = s.word(0x0D) {
                v.push(f("max# of devices", n));
            }
            "DMI Physical Memory Array"
        }
        17 => {
            let m = mem_device_of(s);
            v.push(f("designation", ds(&m.locator)));
            v.push(f("format", m.form_factor));
            v.push(f("type", m.mem_type));
            if let Some(w) = m.total_width {
                v.push(f("total width", format!("{w} bits")));
            }
            if let Some(w) = m.data_width {
                v.push(f("data width", format!("{w} bits")));
            }
            if m.size_mb > 0 {
                v.push(f("size", size_kb(m.size_mb << 10)));
            }
            // CPU-Z shows the configured speed (in "MHz", really MT/s).
            let sp = if m.configured_speed > 0 { m.configured_speed } else { m.speed };
            if sp > 0 {
                v.push(f("speed", format!("{sp} MHz")));
            }
            v.push(f("manufacturer", ds(&m.manufacturer)));
            v.push(f("part number", ds(&m.part_number)));
            v.push(f("serial number", ds(&m.serial)));
            if let Some(mv) = m.configured_mv.or(m.min_mv) {
                v.push(f("voltage", format!("{:.6}", mv as f64 / 1000.0)));
            }
            if let Some(id) = m.module_mfr_id {
                v.push(f("manufacturer id", format!("0x{id:X}")));
            }
            if let Some(id) = m.module_product_id {
                v.push(f("product id", format!("0x{id:X}")));
            }
            "DMI Memory Device"
        }
        40 => {
            v.push(f("string #", s.strings.len()));
            for (i, x) in s.strings.iter().enumerate() {
                v.push(f(format!("string[{i}]"), ds(x)));
            }
            "DMI Additional Information Device"
        }
        8 => {
            push_str(&mut v, "designation", s.str(4).map(|x| format!("{x} (internal)")).as_deref());
            push_str(&mut v, "designation", s.str(6).map(|x| format!("{x} (external)")).as_deref());
            if let Some(p) = s.byte(8).filter(|&p| p != 0).and_then(port_type) {
                v.push(f("port type", p));
            }
            for o in [5, 7] {
                if let Some(c) = s.byte(o).filter(|&c| c != 0).and_then(connector_type) {
                    v.push(f("connector", c));
                }
            }
            "DMI Port Connector"
        }
        9 => {
            push_str(&mut v, "designation", s.str(4));
            if let Some(b) = s.byte(5) {
                v.push(f("type", slot_type(b)));
            }
            if let Some(u) = s.byte(7) {
                v.push(f("populated", if u == 4 { "yes" } else { "no" }));
            }
            "DMI Extension Slot"
        }
        _ => return None,
    };
    Some(g(title, v))
}

/// Group order of CPU-Z's DMI page.
const ORDER: [u8; 13] = [0, 1, 2, 3, 11, 4, 5, 6, 16, 17, 40, 8, 9];

fn dmi_section(t: &Table) -> Section {
    let mut nodes = Vec::new();
    if t.major > 0 {
        nodes.push(f("SMBIOS Version", format!("{}.{}", t.major, t.minor)));
    }
    for ty in ORDER {
        nodes.extend(t.of(ty).filter_map(|s| dmi_nodes(t, s)));
    }
    crate::report::section("DMI", nodes)
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    if !ctx.cfg.dmi {
        return Vec::new();
    }
    get(ctx).map(dmi_section).into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(ty: u8, handle: u16, body: &[u8], strings: &[&str]) -> Vec<u8> {
        let mut v = vec![ty, (body.len() + 4) as u8];
        v.extend(handle.to_le_bytes());
        v.extend(body);
        if strings.is_empty() {
            v.push(0);
        }
        for s in strings {
            v.extend(s.as_bytes());
            v.push(0);
        }
        v.push(0);
        v
    }

    fn sample() -> Table {
        let mut raw = st(
            0,
            0,
            &[1, 2, 0, 0xF0, 3, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0x23, 0xFF, 0xFF, 0x20, 0],
            &["Vendor, Inc.", "1.0", "12/17/2024"],
        );
        // Type 17: DDR5 DIMM 16 GB, 4800 rated / 6000 configured, mfr "Unknown" + JEP106 bank 5 0xEF.
        let mut m = vec![0u8; 0x64 - 4];
        let put = |m: &mut Vec<u8>, o: usize, b: &[u8]| m[o - 4..o - 4 + b.len()].copy_from_slice(b);
        put(&mut m, 0x08, &[64, 0, 64, 0, 0x00, 0x40, 0x09, 0, 1, 2, 0x22]);
        put(&mut m, 0x15, &[0xC0, 0x12, 3, 4, 0, 5, 1]);
        put(&mut m, 0x20, &[0x70, 0x17, 0x4C, 0x04, 0x4C, 0x04, 0x4C, 0x04]);
        put(&mut m, 0x2C, &[0x04, 0xEF]);
        raw.extend(st(17, 0x13, &m, &["DIMMA2", "P0 CHANNEL A", "Unknown", "12345678", "UD5-6000  "]));
        raw.extend(st(8, 0x1D, &[1, 0, 2, 0x12, 0x10], &["J1500", "USB-C"]));
        raw.extend(st(40, 0x1C, &[1, 9, 0, 0, 5, 1, 0, 0, 0, 0], &["AGESA!V9 ComboAm5PI 1.2.0.2b"]));
        raw.extend(st(127, 0xFFFF, &[], &[]));
        raw.extend([0xAA; 8]); // garbage after the end marker
        Table::parse(3, 7, &raw)
    }

    #[test]
    fn parse_and_decode() {
        let t = sample();
        assert_eq!(t.structs.len(), 5);
        assert_eq!(t.structs[4].ty, 127);
        let b = bios_of(t.first(0).unwrap());
        assert_eq!(b.date.as_deref(), Some("12/17/2024"));
        assert_eq!(b.rom_kb, Some(32 * 1024));
        let m = mem_device_of(t.first(17).unwrap());
        assert_eq!((m.size_mb, m.mem_type, m.form_factor), (16384, "DDR5", "DIMM"));
        assert_eq!((m.speed, m.configured_speed, m.configured_mv), (4800, 6000, Some(1100)));
        assert_eq!(m.manufacturer, "TEAMGROUP Inc.");
        assert_eq!(m.module_mfr_id, Some(0xEF04));
        assert_eq!(agesa(&t).as_deref(), Some("ComboAm5PI 1.2.0.2b"));

        let txt = crate::report::to_txt(&[dmi_section(&t)]);
        assert!(txt.contains("SMBIOS Version\t\t\t3.7\n"));
        assert!(txt.contains("\tvendor\t\t\tVendor Inc.\n"));
        assert!(txt.contains("\tspeed\t\t\t6000 MHz\n"));
        assert!(txt.contains("\tvoltage\t\t\t1.100000\n"));
        assert!(txt.contains("\tdesignation\t\tJ1500 (internal)\n\tdesignation\t\tUSB-C (external)\n\tport type\t\tUSB\n\tconnector\t\tAccess Bus (USB)\n"));
        assert!(txt.contains("\tstring #\t\t1\n"));
    }

    #[test]
    fn truncated_and_uuid() {
        // Truncated string set must not loop or panic.
        assert!(Table::parse(2, 8, &[1, 0x1B, 1, 0, 1, 2, 3]).structs.is_empty());
        let t = Table::parse(2, 8, &[1, 5, 1, 0, 1, b'a', b'b']);
        assert_eq!((t.structs.len(), t.structs[0].str(4)), (1, None));
        let mut s = synth(1, 0x1B, vec![(4, Some("X".into()))], &[]);
        s.data[8..0x18].copy_from_slice(&uuid_bytes("03000200-0400-0500-0006-000700080009").unwrap());
        let t = Table { major: 0, minor: 0, structs: vec![s] };
        assert_eq!(uuid_text(&t, &t.structs[0]).as_deref(), Some("{03000200-0400-0500-0006-000700080009}"));
        assert_eq!(entry_point_version(b"_SM3_\x18\x00\x03\x07"), Some((3, 7)));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn sysfs_fallback() {
        let dir = std::env::temp_dir().join(format!("cz-dmi-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (k, v) in [
            ("bios_date", "12/17/2024"),
            ("board_name", "PRO X870-P WIFI (MS-7E47)"),
            ("chassis_type", "3"),
            ("sys_vendor", "Micro-Star"),
        ] {
            std::fs::write(dir.join(k), format!("{v}\n")).unwrap();
        }
        let t = sysfs_table(dir.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let txt = crate::report::to_txt(&[dmi_section(&t)]);
        assert!(!txt.contains("SMBIOS Version") && !txt.contains("UUID") && !txt.contains("ROM size"));
        assert!(txt.contains("\tdate\t\t\t12/17/2024\n") && txt.contains("\tmodel\t\t\tPRO X870-P WIFI (MS-7E47)\n"));
        assert!(txt.contains("\tchassis type\t\tDesktop\n") && txt.contains("\tmanufacturer\t\tMicro-Star\n"));
    }
}

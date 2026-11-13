//! SMBIOS/DMI: fetching the table on each OS. The parser, accessors and the
//! DMI section are in `cpuzeez_core::smbios` (re-exported here).
pub use cpuzeez_core::smbios::*;

use crate::Ctx;
use crate::report::Section;

pub fn get(ctx: &Ctx) -> Option<&Table> {
    ctx.smbios.get_or_init(fetch).as_ref()
}

// ---------------------------------------------------------------- fetching

#[cfg(windows)]
fn fetch() -> Option<Table> {
    // RawSMBIOSData: Used20CallingMethod, Major, Minor, DmiRevision, u32 Length, table.
    let b = cpuzeez_platform::firmware::table(u32::from_be_bytes(*b"RSMB"), 0).filter(|b| b.len() >= 8)?;
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

// ---------------------------------------------------------------- Ctx accessors

pub fn bios(ctx: &Ctx) -> Option<Bios> {
    bios_info(get(ctx)?)
}

pub fn board(ctx: &Ctx) -> Option<Board> {
    board_info(get(ctx)?)
}

pub fn memory_devices(ctx: &Ctx) -> Vec<MemDevice> {
    get(ctx).map_or_else(Vec::new, memory_devices_of)
}

/// (external clock, max speed, current speed) in MHz from type 4; 0 = unknown.
pub fn processor_clocks(ctx: &Ctx) -> Option<(u32, u32, u32)> {
    processor_clocks_of(get(ctx)?)
}

/// Socket designation string of the CPU (e.g. "AM5", "LGA1700").
pub fn socket(ctx: &Ctx) -> Option<String> {
    socket_of(get(ctx)?)
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    if !ctx.cfg.dmi {
        return Vec::new();
    }
    get(ctx).map(dmi_section).into_iter().collect()
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

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

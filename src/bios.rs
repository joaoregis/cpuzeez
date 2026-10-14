//! BIOS section (FEATURES.md 3.7): UEFI yes/no, BIOS date, AMD AGESA / Intel microcode.
use crate::Ctx;
use crate::report::{Node, Section, f};

pub fn section(ctx: &Ctx) -> Section {
    let mut v: Vec<Node> = Vec::new();
    if let Some(u) = uefi() {
        v.push(f("UEFI", u));
    }
    let b = crate::smbios::bios(ctx).unwrap_or_default();
    if let Some(d) = b.date {
        v.push(f("BIOS Date", d));
    }
    if cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && let Some(ver) = b.version
    {
        v.push(f("Firmware Version", ver)); // iBoot
    }
    let cpu = crate::cpu::get(ctx);
    if cpu.vendor == "AuthenticAMD" || cpu.vendor == "HygonGenuine" {
        if let Some(a) = crate::smbios::get(ctx).and_then(crate::smbios::agesa).or_else(|| rom_agesa(ctx)) {
            v.push(f("AMD AGESA", a));
        }
    } else if let Some(m) = &cpu.microcode {
        v.push(f("Microcode Revision", m));
    }
    crate::report::section("BIOS", v)
}

#[cfg(windows)]
fn uefi() -> Option<&'static str> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFirmwareType(t: *mut u32) -> i32;
    }
    let mut t = 0u32;
    // FirmwareTypeBios = 1, FirmwareTypeUefi = 2.
    match unsafe { GetFirmwareType(&mut t) } {
        0 => None,
        _ => match t {
            1 => Some("No"),
            2 => Some("Yes"),
            _ => None,
        },
    }
}

#[cfg(target_os = "linux")]
fn uefi() -> Option<&'static str> {
    Some(if std::path::Path::new("/sys/firmware/efi").exists() { "Yes" } else { "No" })
}

#[cfg(target_os = "macos")]
fn uefi() -> Option<&'static str> {
    Some(if cfg!(target_arch = "x86_64") { "Yes" } else { "No (iBoot)" })
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn uefi() -> Option<&'static str> {
    None
}

/// Scan the memory-mapped BIOS flash (top 16 MB below 4 GB) for "AGESA!Vn<sep>version".
/// Only works when the hardware backend can read physical memory.
fn rom_agesa(ctx: &Ctx) -> Option<String> {
    const BASE: u64 = 0xFF00_0000;
    const SIZE: u64 = 0x100_0000;
    const CHUNK: usize = 0x10000;
    const TAIL: usize = 64; // overlap so a match spanning two chunks isn't lost
    let mut buf = vec![0u8; CHUNK + TAIL];
    for off in (0..SIZE).step_by(CHUNK) {
        let n = if off + (CHUNK + TAIL) as u64 > SIZE { CHUNK } else { CHUNK + TAIL };
        if !ctx.hw.phys_read(BASE + off, &mut buf[..n]) {
            return None;
        }
        if let Some(a) = find_agesa(&buf[..n]) {
            return Some(a);
        }
    }
    None
}

fn find_agesa(b: &[u8]) -> Option<String> {
    let pat = b"AGESA!V";
    let mut i = 0;
    while let Some(p) = b[i..].windows(pat.len()).position(|w| w == pat) {
        let start = i + p + pat.len() + 1; // skip the interface digit
        let rest = b.get(start..)?;
        let rest = &rest[rest.iter().position(|&c| c != 0 && c != b' ').unwrap_or(rest.len())..];
        let end = rest.iter().position(|&c| !(0x20..0x7F).contains(&c)).unwrap_or(rest.len());
        let s = String::from_utf8_lossy(&rest[..end]).trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
        i = start;
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn agesa_in_rom() {
        let mut rom = vec![0xFFu8; 100];
        rom.extend(b"AGESA!V9\0ComboAm5PI 1.2.0.2b\0\xFF");
        assert_eq!(super::find_agesa(&rom).as_deref(), Some("ComboAm5PI 1.2.0.2b"));
        assert_eq!(super::find_agesa(b"AGESA!V9\0\0"), None);
    }
}

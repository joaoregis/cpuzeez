//! Apple SMC over IOKit (user client selector 2 with the widely documented
//! 80-byte SMCParamStruct) and the battery from `ioreg AppleSmartBattery`.
//! No root needed on either Intel or Apple Silicon Macs.

use super::{Kind, Monitor};
use std::ffi::{c_char, c_void};
use std::sync::OnceLock;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingService(main: u32, matching: *mut c_void) -> u32;
    fn IOServiceOpen(svc: u32, task: u32, ty: u32, conn: *mut u32) -> i32;
    fn IOObjectRelease(o: u32) -> i32;
    fn IOConnectCallStructMethod(
        conn: u32,
        sel: u32,
        inp: *const c_void,
        in_size: usize,
        out: *mut c_void,
        out_size: *mut usize,
    ) -> i32;
}
unsafe extern "C" {
    static mach_task_self_: u32;
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Vers {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PLimit {
    version: u16,
    length: u16,
    cpu: u32,
    gpu: u32,
    mem: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct KeyInfo {
    size: u32,
    ty: u32,
    attr: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Param {
    key: u32,
    vers: Vers,
    plimit: PLimit,
    info: KeyInfo,
    result: u8,
    status: u8,
    data8: u8,
    data32: u32,
    bytes: [u8; 32],
}
const _: () = assert!(std::mem::size_of::<Param>() == 80);

const READ_KEY: u8 = 5;
const KEY_AT_INDEX: u8 = 8;
const KEY_INFO: u8 = 9;

fn conn() -> Option<u32> {
    static C: OnceLock<Option<u32>> = OnceLock::new();
    *C.get_or_init(|| unsafe {
        // SAFETY: documented IOKit calls; the matching dict is consumed by
        // IOServiceGetMatchingService. The connection lives for the process.
        let svc = IOServiceGetMatchingService(0, IOServiceMatching(c"AppleSMC".as_ptr()));
        if svc == 0 {
            return None;
        }
        let mut c = 0u32;
        let kr = IOServiceOpen(svc, mach_task_self_, 0, &mut c);
        IOObjectRelease(svc);
        (kr == 0).then_some(c)
    })
}

fn call(c: u32, p: &Param) -> Option<Param> {
    let mut out = Param::default();
    let mut sz = std::mem::size_of::<Param>();
    // SAFETY: in/out are 80-byte SMCParamStruct values.
    let kr = unsafe {
        IOConnectCallStructMethod(c, 2, (p as *const Param).cast(), sz, (&mut out as *mut Param).cast(), &mut sz)
    };
    (kr == 0 && out.result == 0).then_some(out)
}

fn key(s: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*s)
}

/// (type code, bytes) of an SMC key.
fn read(k: u32) -> Option<(u32, Vec<u8>)> {
    let c = conn()?;
    let info = call(c, &Param { key: k, data8: KEY_INFO, ..Default::default() })?.info;
    let o = call(c, &Param { key: k, data8: READ_KEY, info, ..Default::default() })?;
    Some((info.ty, o.bytes[..(info.size as usize).min(32)].to_vec()))
}

/// Decode the numeric SMC types we care about.
fn decode(ty: u32, b: &[u8]) -> Option<f64> {
    let be16 = || Some(u16::from_be_bytes([*b.first()?, *b.get(1)?]));
    Some(match &ty.to_be_bytes() {
        b"sp78" => be16()? as i16 as f64 / 256.0,
        b"fpe2" => be16()? as f64 / 4.0,
        b"flt " => f32::from_le_bytes(b.get(..4)?.try_into().ok()?) as f64,
        b"ui8 " => *b.first()? as f64,
        b"ui16" => be16()? as f64,
        b"ui32" => u32::from_be_bytes(b.get(..4)?.try_into().ok()?) as f64,
        b"si8 " => *b.first()? as i8 as f64,
        b"si16" => be16()? as i16 as f64,
        _ => return None,
    })
}

fn value(k: &[u8; 4]) -> Option<f64> {
    let (ty, b) = read(key(k))?;
    decode(ty, &b).filter(|v| v.is_finite())
}

/// Every temperature key, enumerated once (thousands of keys on Apple Silicon).
fn temp_keys() -> &'static [[u8; 4]] {
    static K: OnceLock<Vec<[u8; 4]>> = OnceLock::new();
    K.get_or_init(|| {
        let Some(c) = conn() else { return Vec::new() };
        let n = value(b"#KEY").unwrap_or(0.0) as u32;
        (0..n.min(8192))
            .filter_map(|i| call(c, &Param { data8: KEY_AT_INDEX, data32: i, ..Default::default() }))
            .map(|o| o.key.to_be_bytes())
            .filter(|k| k[0] == b'T' && k.iter().all(u8::is_ascii_graphic))
            .collect()
    })
}

fn temp_label(k: &[u8; 4]) -> String {
    let s = String::from_utf8_lossy(k).to_string();
    let named = match k {
        b"TC0P" => "CPU Proximity",
        b"TC0D" | b"TC0E" | b"TC0F" => "CPU Die",
        b"TG0P" => "GPU Proximity",
        b"TG0D" => "GPU Die",
        b"TA0P" => "Ambient",
        b"TB0T" => "Battery",
        b"Ts0P" => "Palm Rest",
        b"TW0P" => "Wireless",
        b"TM0P" => "Memory Proximity",
        _ if k.starts_with(b"Tp") => return format!("CPU P-core {s}"),
        _ if k.starts_with(b"Te") => return format!("CPU E-core {s}"),
        _ if k.starts_with(b"Tg") => return format!("GPU {s}"),
        _ => return s,
    };
    format!("{named} {s}")
}

pub fn monitors() -> Vec<Monitor> {
    let mut v = Vec::new();
    if conn().is_some() {
        let mut m = Monitor::new("Apple SMC");
        for k in temp_keys() {
            if let Some(t) = value(k).filter(|t| (1.0..130.0).contains(t)) {
                m.push(Kind::Temperature, temp_label(k), t, None);
            }
        }
        for (k, l) in [(b"VD0R", "DC In"), (b"VBAT", "Battery"), (b"VP0R", "+12V")] {
            if let Some(x) = value(k).filter(|x| *x > 0.0) {
                m.push(Kind::Voltage, l, x, None);
            }
        }
        for (k, l) in [(b"ID0R", "DC In"), (b"IC0R", "CPU")] {
            if let Some(x) = value(k).filter(|x| *x > 0.0) {
                m.push(Kind::Current, l, x, None);
            }
        }
        for (k, l) in [
            (b"PSTR", "System Total"),
            (b"PCPC", "CPU Package"),
            (b"PCPG", "GPU"),
            (b"PDTR", "DC In"),
            (b"PPBR", "Battery"),
        ] {
            if let Some(x) = value(k).filter(|x| *x > 0.0) {
                m.push(Kind::Power, l, x, None);
            }
        }
        let fans = value(b"FNum").unwrap_or(0.0) as u8;
        for i in 0..fans.min(10) {
            if let Some(rpm) = value(&[b'F', b'0' + i, b'A', b'c']) {
                m.push(Kind::Fan, format!("Fan #{i}"), rpm, None);
            }
        }
        v.push(m);
    }
    v.extend(battery());
    v
}

fn battery() -> Option<Monitor> {
    let x = crate::os::cmd("ioreg", &["-rc", "AppleSmartBattery", "-a"])?;
    let mut m = Monitor::new("Battery");
    let (cur, max) = (plist_int(&x, "CurrentCapacity"), plist_int(&x, "MaxCapacity"));
    if let (Some(c), Some(mx)) = (cur, max.filter(|&m| m > 0)) {
        m.push(Kind::Level, "Charge", c as f64 * 100.0 / mx as f64, None);
    }
    if let Some(mv) = plist_int(&x, "Voltage") {
        m.push(Kind::Voltage, "Battery", mv as f64 / 1000.0, None);
    }
    if let Some(ma) = plist_int(&x, "Amperage") {
        m.push(Kind::Current, "Battery", ma as f64 / 1000.0, None);
    }
    if let Some(t) = plist_int(&x, "Temperature") {
        m.push(Kind::Temperature, "Battery", t as f64 / 100.0, None);
    }
    (!m.readings.is_empty()).then_some(m)
}

/// Top-level `<key>K</key><integer>N</integer>` of `ioreg -a` output (an
/// array holding one dict, so top-level keys are indented by two tabs).
/// Negative values are printed as their 64-bit two's complement.
fn plist_int(x: &str, k: &str) -> Option<i64> {
    let i = x.find(&format!("\n\t\t<key>{k}</key>"))?;
    let rest = x[i..].split_once("</key>")?.1.trim_start().strip_prefix("<integer>")?;
    let n: i128 = rest.split('<').next()?.trim().parse().ok()?;
    Some(n as u64 as i64)
}

#[cfg(test)]
mod tests {
    #[test]
    fn decode() {
        assert_eq!(super::decode(super::key(b"sp78"), &[0x2D, 0x80]), Some(45.5));
        assert_eq!(super::decode(super::key(b"fpe2"), &[0x1F, 0x40]), Some(2000.0));
        assert_eq!(super::decode(super::key(b"flt "), &42.5f32.to_le_bytes()), Some(42.5));
        let x = "<array>\n\t<dict>\n\t\t<key>Amperage</key>\n\t\t<integer>18446744073709550616</integer>\n\t</dict>\n</array>";
        assert_eq!(super::plist_int(x, "Amperage"), Some(-1000));
    }
}

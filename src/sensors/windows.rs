//! Windows user-mode sensors: PDH performance counters (per-CPU effective
//! clock, ACPI thermal zones; no admin needed, unlike WMI MSAcpi_*) and the
//! battery from GetSystemPowerStatus.

use super::{Kind, Monitor};
use std::ffi::c_void;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[link(name = "pdh")]
unsafe extern "system" {
    fn PdhOpenQueryW(src: *const u16, user: usize, q: *mut isize) -> u32;
    fn PdhAddEnglishCounterW(q: isize, path: *const u16, user: usize, c: *mut isize) -> u32;
    fn PdhCollectQueryData(q: isize) -> u32;
    fn PdhGetFormattedCounterArrayW(c: isize, fmt: u32, size: *mut u32, count: *mut u32, items: *mut c_void) -> u32;
}

#[repr(C)]
struct SystemPowerStatus {
    ac: u8,
    flag: u8,
    percent: u8,
    saver: u8,
    life: u32,
    full_life: u32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetSystemPowerStatus(s: *mut SystemPowerStatus) -> i32;
}

const PDH_FMT_DOUBLE: u32 = 0x200;
const PDH_MORE_DATA: u32 = 0x8000_07D2;

#[repr(C)]
struct FmtValue {
    status: u32,
    value: f64,
}

#[repr(C)]
struct Item {
    name: *const u16,
    value: FmtValue,
}

struct Pdh {
    q: isize,
    perf: isize,
    freq: isize,
    thermal: isize,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// One PDH query kept open for the process: rate counters need two samples,
/// so the first call collects twice and later calls diff against the last.
/// A sample younger than 500 ms is reused: back-to-back callers would
/// otherwise get a near-empty interval (parked cores then read 0).
fn sample() -> Vec<(String, [Option<f64>; 3])> {
    static LAST: Mutex<Option<(Instant, Vec<(String, [Option<f64>; 3])>)>> = Mutex::new(None);
    let Ok(mut last) = LAST.lock() else { return Vec::new() };
    if let Some((_, v)) = last.as_ref().filter(|l| l.0.elapsed() < Duration::from_millis(500)) {
        return v.clone();
    }
    let v = collect();
    *last = Some((Instant::now(), v.clone()));
    v
}

fn collect() -> Vec<(String, [Option<f64>; 3])> {
    static Q: Mutex<Option<Option<Pdh>>> = Mutex::new(None);
    let Ok(mut g) = Q.lock() else { return Vec::new() };
    let first = g.is_none();
    let Some(p) = g.get_or_insert_with(open).as_ref() else { return Vec::new() };
    // SAFETY: handles come from PdhOpenQueryW/PdhAddEnglishCounterW above.
    unsafe {
        if first {
            PdhCollectQueryData(p.q);
            std::thread::sleep(Duration::from_millis(100));
        }
        if PdhCollectQueryData(p.q) != 0 {
            return Vec::new();
        }
    }
    let mut out: Vec<(String, [Option<f64>; 3])> = Vec::new();
    for (slot, c) in [p.perf, p.freq, p.thermal].into_iter().enumerate() {
        for (name, v) in array(c) {
            match out.iter_mut().find(|e| e.0 == name) {
                Some(e) => e.1[slot] = Some(v),
                None => {
                    let mut a = [None; 3];
                    a[slot] = Some(v);
                    out.push((name, a));
                }
            }
        }
    }
    out
}

fn open() -> Option<Pdh> {
    let mut q = 0isize;
    // SAFETY: out-params per pdh.h; counter paths are NUL-terminated UTF-16.
    unsafe {
        if PdhOpenQueryW(std::ptr::null(), 0, &mut q) != 0 {
            return None;
        }
        let add = |path: &str| {
            let mut c = 0isize;
            if PdhAddEnglishCounterW(q, wide(path).as_ptr(), 0, &mut c) == 0 { c } else { 0 }
        };
        Some(Pdh {
            q,
            perf: add(r"\Processor Information(*)\% Processor Performance"),
            freq: add(r"\Processor Information(*)\Processor Frequency"),
            thermal: add(r"\Thermal Zone Information(*)\Temperature"),
        })
    }
}

/// (instance name, value) for every instance of a wildcard counter.
fn array(c: isize) -> Vec<(String, f64)> {
    if c == 0 {
        return Vec::new();
    }
    let (mut size, mut n) = (0u32, 0u32);
    // SAFETY: size query first, then a u64-aligned buffer of that many bytes;
    // PDH stores the items followed by their name strings in it.
    unsafe {
        if PdhGetFormattedCounterArrayW(c, PDH_FMT_DOUBLE, &mut size, &mut n, std::ptr::null_mut()) != PDH_MORE_DATA {
            return Vec::new();
        }
        let mut buf = vec![0u64; (size as usize).div_ceil(8) + 1];
        if PdhGetFormattedCounterArrayW(c, PDH_FMT_DOUBLE, &mut size, &mut n, buf.as_mut_ptr().cast()) != 0 {
            return Vec::new();
        }
        let items = std::slice::from_raw_parts(buf.as_ptr() as *const Item, n as usize);
        items
            .iter()
            .filter(|it| it.value.status <= 1 && !it.name.is_null()) // PDH_CSTATUS_VALID_DATA / NEW_DATA
            .map(|it| {
                let len = (0..).take_while(|&i| *it.name.add(i) != 0).count();
                (String::from_utf16_lossy(std::slice::from_raw_parts(it.name, len)), it.value.value)
            })
            .collect()
    }
}

/// Effective MHz per global logical CPU index ("g,n" instances).
pub fn cpu_mhz() -> Vec<Option<f64>> {
    let groups = crate::os::cpu_groups();
    let mut v = vec![None; groups.iter().map(|g| g.1 as usize).sum()];
    for (name, [perf, freq, _]) in sample() {
        let Some((g, n)) = name.split_once(',') else { continue };
        let (Ok(g), Ok(n)) = (g.parse::<u16>(), n.parse::<usize>()) else { continue };
        let off: usize = groups.iter().take_while(|x| x.0 < g).map(|x| x.1 as usize).sum();
        if let (Some(p), Some(f), Some(slot)) = (perf, freq, v.get_mut(off + n))
            && f * p > 0.0
        {
            *slot = Some(f * p / 100.0);
        }
    }
    v
}

pub fn monitors() -> Vec<Monitor> {
    let mut out = Vec::new();
    let mut tz = Monitor::new("ACPI Thermal Zone");
    for (name, [_, _, k]) in sample() {
        if let Some(k) = k.filter(|&k| k > 0.0) {
            let label = name.rsplit('.').next().unwrap_or(&name).to_string();
            tz.push(Kind::Temperature, label, k - 273.15, None);
        }
    }
    out.push(tz);
    let mut s = SystemPowerStatus { ac: 0, flag: 0, percent: 0, saver: 0, life: 0, full_life: 0 };
    // SAFETY: plain out-param.
    if unsafe { GetSystemPowerStatus(&mut s) } != 0 && s.flag & 0x80 == 0 && s.flag != 0xFF && s.percent <= 100 {
        let mut b = Monitor::new("Battery");
        b.push(Kind::Level, if s.ac == 1 { "Charge (AC)" } else { "Charge" }, s.percent as f64, None);
        out.push(b);
    }
    out
}

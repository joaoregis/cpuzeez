//! Timers section and Tools > Timers (FEATURES.md 1.4, 3.4).
//!
//! - ACPI timer: the ACPI PM timer, 3.579545 MHz by definition (ACPI spec
//!   4.8.3.3). Its I/O port comes from the FADT; it is measured through the
//!   privileged backend when possible, else reported at its nominal rate when
//!   the FADT (or the Linux clocksource list) says it exists.
//! - Perf timer: the OS high-resolution counter (QPC, the Linux clocksource,
//!   mach_absolute_time).
//! - Sys timer: the scheduler tick (Windows timer resolution, Linux CONFIG_HZ,
//!   macOS kern.clockrate).

use crate::Ctx;
use crate::report::{self, Section, f};
use std::time::{Duration, Instant};

pub const ACPI_PM_HZ: f64 = 3_579_545.0;

pub fn section(ctx: &Ctx) -> Section {
    report::section("Timers", timers(ctx).into_iter().map(|(k, hz)| f(k, hz_text(hz))).collect())
}

/// ("ACPI timer" | "Perf timer" | "Sys timer", Hz) for every timer found.
pub fn timers(ctx: &Ctx) -> Vec<(String, f64)> {
    let mut v = Vec::new();
    if ctx.cfg.acpi
        && let Some(hz) = acpi_timer(ctx)
    {
        v.push(("ACPI timer".to_string(), hz));
    }
    if let Some(hz) = perf_timer() {
        v.push(("Perf timer".to_string(), hz));
    }
    if let Some(hz) = sys_timer() {
        v.push(("Sys timer".to_string(), hz));
    }
    v
}

/// "3.580 MHz", "10.000 MHz", "1.000 KHz" (CPU-Z spelling).
pub fn hz_text(hz: f64) -> String {
    if hz >= 1e9 {
        format!("{:.3} GHz", hz / 1e9)
    } else if hz >= 1e6 {
        format!("{:.3} MHz", hz / 1e6)
    } else if hz >= 1e3 {
        format!("{:.3} KHz", hz / 1e3)
    } else {
        format!("{hz:.3} Hz")
    }
}

// ---- ACPI PM timer ----

/// (I/O port, 32-bit counter) from a raw FADT, None when the platform has no
/// PM timer (zero block, hardware-reduced ACPI, or a memory-mapped X block).
fn fadt_pm_timer(fadt: &[u8]) -> Option<(u16, bool)> {
    if fadt.get(0..4)? != b"FACP" {
        return None;
    }
    let u32_at = |o: usize| fadt.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let flags = u32_at(112).unwrap_or(0);
    if flags & (1 << 20) != 0 {
        return None; // HW_REDUCED_ACPI
    }
    let ext32 = flags & (1 << 8) != 0; // TMR_VAL_EXT
    // X_PM_TMR_BLK (ACPI 2.0+): generic address, space 1 = system I/O.
    if let Some(gas) = fadt.get(208..220) {
        let addr = u64::from_le_bytes(gas[4..12].try_into().unwrap());
        if addr != 0 {
            return (gas[0] == 1 && addr <= 0xFFFF).then_some((addr as u16, ext32));
        }
    }
    let port = u32_at(76)?;
    let len = *fadt.get(91).unwrap_or(&4);
    (port != 0 && port <= 0xFFFF && len >= 4).then_some((port as u16, ext32))
}

fn acpi_timer(ctx: &Ctx) -> Option<f64> {
    let Some(fadt) = read_fadt() else {
        // Unprivileged Linux can't read the FADT but still lists the timer.
        #[cfg(target_os = "linux")]
        if crate::os::read_trim("/sys/devices/system/clocksource/clocksource0/available_clocksource")?
            .contains("acpi_pm")
        {
            return Some(ACPI_PM_HZ);
        }
        return None;
    };
    let (port, ext32) = fadt_pm_timer(&fadt)?;
    // A bogus port (all ones, stuck) gives an implausible rate: keep the nominal one.
    Some(measure_pm(ctx, port, ext32).filter(|hz| (hz / ACPI_PM_HZ - 1.0).abs() < 0.05).unwrap_or(ACPI_PM_HZ))
}

fn measure_pm(ctx: &Ctx, port: u16, ext32: bool) -> Option<f64> {
    let mask = if ext32 { u32::MAX } else { 0x00FF_FFFF };
    let read = || ctx.hw.io_in32(port).map(|v| v & mask);
    let (mut last, t0) = (read()?, Instant::now());
    let mut ticks = 0u64;
    // Sample often enough that a 24-bit counter (4.7 s period) can't wrap twice.
    while t0.elapsed() < Duration::from_millis(100) {
        let v = read()?;
        ticks += (v.wrapping_sub(last) & mask) as u64;
        last = v;
    }
    let s = t0.elapsed().as_secs_f64();
    (ticks > 0).then(|| ticks as f64 / s)
}

#[cfg(windows)]
fn read_fadt() -> Option<Vec<u8>> {
    use crate::os::win::GetSystemFirmwareTable;
    let (prov, id) = (u32::from_be_bytes(*b"ACPI"), u32::from_le_bytes(*b"FACP"));
    // SAFETY: two-call pattern; the second call gets a buffer of the reported size.
    unsafe {
        let n = GetSystemFirmwareTable(prov, id, std::ptr::null_mut(), 0);
        if n == 0 {
            return None;
        }
        let mut b = vec![0u8; n as usize];
        let got = GetSystemFirmwareTable(prov, id, b.as_mut_ptr(), n);
        (got == n).then_some(b)
    }
}

#[cfg(target_os = "linux")]
fn read_fadt() -> Option<Vec<u8>> {
    std::fs::read("/sys/firmware/acpi/tables/FACP").ok()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn read_fadt() -> Option<Vec<u8>> {
    None // macOS: no ACPI table access (and no ACPI at all on Apple Silicon).
}

// ---- Perf timer ----

#[cfg(windows)]
fn perf_timer() -> Option<f64> {
    let mut f = 0i64;
    // SAFETY: plain out-parameter.
    (unsafe { crate::os::win::QueryPerformanceFrequency(&mut f) } != 0 && f > 0).then_some(f as f64)
}

#[cfg(target_os = "linux")]
fn perf_timer() -> Option<f64> {
    // The rate of the hardware behind CLOCK_MONOTONIC; clock_getres only
    // reports 1 ns for every high-resolution clocksource.
    let cs =
        crate::os::read_trim("/sys/devices/system/clocksource/clocksource0/current_clocksource").unwrap_or_default();
    match cs.as_str() {
        "tsc" => return crate::cpu::clocks::tsc_hz(),
        "hpet" => return Some(14_318_180.0),
        "acpi_pm" => return Some(ACPI_PM_HZ),
        s if s.starts_with("hyperv") => return Some(10_000_000.0), // Hyper-V reference time, 100 ns
        "arch_sys_counter" => {
            if let Some(hz) = arm_cntfrq() {
                return Some(hz);
            }
        }
        _ => {}
    }
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: plain out-parameter.
    let ok = unsafe { libc::clock_getres(libc::CLOCK_MONOTONIC, &mut ts) } == 0;
    let ns = ts.tv_sec as f64 * 1e9 + ts.tv_nsec as f64;
    (ok && ns > 0.0).then(|| 1e9 / ns)
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
fn arm_cntfrq() -> Option<f64> {
    let f: u64;
    // SAFETY: CNTFRQ_EL0 is readable from EL0 on Linux.
    unsafe { std::arch::asm!("mrs {}, cntfrq_el0", out(reg) f, options(nomem, nostack)) };
    (f > 0).then_some(f as f64)
}
#[cfg(all(target_os = "linux", not(target_arch = "aarch64")))]
fn arm_cntfrq() -> Option<f64> {
    None
}

#[cfg(target_os = "macos")]
fn perf_timer() -> Option<f64> {
    #[repr(C)]
    struct Timebase {
        numer: u32,
        denom: u32,
    }
    unsafe extern "C" {
        fn mach_timebase_info(info: *mut Timebase) -> i32;
    }
    let mut t = Timebase { numer: 0, denom: 0 };
    // SAFETY: plain out-parameter. ticks * numer / denom = ns.
    (unsafe { mach_timebase_info(&mut t) } == 0 && t.numer > 0).then(|| 1e9 * t.denom as f64 / t.numer as f64)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn perf_timer() -> Option<f64> {
    None
}

// ---- Sys timer ----

#[cfg(windows)]
fn sys_timer() -> Option<f64> {
    #[link(name = "ntdll", kind = "raw-dylib")]
    unsafe extern "system" {
        fn NtQueryTimerResolution(coarsest: *mut u32, finest: *mut u32, current: *mut u32) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemTimeAdjustment(adj: *mut u32, inc: *mut u32, disabled: *mut i32) -> i32;
    }
    let (mut a, mut b, mut cur) = (0u32, 0u32, 0u32);
    // SAFETY: plain out-parameters; units are 100 ns.
    if unsafe { NtQueryTimerResolution(&mut a, &mut b, &mut cur) } >= 0 && cur > 0 {
        return Some(1e7 / cur as f64);
    }
    let (mut adj, mut inc, mut dis) = (0u32, 0u32, 0i32);
    (unsafe { GetSystemTimeAdjustment(&mut adj, &mut inc, &mut dis) } != 0 && inc > 0).then(|| 1e7 / inc as f64)
}

#[cfg(target_os = "linux")]
fn sys_timer() -> Option<f64> {
    // CONFIG_HZ from the kernel config, else measured from jiffies in
    // /proc/timer_list (root), else USER_HZ as a last resort.
    let rel = crate::os::read_trim("/proc/sys/kernel/osrelease").unwrap_or_default();
    if let Ok(cfg) = std::fs::read_to_string(format!("/boot/config-{rel}"))
        && let Some(hz) = cfg.lines().find_map(|l| l.strip_prefix("CONFIG_HZ=")?.trim().parse::<f64>().ok())
    {
        return Some(hz);
    }
    let jiffies = || -> Option<(u64, u64)> {
        let s = std::fs::read_to_string("/proc/timer_list").ok()?;
        let now = s.lines().find_map(|l| l.strip_prefix("now at ")?.split_whitespace().next()?.parse().ok())?;
        let j = s.lines().find_map(|l| l.trim().strip_prefix("jiffies:")?.trim().parse().ok())?;
        Some((now, j))
    };
    if let Some((n0, j0)) = jiffies() {
        std::thread::sleep(Duration::from_millis(200));
        if let Some((n1, j1)) = jiffies().filter(|&(n1, _)| n1 > n0) {
            let hz = (j1 - j0) as f64 * 1e9 / (n1 - n0) as f64;
            // Snap to the usual CONFIG_HZ choices.
            return Some(
                [100.0, 250.0, 300.0, 1000.0]
                    .into_iter()
                    .min_by(|a, b| (a - hz).abs().total_cmp(&(b - hz).abs()))
                    .unwrap(),
            );
        }
    }
    // SAFETY: sysconf has no preconditions.
    let t = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    (t > 0).then_some(t as f64)
}

#[cfg(target_os = "macos")]
fn sys_timer() -> Option<f64> {
    // struct clockinfo { int hz, tick, tickadj, stathz, profhz; }
    let mut ci = [0i32; 5];
    let mut len = std::mem::size_of_val(&ci);
    // SAFETY: fixed-size out buffer of the struct's size.
    let r = unsafe {
        libc::sysctlbyname(c"kern.clockrate".as_ptr(), ci.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0)
    };
    (r == 0 && ci[0] > 0).then_some(ci[0] as f64)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn sys_timer() -> Option<f64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text() {
        assert_eq!(hz_text(ACPI_PM_HZ), "3.580 MHz");
        assert_eq!(hz_text(1e7), "10.000 MHz");
        assert_eq!(hz_text(1000.0), "1.000 KHz");
    }

    #[test]
    fn fadt() {
        let mut t = vec![0u8; 244];
        t[..4].copy_from_slice(b"FACP");
        t[76..80].copy_from_slice(&0x808u32.to_le_bytes());
        t[91] = 4;
        assert_eq!(fadt_pm_timer(&t), Some((0x808, false)));
        t[112] = 1; // bit 8 is in byte 113
        t[113] = 1;
        t[208] = 1;
        t[212..220].copy_from_slice(&0x408u64.to_le_bytes());
        assert_eq!(fadt_pm_timer(&t), Some((0x408, true)));
        t[114] = 0x10; // HW_REDUCED_ACPI (bit 20)
        assert_eq!(fadt_pm_timer(&t), None);
    }
}

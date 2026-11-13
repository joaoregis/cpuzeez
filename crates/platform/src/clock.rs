//! Timer rates and the CPU's own counters: the raw calls behind the Timers
//! section and the clock measurements, as safe functions (v1.1 M4-02).

/// QueryPerformanceFrequency, in Hz.
#[cfg(windows)]
pub fn qpc_hz() -> Option<f64> {
    let mut f = 0i64;
    // SAFETY: plain out-parameter.
    (unsafe { crate::os::win::QueryPerformanceFrequency(&mut f) } != 0 && f > 0).then_some(f as f64)
}

/// The kernel timer's current period in 100 ns units: NtQueryTimerResolution,
/// else GetSystemTimeAdjustment's increment.
#[cfg(windows)]
pub fn timer_period_100ns() -> Option<u32> {
    #[link(name = "ntdll", kind = "raw-dylib")]
    unsafe extern "system" {
        fn NtQueryTimerResolution(coarsest: *mut u32, finest: *mut u32, current: *mut u32) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemTimeAdjustment(adj: *mut u32, inc: *mut u32, disabled: *mut i32) -> i32;
    }
    let (mut a, mut b, mut cur) = (0u32, 0u32, 0u32);
    // SAFETY: plain out-parameters.
    if unsafe { NtQueryTimerResolution(&mut a, &mut b, &mut cur) } >= 0 && cur > 0 {
        return Some(cur);
    }
    let (mut adj, mut inc, mut dis) = (0u32, 0u32, 0i32);
    // SAFETY: plain out-parameters.
    (unsafe { GetSystemTimeAdjustment(&mut adj, &mut inc, &mut dis) } != 0 && inc > 0).then_some(inc)
}

/// clock_getres(CLOCK_MONOTONIC), in ns.
#[cfg(target_os = "linux")]
pub fn monotonic_res_ns() -> Option<f64> {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: plain out-parameter.
    let ok = unsafe { libc::clock_getres(libc::CLOCK_MONOTONIC, &mut ts) } == 0;
    let ns = ts.tv_sec as f64 * 1e9 + ts.tv_nsec as f64;
    (ok && ns > 0.0).then_some(ns)
}

/// sysconf(_SC_CLK_TCK): USER_HZ.
#[cfg(target_os = "linux")]
pub fn clk_tck() -> Option<f64> {
    // SAFETY: sysconf has no preconditions.
    let t = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    (t > 0).then_some(t as f64)
}

/// CNTFRQ_EL0, the generic timer's rate, in Hz.
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub fn cntfrq() -> Option<f64> {
    let f: u64;
    // SAFETY: CNTFRQ_EL0 is readable from EL0 on Linux.
    unsafe { std::arch::asm!("mrs {}, cntfrq_el0", out(reg) f, options(nomem, nostack)) };
    (f > 0).then_some(f as f64)
}

/// mach_timebase_info as (numer, denom): ticks * numer / denom = ns.
#[cfg(target_os = "macos")]
pub fn mach_timebase() -> Option<(u32, u32)> {
    #[repr(C)]
    struct Timebase {
        numer: u32,
        denom: u32,
    }
    unsafe extern "C" {
        fn mach_timebase_info(info: *mut Timebase) -> i32;
    }
    let mut t = Timebase { numer: 0, denom: 0 };
    // SAFETY: plain out-parameter.
    (unsafe { mach_timebase_info(&mut t) } == 0 && t.numer > 0 && t.denom > 0).then_some((t.numer, t.denom))
}

/// kern.clockrate's `hz`.
#[cfg(target_os = "macos")]
pub fn clockrate_hz() -> Option<f64> {
    // struct clockinfo { int hz, tick, tickadj, stathz, profhz; }
    let mut ci = [0i32; 5];
    let mut len = std::mem::size_of_val(&ci);
    // SAFETY: fixed-size out buffer of the struct's size.
    let r = unsafe {
        libc::sysctlbyname(c"kern.clockrate".as_ptr(), ci.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0)
    };
    (r == 0 && ci[0] > 0).then_some(ci[0] as f64)
}

/// The time-stamp counter.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn rdtsc() -> u64 {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::_rdtsc;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::_rdtsc;
    // SAFETY: rdtsc is unprivileged on every OS we target.
    unsafe { _rdtsc() }
}

/// `iters` loops of 16 dependent register+register adds; the counter and
/// branch run in parallel, so it takes 16 × `iters` cycles. Register operands
/// matter: Intel Golden Cove and later fold chains of `add reg, imm` at
/// rename, which would read as ~20 GHz.
#[inline(never)]
pub fn add_chain(iters: u64) {
    #[cfg(target_arch = "x86_64")]
    // SAFETY: registers only (nomem, nostack); every operand is declared.
    unsafe {
        std::arch::asm!(
            "2:",
            "add {x}, {y}", "add {x}, {y}", "add {x}, {y}", "add {x}, {y}",
            "add {x}, {y}", "add {x}, {y}", "add {x}, {y}", "add {x}, {y}",
            "add {x}, {y}", "add {x}, {y}", "add {x}, {y}", "add {x}, {y}",
            "add {x}, {y}", "add {x}, {y}", "add {x}, {y}", "add {x}, {y}",
            "dec {n}",
            "jnz 2b",
            x = inout(reg) 0u64 => _,
            y = in(reg) std::hint::black_box(1u64),
            n = inout(reg) iters => _,
            options(nomem, nostack),
        );
    }
    #[cfg(target_arch = "aarch64")]
    // SAFETY: registers only (nomem, nostack); every operand is declared.
    unsafe {
        std::arch::asm!(
            "2:",
            "add {x}, {x}, {y}", "add {x}, {x}, {y}", "add {x}, {x}, {y}", "add {x}, {x}, {y}",
            "add {x}, {x}, {y}", "add {x}, {x}, {y}", "add {x}, {x}, {y}", "add {x}, {x}, {y}",
            "add {x}, {x}, {y}", "add {x}, {x}, {y}", "add {x}, {x}, {y}", "add {x}, {x}, {y}",
            "add {x}, {x}, {y}", "add {x}, {x}, {y}", "add {x}, {x}, {y}", "add {x}, {x}, {y}",
            "subs {n}, {n}, #1",
            "b.ne 2b",
            x = inout(reg) 0u64 => _,
            y = in(reg) std::hint::black_box(1u64),
            n = inout(reg) iters => _,
            options(nomem, nostack),
        );
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let mut x = 0u64;
        for _ in 0..iters * 16 {
            x = std::hint::black_box(x + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_chain_runs_and_counters_tick() {
        super::add_chain(1000);
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            let a = super::rdtsc();
            super::add_chain(1000);
            assert!(super::rdtsc() > a);
        }
        #[cfg(windows)]
        assert!(super::qpc_hz().is_some_and(|f| f > 0.0));
    }
}

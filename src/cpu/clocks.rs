//! Core clock measurement. Best source first:
//! 1. APERF/MPERF MSRs (needs the privileged backend), the same counters CPU-Z reads.
//! 2. The OS's current-frequency report (Linux cpufreq).
//! 3. A timed chain of dependent integer adds: one add retires per cycle on
//!    every x86 and ARM core, so adds/second = cycles/second.

use crate::Ctx;
use std::time::{Duration, Instant};

const MSR_MPERF: u32 = 0xE7;
const MSR_APERF: u32 = 0xE8;

#[derive(Debug, Clone, Copy)]
pub enum Source {
    AperfMperf,
    Os,
    AddChain,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::AperfMperf => "APERF/MPERF",
            Source::Os => "OS",
            Source::AddChain => "measured",
        }
    }
}

/// Invariant TSC rate in Hz (x86), measured against the OS monotonic clock.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn tsc_hz() -> Option<f64> {
    use std::sync::OnceLock;
    static HZ: OnceLock<f64> = OnceLock::new();
    Some(*HZ.get_or_init(|| {
        #[cfg(target_arch = "x86")]
        use core::arch::x86::_rdtsc;
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::_rdtsc;
        let t0 = Instant::now();
        // SAFETY: rdtsc is unprivileged on every OS we target.
        let c0 = unsafe { _rdtsc() };
        while t0.elapsed() < Duration::from_millis(60) {}
        let c1 = unsafe { _rdtsc() };
        (c1 - c0) as f64 / t0.elapsed().as_secs_f64()
    }))
}
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
pub fn tsc_hz() -> Option<f64> {
    None
}

/// Effective MHz of logical CPU `cpu` while it is busy.
pub fn core_mhz(ctx: &Ctx, cpu: usize) -> Option<(f64, Source)> {
    if let Some(m) = aperf_mperf(ctx, cpu) {
        return Some((m, Source::AperfMperf));
    }
    #[cfg(target_os = "linux")]
    if let Some(k) = crate::os::read_u64(format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_cur_freq"))
        && k > 0
    {
        return Some((k as f64 / 1000.0, Source::Os));
    }
    let run = move || add_chain_mhz(Duration::from_millis(80));
    crate::os::on_cpu(cpu, run).or_else(|| (cpu == 0).then(run)).map(|m| (m, Source::AddChain))
}

/// (APERF, MPERF) MSR numbers. AMD also mirrors them read-only at C00000E8/E7,
/// and some PawnIO allow-lists only permit those.
fn aperf_mperf_regs(ctx: &Ctx) -> (u32, u32) {
    static REGS: std::sync::OnceLock<(u32, u32)> = std::sync::OnceLock::new();
    *REGS.get_or_init(|| {
        if ctx.hw.rdmsr(0, MSR_APERF).is_some() { (MSR_APERF, MSR_MPERF) } else { (0xC000_00E8, 0xC000_00E7) }
    })
}

/// Clock `cpu` actually ran at since the previous call for it: TSC rate ×
/// ΔAPERF/ΔMPERF. Both counters only advance while the core is awake (C0), so
/// this adds no load of its own, unlike `core_mhz`, which keeps the core busy
/// to report its loaded clock. Used by the live sampler. None on the first call
/// for a CPU, when the core slept for the whole interval, or without MSR access.
pub fn active_mhz(ctx: &Ctx, cpu: usize) -> Option<f64> {
    use std::sync::Mutex;
    static LAST: Mutex<Vec<Option<(u64, u64)>>> = Mutex::new(Vec::new());
    let tsc = tsc_hz()?;
    let (ra, rm) = aperf_mperf_regs(ctx);
    let (a, m) = (ctx.hw.rdmsr(cpu, ra)?, ctx.hw.rdmsr(cpu, rm)?);
    let prev = {
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        if last.len() <= cpu {
            last.resize(cpu + 1, None);
        }
        last[cpu].replace((a, m))
    };
    let (a0, m0) = prev?;
    active_from_deltas(tsc, a.wrapping_sub(a0), m.wrapping_sub(m0))
}

/// MHz from counter deltas; below ~1 µs of MPERF the core was asleep throughout.
fn active_from_deltas(tsc_hz: f64, d_aperf: u64, d_mperf: u64) -> Option<f64> {
    (d_mperf as f64 > tsc_hz * 1e-6).then(|| tsc_hz * d_aperf as f64 / d_mperf as f64 / 1e6)
}

fn aperf_mperf(ctx: &Ctx, cpu: usize) -> Option<f64> {
    let tsc = tsc_hz()?;
    let hw = &ctx.hw;
    let (ra, rm) = aperf_mperf_regs(ctx);
    let a0 = hw.rdmsr(cpu, ra)?;
    let m0 = hw.rdmsr(cpu, rm)?;
    // Keep that core busy while sampling, else we measure an idle core.
    crate::os::on_cpu(cpu, || {
        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(30) {
            std::hint::spin_loop();
        }
    });
    let a1 = hw.rdmsr(cpu, ra)?;
    let m1 = hw.rdmsr(cpu, rm)?;
    let (da, dm) = (a1.wrapping_sub(a0), m1.wrapping_sub(m0));
    (dm > 0).then(|| tsc * da as f64 / dm as f64 / 1e6)
}

/// MHz of `cpu` while it alone is loaded, after `warm` of load so the boost
/// clock has settled (CPU-Z's "max ratio (effective)"). APERF/MPERF only.
pub fn loaded_mhz(ctx: &Ctx, cpu: usize, warm: Duration) -> Option<f64> {
    use std::sync::atomic::{AtomicBool, Ordering};
    let tsc = tsc_hz()?;
    let hw = &ctx.hw;
    let (ra, rm) = if hw.rdmsr(cpu, MSR_APERF).is_some() { (MSR_APERF, MSR_MPERF) } else { (0xC000_00E8, 0xC000_00E7) };
    hw.rdmsr(cpu, rm)?;
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| {
        s.spawn(|| {
            if crate::os::pin(cpu) {
                while !stop.load(Ordering::Relaxed) {
                    std::hint::spin_loop();
                }
            }
        });
        std::thread::sleep(warm);
        let a0 = hw.rdmsr(cpu, ra);
        let m0 = hw.rdmsr(cpu, rm);
        std::thread::sleep(Duration::from_millis(50));
        let (a1, m1) = (hw.rdmsr(cpu, ra), hw.rdmsr(cpu, rm));
        stop.store(true, Ordering::Relaxed);
        let (da, dm) = (a1?.wrapping_sub(a0?), m1?.wrapping_sub(m0?));
        (dm > 0).then(|| tsc * da as f64 / dm as f64 / 1e6)
    })
}

/// Run on the calling thread. Warms up so boost clocks have ramped.
pub fn add_chain_mhz(span: Duration) -> f64 {
    let _ = chain(Duration::from_millis(40));
    chain(span)
}

fn chain(span: Duration) -> f64 {
    const ITERS: u64 = 1 << 20;
    const ADDS: u64 = 16;
    let t = Instant::now();
    let mut n = 0u64;
    while t.elapsed() < span {
        adds(ITERS);
        n += ITERS * ADDS;
    }
    n as f64 / t.elapsed().as_secs_f64() / 1e6
}

#[inline(never)]
fn adds(iters: u64) {
    // 16 dependent register+register adds per iteration; the counter/branch
    // run in parallel. Register operands matter: Intel Golden Cove and later
    // fold chains of `add reg, imm` at rename, which would read as ~20 GHz.
    #[cfg(target_arch = "x86_64")]
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
    fn active_clock_from_deltas() {
        let tsc = 4.7e9;
        // Ran at 5.2 GHz while awake, however long it slept in between.
        let m = super::active_from_deltas(tsc, 5_200_000, 4_700_000).unwrap();
        assert!((m - 5200.0).abs() < 1e-6, "{m}");
        // Asleep for the whole interval: no reading rather than a made-up one.
        assert_eq!(super::active_from_deltas(tsc, 10, 100), None);
    }

    #[test]
    fn plausible_clock() {
        let m = super::add_chain_mhz(std::time::Duration::from_millis(50));
        assert!((300.0..10_000.0).contains(&m), "{m} MHz");
    }
}

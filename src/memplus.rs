//! More memory tests (S1-K04): STREAM triad, GUPS (random updates), the
//! huge-page effect on a random walk, and DRAM latency from each CCD / NUMA
//! node. Our own method, like the rest of the memory benchmark.
//!
//! - STREAM triad: `a[i] = b[i] + s·c[i]` over three arrays far larger than
//!   the caches; GB/s counts 24 bytes per element (STREAM's convention, no
//!   write-allocate traffic).
//! - GUPS: `t[r & mask] ^= r` with random r over a large table; billions of
//!   updates per second. Threads race on purpose (as HPCC allows), through
//!   relaxed atomics so it stays defined behaviour.
//! - Huge pages: one random cycle through every 64-byte line of a 512 MiB
//!   region, first on 4 KiB pages, then on 2 MiB pages; the difference is
//!   the cost of TLB misses.
//! - Per CCD/NUMA: the 4 KiB-page walk from the first core of each CCD (or
//!   node), so a far node or an uneven fabric shows.

use crate::benchkit::{Ctl, Sub};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const LINE: usize = 64;

pub struct Plan {
    pub cpus: Vec<usize>,
    /// (label, logical CPU) per CCD / NUMA node; fewer than 2 = skipped.
    pub domains: Vec<(String, usize)>,
    /// Upper bound for any one buffer (¼ of RAM).
    pub max_bytes: usize,
    pub secs: f64,
}

fn xorshift(x: &mut u64) -> u64 {
    *x ^= *x << 13;
    *x ^= *x >> 7;
    *x ^= *x << 17;
    *x
}

fn pin(cpu: Option<usize>) {
    if let Some(c) = cpu {
        let _ = crate::os::pin(c);
    }
}

/// STREAM triad GB/s with one worker per entry of `cpus`.
pub fn triad(cpus: &[Option<usize>], elems: usize, secs: f64, ctl: &Ctl) -> f64 {
    let mut a = vec![0f64; elems];
    let b = vec![1f64; elems];
    let c = vec![2f64; elems];
    let chunk = elems.div_ceil(cpus.len().max(1));
    let total: f64 = std::thread::scope(|s| {
        let hs: Vec<_> = a
            .chunks_mut(chunk)
            .zip(b.chunks(chunk).zip(c.chunks(chunk)))
            .zip(cpus)
            .map(|((a, (b, c)), &cpu)| {
                s.spawn(move || {
                    pin(cpu);
                    let pass = |a: &mut [f64]| {
                        for ((x, y), z) in a.iter_mut().zip(b).zip(c) {
                            *x = y + 3.0 * z;
                        }
                    };
                    pass(a); // first touch + warm
                    let (t0, mut n) = (Instant::now(), 0usize);
                    while t0.elapsed().as_secs_f64() < secs && !ctl.cancelled() {
                        pass(a);
                        n += a.len();
                    }
                    std::hint::black_box(&a);
                    (n * 24) as f64 / t0.elapsed().as_secs_f64().max(1e-9)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap_or(0.0)).sum()
    });
    total / 1e9
}

/// Giga-updates per second over a table of `entries` (a power of two).
pub fn gups(cpus: &[Option<usize>], entries: usize, secs: f64, ctl: &Ctl) -> f64 {
    let t: Vec<AtomicU64> = (0..entries as u64).map(AtomicU64::new).collect();
    let mask = entries - 1;
    let total: f64 = std::thread::scope(|s| {
        let hs: Vec<_> = cpus
            .iter()
            .enumerate()
            .map(|(k, &cpu)| {
                let t = &t;
                s.spawn(move || {
                    pin(cpu);
                    let mut r = 0x9E37_79B9_7F4A_7C15u64 ^ (k as u64 + 1);
                    let (t0, mut n) = (Instant::now(), 0u64);
                    while t0.elapsed().as_secs_f64() < secs && !ctl.cancelled() {
                        for _ in 0..4096 {
                            let v = xorshift(&mut r);
                            let e = &t[v as usize & mask];
                            e.store(e.load(Ordering::Relaxed) ^ v, Ordering::Relaxed);
                        }
                        n += 4096;
                    }
                    n as f64 / t0.elapsed().as_secs_f64().max(1e-9)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap_or(0.0)).sum()
    });
    total / 1e9
}

/// Links every 64-byte line of `buf` into one random cycle (Sattolo);
/// returns the first line's address.
fn chain(buf: &mut [u8], seed: u64) -> usize {
    let n = buf.len() / LINE;
    let base = buf.as_ptr() as usize;
    let mut p: Vec<u32> = (0..n as u32).collect();
    let mut r = seed | 1;
    for i in (1..n).rev() {
        p.swap(i, (xorshift(&mut r) % i as u64) as usize);
    }
    for (i, &next) in p.iter().enumerate() {
        buf[i * LINE..i * LINE + 8].copy_from_slice(&((base + next as usize * LINE) as u64).to_le_bytes());
    }
    base
}

/// ns per dependent load walking the cycle from `start` for `secs`.
///
/// SAFETY: `start` must be a chain built by `chain` whose buffer is alive.
unsafe fn walk(start: usize, secs: f64) -> f64 {
    let mut p = start as *const u64;
    let step = |mut p: *const u64, n: usize| {
        for _ in 0..n {
            // SAFETY: every word 0 of the chain holds the next line's address.
            p = unsafe { p.read_volatile() } as *const u64;
        }
        p
    };
    p = step(p, 1 << 16); // warm
    let (t0, mut n) = (Instant::now(), 0usize);
    while t0.elapsed().as_secs_f64() < secs {
        p = step(p, 1 << 16);
        n += 1 << 16;
    }
    std::hint::black_box(p);
    t0.elapsed().as_nanos() as f64 / n as f64
}

/// Random-walk latency on a region with the chosen page size, from `cpu`.
pub fn latency(bytes: usize, large: bool, cpu: Option<usize>, secs: f64) -> Result<f64, String> {
    let mut r = cpuzeez_platform::large_pages::alloc(bytes, large)?;
    let start = chain(r.bytes(), 0xC0DE);
    let ns = std::thread::scope(|s| {
        s.spawn(|| {
            pin(cpu);
            // SAFETY: the chain lives in `r`, alive for this scope.
            unsafe { walk(start, secs) }
        })
        .join()
        .unwrap_or(0.0)
    });
    Ok(ns)
}

/// Every test; `notes` gets the reason for each one that couldn't run.
pub fn run(plan: &Plan, ctl: &Ctl, notes: &mut Vec<String>) -> Result<Vec<Sub>, String> {
    let all: Vec<Option<usize>> = plan.cpus.iter().map(|&c| Some(c)).collect();
    let one = &all[..1];
    let elems = (128usize << 20).min(plan.max_bytes / 3) / 8;
    let entries = (256usize << 20).min(plan.max_bytes).next_power_of_two() / 2 / 8;
    let walk_bytes = (512usize << 20).min(plan.max_bytes);
    let steps = 6.0 + plan.domains.len() as f32;
    let mut out = Vec::new();
    let mut step = 0.0;
    let mut stage = |s: &str| {
        ctl.set(s, step / steps);
        step += 1.0;
        ctl.check()
    };
    stage("STREAM triad")?;
    out.push(Sub::new("STREAM triad ST", triad(one, elems, plan.secs, ctl), "GB/s", true));
    stage("STREAM triad")?;
    out.push(Sub::new("STREAM triad MT", triad(&all, elems, plan.secs, ctl), "GB/s", true));
    stage("GUPS")?;
    out.push(Sub::new("GUPS ST", gups(one, entries, plan.secs, ctl), "GUPS", true));
    stage("GUPS")?;
    out.push(Sub::new("GUPS MT", gups(&all, entries, plan.secs, ctl), "GUPS", true));
    stage("4 KiB pages")?;
    let small = latency(walk_bytes, false, one[0], plan.secs)?;
    out.push(Sub::new("Random walk, 4 KiB pages", small, "ns", false));
    stage("Large pages")?;
    match latency(walk_bytes, true, one[0], plan.secs) {
        Ok(ns) => out.push(Sub::new("Random walk, large pages", ns, "ns", false)),
        Err(e) => notes.push(format!("Large pages: {e}")),
    }
    if plan.domains.len() > 1 {
        for (label, cpu) in &plan.domains {
            stage(label)?;
            out.push(Sub::new(
                &format!("Latency · {label}"),
                latency(walk_bytes / 2, false, Some(*cpu), plan.secs)?,
                "ns",
                false,
            ));
        }
    }
    ctl.check()?;
    Ok(out)
}

/// One run with the plan's tests, as a background job.
pub fn start(plan: Plan) -> crate::benchkit::Job<(Vec<Sub>, Vec<String>)> {
    crate::benchkit::Job::spawn(move |ctl| {
        let mut notes = Vec::new();
        run(&plan, ctl, &mut notes).map(|s| (s, notes))
    })
}

/// The first CPU of each CCD, else of each NUMA node (empty when uniform).
pub fn domains(ctx: &crate::Ctx) -> Vec<(String, usize)> {
    let c = crate::cpu::get(ctx);
    let by = |f: &dyn Fn(&crate::cpu::Logical) -> Option<u32>, name: &str| {
        let mut v: Vec<(String, usize)> = Vec::new();
        let mut seen = Vec::new();
        for l in &c.logical {
            if let Some(k) = f(l)
                && !seen.contains(&k)
            {
                seen.push(k);
                v.push((format!("{name} {k}"), l.cpu));
            }
        }
        v
    };
    let ccd = by(&|l| l.ccd, "CCD");
    if ccd.len() > 1 { ccd } else { by(&|l| l.node, "Node") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_is_one_cycle() {
        let mut buf = vec![0u8; 64 * 1000];
        let start = chain(&mut buf, 3);
        let base = buf.as_ptr() as usize;
        let (mut p, mut seen) = (start, std::collections::HashSet::new());
        for _ in 0..1000 {
            assert!(seen.insert(p), "revisited a line before the lap ended");
            let off = p - base;
            p = u64::from_le_bytes(buf[off..off + 8].try_into().unwrap()) as usize;
        }
        assert_eq!(p, start, "the walk closes after visiting every line once");
    }

    #[test]
    fn small_run() {
        let ctl = Ctl::default();
        let one = [None];
        assert!(triad(&one, 1 << 16, 0.05, &ctl) > 0.0);
        assert!(gups(&[None, None], 1 << 16, 0.05, &ctl) > 0.0);
        assert!(latency(4 << 20, false, None, 0.05).unwrap() > 0.0);
    }
}

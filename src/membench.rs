//! Memory and cache benchmark: latency by working-set size (pointer chasing,
//! 4 KiB … 1 GiB, reveals L1/L2/L3/DRAM) and bandwidth (read / write / copy) per
//! cache level and DRAM, single- and multi-threaded. Our own method, like the
//! CPU benchmark: results are not comparable with other tools' numbers.
//!
//! Method
//! - Latency: one 64-byte line per element, linked into a single random cycle
//!   (each line visited once per lap, so the reuse distance is the whole working
//!   set). Lines are shuffled within 2 MiB blocks, blocks chained in order, so
//!   each stretch of the walk stays inside the second-level TLB's reach and
//!   4 KiB pages don't turn every access into a page walk (no large pages).
//!   The lap is split in 4 phases by line index mod 4: neighbouring lines are
//!   visited a quarter-lap apart, so what the adjacent-line / next-line
//!   prefetchers bring in is evicted before use once the set exceeds the caches
//!   (without this, DRAM reads ~35% too fast on Zen 4). Reported as ns per
//!   dependent load, best of 8 samples per size.
//! - Bandwidth: per level, a buffer of ½ L1D, ½ L2, ½ L3 (divided by the threads
//!   sharing that cache in the all-threads run) and DRAM = max(8 × total L3,
//!   512 MiB) capped at ¼ of RAM. Read = 64-bit integer sum (auto-vectorised,
//!   AVX2 when available); write = plain stores for cache levels, non-temporal
//!   stores (AVX2 `vmovntdq` / NEON `stnp`) for DRAM when available; copy =
//!   memcpy from the first half of the buffer to the second, counted as bytes
//!   read + bytes written (STREAM's convention). GB/s = 10^9 bytes per second.
//!   One worker per logical CPU (pinned where the OS allows) for all-threads.

use crate::Ctx;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LatencyPoint {
    pub bytes: u64,
    pub ns: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bandwidth {
    /// "L1", "L2", "L3", "DRAM".
    pub level: String,
    pub threads: usize,
    pub read_gbs: f64,
    pub write_gbs: f64,
    pub copy_gbs: f64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MemResult {
    pub latency: Vec<LatencyPoint>,
    pub bandwidth: Vec<Bandwidth>,
    /// DRAM latency summary (ns) for history/CLI.
    pub dram_latency_ns: f64,
}

const LINE: usize = 64;
/// Shuffle granularity of the latency chain (see module doc).
const BLOCK_LINES: usize = (2 << 20) / LINE;
/// Pointer-chase steps between clock checks.
const CHASE_STEP: usize = 1 << 16;
/// Lines i, i+1, … are visited a quarter-lap apart (see module doc).
const PHASES: usize = 4;

#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct Line([u64; 8]);

fn alloc(bytes: usize) -> Vec<Line> {
    vec![Line([1; 8]); bytes.div_ceil(LINE).max(2)] // non-zero: every page really touched
}

// ---- latency ----

fn splitmix(s: &mut u64) -> u64 {
    *s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Link every line of `buf` into one cycle (word 0 = address of the next
/// line); returns the start address.
fn build_chain(buf: &mut [Line], seed: u64) -> *const u64 {
    let n = buf.len();
    let base = buf.as_ptr() as u64;
    let addr = |i: usize| base + (i * LINE) as u64;
    let mut rng = seed | 1;
    let (mut first, mut prev) = (None, None::<usize>);
    let mut perm = Vec::with_capacity(BLOCK_LINES.min(n));
    for phase in 0..PHASES {
        for start in (0..n).step_by(BLOCK_LINES) {
            perm.clear();
            perm.extend((start..(start + BLOCK_LINES).min(n)).filter(|i| i % PHASES == phase));
            for i in (1..perm.len()).rev() {
                perm.swap(i, (splitmix(&mut rng) % (i as u64 + 1)) as usize);
            }
            for &i in &perm {
                match prev {
                    Some(p) => buf[p].0[0] = addr(i),
                    None => first = Some(i),
                }
                prev = Some(i);
            }
        }
    }
    let (first, last) = (first.unwrap(), prev.unwrap());
    buf[last].0[0] = addr(first);
    addr(first) as *const u64
}

/// Follow the chain `n` steps.
///
/// SAFETY: `p` must point into a chain built by `build_chain` whose buffer is alive.
#[inline(never)]
unsafe fn chase(mut p: *const u64, n: usize) -> *const u64 {
    for _ in 0..n {
        p = unsafe { *p } as *const u64;
    }
    p
}

/// ns per access over the first `bytes` of `buf` (best of 8 samples, since
/// other processes only ever add time); None if cancelled.
fn latency(buf: &mut [Line], bytes: usize, secs: f64, stop: &AtomicBool) -> Option<f64> {
    let lines = (bytes / LINE).clamp(2, buf.len());
    let start = build_chain(&mut buf[..lines], bytes as u64);
    // SAFETY: chain just built over `buf`, which outlives this function body.
    let mut p = unsafe { chase(start, lines.min(1 << 20)) }; // warm caches / TLB
    let mut best = f64::MAX;
    for _ in 0..8 {
        let (t0, mut n) = (Instant::now(), 0usize);
        loop {
            p = unsafe { chase(p, CHASE_STEP) };
            n += CHASE_STEP;
            if stop.load(Ordering::Relaxed) {
                return None;
            }
            let e = t0.elapsed().as_secs_f64();
            if e >= secs / 8.0 {
                best = best.min(e * 1e9 / n as f64);
                break;
            }
        }
    }
    std::hint::black_box(p);
    Some(best)
}

// ---- bandwidth kernels ----

#[derive(Clone, Copy, PartialEq, Debug)]
enum Op {
    Read,
    Write,
    Copy,
}

const OPS: [Op; 3] = [Op::Read, Op::Write, Op::Copy];

impl Op {
    fn name(self) -> &'static str {
        match self {
            Op::Read => "read",
            Op::Write => "write",
            Op::Copy => "copy",
        }
    }
}

#[inline(always)]
fn sum_body(b: &[Line]) -> u64 {
    let mut acc = [0u64; 8];
    for l in b {
        for (a, &x) in acc.iter_mut().zip(&l.0) {
            *a = a.wrapping_add(x);
        }
    }
    acc.iter().fold(0, |a, &x| a.wrapping_add(x))
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn sum_avx2(b: &[Line]) -> u64 {
    sum_body(b)
}

fn has_avx2() -> bool {
    #[cfg(target_arch = "x86_64")]
    return is_x86_feature_detected!("avx2");
    #[allow(unreachable_code)]
    false
}

fn sum(b: &[Line]) -> u64 {
    #[cfg(target_arch = "x86_64")]
    if has_avx2() {
        // SAFETY: AVX2 checked at runtime.
        return unsafe { sum_avx2(b) };
    }
    sum_body(b)
}

/// Non-temporal stores are available (and used for the DRAM write test).
fn nt_available() -> bool {
    cfg!(target_arch = "aarch64") || has_avx2()
}

/// Label of the write kernel used for DRAM.
pub fn variant() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "NEON"
    } else if has_avx2() {
        "AVX2"
    } else {
        "Scalar"
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn fill_nt_avx2(b: &mut [Line], v: u64) {
    use std::arch::x86_64::*;
    let x = _mm256_set1_epi64x(v as i64);
    for l in b {
        let p = l.0.as_mut_ptr().cast::<__m256i>();
        // SAFETY: Line is 64-byte aligned, so both 32-byte halves are aligned.
        unsafe {
            _mm256_stream_si256(p, x);
            _mm256_stream_si256(p.add(1), x);
        }
    }
    _mm_sfence();
}

#[cfg(target_arch = "aarch64")]
fn fill_nt_neon(b: &mut [Line], v: u64) {
    for l in b {
        // SAFETY: stores 64 bytes into this Line only.
        unsafe {
            std::arch::asm!(
                "dup v0.2d, {v}",
                "stnp q0, q0, [{p}]",
                "stnp q0, q0, [{p}, #32]",
                p = in(reg) l.0.as_mut_ptr(),
                v = in(reg) v,
                out("v0") _,
                options(nostack, preserves_flags),
            );
        }
    }
}

fn fill(b: &mut [Line], v: u64, nt: bool) {
    if nt {
        #[cfg(target_arch = "x86_64")]
        if has_avx2() {
            // SAFETY: AVX2 checked at runtime.
            return unsafe { fill_nt_avx2(b, v) };
        }
        #[cfg(target_arch = "aarch64")]
        return fill_nt_neon(b, v);
    }
    b.fill(Line([v; 8]));
}

/// One pass of `op`; returns bytes moved.
fn pass(op: Op, b: &mut [Line], nt: bool, iter: u64) -> usize {
    match op {
        Op::Read => {
            std::hint::black_box(sum(b));
            b.len() * LINE
        }
        Op::Write => {
            fill(b, iter, nt);
            b.len() * LINE
        }
        Op::Copy => {
            let (src, dst) = b.split_at_mut(b.len() / 2);
            dst[..src.len()].copy_from_slice(src);
            2 * src.len() * LINE
        }
    }
}

/// GB/s of each op over `bytes` per thread, one thread per entry of `cpus`.
fn bandwidth(
    cpus: &[usize],
    bytes: usize,
    secs: f64,
    nt: bool,
    stop: &AtomicBool,
    on_op: &(dyn Fn(Op) + Sync),
) -> Option<[f64; 3]> {
    let barrier = Barrier::new(cpus.len());
    let reps = ((1usize << 20) / bytes.max(1)).max(1); // passes between clock checks
    // Single thread: best of 5 windows (other processes only slow it down). All
    // threads: one window, so per-thread bests from different moments aren't summed.
    let samples = if cpus.len() == 1 { 5 } else { 1 };
    let per_thread: Vec<[f64; 3]> = std::thread::scope(|s| {
        let hs: Vec<_> = cpus
            .iter()
            .enumerate()
            .map(|(t, &cpu)| {
                let barrier = &barrier;
                s.spawn(move || {
                    let _ = crate::os::pin(cpu);
                    let mut buf = alloc(bytes);
                    let mut out = [0.0; 3];
                    for (k, &op) in OPS.iter().enumerate() {
                        if t == 0 {
                            on_op(op);
                        }
                        pass(op, &mut buf, nt, 0); // warm
                        barrier.wait();
                        let mut it = 1u64;
                        for _ in 0..samples {
                            let (t0, mut moved) = (Instant::now(), 0usize);
                            while !stop.load(Ordering::Relaxed) {
                                for _ in 0..reps {
                                    moved += pass(op, &mut buf, nt, it);
                                    it += 1;
                                }
                                if t0.elapsed().as_secs_f64() >= secs / samples as f64 {
                                    break;
                                }
                            }
                            out[k] = f64::max(out[k], moved as f64 / t0.elapsed().as_secs_f64() / 1e9);
                        }
                        barrier.wait();
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    if stop.load(Ordering::Relaxed) {
        return None;
    }
    Some(per_thread.iter().fold([0.0; 3], |a, r| [a[0] + r[0], a[1] + r[1], a[2] + r[2]]))
}

// ---- plan ----

struct Level {
    name: &'static str,
    st_bytes: usize,
    /// Per thread in the all-threads run.
    mt_bytes: usize,
}

struct Plan {
    lat_sizes: Vec<usize>,
    lat_secs: f64,
    levels: Vec<Level>,
    bw_secs: f64,
    cpus: Vec<usize>,
}

impl Plan {
    fn steps(&self) -> usize {
        self.lat_sizes.len() + self.levels.len() * 2 * OPS.len()
    }
}

fn ram_bytes(ctx: &Ctx) -> u64 {
    let mb = crate::chipset::summary(ctx).memory.size_mb;
    let mb = if mb > 0 { mb } else { crate::smbios::memory_devices(ctx).iter().map(|d| d.size_mb).sum() };
    if mb > 0 { mb << 20 } else { 2 << 30 } // unknown: assume 2 GiB
}

fn plan(ctx: &Ctx, quick: bool) -> Plan {
    let cpu = crate::cpu::get(ctx);
    let quarter = (ram_bytes(ctx) / 4) as usize;
    let max_lat = (if quick { 256usize << 20 } else { 1 << 30 }).min(quarter).max(64 << 10);
    let mut lat_sizes = Vec::new();
    let mut s = 4usize << 10;
    while s <= max_lat {
        lat_sizes.push(s);
        if !quick && s + s / 2 <= max_lat {
            lat_sizes.push(s + s / 2);
        }
        s *= 2;
    }
    let cache = |level: u32| {
        cpu.caches.iter().filter(|c| c.level == level && c.kind != 'I' && c.size > 0).max_by_key(|c| c.size)
    };
    let threads = crate::os::logical_cpus().max(1);
    let mut levels = Vec::new();
    for (lvl, name) in [(1, "L1"), (2, "L2"), (3, "L3")] {
        if let Some(c) = cache(lvl) {
            let half = (c.size / 2) as usize;
            let share = (c.sharing as usize).clamp(1, threads);
            levels.push(Level { name, st_bytes: half, mt_bytes: (half / share).max(4 << 10) });
        }
    }
    let l3_total = cache(3).map_or(0, |c| c.size as usize * c.count.max(1));
    let dram = (8 * l3_total).max(512 << 20).min(quarter).max(64 << 20);
    levels.push(Level { name: "DRAM", st_bytes: dram, mt_bytes: (dram / threads).max(16 << 20) });
    Plan {
        lat_sizes,
        lat_secs: if quick { 0.08 } else { 0.3 },
        levels,
        bw_secs: if quick { 0.25 } else { 0.9 },
        cpus: (0..threads).collect(),
    }
}

pub fn size_label(b: u64) -> String {
    let (v, u) = match b {
        b if b >= 1 << 30 => (b as f64 / (1u64 << 30) as f64, "GiB"),
        b if b >= 1 << 20 => (b as f64 / (1u64 << 20) as f64, "MiB"),
        b => (b as f64 / 1024.0, "KiB"),
    };
    if v.fract() == 0.0 { format!("{v:.0} {u}") } else { format!("{v:.1} {u}") }
}

// ---- runner ----

struct Shared {
    stop: AtomicBool,
    done: AtomicUsize,
    total: usize,
    stage: Mutex<String>,
    result: Mutex<Option<MemResult>>,
    finished: AtomicBool,
}

impl Shared {
    fn set_stage(&self, s: String) {
        *self.stage.lock().unwrap() = s;
    }
    fn step(&self) {
        self.done.fetch_add(1, Ordering::Relaxed);
    }
}

fn run(p: &Plan, sh: &Shared) -> Option<MemResult> {
    let stop = &sh.stop;
    let mut r = MemResult::default();
    std::thread::scope(|s| {
        // Latency on its own pinned thread, so the runner (whose affinity Linux
        // threads inherit) stays unpinned.
        s.spawn(|| {
            let _ = crate::os::pin(p.cpus[0]);
            let mut buf = alloc(*p.lat_sizes.last()?);
            for &b in &p.lat_sizes {
                sh.set_stage(format!("Latency {}", size_label(b as u64)));
                let ns = latency(&mut buf, b, p.lat_secs, stop)?;
                r.latency.push(LatencyPoint { bytes: b as u64, ns });
                sh.step();
            }
            Some(())
        })
        .join()
        .ok()
        .flatten()
    })?;
    let nt_ok = nt_available();
    for lvl in &p.levels {
        let nt = nt_ok && lvl.name == "DRAM";
        for (cpus, bytes) in [(&p.cpus[..1], lvl.st_bytes), (&p.cpus[..], lvl.mt_bytes)] {
            let n = cpus.len();
            let on_op = |op: Op| {
                let t = if n == 1 { "1 thread".to_string() } else { format!("{n} threads") };
                sh.set_stage(format!("Bandwidth {} {}, {t}", lvl.name, op.name()));
                sh.step();
            };
            let [read_gbs, write_gbs, copy_gbs] = bandwidth(cpus, bytes, p.bw_secs, nt, stop, &on_op)?;
            r.bandwidth.push(Bandwidth { level: lvl.name.into(), threads: n, read_gbs, write_gbs, copy_gbs });
        }
    }
    r.dram_latency_ns = r.latency.last().map_or(0.0, |l| l.ns);
    Some(r)
}

/// A run on background threads (GUI).
pub struct MemBench {
    shared: Arc<Shared>,
}

impl MemBench {
    pub fn start(ctx: &Ctx, quick: bool) -> MemBench {
        let p = plan(ctx, quick);
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            done: AtomicUsize::new(0),
            total: p.steps().max(1),
            stage: Mutex::new(String::new()),
            result: Mutex::new(None),
            finished: AtomicBool::new(false),
        });
        let sh = shared.clone();
        std::thread::spawn(move || {
            let r = run(&p, &sh);
            if !sh.stop.load(Ordering::Relaxed) {
                *sh.result.lock().unwrap() = r;
            }
            sh.set_stage(String::new());
            sh.finished.store(true, Ordering::Release);
        });
        MemBench { shared }
    }
    /// 0.0 ..= 1.0
    pub fn progress(&self) -> f32 {
        if self.finished() {
            return 1.0;
        }
        (self.shared.done.load(Ordering::Relaxed) as f32 / self.shared.total as f32).min(0.99)
    }
    /// What is being measured now ("Latency 32 MiB", "Bandwidth L2 copy"…).
    pub fn stage(&self) -> String {
        self.shared.stage.lock().unwrap().clone()
    }
    pub fn result(&self) -> Option<MemResult> {
        self.shared.result.lock().unwrap().clone()
    }
    pub fn finished(&self) -> bool {
        self.shared.finished.load(Ordering::Acquire)
    }
    pub fn cancel(&self) {
        self.shared.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for MemBench {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// `-membench` (`-membench=quick` for the ~10 s run)
pub fn cli(ctx: &Ctx) {
    let quick = std::env::args().any(|a| a.to_ascii_lowercase().ends_with("membench=quick"));
    let cpu = crate::cpu::get(ctx);
    let b = MemBench::start(ctx, quick);
    let mut last = String::new();
    while !b.finished() {
        let s = b.stage();
        if s != last && !s.is_empty() {
            eprintln!("{:3.0}%  {s}", b.progress() * 100.0);
            last = s;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let Some(r) = b.result() else { return eprintln!("memory benchmark failed") };
    print!("{}", table(&cpu.name, &r));
    let mut e = crate::history::system_entry(ctx, "Memory", r.dram_latency_ns, "ns");
    e.threads = 1;
    e.variant = variant().into();
    e.label = if quick { "quick".into() } else { String::new() };
    e.details = serde_json::to_value(&r).unwrap_or_default();
    crate::history::add(e);
    eprintln!("saved to benchmark history");
}

fn table(cpu: &str, r: &MemResult) -> String {
    use std::fmt::Write;
    let mut o = format!(
        "{} {} memory benchmark\n(own method: results are not comparable with other tools)\n\nCPU\t\t\t{cpu}\n\n\
         Latency (random pointer chase)\n",
        crate::DISPLAY_NAME,
        crate::VERSION
    );
    for l in &r.latency {
        let _ = writeln!(o, "  {:>9}  {:8.2} ns", size_label(l.bytes), l.ns);
    }
    let _ = writeln!(o, "\nBandwidth (GB/s)        Read      Write       Copy");
    for b in &r.bandwidth {
        let t = if b.threads == 1 { "1 thread".to_string() } else { format!("{} threads", b.threads) };
        let _ = writeln!(o, "  {:<5}{t:<12}{:>10.1} {:>10.1} {:>10.1}", b.level, b.read_gbs, b.write_gbs, b.copy_gbs);
    }
    let _ = writeln!(o, "\nDRAM latency\t\t{:.1} ns", r.dram_latency_ns);
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_is_one_cycle_over_all_lines() {
        // Spans several shuffle blocks plus a partial one.
        for lines in [2, 3, 64, BLOCK_LINES * 2 + 77] {
            let mut buf = alloc(lines * LINE);
            let start = build_chain(&mut buf, 42);
            let base = buf.as_ptr() as usize;
            let mut seen = vec![false; lines];
            let mut p = start;
            for _ in 0..lines {
                let i = (p as usize - base) / LINE;
                assert!(!seen[i], "line {i} visited twice");
                seen[i] = true;
                p = unsafe { chase(p, 1) };
            }
            assert_eq!(p, start);
            assert!(seen.iter().all(|&s| s));
        }
    }

    #[test]
    fn kernels() {
        let mut b = alloc(64 * LINE);
        for (i, l) in b.iter_mut().enumerate() {
            l.0 = std::array::from_fn(|k| (i * 8 + k) as u64);
        }
        let n = 64 * 8u64;
        assert_eq!(sum(&b), n * (n - 1) / 2);
        assert_eq!(sum_body(&b), n * (n - 1) / 2);
        for nt in [false, true] {
            assert_eq!(pass(Op::Write, &mut b, nt, 3), 64 * LINE);
            assert_eq!(sum(&b), 3 * n);
        }
        b[..32].iter_mut().for_each(|l| l.0 = [5; 8]);
        assert_eq!(pass(Op::Copy, &mut b, false, 0), 64 * LINE);
        assert_eq!(sum(&b), 5 * n);
    }

    #[test]
    fn short_runs() {
        let stop = AtomicBool::new(false);
        let mut buf = alloc(64 << 10);
        assert!(latency(&mut buf, 32 << 10, 0.01, &stop).unwrap() > 0.0);
        let r = bandwidth(&[0, 1], 64 << 10, 0.01, false, &stop, &|_| {}).unwrap();
        assert!(r.iter().all(|&g| g > 0.0));
        stop.store(true, Ordering::Relaxed);
        assert!(latency(&mut buf, 32 << 10, 10.0, &stop).is_none());
        assert_eq!(size_label(4096), "4 KiB");
        assert_eq!(size_label(3 << 19), "1.5 MiB");
    }
}

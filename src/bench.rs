//! Bench tab, -bench and Stress CPU (FEATURES.md 10).
//!
//! This is cpuzeez's own benchmark. It shares nothing with CPU-Z's (neither
//! the algorithm nor the reference scores), so the numbers are NOT comparable
//! with CPU-Z's.
//!
//! Workload: 16 independent lanes, each step of each lane does
//! - a dependent load from a 64 KB pseudo-random table (cache-resident
//!   data-structure walk, L1/L2 latency bound),
//! - a 32-bit integer hash (murmur3's fmix32: multiplies, shifts, xors),
//! - FP64 math: a logistic-map step and a square root.
//!
//! Every variant (portable scalar code, AVX2, AVX-512, NEON) does the exact
//! same IEEE operations in the same order, so all produce the same checksum.
//! Score = lane-steps per second / `SCALE`, chosen so one ~5 GHz Zen 4 core
//! scores in the hundreds with the scalar variant.

#![cfg_attr(not(feature = "gui"), allow(dead_code))] // Bench tab API is for the GUI

use crate::Ctx;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

const LANES: usize = 16;
const TABLE_BITS: u32 = 14; // 16 K x u32 = 64 KB
const MASK: u32 = (1 << TABLE_BITS) - 1;
const CHUNK: u64 = 1 << 12; // steps between clock checks (well under 1 ms)
const R: f64 = 3.9; // logistic map in its chaotic regime: x stays in (0, 1)
const M1: u32 = 0x85eb_ca6b;
const M2: u32 = 0xc2b2_ae35;
/// Lane-steps per second per point.
pub const SCALE: f64 = 1e6;
pub const RUN: Duration = Duration::from_secs(2);
const WARMUP: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Scalar,
    Avx2,
    Avx512,
    Neon,
}

impl Variant {
    pub fn label(self) -> &'static str {
        match self {
            Variant::Scalar => "Scalar",
            Variant::Avx2 => "AVX2",
            Variant::Avx512 => "AVX-512",
            Variant::Neon => "NEON",
        }
    }

    pub fn available(self) -> bool {
        match self {
            Variant::Scalar => true,
            #[cfg(target_arch = "x86_64")]
            Variant::Avx2 => is_x86_feature_detected!("avx2"),
            #[cfg(target_arch = "x86_64")]
            Variant::Avx512 => is_x86_feature_detected!("avx512f"),
            #[cfg(target_arch = "aarch64")]
            Variant::Neon => std::arch::is_aarch64_feature_detected!("neon"),
            #[allow(unreachable_patterns)]
            _ => false,
        }
    }

    /// Variants this CPU can run, scalar first.
    pub fn all() -> Vec<Variant> {
        [Variant::Scalar, Variant::Avx2, Variant::Avx512, Variant::Neon].into_iter().filter(|v| v.available()).collect()
    }

    /// Widest SIMD variant available.
    pub fn best() -> Variant {
        *Variant::all().last().unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Score {
    pub points: f64,
    pub threads: usize,
    pub variant: Variant,
    pub seconds: f64,
}

// ---- workload ----

#[derive(Clone, Copy)]
#[repr(C, align(64))]
struct State {
    h: [u32; LANES],
    idx: [u32; LANES],
    x: [f64; LANES],
    y: [f64; LANES],
}

impl State {
    fn new(seed: u32) -> State {
        let mut s = State { h: [0; LANES], idx: [0; LANES], x: [0.0; LANES], y: [0.0; LANES] };
        for l in 0..LANES {
            s.h[l] = fmix(seed ^ l as u32);
            s.idx[l] = s.h[l] & MASK;
            s.x[l] = 0.1 + l as f64 / 32.0;
        }
        s
    }

    fn checksum(&self) -> u64 {
        let mut c = 0u64;
        for l in 0..LANES {
            c = c.rotate_left(7) ^ (self.h[l] as u64) ^ ((self.idx[l] as u64) << 32);
            c = c.rotate_left(13) ^ self.x[l].to_bits() ^ self.y[l].to_bits().rotate_left(29);
        }
        c
    }
}

fn fmix(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(M1);
    h ^= h >> 13;
    h = h.wrapping_mul(M2);
    h ^ (h >> 16)
}

fn table() -> &'static [u32] {
    static T: OnceLock<Vec<u32>> = OnceLock::new();
    T.get_or_init(|| (0..1u32 << TABLE_BITS).map(|i| fmix(i.wrapping_mul(0x9E37_79B9) ^ 0x5bd1_e995)).collect())
}

/// Reference definition of one step; the SIMD versions mirror it lane-wise.
fn steps_scalar(s: &mut State, t: &[u32], n: u64) {
    for _ in 0..n {
        for l in 0..LANES {
            let i = t[((s.h[l] ^ s.idx[l]) & MASK) as usize];
            s.idx[l] = i;
            s.h[l] = fmix(s.h[l].wrapping_add(i));
            let x = s.x[l];
            s.x[l] = R * x * (1.0 - x);
            s.y[l] = s.y[l] * 0.5 + s.x[l].sqrt();
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
fn steps_avx2(s: &mut State, t: &[u32], n: u64) {
    use std::arch::x86_64::*;
    let ld = |p: &[u32]| unsafe { _mm256_loadu_si256(p.as_ptr().cast()) };
    let ldd = |p: &[f64]| unsafe { _mm256_loadu_pd(p.as_ptr()) };
    let mut h = [ld(&s.h[0..]), ld(&s.h[8..])];
    let mut idx = [ld(&s.idx[0..]), ld(&s.idx[8..])];
    let mut x = [ldd(&s.x[0..]), ldd(&s.x[4..]), ldd(&s.x[8..]), ldd(&s.x[12..])];
    let mut y = [ldd(&s.y[0..]), ldd(&s.y[4..]), ldd(&s.y[8..]), ldd(&s.y[12..])];
    let (mask, m1, m2) = (_mm256_set1_epi32(MASK as i32), _mm256_set1_epi32(M1 as i32), _mm256_set1_epi32(M2 as i32));
    let (r, one, half) = (_mm256_set1_pd(R), _mm256_set1_pd(1.0), _mm256_set1_pd(0.5));
    for _ in 0..n {
        for k in 0..2 {
            let o = _mm256_and_si256(_mm256_xor_si256(h[k], idx[k]), mask);
            // SAFETY: every offset is masked into the table.
            let i = unsafe { _mm256_i32gather_epi32::<4>(t.as_ptr().cast(), o) };
            idx[k] = i;
            let mut v = _mm256_add_epi32(h[k], i);
            v = _mm256_xor_si256(v, _mm256_srli_epi32::<16>(v));
            v = _mm256_mullo_epi32(v, m1);
            v = _mm256_xor_si256(v, _mm256_srli_epi32::<13>(v));
            v = _mm256_mullo_epi32(v, m2);
            h[k] = _mm256_xor_si256(v, _mm256_srli_epi32::<16>(v));
        }
        for k in 0..4 {
            x[k] = _mm256_mul_pd(_mm256_mul_pd(r, x[k]), _mm256_sub_pd(one, x[k]));
            y[k] = _mm256_add_pd(_mm256_mul_pd(y[k], half), _mm256_sqrt_pd(x[k]));
        }
    }
    unsafe {
        for k in 0..2 {
            _mm256_storeu_si256(s.h[k * 8..].as_mut_ptr().cast(), h[k]);
            _mm256_storeu_si256(s.idx[k * 8..].as_mut_ptr().cast(), idx[k]);
        }
        for k in 0..4 {
            _mm256_storeu_pd(s.x[k * 4..].as_mut_ptr(), x[k]);
            _mm256_storeu_pd(s.y[k * 4..].as_mut_ptr(), y[k]);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
fn steps_avx512(s: &mut State, t: &[u32], n: u64) {
    use std::arch::x86_64::*;
    let ldd = |p: &[f64]| unsafe { _mm512_loadu_pd(p.as_ptr()) };
    let mut h = unsafe { _mm512_loadu_si512(s.h.as_ptr().cast()) };
    let mut idx = unsafe { _mm512_loadu_si512(s.idx.as_ptr().cast()) };
    let mut x = [ldd(&s.x[0..]), ldd(&s.x[8..])];
    let mut y = [ldd(&s.y[0..]), ldd(&s.y[8..])];
    let (mask, m1, m2) = (_mm512_set1_epi32(MASK as i32), _mm512_set1_epi32(M1 as i32), _mm512_set1_epi32(M2 as i32));
    let (r, one, half) = (_mm512_set1_pd(R), _mm512_set1_pd(1.0), _mm512_set1_pd(0.5));
    for _ in 0..n {
        let o = _mm512_and_si512(_mm512_xor_si512(h, idx), mask);
        // SAFETY: every offset is masked into the table.
        let i = unsafe { _mm512_i32gather_epi32::<4>(o, t.as_ptr().cast()) };
        idx = i;
        let mut v = _mm512_add_epi32(h, i);
        v = _mm512_xor_si512(v, _mm512_srli_epi32::<16>(v));
        v = _mm512_mullo_epi32(v, m1);
        v = _mm512_xor_si512(v, _mm512_srli_epi32::<13>(v));
        v = _mm512_mullo_epi32(v, m2);
        h = _mm512_xor_si512(v, _mm512_srli_epi32::<16>(v));
        for k in 0..2 {
            x[k] = _mm512_mul_pd(_mm512_mul_pd(r, x[k]), _mm512_sub_pd(one, x[k]));
            y[k] = _mm512_add_pd(_mm512_mul_pd(y[k], half), _mm512_sqrt_pd(x[k]));
        }
    }
    unsafe {
        _mm512_storeu_si512(s.h.as_mut_ptr().cast(), h);
        _mm512_storeu_si512(s.idx.as_mut_ptr().cast(), idx);
        for k in 0..2 {
            _mm512_storeu_pd(s.x[k * 8..].as_mut_ptr(), x[k]);
            _mm512_storeu_pd(s.y[k * 8..].as_mut_ptr(), y[k]);
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
fn steps_neon(s: &mut State, t: &[u32], n: u64) {
    use std::arch::aarch64::*;
    // SAFETY (whole fn): loads/stores stay inside the State arrays; NEON has
    // no gather, so the table walk is done lane by lane through a spill slot.
    unsafe {
        let mut h: [uint32x4_t; 4] = std::array::from_fn(|k| vld1q_u32(s.h[k * 4..].as_ptr()));
        let mut idx: [uint32x4_t; 4] = std::array::from_fn(|k| vld1q_u32(s.idx[k * 4..].as_ptr()));
        let mut x: [float64x2_t; 8] = std::array::from_fn(|k| vld1q_f64(s.x[k * 2..].as_ptr()));
        let mut y: [float64x2_t; 8] = std::array::from_fn(|k| vld1q_f64(s.y[k * 2..].as_ptr()));
        let (mask, m1, m2) = (vdupq_n_u32(MASK), vdupq_n_u32(M1), vdupq_n_u32(M2));
        let (r, one, half) = (vdupq_n_f64(R), vdupq_n_f64(1.0), vdupq_n_f64(0.5));
        let mut off = [0u32; 4];
        for _ in 0..n {
            for k in 0..4 {
                vst1q_u32(off.as_mut_ptr(), vandq_u32(veorq_u32(h[k], idx[k]), mask));
                let g = [t[off[0] as usize], t[off[1] as usize], t[off[2] as usize], t[off[3] as usize]];
                let i = vld1q_u32(g.as_ptr());
                idx[k] = i;
                let mut v = vaddq_u32(h[k], i);
                v = veorq_u32(v, vshrq_n_u32::<16>(v));
                v = vmulq_u32(v, m1);
                v = veorq_u32(v, vshrq_n_u32::<13>(v));
                v = vmulq_u32(v, m2);
                h[k] = veorq_u32(v, vshrq_n_u32::<16>(v));
            }
            for k in 0..8 {
                x[k] = vmulq_f64(vmulq_f64(r, x[k]), vsubq_f64(one, x[k]));
                y[k] = vaddq_f64(vmulq_f64(y[k], half), vsqrtq_f64(x[k]));
            }
        }
        for k in 0..4 {
            vst1q_u32(s.h[k * 4..].as_mut_ptr(), h[k]);
            vst1q_u32(s.idx[k * 4..].as_mut_ptr(), idx[k]);
        }
        for k in 0..8 {
            vst1q_f64(s.x[k * 2..].as_mut_ptr(), x[k]);
            vst1q_f64(s.y[k * 2..].as_mut_ptr(), y[k]);
        }
    }
}

fn steps(v: Variant, s: &mut State, t: &[u32], n: u64) {
    assert!(v.available() && t.len() == 1 << TABLE_BITS);
    match v {
        // SAFETY: availability (CPU feature) checked above.
        #[cfg(target_arch = "x86_64")]
        Variant::Avx2 => unsafe { steps_avx2(s, t, n) },
        #[cfg(target_arch = "x86_64")]
        Variant::Avx512 => unsafe { steps_avx512(s, t, n) },
        #[cfg(target_arch = "aarch64")]
        Variant::Neon => unsafe { steps_neon(s, t, n) },
        _ => steps_scalar(s, t, n),
    }
}

/// Checksum after `n` steps from seed 1: identical for every variant.
#[cfg(test)]
fn run_fixed(v: Variant, n: u64) -> u64 {
    let mut s = State::new(1);
    steps(v, &mut s, table(), n);
    s.checksum()
}

// ---- runner ----

struct Shared {
    stop: AtomicBool,
    start: Instant,
    total: Duration,
    result: Mutex<Option<Score>>,
    finished: AtomicBool,
}

/// One timed run on background threads (GUI Bench tab).
pub struct Bench {
    shared: Arc<Shared>,
}

/// Lane-steps per second of one worker: warm up, then count whole chunks
/// until `end` or `stop`.
fn worker(cpu: Option<usize>, v: Variant, warm_end: Instant, end: Instant, stop: &AtomicBool) -> f64 {
    if let Some(c) = cpu {
        let _ = crate::os::pin(c); // best effort; macOS can't pin
    }
    let t = table();
    let mut s = State::new(cpu.unwrap_or(0) as u32 + 7);
    while Instant::now() < warm_end && !stop.load(Ordering::Relaxed) {
        steps(v, &mut s, t, CHUNK);
    }
    let (t0, mut n) = (Instant::now(), 0u64);
    while Instant::now() < end && !stop.load(Ordering::Relaxed) {
        steps(v, &mut s, t, CHUNK);
        n += CHUNK;
    }
    std::hint::black_box(s.checksum());
    n as f64 * LANES as f64 / t0.elapsed().as_secs_f64().max(1e-9)
}

impl Bench {
    /// `threads` unpinned workers.
    pub fn start(threads: usize, variant: Variant) -> Self {
        Self::spawn(vec![None; threads.max(1)], variant, RUN)
    }

    /// One worker pinned to each logical CPU in `cpus` (coremask / one cluster).
    pub fn start_pinned(cpus: &[usize], variant: Variant) -> Self {
        Self::spawn(cpus.iter().map(|&c| Some(c)).collect(), variant, RUN)
    }

    fn spawn(cpus: Vec<Option<usize>>, variant: Variant, run: Duration) -> Self {
        let variant = if variant.available() { variant } else { Variant::Scalar };
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            start: Instant::now(),
            total: WARMUP + run,
            result: Mutex::new(None),
            finished: AtomicBool::new(false),
        });
        let sh = shared.clone();
        std::thread::spawn(move || {
            let (warm_end, end) = (sh.start + WARMUP, sh.start + WARMUP + run);
            let threads = cpus.len();
            let stop = &sh.stop;
            let rate: f64 = std::thread::scope(|sc| {
                let hs: Vec<_> =
                    cpus.iter().map(|&c| sc.spawn(move || worker(c, variant, warm_end, end, stop))).collect();
                hs.into_iter().map(|h| h.join().unwrap_or(0.0)).sum()
            });
            if !sh.stop.load(Ordering::Relaxed) {
                let seconds = sh.start.elapsed().as_secs_f64();
                *sh.result.lock().unwrap() = Some(Score { points: rate / SCALE, threads, variant, seconds });
            }
            sh.finished.store(true, Ordering::Release);
        });
        Bench { shared }
    }

    /// 0.0 ..= 1.0
    pub fn progress(&self) -> f32 {
        if self.shared.finished.load(Ordering::Acquire) {
            return 1.0;
        }
        (self.shared.start.elapsed().as_secs_f32() / self.shared.total.as_secs_f32()).min(0.99)
    }

    /// The score once finished; stays None if cancelled.
    pub fn result(&self) -> Option<Score> {
        *self.shared.result.lock().unwrap()
    }

    pub fn finished(&self) -> bool {
        self.shared.finished.load(Ordering::Acquire)
    }

    pub fn cancel(&self) {
        self.shared.stop.store(true, Ordering::Relaxed);
    }

    /// Block until done (CLI).
    pub fn wait(self) -> Option<Score> {
        while !self.finished() {
            std::thread::sleep(Duration::from_millis(20));
        }
        self.result()
    }
}

impl Drop for Bench {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Stress CPU: workers run the widest variant until stopped (or dropped).
pub struct StressHandle {
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

pub fn stress(threads: usize) -> StressHandle {
    let stop = Arc::new(AtomicBool::new(false));
    let v = Variant::best();
    let threads = (0..threads.max(1))
        .map(|i| {
            let stop = stop.clone();
            std::thread::spawn(move || {
                let mut s = State::new(i as u32);
                while !stop.load(Ordering::Relaxed) {
                    steps(v, &mut s, table(), CHUNK);
                }
                std::hint::black_box(s.checksum());
            })
        })
        .collect();
    StressHandle { stop, threads }
}

impl StressHandle {
    pub fn stop(mut self) {
        self.halt();
    }
    fn halt(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Drop for StressHandle {
    fn drop(&mut self) {
        self.halt();
    }
}

/// Logical CPUs of one core set ("P-Cores", "E-Cores", "LP-Cores") for the
/// single-cluster option; empty on non-hybrid CPUs.
pub fn cluster_cpus(ctx: &Ctx, kind: &str) -> Vec<usize> {
    crate::cpu::get(ctx).logical.iter().filter(|l| l.kind.as_deref() == Some(kind)).map(|l| l.cpu).collect()
}

/// Logical CPUs selected by a -coremask bit mask.
pub fn mask_cpus(mask: u64) -> Vec<usize> {
    (0..64.min(crate::os::logical_cpus())).filter(|i| mask >> i & 1 == 1).collect()
}

// ---- -bench ----

pub fn cli(ctx: &Ctx, coremask: Option<u64>) {
    let cpu = crate::cpu::get(ctx);
    let cpus =
        coremask.map(mask_cpus).filter(|c| !c.is_empty()).unwrap_or_else(|| (0..crate::os::logical_cpus()).collect());
    let mut out = format!(
        "{} {} benchmark\n(own workload: scores are not comparable with CPU-Z's)\n\nCPU\t\t\t{}\n",
        crate::DISPLAY_NAME,
        crate::VERSION,
        cpu.name
    );
    if let Some(m) = coremask {
        out.push_str(&format!("Core mask\t\t0x{m:X}\n"));
    }
    let variants: Vec<Variant> =
        if Variant::best() == Variant::Scalar { vec![Variant::Scalar] } else { vec![Variant::Scalar, Variant::best()] };
    for v in variants {
        eprintln!("running {} single thread...", v.label());
        let st = Bench::start_pinned(&cpus[..1], v).wait();
        eprintln!("running {} multi thread ({} threads)...", v.label(), cpus.len());
        let mt = Bench::start_pinned(&cpus, v).wait();
        if let (Some(st), Some(mt)) = (st, mt) {
            crate::history::record(ctx, &st, "");
            crate::history::record(ctx, &mt, "");
            out.push_str(&format!(
                "\n[{}]\nSingle thread\t\t{:.1}\nMulti thread\t\t{:.1}\nThreads\t\t\t{}\nMT ratio\t\t{:.2}\n",
                v.label(),
                st.points,
                mt.points,
                mt.threads,
                mt.points / st.points
            ));
        }
    }
    print!("{out}");
    let path = format!("{}.txt", hostname());
    match std::fs::write(&path, &out) {
        Ok(()) => eprintln!("wrote {path}"),
        Err(e) => eprintln!("{path}: {e}"),
    }
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .or_else(|| crate::os::cmd("hostname", &[]))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.contains(['/', '\\']))
        .unwrap_or_else(|| crate::NAME.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Locks the workload definition: change this constant only on purpose.
    const CHECK_10K: u64 = 0x6a50_f582_8004_c498;

    #[test]
    fn deterministic() {
        let a = run_fixed(Variant::Scalar, 10_000);
        assert_eq!(a, run_fixed(Variant::Scalar, 10_000));
        for v in Variant::all() {
            assert_eq!(run_fixed(v, 10_000), a, "{}", v.label());
        }
        assert_eq!(a, CHECK_10K, "workload changed: {a:#x}");
    }

    #[test]
    fn short_run() {
        let b = Bench::spawn(vec![None, None], Variant::best(), Duration::from_millis(100));
        assert!(b.progress() < 1.0);
        let s = b.wait().unwrap();
        assert!(s.points > 0.0 && s.threads == 2);
        let b = Bench::spawn(vec![None], Variant::Scalar, Duration::from_secs(10));
        b.cancel();
        assert!(b.wait().is_none());
        stress(2).stop();
    }
}

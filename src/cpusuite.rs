//! CPU suite (S1-K02): integer (LZ-style compression), floating point
//! (n-body), SIMD GEMM (GFLOPS with the widest unit: SSE2/AVX2/AVX-512/NEON),
//! crypto (AES-128 with and without the CPU's AES instructions, SHA-256),
//! a ray tracer and branch-heavy sorting, each single- and multi-threaded;
//! plus thread scaling (1…N), P × E cores on hybrids and a sustained run
//! whose drop over time shows throttling.
//!
//! Every test does a fixed chunk of work in a loop for a fixed time per thread
//! (after a warm-up) and reports units per second. Points: 1000 = the
//! reference machine (see `benchkit`). Our own workloads: not comparable with
//! other tools.

use crate::benchkit::{Ctl, Sub};
use std::time::{Duration, Instant};

const WARMUP: Duration = Duration::from_millis(200);
/// Time per sub-test.
pub const SECS: f64 = 1.0;

type Work = Box<dyn FnMut() -> f64 + Send>;

pub struct Test {
    pub name: &'static str,
    pub unit: &'static str,
    /// Units per second → shown value.
    scale: f64,
    make: fn() -> Work,
}

pub const TESTS: [Test; 8] = [
    Test { name: "Integer", unit: "MB/s", scale: 1e-6, make: integer },
    Test { name: "Floating point", unit: "M interactions/s", scale: 1e-6, make: nbody },
    Test { name: "SIMD GEMM", unit: "GFLOPS", scale: 1e-9, make: gemm },
    Test { name: "AES-128 (hardware)", unit: "MB/s", scale: 1e-6, make: aes_hw },
    Test { name: "AES-128 (software)", unit: "MB/s", scale: 1e-6, make: aes_sw },
    Test { name: "SHA-256", unit: "MB/s", scale: 1e-6, make: sha },
    Test { name: "Ray tracer", unit: "M rays/s", scale: 1e-6, make: ray },
    Test { name: "Branchy sort", unit: "M items/s", scale: 1e-6, make: sort },
];

/// The reference machine's results (Ryzen 9 7900X, 12C/24T, DDR5-6000,
/// Windows 11, balanced plan): (sub-test name, value). 1000 points each.
const REFERENCE: &[(&str, f64)] = &[
    ("Integer ST", 835.2),
    ("Integer MT", 14361.9),
    ("Floating point ST", 204.2),
    ("Floating point MT", 2250.1),
    ("SIMD GEMM ST", 78.4),
    ("SIMD GEMM MT", 848.9),
    ("AES-128 (hardware) ST", 14389.5),
    ("AES-128 (hardware) MT", 166695.7),
    ("AES-128 (software) ST", 432.4),
    ("AES-128 (software) MT", 5522.8),
    ("SHA-256 ST", 358.5),
    ("SHA-256 MT", 4442.8),
    ("Ray tracer ST", 44.9),
    ("Ray tracer MT", 733.7),
    ("Branchy sort ST", 104.7),
    ("Branchy sort MT", 1466.7),
];

impl Test {
    /// AES hardware needs the CPU's AES instructions.
    pub fn available(&self) -> bool {
        !self.name.contains("(hardware)") || Aes::hardware()
    }
}

fn reference(name: &str) -> Option<f64> {
    REFERENCE.iter().find(|r| r.0 == name).map(|r| r.1)
}

/// The SIMD unit the GEMM test uses here.
pub fn simd() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("fma") {
            return "AVX-512";
        }
        if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
            return "AVX2";
        }
        "SSE2"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "NEON"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        "Scalar"
    }
}

/// Shown-units per second of `t` with one worker per entry of `cpus`
/// (pinned when Some), for `secs`.
pub fn rate(t: &Test, cpus: &[Option<usize>], secs: f64, ctl: &Ctl) -> f64 {
    let units: f64 = std::thread::scope(|s| {
        let hs: Vec<_> = cpus
            .iter()
            .map(|&cpu| {
                s.spawn(move || {
                    if let Some(c) = cpu {
                        let _ = crate::os::pin(c);
                    }
                    let mut w = (t.make)();
                    let warm = Instant::now() + WARMUP;
                    while Instant::now() < warm && !ctl.cancelled() {
                        w();
                    }
                    let (t0, mut n) = (Instant::now(), 0.0);
                    let end = t0 + Duration::from_secs_f64(secs);
                    while Instant::now() < end && !ctl.cancelled() {
                        n += w();
                    }
                    n / t0.elapsed().as_secs_f64().max(1e-9)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap_or(0.0)).sum()
    });
    units * t.scale
}

fn pinned(cpus: &[usize]) -> Vec<Option<usize>> {
    cpus.iter().map(|&c| Some(c)).collect()
}

/// The single-thread CPU: the third logical CPU when there is one, because
/// the OS sends most interrupts and background work to CPU 0 (and its SMT
/// sibling), which made single-thread results jump by up to 45 % between runs.
pub fn st_cpu(cpus: &[usize]) -> &[usize] {
    let i = if cpus.len() > 2 { 2 } else { 0 };
    &cpus[i..i + 1]
}

/// The whole suite once: every test on one CPU, then on all of `cpus`.
pub fn suite(cpus: &[usize], ctl: &Ctl, run: usize, runs: usize) -> Result<Vec<Sub>, String> {
    let mut out = Vec::new();
    let steps = (TESTS.len() * 2 * runs.max(1)) as f32;
    for (k, t) in TESTS.iter().enumerate().filter(|(_, t)| t.available()) {
        for (m, (tag, set)) in [("ST", st_cpu(cpus)), ("MT", cpus)].into_iter().enumerate() {
            ctl.check()?;
            let name = format!("{} {tag}", t.name);
            ctl.set(&name, ((run * TESTS.len() + k) * 2 + m) as f32 / steps);
            let v = rate(t, &pinned(set), SECS, ctl);
            let s = Sub::new(&name, v, t.unit, true);
            out.push(match reference(&name) {
                Some(r) => s.scored(r),
                None => s,
            });
        }
    }
    ctl.check()?;
    Ok(out)
}

/// Integer test at 1, 2, 4… threads and all of `cpus`.
pub fn scaling(cpus: &[usize], ctl: &Ctl) -> Result<Vec<Sub>, String> {
    let mut counts: Vec<usize> =
        std::iter::successors(Some(1usize), |n| Some(n * 2)).take_while(|&n| n < cpus.len()).collect();
    counts.push(cpus.len());
    let mut out = Vec::new();
    for (i, &n) in counts.iter().enumerate() {
        ctl.check()?;
        ctl.set(&format!("{n} threads"), i as f32 / counts.len() as f32);
        out.push(Sub::new(
            &format!("{n} threads"),
            rate(&TESTS[0], &pinned(&cpus[..n]), SECS, ctl),
            TESTS[0].unit,
            true,
        ));
    }
    ctl.check()?;
    Ok(out)
}

/// Every test single-threaded on the first CPU of each core type (hybrids).
pub fn p_vs_e(types: &[(String, usize)], ctl: &Ctl) -> Result<Vec<Sub>, String> {
    let mut out = Vec::new();
    let steps = (TESTS.len() * types.len()) as f32;
    for (k, t) in TESTS.iter().enumerate().filter(|(_, t)| t.available()) {
        for (j, (kind, cpu)) in types.iter().enumerate() {
            ctl.check()?;
            let name = format!("{} · {kind}", t.name);
            ctl.set(&name, (k * types.len() + j) as f32 / steps);
            out.push(Sub::new(&name, rate(t, &[Some(*cpu)], SECS, ctl), t.unit, true));
        }
    }
    ctl.check()?;
    Ok(out)
}

/// SIMD GEMM on all `cpus` for `minutes`, sampled every `every`: the score
/// over time and how much the last minute lost against the first.
pub fn sustained(cpus: &[usize], minutes: f64, every: f64, ctl: &Ctl) -> Result<(Vec<Sub>, Vec<(f64, f64)>), String> {
    let total = minutes * 60.0;
    let (t0, mut series) = (Instant::now(), Vec::new());
    while t0.elapsed().as_secs_f64() < total {
        ctl.check()?;
        let at = t0.elapsed().as_secs_f64();
        ctl.set("SIMD GEMM MT", (at / total) as f32);
        let v = rate(&TESTS[2], &pinned(cpus), every, ctl);
        series.push((t0.elapsed().as_secs_f64(), v));
    }
    ctl.check()?;
    Ok((sustained_subs(&series), series))
}

/// First minute, last minute and the drop between them.
pub fn sustained_subs(series: &[(f64, f64)]) -> Vec<Sub> {
    let end = series.last().map_or(0.0, |p| p.0);
    let avg = |f: &dyn Fn(f64) -> bool| {
        let v: Vec<f64> = series.iter().filter(|p| f(p.0)).map(|p| p.1).collect();
        if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 }
    };
    let first = avg(&|t| t <= 60.0_f64.min(end / 2.0).max(series.first().map_or(0.0, |p| p.0)));
    let last = avg(&|t| t >= (end - 60.0).max(end / 2.0));
    let drop = if first > 0.0 { (first - last) / first * 100.0 } else { 0.0 };
    vec![
        Sub::new("First minute", first, "GFLOPS", true),
        Sub::new("Last minute", last, "GFLOPS", true),
        Sub::new("Drop", drop.max(0.0), "%", false),
    ]
}

// ---- workloads (each call does one chunk and returns the units it did) ----

fn xorshift(x: &mut u64) -> u64 {
    *x ^= *x << 13;
    *x ^= *x >> 7;
    *x ^= *x << 17;
    *x
}

/// Text-like input: words from a small vocabulary.
fn text(n: usize) -> Vec<u8> {
    const WORDS: [&str; 16] = [
        "the",
        "processor",
        "memory",
        "cache",
        "of",
        "and",
        "benchmark",
        "a",
        "thread",
        "clock",
        "to",
        "core",
        "voltage",
        "in",
        "frequency",
        "latency",
    ];
    let (mut v, mut s) = (Vec::with_capacity(n + 16), 0x1234_5678u64);
    while v.len() < n {
        v.extend_from_slice(WORDS[(xorshift(&mut s) % 16) as usize].as_bytes());
        v.push(if xorshift(&mut s).is_multiple_of(9) { b'\n' } else { b' ' });
    }
    v.truncate(n);
    v
}

/// Greedy LZ77 match finding (hash of 4 bytes → last position): the core of
/// LZ4/deflate-style compressors. Returns tokens emitted.
fn lz(input: &[u8], table: &mut [u32]) -> usize {
    table.fill(u32::MAX);
    let (mut i, mut tokens) = (0, 0);
    while i + 4 <= input.len() {
        let w = u32::from_le_bytes([input[i], input[i + 1], input[i + 2], input[i + 3]]);
        let h = (w.wrapping_mul(2_654_435_761) >> 18) as usize;
        let cand = table[h] as usize;
        table[h] = i as u32;
        if cand < i && i - cand < 65_536 && input[cand..cand + 4] == input[i..i + 4] {
            let mut l = 4;
            while i + l < input.len() && l < 258 && input[cand + l] == input[i + l] {
                l += 1;
            }
            i += l;
        } else {
            i += 1;
        }
        tokens += 1;
    }
    tokens
}

fn integer() -> Work {
    let (input, mut table) = (text(256 << 10), vec![0u32; 1 << 14]);
    Box::new(move || {
        std::hint::black_box(lz(&input, &mut table));
        input.len() as f64
    })
}

/// n-body: 5 bodies, 10 pair interactions per step.
fn nbody() -> Work {
    let mut p = [[0.0f64; 3]; 5];
    let mut v = [[0.0f64; 3]; 5];
    let m = [39.47, 0.037, 0.011, 0.0017, 0.002];
    for i in 0..5 {
        p[i] = [i as f64 * 1.3, (i as f64 * 0.7).sin() * 4.0, (i as f64).cos()];
        v[i] = [0.0, 0.1 * i as f64, 0.01];
    }
    const STEPS: usize = 2_000;
    Box::new(move || {
        for _ in 0..STEPS {
            for i in 0..5 {
                for j in i + 1..5 {
                    let d = [p[i][0] - p[j][0], p[i][1] - p[j][1], p[i][2] - p[j][2]];
                    let d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2] + 0.01;
                    let mag = 0.001 / (d2 * d2.sqrt());
                    for k in 0..3 {
                        v[i][k] -= d[k] * m[j] * mag;
                        v[j][k] += d[k] * m[i] * mag;
                    }
                }
            }
            for i in 0..5 {
                for k in 0..3 {
                    p[i][k] += 0.001 * v[i][k];
                }
            }
        }
        std::hint::black_box(&p);
        (STEPS * 10) as f64
    })
}

const N: usize = 128;

#[inline(always)]
fn gemm_body<const FMA: bool>(a: &[f32], b: &[f32], c: &mut [f32]) {
    for i in 0..N {
        let crow = &mut c[i * N..][..N];
        for k in 0..N {
            let aik = a[i * N + k];
            let brow = &b[k * N..][..N];
            for j in 0..N {
                crow[j] = if FMA { aik.mul_add(brow[j], crow[j]) } else { crow[j] + aik * brow[j] };
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f,fma")]
fn gemm_avx512(a: &[f32], b: &[f32], c: &mut [f32]) {
    gemm_body::<true>(a, b, c)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma")]
fn gemm_avx2(a: &[f32], b: &[f32], c: &mut [f32]) {
    gemm_body::<true>(a, b, c)
}

fn gemm_any(a: &[f32], b: &[f32], c: &mut [f32]) {
    match simd() {
        // SAFETY: simd() checked the CPU features.
        #[cfg(target_arch = "x86_64")]
        "AVX-512" => unsafe { gemm_avx512(a, b, c) },
        #[cfg(target_arch = "x86_64")]
        "AVX2" => unsafe { gemm_avx2(a, b, c) },
        // aarch64 always has fused multiply-add in NEON.
        "NEON" => gemm_body::<true>(a, b, c),
        _ => gemm_body::<false>(a, b, c),
    }
}

/// C = A × B for 128 × 128 f32 matrices (fits the L2): 2·N³ FLOPs.
fn gemm() -> Work {
    let mut s = 7u64;
    let mut r = || (xorshift(&mut s) % 1000) as f32 / 1000.0;
    // Rows on 64-byte boundaries: AVX-512 loads that split cache lines made
    // the result jump between ~28 and ~44 GFLOPS depending on the allocation.
    let (mut a, mut b, mut c) = (vec![0f32; N * N + 16], vec![0f32; N * N + 16], vec![0f32; N * N + 16]);
    let (oa, ob, oc) = (a.as_ptr().align_offset(64), b.as_ptr().align_offset(64), c.as_ptr().align_offset(64));
    a[oa..oa + N * N].iter_mut().chain(b[ob..ob + N * N].iter_mut()).for_each(|x| *x = r());
    Box::new(move || {
        let c = &mut c[oc..oc + N * N];
        c.fill(0.0);
        gemm_any(&a[oa..oa + N * N], &b[ob..ob + N * N], c);
        std::hint::black_box(c);
        (2 * N * N * N) as f64
    })
}

// ---- AES-128 ----

/// The AES S-box, generated (GF(2⁸) inverse + affine map) instead of typed in.
fn sbox() -> [u8; 256] {
    let mut s = [0u8; 256];
    let (mut p, mut q) = (1u8, 1u8);
    loop {
        p ^= (p << 1) ^ if p & 0x80 != 0 { 0x1B } else { 0 };
        q ^= q << 1;
        q ^= q << 2;
        q ^= q << 4;
        if q & 0x80 != 0 {
            q ^= 0x09;
        }
        s[p as usize] = q ^ q.rotate_left(1) ^ q.rotate_left(2) ^ q.rotate_left(3) ^ q.rotate_left(4) ^ 0x63;
        if p == 1 {
            break;
        }
    }
    s[0] = 0x63;
    s
}

pub struct Aes {
    sbox: [u8; 256],
    te: [u32; 256],
    /// 11 round keys, 4 little-endian columns each.
    rk: [[u32; 4]; 11],
}

impl Aes {
    pub fn new(key: [u8; 16]) -> Aes {
        let sbox = sbox();
        let xt = |x: u8| (x << 1) ^ if x & 0x80 != 0 { 0x1B } else { 0 };
        let te = std::array::from_fn(|i| {
            let s = sbox[i];
            u32::from_le_bytes([xt(s), s, s, xt(s) ^ s])
        });
        let sub = |w: u32| u32::from_le_bytes(w.to_le_bytes().map(|b| sbox[b as usize]));
        let mut w = [0u32; 44];
        for (i, c) in key.chunks(4).enumerate() {
            w[i] = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        }
        let mut rcon = 1u8;
        for i in 4..44 {
            let mut t = w[i - 1];
            if i % 4 == 0 {
                t = sub(t.rotate_right(8)) ^ rcon as u32;
                rcon = xt(rcon);
            }
            w[i] = w[i - 4] ^ t;
        }
        Aes { sbox, te, rk: std::array::from_fn(|r| [w[4 * r], w[4 * r + 1], w[4 * r + 2], w[4 * r + 3]]) }
    }

    /// One block in software (T-tables).
    pub fn encrypt(&self, block: [u8; 16]) -> [u8; 16] {
        let col = |c: usize| u32::from_le_bytes([block[4 * c], block[4 * c + 1], block[4 * c + 2], block[4 * c + 3]]);
        let mut s: [u32; 4] = std::array::from_fn(|c| col(c) ^ self.rk[0][c]);
        let b = |x: u32, r: usize| ((x >> (8 * r)) & 0xFF) as usize;
        for r in 1..10 {
            s = std::array::from_fn(|c| {
                self.te[b(s[c], 0)]
                    ^ self.te[b(s[(c + 1) % 4], 1)].rotate_left(8)
                    ^ self.te[b(s[(c + 2) % 4], 2)].rotate_left(16)
                    ^ self.te[b(s[(c + 3) % 4], 3)].rotate_left(24)
                    ^ self.rk[r][c]
            });
        }
        let mut out = [0u8; 16];
        for c in 0..4 {
            for r in 0..4 {
                out[4 * c + r] = self.sbox[b(s[(c + r) % 4], r)] ^ self.rk[10][c].to_le_bytes()[r];
            }
        }
        out
    }

    fn keys(&self) -> [[u8; 16]; 11] {
        self.rk.map(|k| {
            let mut b = [0u8; 16];
            for c in 0..4 {
                b[4 * c..4 * c + 4].copy_from_slice(&k[c].to_le_bytes());
            }
            b
        })
    }

    /// Whether this CPU has AES instructions.
    pub fn hardware() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            is_x86_feature_detected!("aes")
        }
        #[cfg(target_arch = "aarch64")]
        {
            std::arch::is_aarch64_feature_detected!("aes")
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            false
        }
    }

    /// `blocks` in place with the CPU's AES instructions (caller checks
    /// [`Aes::hardware`]).
    pub fn encrypt_hw(&self, blocks: &mut [[u8; 16]]) {
        assert!(Aes::hardware());
        let k = self.keys();
        // SAFETY: feature checked above; loads/stores stay inside the arrays.
        #[cfg(target_arch = "x86_64")]
        unsafe {
            aes_ni(&k, blocks)
        }
        #[cfg(target_arch = "aarch64")]
        unsafe {
            aes_arm(&k, blocks)
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let _ = (k, blocks);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "aes,sse2")]
unsafe fn aes_ni(k: &[[u8; 16]; 11], blocks: &mut [[u8; 16]]) {
    use std::arch::x86_64::*;
    unsafe {
        let rk: [__m128i; 11] = std::array::from_fn(|i| _mm_loadu_si128(k[i].as_ptr().cast()));
        for b in blocks {
            let mut x = _mm_xor_si128(_mm_loadu_si128(b.as_ptr().cast()), rk[0]);
            for r in &rk[1..10] {
                x = _mm_aesenc_si128(x, *r);
            }
            x = _mm_aesenclast_si128(x, rk[10]);
            _mm_storeu_si128(b.as_mut_ptr().cast(), x);
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "aes,neon")]
unsafe fn aes_arm(k: &[[u8; 16]; 11], blocks: &mut [[u8; 16]]) {
    use std::arch::aarch64::*;
    unsafe {
        let rk: [uint8x16_t; 11] = std::array::from_fn(|i| vld1q_u8(k[i].as_ptr()));
        for b in blocks {
            let mut x = vld1q_u8(b.as_ptr());
            for r in &rk[..9] {
                x = vaesmcq_u8(vaeseq_u8(x, *r));
            }
            x = veorq_u8(vaeseq_u8(x, rk[9]), rk[10]);
            vst1q_u8(b.as_mut_ptr(), x);
        }
    }
}

const AES_BLOCKS: usize = 4096;

fn counters() -> Vec<[u8; 16]> {
    (0..AES_BLOCKS as u128).map(|i| i.to_le_bytes()).collect()
}

fn aes_hw() -> Work {
    let (aes, mut buf) = (Aes::new([7; 16]), counters());
    Box::new(move || {
        if !Aes::hardware() {
            return 0.0;
        }
        aes.encrypt_hw(&mut buf);
        std::hint::black_box(&buf);
        (AES_BLOCKS * 16) as f64
    })
}

fn aes_sw() -> Work {
    let (aes, mut buf) = (Aes::new([7; 16]), counters());
    Box::new(move || {
        for b in buf.iter_mut() {
            *b = aes.encrypt(*b);
        }
        std::hint::black_box(&buf);
        (AES_BLOCKS * 16) as f64
    })
}

fn sha() -> Work {
    let data = text(64 << 10);
    Box::new(move || {
        let mut h = crate::update::Sha256::new();
        h.update(&data);
        std::hint::black_box(h.finish());
        data.len() as f64
    })
}

// ---- ray tracer ----

type V3 = [f64; 3];
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: V3) -> V3 {
    mul(a, 1.0 / dot(a, a).sqrt())
}

/// Spheres (centre, radius, reflectivity); the first is the floor.
const SCENE: [(V3, f64, f64); 6] = [
    ([0.0, -1000.0, 0.0], 999.0, 0.2),
    ([0.0, 0.0, 5.0], 1.0, 0.6),
    ([-2.2, -0.3, 6.0], 0.7, 0.3),
    ([2.0, -0.4, 4.5], 0.6, 0.8),
    ([0.8, 1.4, 7.0], 0.9, 0.1),
    ([-0.9, -0.7, 3.8], 0.3, 0.5),
];
const LIGHT: V3 = [-5.0, 8.0, -2.0];

fn hit(o: V3, d: V3) -> Option<(f64, usize)> {
    let mut best: Option<(f64, usize)> = None;
    for (i, (c, r, _)) in SCENE.iter().enumerate() {
        let oc = sub(o, *c);
        let b = dot(oc, d);
        let disc = b * b - (dot(oc, oc) - r * r);
        if disc > 0.0 {
            let t = -b - disc.sqrt();
            if t > 1e-4 && best.is_none_or(|x| t < x.0) {
                best = Some((t, i));
            }
        }
    }
    best
}

/// Brightness along a ray; counts every ray cast in `rays`.
fn trace(o: V3, d: V3, depth: u32, rays: &mut u64) -> f64 {
    *rays += 1;
    let Some((t, i)) = hit(o, d) else { return 0.1 };
    let p = add(o, mul(d, t));
    let n = norm(sub(p, SCENE[i].0));
    let l = norm(sub(LIGHT, p));
    *rays += 1;
    let lit = if hit(p, l).is_none() { dot(n, l).max(0.0) } else { 0.0 };
    let refl = SCENE[i].2;
    if depth == 0 || refl == 0.0 {
        return lit;
    }
    let r = sub(d, mul(n, 2.0 * dot(d, n)));
    lit * (1.0 - refl) + refl * trace(p, r, depth - 1, rays)
}

fn ray() -> Work {
    const W: usize = 64;
    Box::new(|| {
        let (mut rays, mut sum) = (0u64, 0.0);
        for y in 0..W {
            for x in 0..W {
                let d = norm([(x as f64 - W as f64 / 2.0) / W as f64, (W as f64 / 2.0 - y as f64) / W as f64, 1.0]);
                sum += trace([0.0, 0.0, 0.0], d, 3, &mut rays);
            }
        }
        std::hint::black_box(sum);
        rays as f64
    })
}

fn sort() -> Work {
    const LEN: usize = 1 << 16;
    let mut seed = 99u64;
    let mut v = vec![0u32; LEN];
    Box::new(move || {
        for x in v.iter_mut() {
            *x = xorshift(&mut seed) as u32;
        }
        v.sort_unstable();
        std::hint::black_box(&v);
        LEN as f64
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_matches_fips_197() {
        let s = sbox();
        assert_eq!((s[0], s[1], s[0x53]), (0x63, 0x7C, 0xED));
        let key: [u8; 16] = std::array::from_fn(|i| i as u8);
        let pt: [u8; 16] = std::array::from_fn(|i| (i as u8) * 0x11);
        let ct = [0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4, 0xc5, 0x5a];
        let aes = Aes::new(key);
        assert_eq!(aes.encrypt(pt), ct);
        if Aes::hardware() {
            let mut b = [pt];
            aes.encrypt_hw(&mut b);
            assert_eq!(b[0], ct, "hardware AES = software AES");
        }
    }

    #[test]
    fn lz_finds_matches() {
        let t = text(4096);
        let tokens = lz(&t, &mut vec![0; 1 << 14]);
        assert!(tokens < t.len() / 3, "repetitive text compresses: {tokens} tokens for {} bytes", t.len());
    }

    #[test]
    fn gemm_is_right() {
        let a: Vec<f32> = (0..N * N).map(|i| (i % 7) as f32).collect();
        let id: Vec<f32> = (0..N * N).map(|i| if i % (N + 1) == 0 { 1.0 } else { 0.0 }).collect();
        let mut c = vec![0f32; N * N];
        gemm_any(&a, &id, &mut c);
        assert_eq!(c, a, "A × I = A with the {} kernel", simd());
    }

    #[test]
    fn every_test_runs() {
        let _heavy = crate::heavy_test();
        let ctl = Ctl::default();
        for t in &TESTS {
            let v = rate(t, &[None], 0.05, &ctl);
            assert!(v > 0.0 || (t.name.contains("hardware") && !Aes::hardware()), "{}: {v}", t.name);
        }
        let s = scaling(&[0, 1, 2], &Ctl::default());
        // Pinning can fail on a runner with fewer CPUs; the labels are what matter here.
        if let Ok(s) = s {
            assert_eq!(s.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["1 threads", "2 threads", "3 threads"]);
        }
    }

    #[test]
    fn sustained_drop() {
        let series: Vec<(f64, f64)> = (0..60).map(|i| (i as f64 * 10.0, if i < 30 { 100.0 } else { 80.0 })).collect();
        let s = sustained_subs(&series);
        assert_eq!((s[0].value, s[1].value), (100.0, 80.0));
        assert!((s[2].value - 20.0).abs() < 1e-9);
    }
}

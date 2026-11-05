//! Core-to-core latency (S1-K03): two threads pinned to two cores bounce a
//! value through one cache line; half the round trip is the one-way latency
//! between them. Shows the CCD/CCX (or ring/mesh) layout: cores sharing an L3
//! answer in a few tens of ns, cores across dies in ~100 ns or more.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

/// Round trips per pair (after a warm-up). ~1–2 ms per pair on a desktop CPU.
pub const ROUNDS: u64 = 20_000;
const WARMUP: u64 = 2_000;

/// One-way latency (ns) for every ordered pair of `cpus` (logical CPU ids);
/// `m[i][j]` is None on the diagonal or where a thread couldn't be pinned
/// (macOS has no hard affinity). `done` counts finished pairs.
pub fn measure(cpus: &[usize], rounds: u64, done: &AtomicUsize) -> Vec<Vec<Option<f64>>> {
    let n = cpus.len();
    let mut m = vec![vec![None; n]; n];
    for i in 0..n {
        for j in i + 1..n {
            let ns = pair(Some(cpus[i]), Some(cpus[j]), rounds);
            m[i][j] = ns;
            m[j][i] = ns;
            done.fetch_add(1, Ordering::Relaxed);
        }
    }
    m
}

/// One ping-pong between two threads, pinned to `a` and `b` when given.
fn pair(a: Option<usize>, b: Option<usize>, rounds: u64) -> Option<f64> {
    // Own cache line, so nothing else shares it.
    #[repr(align(128))]
    struct Line(AtomicU64);
    let line = Arc::new(Line(AtomicU64::new(0)));
    let total = WARMUP + rounds;
    let l2 = line.clone();
    let peer = std::thread::spawn(move || {
        if b.is_some_and(|b| !cpuzeez_platform::os::pin(b)) {
            l2.0.store(u64::MAX, Ordering::Release);
            return;
        }
        for k in 0..total {
            loop {
                match l2.0.load(Ordering::Acquire) {
                    v if v == 2 * k + 1 => break,
                    u64::MAX => return,
                    _ => std::hint::spin_loop(),
                }
            }
            l2.0.store(2 * k + 2, Ordering::Release);
        }
    });
    let ok = std::thread::scope(|s| {
        s.spawn(|| {
            if a.is_some_and(|a| !cpuzeez_platform::os::pin(a)) {
                line.0.store(u64::MAX, Ordering::Release);
                return None;
            }
            let mut start = Instant::now();
            for k in 0..total {
                if k == WARMUP {
                    start = Instant::now();
                }
                line.0.store(2 * k + 1, Ordering::Release);
                loop {
                    match line.0.load(Ordering::Acquire) {
                        v if v == 2 * k + 2 => break,
                        u64::MAX => return None,
                        _ => std::hint::spin_loop(),
                    }
                }
            }
            Some(start.elapsed().as_nanos() as f64 / (2.0 * rounds as f64))
        })
        .join()
        .ok()
        .flatten()
    });
    let _ = peer.join();
    ok
}

/// Statistical mode, for an OS that can't pin threads (macOS): `samples`
/// ping-pongs between two unpinned threads, wherever the scheduler puts them.
/// The spread shows same-cluster vs cross-cluster latency without naming the
/// cores. `done` counts finished samples.
pub fn statistical(samples: usize, rounds: u64, done: &AtomicUsize) -> Vec<f64> {
    let mut v: Vec<f64> = (0..samples)
        .filter_map(|_| {
            let ns = pair(None, None, rounds);
            done.fetch_add(1, Ordering::Relaxed);
            ns
        })
        .collect();
    v.sort_by(f64::total_cmp);
    v
}

/// (min, median, max) over the measured pairs.
pub fn summary(m: &[Vec<Option<f64>>]) -> Option<(f64, f64, f64)> {
    let mut v: Vec<f64> = m.iter().enumerate().flat_map(|(i, r)| r.iter().skip(i + 1).flatten().copied()).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some((v[0], v[v.len() / 2], v[v.len() - 1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_reads_the_upper_triangle() {
        let m = vec![
            vec![None, Some(20.0), Some(80.0)],
            vec![Some(20.0), None, Some(25.0)],
            vec![Some(80.0), Some(25.0), None],
        ];
        assert_eq!(summary(&m), Some((20.0, 25.0, 80.0)));
        assert_eq!(summary(&[vec![None]]), None);
    }

    #[test]
    #[cfg(any(windows, target_os = "linux"))]
    fn two_cores_answer() {
        let _heavy = crate::heavy_test();
        if cpuzeez_platform::os::logical_cpus() < 2 {
            return;
        }
        let done = AtomicUsize::new(0);
        let m = measure(&[0, 1], 2_000, &done);
        let ns = m[0][1].expect("pinned pair");
        assert!(ns > 0.0 && ns < 100_000.0, "{ns} ns");
        assert_eq!(done.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn statistical_mode_measures_unpinned() {
        let _heavy = crate::heavy_test();
        let done = AtomicUsize::new(0);
        let v = statistical(5, 1_000, &done);
        assert_eq!((v.len(), done.load(Ordering::Relaxed)), (5, 5));
        assert!(v.windows(2).all(|w| w[0] <= w[1]) && v[0] > 0.0, "{v:?}");
    }
}

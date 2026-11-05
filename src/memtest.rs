//! User-mode memory test (S1-K07): fills the chosen amount of RAM with
//! patterns and reads it back, over and over, counting mismatches. It only
//! reaches memory the OS gives this process, so it is **not** a replacement
//! for MemTest86 / TestMem5; it's a quick check that an overclock or XMP
//! profile holds under load.
//!
//! Patterns per round: address-in-address, its inverse, a walking one,
//! checkerboard and a seeded pseudo-random stream. Each thread tests its own
//! block.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub const PATTERNS: u64 = 5;

fn xorshift(x: &mut u64) -> u64 {
    *x ^= *x << 13;
    *x ^= *x >> 7;
    *x ^= *x << 17;
    *x
}

/// The value word `i` holds in pattern `pat` of round `round`.
fn value(pat: u64, round: u64, i: usize, rng: &mut u64) -> u64 {
    let i = i as u64;
    match pat {
        0 => i.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ round,
        1 => !(i.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ round),
        2 => 1u64 << ((i + round) % 64),
        3 => {
            if (i + round).is_multiple_of(2) {
                0xAAAA_AAAA_AAAA_AAAA
            } else {
                0x5555_5555_5555_5555
            }
        }
        _ => xorshift(rng),
    }
}

fn seed(round: u64, block: usize) -> u64 {
    (round + 1).wrapping_mul(0xD1B5_4A32_D192_ED03) ^ block as u64 | 1
}

pub fn fill(buf: &mut [u64], pat: u64, round: u64, block: usize) {
    let mut r = seed(round, block);
    for (i, w) in buf.iter_mut().enumerate() {
        *w = value(pat, round, i, &mut r);
    }
}

/// Mismatching words, and the index of the first one.
pub fn verify(buf: &[u64], pat: u64, round: u64, block: usize) -> (u64, Option<usize>) {
    let mut r = seed(round, block);
    let (mut bad, mut first) = (0, None);
    for (i, &w) in buf.iter().enumerate() {
        if w != value(pat, round, i, &mut r) {
            bad += 1;
            first.get_or_insert(i);
        }
    }
    (bad, first)
}

/// A running test; stops when dropped.
pub struct MemTest {
    stop: Arc<AtomicBool>,
    /// Patterns finished, summed over threads.
    done: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    /// First error, or why the test couldn't start.
    note: Arc<Mutex<Option<String>>>,
    threads: Vec<JoinHandle<()>>,
}

impl MemTest {
    pub fn start(bytes: u64, threads: usize) -> MemTest {
        let threads = threads.max(1);
        let words = (bytes / 8 / threads as u64) as usize;
        let mut t = MemTest {
            stop: Arc::default(),
            done: Arc::default(),
            errors: Arc::default(),
            note: Arc::default(),
            threads: Vec::new(),
        };
        t.threads = (0..threads)
            .map(|block| {
                let (stop, done, errors, note) = (t.stop.clone(), t.done.clone(), t.errors.clone(), t.note.clone());
                std::thread::spawn(move || {
                    let mut buf = Vec::new();
                    if buf.try_reserve_exact(words).is_err() {
                        *note.lock().unwrap_or_else(|e| e.into_inner()) = Some("not enough free memory".into());
                        stop.store(true, Ordering::Relaxed);
                        return;
                    }
                    buf.resize(words, 0u64);
                    let mut round = 0;
                    while !stop.load(Ordering::Relaxed) {
                        for pat in 0..PATTERNS {
                            if stop.load(Ordering::Relaxed) {
                                return;
                            }
                            fill(&mut buf, pat, round, block);
                            std::hint::black_box(&mut buf);
                            let (bad, first) = verify(&buf, pat, round, block);
                            if bad > 0 {
                                errors.fetch_add(bad, Ordering::Relaxed);
                                let mut n = note.lock().unwrap_or_else(|e| e.into_inner());
                                if n.is_none() {
                                    *n = Some(format!(
                                        "block {block}, word {} (pattern {pat}, round {round})",
                                        first.unwrap_or(0)
                                    ));
                                }
                            }
                            done.fetch_add(1, Ordering::Relaxed);
                        }
                        round += 1;
                    }
                })
            })
            .collect();
        t
    }

    /// Full rounds (every pattern on every block).
    pub fn rounds(&self) -> u64 {
        self.done.load(Ordering::Relaxed) / PATTERNS / self.threads.len().max(1) as u64
    }

    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    pub fn note(&self) -> Option<String> {
        self.note.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// False once every thread ended (stopped, or couldn't allocate).
    pub fn running(&self) -> bool {
        self.threads.iter().any(|h| !h.is_finished())
    }

    pub fn stop(mut self) {
        self.halt();
    }

    fn halt(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for h in self.threads.drain(..) {
            let _ = h.join();
        }
    }
}

impl Drop for MemTest {
    fn drop(&mut self) {
        self.halt();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pattern_catches_a_flipped_bit() {
        let mut buf = vec![0u64; 4096];
        for pat in 0..PATTERNS {
            fill(&mut buf, pat, 3, 1);
            assert_eq!(verify(&buf, pat, 3, 1), (0, None), "pattern {pat} reads back");
            buf[1234] ^= 1 << 17;
            assert_eq!(verify(&buf, pat, 3, 1), (1, Some(1234)), "pattern {pat} finds the flip");
        }
    }

    #[test]
    fn runs_and_stops() {
        let _heavy = crate::heavy_test();
        let t = MemTest::start(8 << 20, 2);
        let start = std::time::Instant::now();
        while t.rounds() < 1 && start.elapsed().as_secs() < 20 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(t.rounds() >= 1 && t.errors() == 0, "{} rounds, {} errors", t.rounds(), t.errors());
        t.stop();
    }
}

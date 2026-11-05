//! Storage benchmark (S1-K06), CrystalDiskMark-style: sequential 1 MiB and
//! random 4 KiB, with 1 and 8 requests in flight (one thread per request),
//! write then read, on an uncached test file that is always deleted.
//! Results are this app's own measurement: comparable run to run on the same
//! drive, not with other tools.

use cpuzeez_platform::direct_io::{self, Buffer, write_at};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct Test {
    pub name: &'static str,
    pub block: usize,
    pub threads: usize,
    pub random: bool,
}

pub const TESTS: [Test; 4] = [
    Test { name: "SEQ1M T8", block: 1 << 20, threads: 8, random: false },
    Test { name: "SEQ1M T1", block: 1 << 20, threads: 1, random: false },
    Test { name: "RND4K T8", block: 4096, threads: 8, random: true },
    Test { name: "RND4K T1", block: 4096, threads: 1, random: true },
];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Score {
    pub name: &'static str,
    pub read_mbps: f64,
    pub write_mbps: f64,
    pub read_iops: f64,
    pub write_iops: f64,
}

/// A run in progress; poll it from the GUI.
pub struct Run {
    pub stage: Arc<Mutex<(String, f32)>>,
    pub cancel: Arc<AtomicBool>,
    pub result: Arc<Mutex<Option<Result<Vec<Score>, String>>>>,
}

/// Deletes the test file however the run ends.
struct TempFile(PathBuf);
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// MB/s and IOPS from bytes and operations over `secs`.
pub fn rates(bytes: u64, ops: u64, secs: f64) -> (f64, f64) {
    if secs <= 0.0 { (0.0, 0.0) } else { (bytes as f64 / secs / 1e6, ops as f64 / secs) }
}

/// xorshift64*: cheap incompressible data and random offsets.
fn next(x: &mut u64) -> u64 {
    *x ^= *x >> 12;
    *x ^= *x << 25;
    *x ^= *x >> 27;
    x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

/// Starts the benchmark in `dir` with a `size`-byte file, each test phase
/// running `secs` seconds.
pub fn start(dir: &Path, size: u64, secs: f64) -> Run {
    let run = Run {
        stage: Arc::new(Mutex::new((String::new(), 0.0))),
        cancel: Arc::new(AtomicBool::new(false)),
        result: Arc::new(Mutex::new(None)),
    };
    let (stage, cancel, result) = (run.stage.clone(), run.cancel.clone(), run.result.clone());
    let path = dir.join("cpuzeez-bench.tmp");
    std::thread::spawn(move || {
        let r = bench(&path, size, secs, &stage, &cancel);
        *result.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
    });
    run
}

fn bench(
    path: &Path,
    size: u64,
    secs: f64,
    stage: &Mutex<(String, f32)>,
    cancel: &AtomicBool,
) -> Result<Vec<Score>, String> {
    let size = size / (1 << 20) * (1 << 20);
    // Dropped last: the file closes first, then it's deleted.
    let _guard = TempFile(path.to_path_buf());
    let file = Arc::new(direct_io::open(path).map_err(|e| format!("{}: {e}", path.display()))?);
    let set = |s: &str, f: f32| *stage.lock().unwrap_or_else(|e| e.into_inner()) = (s.to_string(), f);
    // Fill the file so reads hit real data (and the drive can't skip it).
    let mut buf = Buffer::new(1 << 20);
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    for (i, off) in (0..size).step_by(1 << 20).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        buf.bytes().as_chunks_mut::<8>().0.iter_mut().for_each(|c| *c = next(&mut seed).to_le_bytes());
        write_at(&file, buf.bytes(), off).map_err(|e| e.to_string())?;
        set("Preparing", i as f32 * (1 << 20) as f32 / size as f32 * 0.1);
    }
    let mut out = Vec::new();
    let phases = (TESTS.len() * 2) as f32;
    for (k, t) in TESTS.iter().enumerate() {
        let mut score = Score { name: t.name, ..Default::default() };
        for (w, write) in [true, false].into_iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Err("cancelled".into());
            }
            set(t.name, 0.1 + 0.9 * (k * 2 + w) as f32 / phases);
            let (bytes, ops, el) = phase(&file, size, *t, write, secs, cancel)?;
            let (mbps, iops) = rates(bytes, ops, el);
            if write {
                (score.write_mbps, score.write_iops) = (mbps, iops);
            } else {
                (score.read_mbps, score.read_iops) = (mbps, iops);
            }
        }
        out.push(score);
    }
    set("Done", 1.0);
    Ok(out)
}

/// One test phase: `t.threads` threads doing `t.block`-sized I/O for `secs`.
fn phase(
    file: &Arc<std::fs::File>,
    size: u64,
    t: Test,
    write: bool,
    secs: f64,
    cancel: &AtomicBool,
) -> Result<(u64, u64, f64), String> {
    let (bytes, ops) = (Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0)));
    let stop = Arc::new(AtomicBool::new(false));
    let error: Arc<Mutex<Option<String>>> = Arc::default();
    let blocks = size / t.block as u64;
    let begin = Instant::now();
    let handles: Vec<_> = (0..t.threads)
        .map(|n| {
            let (file, bytes, ops, stop, error) =
                (file.clone(), bytes.clone(), ops.clone(), stop.clone(), error.clone());
            std::thread::spawn(move || {
                let mut buf = Buffer::new(t.block);
                let mut seed = 0xC0FF_EE00 + n as u64;
                // Sequential: each thread walks its own slice of the file.
                let slice = blocks / t.threads as u64;
                let mut i = 0u64;
                while !stop.load(Ordering::Relaxed) {
                    let block = if t.random { next(&mut seed) % blocks } else { n as u64 * slice + i % slice.max(1) };
                    let off = block * t.block as u64;
                    let r = if write {
                        write_at(&file, buf.bytes(), off)
                    } else {
                        direct_io::read_at(&file, buf.bytes(), off)
                    };
                    if let Err(e) = r {
                        *error.lock().unwrap_or_else(|p| p.into_inner()) = Some(e.to_string());
                        return;
                    }
                    bytes.fetch_add(t.block as u64, Ordering::Relaxed);
                    ops.fetch_add(1, Ordering::Relaxed);
                    i += 1;
                }
            })
        })
        .collect();
    while begin.elapsed() < Duration::from_secs_f64(secs) && !cancel.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(20));
    }
    stop.store(true, Ordering::Relaxed);
    for h in handles {
        let _ = h.join();
    }
    let el = begin.elapsed().as_secs_f64();
    if let Some(e) = error.lock().unwrap_or_else(|p| p.into_inner()).take() {
        return Err(e);
    }
    Ok((bytes.load(Ordering::Relaxed), ops.load(Ordering::Relaxed), el))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_math() {
        assert_eq!(rates(500_000_000, 100, 2.0), (250.0, 50.0));
        assert_eq!(rates(1, 1, 0.0), (0.0, 0.0));
    }

    #[test]
    fn a_tiny_run_measures_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("cz-diskbench-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let run = start(&dir, 16 << 20, 0.15);
        let r = loop {
            if let Some(r) = run.result.lock().unwrap().take() {
                break r;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let scores = r.expect("benchmark ran");
        assert_eq!(scores.len(), TESTS.len());
        assert!(scores.iter().all(|s| s.read_mbps > 0.0 && s.write_mbps > 0.0), "{scores:?}");
        assert!(!dir.join("cpuzeez-bench.tmp").exists(), "the test file is deleted");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

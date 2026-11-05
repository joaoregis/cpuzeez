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
pub type Run = crate::benchkit::Job<Vec<Score>>;

impl Score {
    /// As benchmark sub-tests (history and compare).
    pub fn subs(scores: &[Score]) -> Vec<crate::benchkit::Sub> {
        use crate::benchkit::Sub;
        let mut v = Vec::new();
        for s in scores {
            let random = TESTS.iter().find(|t| t.name == s.name).is_some_and(|t| t.random);
            for (dir, mbps, iops) in [("read", s.read_mbps, s.read_iops), ("write", s.write_mbps, s.write_iops)] {
                v.push(Sub::new(&format!("{} {dir}", s.name), mbps, "MB/s", true));
                if random {
                    v.push(Sub::new(&format!("{} {dir} IOPS", s.name), iops, "IOPS", true));
                }
            }
        }
        v
    }
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
    let path = dir.join("cpuzeez-bench.tmp");
    crate::benchkit::Job::spawn(move |ctl| bench(&path, size, secs, ctl))
}

fn bench(path: &Path, size: u64, secs: f64, ctl: &crate::benchkit::Ctl) -> Result<Vec<Score>, String> {
    let cancel = ctl.cancelled_flag();
    let size = size / (1 << 20) * (1 << 20);
    // Dropped last: the file closes first, then it's deleted.
    let _guard = TempFile(path.to_path_buf());
    let file = Arc::new(direct_io::open(path).map_err(|e| format!("{}: {e}", path.display()))?);
    let set = |s: &str, f: f32| ctl.set(s, f);
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

/// Sustained write (S1-K06): `bytes` written sequentially, uncached, by 4
/// threads each on its own quarter of a test file; the MB/s of every second.
/// Shows the SLC-cache cliff of consumer SSDs. The file is always deleted.
pub fn sustained(dir: &Path, bytes: u64, ctl: &crate::benchkit::Ctl) -> Result<Vec<(f64, f64)>, String> {
    const THREADS: u64 = 4;
    const BLOCK: u64 = 1 << 20;
    let path = dir.join("cpuzeez-sustained.tmp");
    let _guard = TempFile(path.clone());
    let file = direct_io::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let quarter = bytes / THREADS / BLOCK * BLOCK;
    let written = AtomicU64::new(0);
    let error: Mutex<Option<String>> = Mutex::new(None);
    let series = std::thread::scope(|s| {
        let handles: Vec<_> = (0..THREADS)
            .map(|k| {
                let (file, written, error) = (&file, &written, &error);
                s.spawn(move || {
                    let mut buf = Buffer::new(BLOCK as usize);
                    let mut seed = 0xABCD_0000 + k;
                    buf.bytes().as_chunks_mut::<8>().0.iter_mut().for_each(|c| *c = next(&mut seed).to_le_bytes());
                    let mut off = k * quarter;
                    while off < (k + 1) * quarter && !ctl.cancelled() {
                        if let Err(e) = write_at(file, buf.bytes(), off) {
                            *error.lock().unwrap_or_else(|p| p.into_inner()) = Some(e.to_string());
                            return;
                        }
                        written.fetch_add(BLOCK, Ordering::Relaxed);
                        off += BLOCK;
                    }
                })
            })
            .collect();
        let (t0, mut last, mut series) = (Instant::now(), (0.0f64, 0u64), Vec::new());
        let total = quarter * THREADS;
        while !handles.iter().all(|h| h.is_finished()) {
            std::thread::sleep(Duration::from_millis(50));
            let (t, w) = (t0.elapsed().as_secs_f64(), written.load(Ordering::Relaxed));
            if t - last.0 >= 1.0 {
                series.push((t, (w - last.1) as f64 / (t - last.0) / 1e6));
                last = (t, w);
                ctl.set("Sustained write", w as f32 / total.max(1) as f32);
            }
        }
        series
    });
    if let Some(e) = error.lock().unwrap_or_else(|p| p.into_inner()).take() {
        return Err(e);
    }
    ctl.check()?;
    Ok(series)
}

/// Start (first 5 s), end (last 10 %), average and the GB written before the
/// speed first fell under half the start: roughly the SLC cache.
pub fn sustained_subs(series: &[(f64, f64)]) -> Vec<crate::benchkit::Sub> {
    use crate::benchkit::Sub;
    let avg = |v: &[(f64, f64)]| if v.is_empty() { 0.0 } else { v.iter().map(|p| p.1).sum::<f64>() / v.len() as f64 };
    let start = avg(&series[..series.len().min(5)]);
    let end = avg(&series[series.len() - (series.len() / 10).max(1).min(series.len())..]);
    let mut out = vec![
        Sub::new("Start", start, "MB/s", true),
        Sub::new("End", end, "MB/s", true),
        Sub::new("Average", avg(series), "MB/s", true),
    ];
    let mut gb = 0.0;
    let mut prev = 0.0;
    for &(t, mbps) in series {
        if mbps < start / 2.0 {
            out.push(Sub::new("Cache", gb, "GB", true));
            break;
        }
        gb += mbps * (t - prev) / 1000.0;
        prev = t;
    }
    out
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
        let scores = start(&dir, 16 << 20, 0.15).wait().expect("benchmark ran");
        assert_eq!(scores.len(), TESTS.len());
        assert!(scores.iter().all(|s| s.read_mbps > 0.0 && s.write_mbps > 0.0), "{scores:?}");
        assert!(!dir.join("cpuzeez-bench.tmp").exists(), "the test file is deleted");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sustained_finds_the_cache_cliff() {
        // 10 s at 3000 MB/s, then 10 s at 900 MB/s.
        let series: Vec<(f64, f64)> = (1..=20).map(|t| (t as f64, if t <= 10 { 3000.0 } else { 900.0 })).collect();
        let s = sustained_subs(&series);
        assert_eq!((s[0].value, s[1].value), (3000.0, 900.0));
        assert_eq!(s[3].name, "Cache");
        assert!((s[3].value - 30.0).abs() < 1e-9, "10 s × 3 GB/s: {}", s[3].value);
    }

    #[test]
    fn a_tiny_sustained_write_cleans_up() {
        let dir = std::env::temp_dir().join(format!("cz-sustained-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let series = sustained(&dir, 64 << 20, &Default::default()).unwrap();
        assert!(series.iter().all(|p| p.1 >= 0.0));
        assert!(!dir.join("cpuzeez-sustained.tmp").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

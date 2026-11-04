//! Metric Engine (docs/v1/README.md §4.1): one sampler for every live value.
//!
//! Sources produce [`Reading`]s; the engine polls a source only while some
//! consumer holds a matching **lease**. Immediate-mode UIs renew a lease every
//! frame (`want("overview", &["cpu/", "gpu"])`) and it lapses a few seconds
//! after the last renewal, so a hidden page costs nothing. Slow sources (SMART,
//! battery tools) run on their own worker and never stall the fast tick; their
//! latest values are carried into every fast tick so history and CSV rows stay
//! complete. Consumers read current values and statistics from a shared
//! [`Snapshot`] (no copying) and fetch points only for what they draw.
//! Per-tick processing (throttle detection, CSV logging) plugs in as hooks.
//! A source that panics is paused and reported; the others keep running.
//!
//! Ids are `<device>/<kind>/<name>` (e.g. `cpu/temp/package`, `gpu0/clock/core`).

use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Unit {
    Celsius,
    Volt,
    Watt,
    Ampere,
    Mhz,
    Rpm,
    Percent,
    Megabytes,
}

impl Unit {
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::Celsius => "°C",
            Unit::Volt => "V",
            Unit::Watt => "W",
            Unit::Ampere => "A",
            Unit::Mhz => "MHz",
            Unit::Rpm => "RPM",
            Unit::Percent => "%",
            Unit::Megabytes => "MB",
        }
    }
}

/// One value from one read of a source.
#[derive(Debug, Clone, Serialize)]
pub struct Reading {
    /// Stable key, e.g. "cpu/temp/package", "gpu0/temp/core", "board/fan/sys1".
    pub id: String,
    /// Display group: "CPU", "GPU · NVIDIA GeForce RTX 4060", "Mainboard", ...
    pub group: String,
    pub label: String,
    pub unit: Unit,
    pub value: f64,
}

/// Current value and statistics of one metric (no history).
#[derive(Debug, Clone)]
pub struct Stat {
    pub id: String,
    pub group: String,
    pub label: String,
    pub unit: Unit,
    pub current: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    /// Engine time (seconds since start) of the latest value.
    pub at: f64,
}

/// A metric with its kept history, for charts.
#[derive(Debug, Clone)]
pub struct Series {
    pub id: String,
    pub group: String,
    pub label: String,
    pub unit: Unit,
    /// (seconds since the engine started, value), oldest first, bounded by `keep`.
    pub points: Vec<(f64, f64)>,
    pub current: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
}

/// Every metric's current value and statistics after the latest tick.
#[derive(Debug, Default)]
pub struct Snapshot {
    /// Engine time of the tick.
    pub t: f64,
    pub stats: Vec<Stat>,
    index: HashMap<String, usize>,
}

impl Snapshot {
    pub fn get(&self, id: &str) -> Option<&Stat> {
        self.index.get(id).map(|&i| &self.stats[i])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Read on every tick.
    Fast,
    /// Read on its own worker, at most once per [`Source::interval`].
    Slow,
}

pub trait Source: Send {
    /// Short name for diagnostics ("cpu", "storage"…).
    fn name(&self) -> &'static str;
    /// Id prefixes this source produces; leases are matched against them.
    fn prefixes(&self) -> &'static [&'static str];
    fn class(&self) -> Class {
        Class::Fast
    }
    /// Minimum time between two reads of a slow source.
    fn interval(&self) -> Duration {
        Duration::ZERO
    }
    fn read(&mut self, out: &mut Vec<Reading>);
}

/// What a hook sees after each tick.
pub struct Tick<'a> {
    pub t: f64,
    pub wall: SystemTime,
    pub readings: &'a [Reading],
    history: &'a History,
}

impl Tick<'_> {
    /// Highest kept value of `id` (current tick included).
    pub fn peak(&self, id: &str) -> Option<f64> {
        self.history.peak(id)
    }
}

pub type Hook = Box<dyn FnMut(&Tick) + Send>;

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Fast tick period (≥ 100 ms).
    pub period: Duration,
    /// History kept per metric.
    pub keep: Duration,
    /// How long a lease lives after its last renewal.
    pub lease: Duration,
    /// A source busy for longer than this is reported as not responding.
    pub timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            period: Duration::from_secs(1),
            keep: Duration::from_secs(600),
            lease: Duration::from_secs(3),
            timeout: Duration::from_secs(10),
        }
    }
}

/// Pause after a source panics before it is tried again.
const PANIC_BACKOFF: Duration = Duration::from_secs(30);
/// How often the slow worker checks whether a source is due.
const SLOW_POLL: Duration = Duration::from_millis(250);

// ---------------------------------------------------------------- history

struct Buf {
    id: String,
    group: String,
    label: String,
    unit: Unit,
    points: VecDeque<(f64, f64)>,
    current: f64,
    at: f64,
    min: f64,
    max: f64,
    sum: f64,
    n: u64,
}

impl Buf {
    fn reset(&mut self) {
        (self.min, self.max, self.sum, self.n) = (f64::INFINITY, f64::NEG_INFINITY, 0.0, 0);
    }

    /// (min, max, avg); the current value when nothing was sampled since a reset.
    fn stats(&self) -> (f64, f64, f64) {
        if self.n == 0 {
            (self.current, self.current, self.current)
        } else {
            (self.min, self.max, self.sum / self.n as f64)
        }
    }
}

/// Time series per metric, trimmed to `keep` seconds; statistics cover
/// everything since start or the last reset, not only the kept window.
pub struct History {
    keep: f64,
    series: Vec<Buf>,
    index: HashMap<String, usize>,
}

impl History {
    pub fn new(keep: Duration) -> History {
        History { keep: keep.as_secs_f64(), series: Vec::new(), index: HashMap::new() }
    }

    pub fn push(&mut self, t: f64, rs: &[Reading]) {
        for r in rs {
            let i = *self.index.entry(r.id.clone()).or_insert_with(|| {
                self.series.push(Buf {
                    id: r.id.clone(),
                    group: r.group.clone(),
                    label: r.label.clone(),
                    unit: r.unit,
                    points: VecDeque::new(),
                    current: r.value,
                    at: t,
                    min: f64::INFINITY,
                    max: f64::NEG_INFINITY,
                    sum: 0.0,
                    n: 0,
                });
                self.series.len() - 1
            });
            let b = &mut self.series[i];
            b.label.clone_from(&r.label);
            b.points.push_back((t, r.value));
            (b.current, b.at) = (r.value, t);
            b.min = b.min.min(r.value);
            b.max = b.max.max(r.value);
            b.sum += r.value;
            b.n += 1;
        }
        for b in &mut self.series {
            while b.points.front().is_some_and(|p| p.0 < t - self.keep) {
                b.points.pop_front();
            }
        }
    }

    pub fn peak(&self, id: &str) -> Option<f64> {
        self.series[*self.index.get(id)?].points.iter().map(|p| p.1).reduce(f64::max)
    }

    pub fn points(&self, id: &str, since: f64) -> Vec<(f64, f64)> {
        self.index
            .get(id)
            .map_or_else(Vec::new, |&i| self.series[i].points.iter().copied().filter(|p| p.0 >= since).collect())
    }

    pub fn reset_stats(&mut self) {
        self.series.iter_mut().for_each(Buf::reset);
    }

    /// Change how much history is kept; applies from the next push.
    pub fn set_keep(&mut self, keep: Duration) {
        self.keep = keep.as_secs_f64();
    }

    pub fn snapshot(&self, t: f64) -> Snapshot {
        let stats: Vec<Stat> = self
            .series
            .iter()
            .map(|b| {
                let (min, max, avg) = b.stats();
                Stat {
                    id: b.id.clone(),
                    group: b.group.clone(),
                    label: b.label.clone(),
                    unit: b.unit,
                    current: b.current,
                    min,
                    max,
                    avg,
                    at: b.at,
                }
            })
            .collect();
        let index = stats.iter().enumerate().map(|(i, s)| (s.id.clone(), i)).collect();
        Snapshot { t, stats, index }
    }

    /// Every metric, with points only where `with_points(id)` says so.
    pub fn series(&self, with_points: impl Fn(&str) -> bool) -> Vec<Series> {
        self.series
            .iter()
            .map(|b| {
                let (min, max, avg) = b.stats();
                let points = if with_points(&b.id) { b.points.iter().copied().collect() } else { Vec::new() };
                Series {
                    id: b.id.clone(),
                    group: b.group.clone(),
                    label: b.label.clone(),
                    unit: b.unit,
                    points,
                    current: b.current,
                    min,
                    max,
                    avg,
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------- engine

struct Inner {
    history: Mutex<History>,
    snap: Mutex<Arc<Snapshot>>,
    period_ms: AtomicU64,
    lease: Duration,
    timeout: Duration,
    leases: Mutex<HashMap<String, (Vec<String>, Instant)>>,
    /// Read every source regardless of leases (CSV logging).
    always: AtomicBool,
    hooks: Mutex<Vec<Hook>>,
    /// Latest readings of each slow source, carried into fast ticks.
    slow_latest: Mutex<HashMap<&'static str, Vec<Reading>>>,
    failures: Mutex<HashMap<&'static str, String>>,
    /// Per-source read time: (reads, total).
    costs: Mutex<HashMap<&'static str, (u64, Duration)>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Inner {
    /// Does any live lease (or `always`) cover one of `prefixes`?
    fn wanted(&self, prefixes: &[&str]) -> bool {
        if self.always.load(Ordering::Relaxed) {
            return true;
        }
        let now = Instant::now();
        let mut leases = lock(&self.leases);
        leases.retain(|_, (_, at)| now.duration_since(*at) < self.lease);
        leases
            .values()
            .flat_map(|(ps, _)| ps)
            .any(|p| p.is_empty() || prefixes.iter().any(|s| s.starts_with(p.as_str()) || p.starts_with(s)))
    }

    /// Ask `a` to read, unless it is still busy or paused after a panic.
    fn request(&self, a: &mut Actor, now: Instant) {
        if a.busy.is_none() && a.paused_until.is_none_or(|u| now >= u) && a.req.send(()).is_ok() {
            a.busy = Some(now);
        }
    }

    /// `a`'s reply, waiting up to `wait` for it. A source still busy after the
    /// timeout is reported as not responding (and isn't asked again until it
    /// answers); a panic pauses it.
    fn collect(&self, a: &mut Actor, wait: Duration) -> Option<Vec<Reading>> {
        let since = a.busy?;
        let got = if wait.is_zero() { a.rep.try_recv().ok() } else { a.rep.recv_timeout(wait).ok() };
        let Some((r, took)) = got else {
            if since.elapsed() >= self.timeout {
                let msg = format!("not responding for {} s", since.elapsed().as_secs());
                lock(&self.failures).insert(a.name, msg);
            }
            return None;
        };
        a.busy = None;
        {
            let mut costs = lock(&self.costs);
            let e = costs.entry(a.name).or_default();
            (e.0, e.1) = (e.0 + 1, e.1 + took);
        }
        match r {
            Ok(mut out) => {
                lock(&self.failures).remove(a.name);
                out.retain(|r| r.value.is_finite());
                Some(out)
            }
            Err(msg) => {
                lock(&self.failures).insert(a.name, msg);
                a.paused_until = Some(Instant::now() + PANIC_BACKOFF);
                None
            }
        }
    }
}

/// The running engine. Cheap to clone; its workers stop when the last clone drops.
#[derive(Clone)]
pub struct Engine {
    inner: Arc<Inner>,
}

impl Engine {
    pub fn start(sources: Vec<Box<dyn Source>>, cfg: Config) -> Engine {
        let inner = Arc::new(Inner {
            history: Mutex::new(History::new(cfg.keep)),
            snap: Mutex::new(Arc::new(Snapshot::default())),
            period_ms: AtomicU64::new(clamp_period(cfg.period)),
            lease: cfg.lease,
            timeout: cfg.timeout,
            leases: Mutex::new(HashMap::new()),
            always: AtomicBool::new(false),
            hooks: Mutex::new(Vec::new()),
            slow_latest: Mutex::new(HashMap::new()),
            failures: Mutex::new(HashMap::new()),
            costs: Mutex::new(HashMap::new()),
        });
        let (fast, slow): (Vec<_>, Vec<_>) = sources.into_iter().map(actor).partition(|a| a.class == Class::Fast);
        let t0 = Instant::now();
        let w = Arc::downgrade(&inner);
        spawn("metrics-fast", move || fast_worker(w, fast, t0));
        let w = Arc::downgrade(&inner);
        spawn("metrics-slow", move || slow_worker(w, slow));
        Engine { inner }
    }

    /// Renew lease `key` for metrics under `prefixes` ("" = everything).
    pub fn want(&self, key: &str, prefixes: &[&str]) {
        let ps = prefixes.iter().map(|p| p.to_string()).collect();
        lock(&self.inner.leases).insert(key.to_string(), (ps, Instant::now()));
    }

    /// Read everything regardless of leases (on while a CSV log runs).
    pub fn set_always(&self, on: bool) {
        self.inner.always.store(on, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Arc<Snapshot> {
        lock(&self.inner.snap).clone()
    }

    /// Every metric with its statistics; points only where `with_points(id)`.
    pub fn series(&self, with_points: impl Fn(&str) -> bool) -> Vec<Series> {
        lock(&self.inner.history).series(with_points)
    }

    /// Kept points of one metric at or after engine time `since`.
    pub fn points(&self, id: &str, since: f64) -> Vec<(f64, f64)> {
        lock(&self.inner.history).points(id, since)
    }

    pub fn reset_stats(&self) {
        lock(&self.inner.history).reset_stats();
    }

    pub fn set_period(&self, period: Duration) {
        self.inner.period_ms.store(clamp_period(period), Ordering::Relaxed);
    }

    /// Change how much history is kept per metric.
    pub fn set_keep(&self, keep: Duration) {
        lock(&self.inner.history).set_keep(keep);
    }

    pub fn period(&self) -> Duration {
        Duration::from_millis(self.inner.period_ms.load(Ordering::Relaxed))
    }

    /// Run `h` after every tick that produced readings, on the sampler thread.
    pub fn add_hook(&self, h: Hook) {
        lock(&self.inner.hooks).push(h);
    }

    /// Average time per read of each source so far: (name, reads, mean),
    /// slowest first. The performance budget (P6) is checked against this.
    pub fn costs(&self) -> Vec<(&'static str, u64, Duration)> {
        let mut v: Vec<_> =
            lock(&self.inner.costs).iter().map(|(k, (n, t))| (*k, *n, *t / (*n).max(1) as u32)).collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.2));
        v
    }

    /// Sources that panicked recently: (name, message).
    pub fn failures(&self) -> Vec<(&'static str, String)> {
        let mut v: Vec<_> = lock(&self.inner.failures).iter().map(|(k, v)| (*k, v.clone())).collect();
        v.sort();
        v
    }
}

fn clamp_period(p: Duration) -> u64 {
    (p.as_millis() as u64).max(100)
}

fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
    if let Err(e) = std::thread::Builder::new().name(name.into()).spawn(f) {
        eprintln!("{name}: {e}");
    }
}

/// One source on its own thread, so a read that hangs or crashes can't stall
/// the others: the workers send requests and wait for replies with a deadline.
struct Actor {
    name: &'static str,
    prefixes: &'static [&'static str],
    class: Class,
    interval: Duration,
    req: std::sync::mpsc::Sender<()>,
    rep: std::sync::mpsc::Receiver<(Result<Vec<Reading>, String>, Duration)>,
    /// When the pending request was sent.
    busy: Option<Instant>,
    /// When a slow source was last asked.
    last: Option<Instant>,
    paused_until: Option<Instant>,
}

fn panic_message(e: Box<dyn std::any::Any + Send>) -> String {
    e.downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| e.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "panicked".into())
}

fn actor(mut s: Box<dyn Source>) -> Actor {
    let (req, req_rx) = std::sync::mpsc::channel::<()>();
    let (rep_tx, rep) = std::sync::mpsc::channel();
    let (name, prefixes, class, interval) = (s.name(), s.prefixes(), s.class(), s.interval());
    // The thread ends once the worker drops `req` (engine stopped).
    spawn(&format!("metrics-{name}"), move || {
        while req_rx.recv().is_ok() {
            let mut out = Vec::new();
            let t = Instant::now();
            let r = catch_unwind(AssertUnwindSafe(|| s.read(&mut out))).map(|()| out).map_err(panic_message);
            if rep_tx.send((r, t.elapsed())).is_err() {
                return;
            }
        }
    });
    Actor { name, prefixes, class, interval, req, rep, busy: None, last: None, paused_until: None }
}

fn fast_worker(weak: Weak<Inner>, mut actors: Vec<Actor>, t0: Instant) {
    loop {
        let start = Instant::now();
        let Some(inner) = weak.upgrade() else { return };
        let period = Duration::from_millis(inner.period_ms.load(Ordering::Relaxed));
        for a in &mut actors {
            if inner.wanted(a.prefixes) {
                inner.request(a, start);
            }
        }
        // Wait for replies within this tick; a late one is used next tick.
        let deadline = start + period.mul_f32(0.9);
        let mut rs = Vec::new();
        for a in &mut actors {
            if let Some(out) = inner.collect(a, deadline.saturating_duration_since(Instant::now())) {
                rs.extend(out);
            }
        }
        rs.extend(lock(&inner.slow_latest).values().flatten().cloned());
        if !rs.is_empty() {
            let t = t0.elapsed().as_secs_f64();
            let mut h = lock(&inner.history);
            h.push(t, &rs);
            *lock(&inner.snap) = Arc::new(h.snapshot(t));
            let tick = Tick { t, wall: SystemTime::now(), readings: &rs, history: &h };
            for hook in lock(&inner.hooks).iter_mut() {
                hook(&tick);
            }
        }
        drop(inner);
        std::thread::sleep(period.saturating_sub(start.elapsed()));
    }
}

fn slow_worker(weak: Weak<Inner>, mut actors: Vec<Actor>) {
    loop {
        let Some(inner) = weak.upgrade() else { return };
        let now = Instant::now();
        for a in &mut actors {
            if !inner.wanted(a.prefixes) {
                // Nobody looks: don't carry stale values into new ticks.
                lock(&inner.slow_latest).remove(a.name);
                a.last = None;
            } else if a.last.is_none_or(|t| now.duration_since(t) >= a.interval) {
                a.last = Some(now);
                inner.request(a, now);
            }
            if let Some(out) = inner.collect(a, Duration::ZERO) {
                lock(&inner.slow_latest).insert(a.name, out);
            }
        }
        drop(inner);
        std::thread::sleep(SLOW_POLL);
    }
}

// ---------------------------------------------------------------- CSV log

/// CSV log: a header line `time,<id1>,<id2>,…`, then a comment line
/// `# units,<unit1>,…`, then one row per tick with an ISO-8601 UTC timestamp
/// and empty cells for metrics missing from that tick. When a new metric
/// appears, a fresh header + units block is written with the old columns in the
/// same positions and the new ones appended, so a reader restarts its column map
/// on every line that starts with `time,`. Each row goes to the OS immediately.
pub struct Csv {
    pub path: PathBuf,
    file: std::fs::File,
    cols: Vec<(String, Unit)>,
}

impl Csv {
    pub fn create(path: &std::path::Path) -> std::io::Result<Csv> {
        if let Some(d) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(d)?;
        }
        let file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Csv { path: path.to_path_buf(), file, cols: Vec::new() })
    }

    pub fn write(&mut self, wall: SystemTime, rs: &[Reading]) -> std::io::Result<()> {
        let known: std::collections::HashSet<&str> = self.cols.iter().map(|c| c.0.as_str()).collect();
        let new: Vec<(String, Unit)> =
            rs.iter().filter(|r| !known.contains(r.id.as_str())).map(|r| (r.id.clone(), r.unit)).collect();
        let mut s = String::new();
        if !new.is_empty() || self.cols.is_empty() {
            self.cols.extend(new);
            let ids: Vec<&str> = self.cols.iter().map(|c| c.0.as_str()).collect();
            let units: Vec<&str> = self.cols.iter().map(|c| c.1.suffix()).collect();
            s += &format!("time,{}\n# units,{}\n", ids.join(","), units.join(","));
        }
        let by: HashMap<&str, f64> = rs.iter().map(|r| (r.id.as_str(), r.value)).collect();
        s += &iso8601(wall);
        for (id, _) in &self.cols {
            s.push(',');
            if let Some(v) = by.get(id.as_str()) {
                s += &format!("{v:.3}");
            }
        }
        s.push('\n');
        self.file.write_all(s.as_bytes())
    }
}

/// UTC "2026-10-14T12:34:56.789Z" (days-from-civil inverse, Howard Hinnant's algorithm).
pub fn iso8601(t: SystemTime) -> String {
    let d = t.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    let (secs, ms) = (d.as_secs() as i64, d.subsec_millis());
    let (days, sod) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{ms:03}Z", sod / 3600, sod / 60 % 60, sod % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn r(id: &str, unit: Unit, value: f64) -> Reading {
        Reading { id: id.into(), group: "CPU".into(), label: id.into(), unit, value }
    }

    #[test]
    fn history_bounded_by_keep_stats_since_start() {
        let mut h = History::new(Duration::from_secs(10));
        for t in 0..=30 {
            h.push(t as f64, &[r("x", Unit::Celsius, t as f64)]);
        }
        let s = &h.series(|_| true)[0];
        assert_eq!(s.points.len(), 11);
        assert_eq!(s.points[0], (20.0, 20.0));
        assert_eq!((s.min, s.max, s.avg, s.current), (0.0, 30.0, 15.0, 30.0));
        assert_eq!(h.points("x", 28.0), [(28.0, 28.0), (29.0, 29.0), (30.0, 30.0)]);
        assert!(h.series(|_| false)[0].points.is_empty(), "points only on request");
        let snap = h.snapshot(30.0);
        assert_eq!(snap.get("x").map(|s| (s.current, s.at)), Some((30.0, 30.0)));
        h.set_keep(Duration::from_secs(2));
        h.push(31.0, &[r("x", Unit::Celsius, 31.0)]);
        assert_eq!(h.points("x", 0.0).len(), 3, "a shorter window trims on the next push");
    }

    /// Performance budget (P6, S0-T08): the per-tick work the engine itself
    /// adds (history push + snapshot) for 300 metrics with 10 minutes of history.
    #[test]
    fn tick_overhead_budget() {
        let mut h = History::new(Duration::from_secs(600));
        let rs: Vec<Reading> = (0..300).map(|i| r(&format!("cpu/temp/s{i}"), Unit::Celsius, i as f64)).collect();
        for t in 0..600 {
            h.push(t as f64, &rs); // fill the window first
        }
        let t0 = Instant::now();
        let n = 50;
        for t in 600..600 + n {
            h.push(t as f64, &rs);
            let _ = h.snapshot(t as f64);
        }
        let per_tick = t0.elapsed() / n;
        // Debug build on a shared CI runner; a release build is ~10x faster.
        assert!(per_tick < Duration::from_millis(20), "{per_tick:?} per tick");
    }

    #[test]
    fn stats_reset_keeps_graph() {
        let mut h = History::new(Duration::from_secs(100));
        for (t, v) in [1.0, 5.0, 3.0].into_iter().enumerate() {
            h.push(t as f64, &[r("x", Unit::Volt, v)]);
        }
        assert_eq!(h.snapshot(2.0).get("x").map(|s| (s.min, s.max, s.avg)), Some((1.0, 5.0, 3.0)));
        h.reset_stats();
        assert_eq!(h.snapshot(2.0).get("x").map(|s| (s.min, s.max, s.avg)), Some((3.0, 3.0, 3.0)));
        h.push(3.0, &[r("x", Unit::Volt, 4.0)]);
        let s = &h.series(|_| true)[0];
        assert_eq!((s.min, s.max, s.avg, s.points.len()), (4.0, 4.0, 4.0, 4));
    }

    struct Fake {
        name: &'static str,
        prefixes: &'static [&'static str],
        class: Class,
        interval: Duration,
        reads: Arc<AtomicUsize>,
        panic: bool,
    }

    impl Source for Fake {
        fn name(&self) -> &'static str {
            self.name
        }
        fn prefixes(&self) -> &'static [&'static str] {
            self.prefixes
        }
        fn class(&self) -> Class {
            self.class
        }
        fn interval(&self) -> Duration {
            self.interval
        }
        fn read(&mut self, out: &mut Vec<Reading>) {
            let n = self.reads.fetch_add(1, Ordering::SeqCst);
            assert!(!self.panic, "probe failed");
            out.push(r(&format!("{}value", self.prefixes[0]), Unit::Celsius, n as f64));
        }
    }

    fn fake(
        name: &'static str,
        prefixes: &'static [&'static str],
        class: Class,
    ) -> (Box<dyn Source>, Arc<AtomicUsize>) {
        let reads = Arc::new(AtomicUsize::new(0));
        let f =
            Fake { name, prefixes, class, interval: Duration::from_millis(400), reads: reads.clone(), panic: false };
        (Box::new(f), reads)
    }

    fn quick() -> Config {
        Config {
            period: Duration::from_millis(100),
            keep: Duration::from_secs(60),
            lease: Duration::from_millis(400),
            timeout: Duration::from_millis(500),
        }
    }

    /// A probe that hangs (a driver that never answers).
    struct Stuck;

    impl Source for Stuck {
        fn name(&self) -> &'static str {
            "stuck"
        }
        fn prefixes(&self) -> &'static [&'static str] {
            &["board/"]
        }
        fn read(&mut self, _out: &mut Vec<Reading>) {
            std::thread::sleep(Duration::from_secs(3));
        }
    }

    #[test]
    fn a_stuck_source_does_not_stall_the_others() {
        let (cpu, cpu_reads) = fake("cpu", &["cpu/"], Class::Fast);
        let e = Engine::start(vec![Box::new(Stuck), cpu], quick());
        e.set_always(true);
        wait_until("cpu keeps ticking", || cpu_reads.load(Ordering::SeqCst) >= 8);
        wait_until("stuck reported", || {
            e.failures().iter().any(|f| f.0 == "stuck" && f.1.starts_with("not responding"))
        });
    }

    fn wait_until(what: &str, f: impl Fn() -> bool) {
        let t = Instant::now();
        while !f() {
            assert!(t.elapsed() < Duration::from_secs(5), "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn reads_only_what_a_live_lease_covers() {
        let (cpu, cpu_reads) = fake("cpu", &["cpu/"], Class::Fast);
        let (gpu, gpu_reads) = fake("gpu", &["gpu"], Class::Fast);
        let e = Engine::start(vec![cpu, gpu], quick());
        std::thread::sleep(Duration::from_millis(350));
        assert_eq!(cpu_reads.load(Ordering::SeqCst) + gpu_reads.load(Ordering::SeqCst), 0, "no lease, no reads");
        e.want("overview", &["cpu/temp"]);
        wait_until("cpu reads", || cpu_reads.load(Ordering::SeqCst) >= 2);
        assert_eq!(gpu_reads.load(Ordering::SeqCst), 0, "gpu not covered by the lease");
        assert!(e.snapshot().get("cpu/value").is_some());
        // The lease lapses without renewal.
        std::thread::sleep(Duration::from_millis(600));
        let n = cpu_reads.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(350));
        assert_eq!(cpu_reads.load(Ordering::SeqCst), n, "lapsed lease stops reads");
        // "" covers everything; so does `always`.
        e.want("sensors", &[""]);
        wait_until("gpu reads", || gpu_reads.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn slow_sources_run_apart_and_are_carried() {
        let (fast, _) = fake("cpu", &["cpu/"], Class::Fast);
        let (slow, slow_reads) = fake("storage", &["storage/"], Class::Slow);
        let e = Engine::start(vec![fast, slow], quick());
        e.set_always(true);
        wait_until("carried slow value", || e.points("storage/value", 0.0).len() >= 3);
        // Carried into fast ticks (≈ every 100 ms), read at most every 400 ms.
        let ticks = e.points("storage/value", 0.0).len();
        assert!(
            slow_reads.load(Ordering::SeqCst) < ticks,
            "{} reads for {ticks} ticks",
            slow_reads.load(Ordering::SeqCst)
        );
    }

    #[test]
    fn a_panicking_source_is_paused_and_reported() {
        let reads = Arc::new(AtomicUsize::new(0));
        let bad = Fake {
            name: "bad",
            prefixes: &["board/"],
            class: Class::Fast,
            interval: Duration::ZERO,
            reads: reads.clone(),
            panic: true,
        };
        let (good, good_reads) = fake("cpu", &["cpu/"], Class::Fast);
        let e = Engine::start(vec![Box::new(bad), good], quick());
        e.set_always(true);
        wait_until("good reads", || good_reads.load(Ordering::SeqCst) >= 3);
        assert_eq!(reads.load(Ordering::SeqCst), 1, "paused after the panic");
        assert_eq!(e.failures(), [("bad", "probe failed".to_string())]);
    }

    #[test]
    fn hooks_see_each_tick_and_the_peak() {
        let (cpu, _) = fake("cpu", &["cpu/"], Class::Fast);
        let e = Engine::start(vec![cpu], quick());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = seen.clone();
        e.add_hook(Box::new(move |t: &Tick| {
            lock(&s).push((t.readings.len(), t.peak("cpu/value")));
        }));
        e.set_always(true);
        wait_until("hook calls", || lock(&seen).len() >= 3);
        let v = lock(&seen);
        assert!(v.iter().all(|(n, _)| *n == 1));
        assert!(v.windows(2).all(|w| w[1].1 >= w[0].1), "peak is monotonic for a rising value: {v:?}");
    }

    #[test]
    fn engine_stops_with_its_last_handle() {
        let (cpu, reads) = fake("cpu", &["cpu/"], Class::Fast);
        let e = Engine::start(vec![cpu], quick());
        e.set_always(true);
        wait_until("reads", || reads.load(Ordering::SeqCst) >= 1);
        drop(e);
        std::thread::sleep(Duration::from_millis(250));
        let n = reads.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(reads.load(Ordering::SeqCst), n);
    }

    #[test]
    fn timestamps() {
        let at = |s: u64, ms: u64| iso8601(SystemTime::UNIX_EPOCH + Duration::from_millis(s * 1000 + ms));
        assert_eq!(at(0, 0), "1970-01-01T00:00:00.000Z");
        assert_eq!(at(1_700_000_000, 42), "2023-11-14T22:13:20.042Z");
        assert_eq!(at(951_782_400, 0), "2000-02-29T00:00:00.000Z");
    }

    #[test]
    fn csv_header_rows_and_new_columns() {
        let path = std::env::temp_dir().join(format!("cpuzeez_metrics_{}.csv", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut c = Csv::create(&path).unwrap();
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        c.write(t, &[r("cpu/temp/package", Unit::Celsius, 45.5), r("cpu/volt/vid", Unit::Volt, 1.1)]).unwrap();
        c.write(t, &[r("cpu/volt/vid", Unit::Volt, 1.2)]).unwrap();
        c.write(t, &[r("cpu/temp/package", Unit::Celsius, 46.0), r("gpu0/temp/core", Unit::Celsius, 50.0)]).unwrap();
        drop(c);
        let got = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        let ts = "2023-11-14T22:13:20.000Z";
        let want = format!(
            "time,cpu/temp/package,cpu/volt/vid\n# units,°C,V\n{ts},45.500,1.100\n{ts},,1.200\n\
             time,cpu/temp/package,cpu/volt/vid,gpu0/temp/core\n# units,°C,V,°C\n{ts},46.000,,50.000\n"
        );
        assert_eq!(got, want);
    }
}

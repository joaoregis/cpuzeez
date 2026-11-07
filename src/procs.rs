//! Live process sampler for the System hub (S2a-01, S2a-02) and the bottleneck
//! detector (S2c).
//!
//! Runs on its own background thread only while a consumer calls [`Sampler::want`]
//! (e.g. the System › Processes page), so hidden pages cost nothing. Path, user
//! and version-info lookups are cached by `(pid, start_ms)` and `path` so each
//! process is queried once, never every tick.

use cpuzeez_core::gpu::engines;
use cpuzeez_core::procs::{self, HISTORY, Row, Totals};
use cpuzeez_platform::{desktop, pdh, procs as os};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
pub struct Snapshot {
    pub rows: Vec<Row>,
    pub totals: Totals,
    pub windows: Vec<desktop::Window>,
    /// Whether per-process network counters are available.
    pub net: cpuzeez_platform::proc_net::Status,
}

/// Background process sampler. Cheap to clone; stops when dropped.
#[derive(Clone)]
pub struct Sampler {
    latest: Arc<Mutex<Arc<Snapshot>>>,
    until_ms: Arc<AtomicU64>,
    period_ms: Arc<AtomicU64>,
    paused: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    epoch: Instant,
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl Sampler {
    pub fn new(period_ms: u64) -> Self {
        let latest = Arc::new(Mutex::new(Arc::new(Snapshot::default())));
        let epoch = Instant::now();
        let until_ms = Arc::new(AtomicU64::new(0));
        let period = Arc::new(AtomicU64::new(period_ms.clamp(250, 10_000)));
        let paused = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let s = Sampler {
            latest: latest.clone(),
            until_ms: until_ms.clone(),
            period_ms: period.clone(),
            paused: paused.clone(),
            stop: stop.clone(),
            epoch,
        };
        std::thread::Builder::new()
            .name("cz-procs".into())
            .spawn(move || worker(epoch, latest, until_ms, period, paused, stop))
            .ok();
        s
    }

    /// Keep sampling for the next 3 seconds. Call every frame from whatever shows processes.
    pub fn want(&self) {
        let now = self.epoch.elapsed().as_millis() as u64;
        self.until_ms.store(now + 3000, Ordering::Relaxed);
    }

    pub fn set_period_ms(&self, ms: u64) {
        self.period_ms.store(ms.clamp(250, 10_000), Ordering::Relaxed);
    }

    pub fn period_ms(&self) -> u64 {
        self.period_ms.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, on: bool) {
        self.paused.store(on, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.latest.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        if Arc::strong_count(&self.stop) == 2 {
            self.stop.store(true, Ordering::Relaxed);
        }
    }
}

#[derive(Default)]
struct Prev {
    cpu_ns: u64,
    read_bytes: u64,
    write_bytes: u64,
    threads: HashMap<u32, u64>,
    cpu_history: std::collections::VecDeque<f32>,
    mem_history: std::collections::VecDeque<u64>,
    power_history: std::collections::VecDeque<f32>,
    net: Option<(u64, u64)>,
    gui: Option<(u32, u32)>,
}

#[derive(Default)]
struct Meta {
    path: String,
    user: String,
    publisher: String,
    description: String,
    command_line: String,
    token: os::Token,
}

pub(crate) struct Collector {
    prev: HashMap<u64, Prev>,
    prev_at: Option<Instant>,
    meta: HashMap<u64, Meta>,
    ver_cache: HashMap<String, (String, String)>,
    gpu_util: pdh::Array,
    gpu_mem: pdh::Array,
    logical_cpus: usize,
    windows_dir: String,
    net: cpuzeez_platform::proc_net::Status,
    tick: u64,
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector {
    pub fn new() -> Self {
        let logical_cpus = std::thread::available_parallelism().map_or(1, usize::from);
        let windows_dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        Collector {
            prev: HashMap::new(),
            prev_at: None,
            meta: HashMap::new(),
            ver_cache: HashMap::new(),
            gpu_util: pdh::Array::new(r"\GPU Engine(*)\Utilization Percentage"),
            gpu_mem: pdh::Array::new(r"\GPU Process Memory(*)\Dedicated Usage"),
            logical_cpus,
            windows_dir,
            net: cpuzeez_platform::proc_net::start(),
            tick: 0,
        }
    }

    pub fn step(&mut self) -> Snapshot {
        let now = Instant::now();
        let wall_s = self.prev_at.map_or(0.0, |t| (now - t).as_secs_f64());
        self.prev_at = Some(now);

        let raw = os::snapshot(true);
        let wins = desktop::windows();
        let mut win_by_pid: BTreeMap<u32, (&str, bool)> = BTreeMap::new();
        for w in &wins {
            win_by_pid.entry(w.pid).and_modify(|e| e.1 |= w.hung).or_insert((w.title.as_str(), w.hung));
        }

        let gpu_raw = self.gpu_util.read();
        let gpu_by_pid: HashMap<u32, (f64, String)> =
            engines::per_process(gpu_raw.iter().map(|(k, v)| (k.as_str(), *v)))
                .into_iter()
                .map(|g| (g.pid, (g.percent, g.engine)))
                .collect();
        let vram_raw = self.gpu_mem.read();
        let vram_by_pid = engines::per_process_memory(vram_raw.iter().map(|(k, v)| (k.as_str(), *v)));

        let net_on = self.net == cpuzeez_platform::proc_net::Status::Running;
        let net_now = if net_on { cpuzeez_platform::proc_net::totals() } else { HashMap::new() };
        // GDI/USER counts need a handle per process: every 5th tick is plenty.
        self.tick += 1;
        let gui_now = cfg!(windows) && self.tick % 5 == 1;

        let mut next_prev = HashMap::with_capacity(raw.len());
        // Budget new metadata lookups per tick so the first snapshot stays fast.
        let mut meta_budget = 40usize;
        let mut rows = Vec::with_capacity(raw.len());

        for p in raw {
            // System Idle Process (PID 0 on Windows) isn't a real process row.
            if p.pid == 0 {
                continue;
            }
            let key = (p.pid as u64) << 32 ^ (p.start_ms & 0xFFFF_FFFF);
            let total_ns = p.user_ns.saturating_add(p.kernel_ns);
            let old = self.prev.remove(&key);

            let net_total = net_on.then(|| net_now.get(&p.pid).copied().unwrap_or_default());
            let (send_bps, recv_bps) = match (net_total, old.as_ref().and_then(|o| o.net)) {
                (Some(n), Some(o)) if wall_s > 0.0 => {
                    (Some(procs::rate(o.0, n.0, wall_s)), Some(procs::rate(o.1, n.1, wall_s)))
                }
                (Some(_), _) => (Some(0.0), Some(0.0)),
                _ => (None, None),
            };
            let gui = if gui_now { os::gui_objects(p.pid) } else { old.as_ref().and_then(|o| o.gui) };
            let mut power_hist = old.as_ref().map(|o| o.power_history.clone()).unwrap_or_default();

            let (cpu, r_bps, w_bps, top_thread, cpu_hist, mem_hist) = match old {
                Some(mut o) if wall_s > 0.0 => {
                    let cpu = procs::cpu_percent(total_ns.saturating_sub(o.cpu_ns), wall_s, self.logical_cpus);
                    let r_bps = procs::rate(o.read_bytes, p.read_bytes, wall_s);
                    let w_bps = procs::rate(o.write_bytes, p.write_bytes, wall_s);
                    let top = p
                        .thread_list
                        .iter()
                        .filter_map(|t| {
                            let prev_t = o.threads.get(&t.tid).copied().unwrap_or(t.cpu_ns);
                            let pct = procs::cpu_percent(t.cpu_ns.saturating_sub(prev_t), wall_s, 1);
                            (pct > 0.0).then_some((t.tid, pct))
                        })
                        .max_by(|a, b| a.1.total_cmp(&b.1));
                    if o.cpu_history.len() >= HISTORY {
                        o.cpu_history.pop_front();
                    }
                    o.cpu_history.push_back(cpu as f32);
                    if o.mem_history.len() >= HISTORY {
                        o.mem_history.pop_front();
                    }
                    o.mem_history.push_back(p.private);
                    (cpu, r_bps, w_bps, top, o.cpu_history, o.mem_history)
                }
                _ => {
                    let mut ch = std::collections::VecDeque::new();
                    ch.push_back(0.0);
                    let mut mh = std::collections::VecDeque::new();
                    mh.push_back(p.private);
                    (0.0, 0.0, 0.0, None, ch, mh)
                }
            };

            let threads_map: HashMap<u32, u64> = p.thread_list.iter().map(|t| (t.tid, t.cpu_ns)).collect();
            next_prev.insert(
                key,
                Prev {
                    cpu_ns: total_ns,
                    read_bytes: p.read_bytes,
                    write_bytes: p.write_bytes,
                    threads: threads_map,
                    cpu_history: cpu_hist.clone(),
                    mem_history: mem_hist.clone(),
                    power_history: Default::default(),
                    net: net_total,
                    gui,
                },
            );

            if !self.meta.contains_key(&key) && meta_budget > 0 {
                meta_budget -= 1;
                let path = os::path(p.pid).unwrap_or_default();
                let user = os::user(p.pid).unwrap_or_default();
                let (publisher, description) = if path.is_empty() {
                    (String::new(), String::new())
                } else if let Some(v) = self.ver_cache.get(&path) {
                    v.clone()
                } else {
                    let v = os::version_info(&path).unwrap_or_default();
                    self.ver_cache.insert(path.clone(), v.clone());
                    v
                };
                let command_line = os::command_line(p.pid).unwrap_or_default();
                let token = os::token(p.pid, p.uid);
                self.meta.insert(key, Meta { path, user, publisher, description, command_line, token });
            }
            let m = self.meta.get(&key);
            let path = m.map(|m| m.path.clone()).unwrap_or_default();
            let user = m.map(|m| m.user.clone()).unwrap_or_default();
            let publisher = m.map(|m| m.publisher.clone()).unwrap_or_default();
            let description = m.map(|m| m.description.clone()).unwrap_or_default();
            let command_line = m.map(|m| m.command_line.clone()).unwrap_or_default();
            let token = m.map(|m| m.token.clone()).unwrap_or_default();

            let win = win_by_pid.get(&p.pid).copied();
            let kind = Some(procs::classify(p.pid, &path, win.is_some(), &self.windows_dir));
            let (gpu, gpu_engine) = gpu_by_pid.get(&p.pid).cloned().unwrap_or((0.0, String::new()));
            let vram = vram_by_pid.get(&p.pid).copied().unwrap_or(0);
            let net_bps = send_bps.unwrap_or(0.0) + recv_bps.unwrap_or(0.0);
            let power = procs::power_score(cpu, gpu, r_bps + w_bps, net_bps);
            if power_hist.len() >= HISTORY {
                power_hist.pop_front();
            }
            power_hist.push_back(power as f32);
            let power_trend = power_hist.iter().map(|&v| v as f64).sum::<f64>() / power_hist.len() as f64;
            if let Some(pv) = next_prev.get_mut(&key) {
                pv.power_history = power_hist;
            }

            rows.push(Row {
                pid: p.pid,
                ppid: p.ppid,
                start_ms: p.start_ms,
                name: p.name,
                description,
                publisher,
                path,
                user,
                kind,
                depth: 0,
                window: win.map(|(t, _)| t.to_string()),
                not_responding: win.is_some_and(|(_, hung)| hung),
                suspended: p.suspended,
                efficiency: p.priority <= 4 && p.pid > 4,
                cpu,
                cpu_time_s: total_ns as f64 / 1e9,
                working_set: p.working_set,
                private: p.private,
                peak_working_set: p.peak_working_set,
                paged_pool: p.paged_pool,
                nonpaged_pool: p.nonpaged_pool,
                page_faults: p.page_faults,
                disk_read_bps: r_bps,
                disk_write_bps: w_bps,
                gpu,
                gpu_engine,
                vram,
                threads: p.threads,
                handles: p.handles,
                priority: p.priority,
                session: p.session,
                cpu_history: cpu_hist,
                mem_history: mem_hist,
                top_thread,
                command_line,
                arch: token.arch,
                elevated: token.elevated,
                integrity: token.integrity,
                virtualization: token.virtualization,
                io_reads: p.read_ops,
                io_writes: p.write_ops,
                gdi_objects: gui.map(|g| g.0),
                user_objects: gui.map(|g| g.1),
                net_send_bps: send_bps,
                net_recv_bps: recv_bps,
                power,
                power_trend,
            });
        }

        self.prev = next_prev;
        self.meta.retain(|k, _| self.prev.contains_key(k));
        let totals = procs::totals(&rows);
        Snapshot { rows, totals, windows: wins, net: self.net }
    }
}

fn worker(
    epoch: Instant,
    latest: Arc<Mutex<Arc<Snapshot>>>,
    until_ms: Arc<AtomicU64>,
    period_ms: Arc<AtomicU64>,
    paused: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) {
    let mut col: Option<Collector> = None;
    while !stop.load(Ordering::Relaxed) {
        let now = epoch.elapsed().as_millis() as u64;
        let active = now < until_ms.load(Ordering::Relaxed);
        if active && !paused.load(Ordering::Relaxed) {
            let c = col.get_or_insert_with(|| {
                let mut c = Collector::new();
                // Prime the first snapshot so rates work on the next tick.
                let first = Arc::new(c.step());
                *latest.lock().unwrap_or_else(|e| e.into_inner()) = first;
                std::thread::sleep(Duration::from_millis(120));
                c
            });
            let snap = Arc::new(c.step());
            *latest.lock().unwrap_or_else(|e| e.into_inner()) = snap;
            let wait = period_ms.load(Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(wait));
        } else {
            std::thread::sleep(Duration::from_millis(150));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collector_builds_rows_and_rates() {
        let mut c = Collector::new();
        let s1 = c.step();
        assert!(!s1.rows.is_empty());
        std::thread::sleep(Duration::from_millis(80));
        let s2 = c.step();
        let me = std::process::id();
        let us = s2.rows.iter().find(|r| r.pid == me).expect("our process in snapshot");
        assert!(us.working_set > 0 && us.threads >= 1);
        assert_eq!(s2.totals.processes, s2.rows.len());
    }
}

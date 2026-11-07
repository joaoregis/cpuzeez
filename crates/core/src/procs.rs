//! The System hub's process model (S2a), pure so it's tested everywhere:
//! rates from two snapshots, the Task Manager's grouping (Apps / Background /
//! Windows processes), the parent → child tree, the "Problems" filters and
//! the list of processes whose end crashes or reboots the OS.

use std::collections::{BTreeMap, VecDeque};

/// The Task Manager's three groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    App,
    Background,
    System,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::App => "Apps",
            Kind::Background => "Background processes",
            Kind::System => "Windows processes",
        }
    }
}

/// Samples kept per process for the 60 s sparklines and the trend filters.
pub const HISTORY: usize = 120;

/// One process as the table shows it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Row {
    pub pid: u32,
    pub ppid: u32,
    pub start_ms: u64,
    pub name: String,
    pub description: String,
    pub publisher: String,
    pub path: String,
    pub user: String,
    pub kind: Option<Kind>,
    /// Depth in the tree (0 = no parent shown).
    pub depth: usize,
    pub window: Option<String>,
    pub not_responding: bool,
    pub suspended: bool,
    pub efficiency: bool,
    /// % of the whole machine (like the Task Manager).
    pub cpu: f64,
    pub cpu_time_s: f64,
    pub working_set: u64,
    pub private: u64,
    pub peak_working_set: u64,
    pub paged_pool: u64,
    pub nonpaged_pool: u64,
    pub page_faults: u64,
    pub disk_read_bps: f64,
    pub disk_write_bps: f64,
    pub gpu: f64,
    pub gpu_engine: String,
    pub vram: u64,
    pub threads: u32,
    pub handles: u32,
    pub priority: i32,
    pub session: Option<u32>,
    /// Recent CPU % (oldest first).
    pub cpu_history: VecDeque<f32>,
    /// Recent private bytes (oldest first), for the leak trend.
    pub mem_history: VecDeque<u64>,
    /// The heaviest thread's CPU % and id (for the bottleneck detector).
    pub top_thread: Option<(u32, f64)>,
    pub command_line: String,
    /// "x64", "x86", "ARM64", "x86_64"… (empty when unknown).
    pub arch: String,
    /// Windows: elevated token. Unix: root.
    pub elevated: Option<bool>,
    /// Windows integrity level (empty elsewhere).
    pub integrity: String,
    /// Windows UAC virtualization (None elsewhere).
    pub virtualization: Option<bool>,
    /// I/O operations since the process started (Linux: read/write syscalls).
    pub io_reads: u64,
    pub io_writes: u64,
    /// GDI and USER objects (Windows).
    pub gdi_objects: Option<u32>,
    pub user_objects: Option<u32>,
    /// Network bytes/s; None when the OS gives no per-process counters (or needs admin).
    pub net_send_bps: Option<f64>,
    pub net_recv_bps: Option<f64>,
    /// [`power_score`] now and averaged over the history window.
    pub power: f64,
    pub power_trend: f64,
}

impl Row {
    pub fn disk_bps(&self) -> f64 {
        self.disk_read_bps + self.disk_write_bps
    }

    /// The process with the PID only means this one while the start time matches.
    pub fn key(&self) -> u64 {
        (self.pid as u64) << 32 ^ (self.start_ms & 0xFFFF_FFFF)
    }
}

/// CPU % of the whole machine from the CPU time used over `wall_s` seconds.
pub fn cpu_percent(cpu_ns_delta: u64, wall_s: f64, logical_cpus: usize) -> f64 {
    if wall_s <= 0.0 || logical_cpus == 0 {
        return 0.0;
    }
    (cpu_ns_delta as f64 / 1e9 / wall_s / logical_cpus as f64 * 100.0).clamp(0.0, 100.0)
}

/// An energy-impact estimate, 0–100, from what the process makes the machine
/// do (the OS has no per-process watts outside Windows' internal estimator):
/// CPU dominates, then GPU, disk and network.
pub fn power_score(cpu_pct: f64, gpu_pct: f64, disk_bps: f64, net_bps: f64) -> f64 {
    const MB: f64 = 1024.0 * 1024.0;
    (cpu_pct + 0.6 * gpu_pct + 10.0 * disk_bps / (50.0 * MB) + 5.0 * net_bps / (10.0 * MB)).clamp(0.0, 100.0)
}

/// Task Manager's power-usage words for a [`power_score`].
pub fn power_label(score: f64) -> &'static str {
    match score {
        s if s < 0.5 => "Very low",
        s if s < 5.0 => "Low",
        s if s < 15.0 => "Moderate",
        s if s < 35.0 => "High",
        _ => "Very high",
    }
}

/// Bytes per second from two counters.
pub fn rate(before: u64, after: u64, wall_s: f64) -> f64 {
    if wall_s <= 0.0 { 0.0 } else { after.saturating_sub(before) as f64 / wall_s }
}

/// Windows processes the OS can't live without: ending one is a blue screen
/// or an instant reboot. Linux/macOS: init, the display server, the kernel task.
pub fn critical(name: &str, pid: u32) -> bool {
    const WINDOWS: [&str; 7] =
        ["csrss.exe", "wininit.exe", "smss.exe", "lsass.exe", "services.exe", "winlogon.exe", "System"];
    const UNIX: [&str; 6] = ["launchd", "WindowServer", "kernel_task", "Xorg", "Xwayland", "systemd"];
    pid <= 4 || WINDOWS.iter().any(|n| n.eq_ignore_ascii_case(name)) || (pid == 1) || UNIX.contains(&name)
}

/// Anti-cheat services and the games they protect. CpuZeeZ never opens these
/// with more than the limited query right (P4): no memory dumps, no reads.
pub fn anti_cheat(name: &str) -> bool {
    const NAMES: [&str; 22] = [
        "vgc.exe",
        "vgtray.exe",
        "valorant-win64-shipping.exe",
        "riotclientservices.exe",
        "easyanticheat.exe",
        "easyanticheat_eos.exe",
        "beservice.exe",
        "beservice_x64.exe",
        "faceit.exe",
        "faceitservice.exe",
        "faceitclient.exe",
        "vanguard.exe",
        "eac_launcher.exe",
        "gameguard.erl",
        "nprotect.exe",
        "xigncode.exe",
        "ricochet.exe",
        "cs2.exe",
        "fortniteclient-win64-shipping.exe",
        "r5apex.exe",
        "leagueclient.exe",
        "league of legends.exe",
    ];
    NAMES.iter().any(|n| n.eq_ignore_ascii_case(name))
}

/// Rows in tree order: each process after its parent (when the parent is
/// alive and started first, so a reused PID isn't taken for a parent), with
/// `depth` set. Roots keep their incoming order.
pub fn tree(mut rows: Vec<Row>) -> Vec<Row> {
    let index: BTreeMap<u32, usize> = rows.iter().enumerate().map(|(i, r)| (r.pid, i)).collect();
    let parent_of = |r: &Row| -> Option<usize> {
        let &p = index.get(&r.ppid)?;
        (rows[p].pid != r.pid && rows[p].start_ms <= r.start_ms).then_some(p)
    };
    let parents: Vec<Option<usize>> = rows.iter().map(parent_of).collect();
    let mut children: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, p) in parents.iter().enumerate() {
        if let Some(p) = p {
            children.entry(*p).or_default().push(i);
        }
    }
    let mut order = Vec::with_capacity(rows.len());
    let mut seen = vec![false; rows.len()];
    fn walk(i: usize, d: usize, ch: &BTreeMap<usize, Vec<usize>>, order: &mut Vec<(usize, usize)>, seen: &mut [bool]) {
        if seen[i] || d > 64 {
            return;
        }
        seen[i] = true;
        order.push((i, d));
        for &c in ch.get(&i).into_iter().flatten() {
            walk(c, d + 1, ch, order, seen);
        }
    }
    for (i, p) in parents.iter().enumerate() {
        if p.is_none() {
            walk(i, 0, &children, &mut order, &mut seen);
        }
    }
    // Cycles (shouldn't happen): whatever is left, flat.
    for (i, s) in seen.iter().enumerate() {
        if !*s {
            order.push((i, 0));
        }
    }
    let mut slots: Vec<Option<Row>> = rows.drain(..).map(Some).collect();
    order
        .into_iter()
        .filter_map(|(i, d)| {
            let mut r = slots[i].take()?;
            r.depth = d;
            Some(r)
        })
        .collect()
}

/// Why a process shows under "Problems" (never alarmist: each says why).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    NotResponding,
    /// Private memory grew steadily over the kept history.
    MemoryGrowing,
    /// Above `HIGH_CPU` % for the last minute.
    HighCpu,
    /// Runs from a temporary folder.
    FromTemp,
}

impl Problem {
    pub fn reason(self) -> &'static str {
        match self {
            Problem::NotResponding => "Not responding",
            Problem::MemoryGrowing => "Memory keeps growing",
            Problem::HighCpu => "High CPU for over a minute",
            Problem::FromTemp => "Runs from a temporary folder",
        }
    }
}

/// % of the machine that counts as "high" for a single process.
pub const HIGH_CPU: f32 = 50.0;

pub fn problems(r: &Row, samples_per_min: usize) -> Vec<Problem> {
    let mut v = Vec::new();
    if r.not_responding {
        v.push(Problem::NotResponding);
    }
    let h = &r.mem_history;
    if h.len() >= 10 {
        let (first, last) = (h[0], h[h.len() - 1]);
        // Grew by 20 % and 64 MiB, never dropping much along the way.
        let monotone = h.iter().zip(h.iter().skip(1)).filter(|(a, b)| b < a).count() <= h.len() / 10;
        if last > first + (64 << 20) && last as f64 > first as f64 * 1.2 && monotone {
            v.push(Problem::MemoryGrowing);
        }
    }
    let n = samples_per_min.max(1);
    if r.cpu_history.len() >= n && r.cpu_history.iter().rev().take(n).all(|&c| c >= HIGH_CPU) {
        v.push(Problem::HighCpu);
    }
    let p = r.path.to_ascii_lowercase().replace('\\', "/");
    if p.contains("/appdata/local/temp/")
        || p.contains("/windows/temp/")
        || p.starts_with("/tmp/")
        || p.contains("/var/tmp/")
    {
        v.push(Problem::FromTemp);
    }
    v
}

/// Group a process the Task Manager's way: a visible window = App; the OS's
/// own (under the Windows folder or the kernel's) = System; the rest Background.
pub fn classify(pid: u32, path: &str, has_window: bool, windows_dir: &str) -> Kind {
    let p = path.to_ascii_lowercase();
    let w = windows_dir.to_ascii_lowercase();
    let os = pid <= 4 || (!w.is_empty() && p.starts_with(&w) && !p.contains("\\systemapps\\"));
    if has_window && !os {
        Kind::App
    } else if os || path.is_empty() && pid < 1000 {
        Kind::System
    } else if has_window {
        Kind::App
    } else {
        Kind::Background
    }
}

/// Totals for the status line and the Performance tab.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Totals {
    pub processes: usize,
    pub threads: u64,
    pub handles: u64,
}

pub fn totals(rows: &[Row]) -> Totals {
    Totals {
        processes: rows.len(),
        threads: rows.iter().map(|r| r.threads as u64).sum(),
        handles: rows.iter().map(|r| r.handles as u64).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, ppid: u32, start: u64) -> Row {
        Row { pid, ppid, start_ms: start, name: format!("p{pid}"), ..Default::default() }
    }

    #[test]
    fn power_estimate() {
        assert_eq!(power_label(power_score(0.0, 0.0, 0.0, 0.0)), "Very low");
        assert_eq!(power_label(power_score(2.0, 0.0, 0.0, 0.0)), "Low");
        assert_eq!(power_label(power_score(10.0, 5.0, 0.0, 0.0)), "Moderate");
        assert_eq!(power_label(power_score(30.0, 50.0, 0.0, 0.0)), "Very high");
        assert_eq!(power_score(500.0, 0.0, 0.0, 0.0), 100.0, "clamped");
        assert!(power_score(0.0, 0.0, 50.0 * 1048576.0, 0.0) > 5.0, "a busy disk counts");
    }

    #[test]
    fn rates() {
        // 1 s of CPU over 1 s on 4 CPUs = 25 % of the machine.
        assert_eq!(cpu_percent(1_000_000_000, 1.0, 4), 25.0);
        assert_eq!(cpu_percent(5, 0.0, 4), 0.0);
        assert_eq!(rate(100, 300, 2.0), 100.0);
        assert_eq!(rate(300, 100, 2.0), 0.0, "a counter going backwards is a new process");
    }

    #[test]
    fn tree_orders_children_after_parents() {
        let rows = vec![row(10, 1, 5), row(20, 10, 6), row(30, 20, 7), row(40, 10, 8), row(50, 99, 9)];
        let t: Vec<(u32, usize)> = tree(rows).iter().map(|r| (r.pid, r.depth)).collect();
        assert_eq!(t, [(10, 0), (20, 1), (30, 2), (40, 1), (50, 0)]);
        // A parent that started later is a reused PID, not the parent.
        let t: Vec<(u32, usize)> =
            tree(vec![row(10, 1, 50), row(20, 10, 6)]).iter().map(|r| (r.pid, r.depth)).collect();
        assert_eq!(t, [(10, 0), (20, 0)]);
        // Self-parenting (PID 0 on Windows) doesn't loop.
        assert_eq!(tree(vec![row(0, 0, 0)]).len(), 1);
    }

    #[test]
    fn problems_explain_themselves() {
        let mut r = row(5, 1, 0);
        r.not_responding = true;
        r.path = r"C:\Users\x\AppData\Local\Temp\setup.exe".into();
        r.cpu_history = std::iter::repeat_n(80.0, 60).collect();
        r.mem_history = (0..30).map(|i| (100 << 20) + i * (8 << 20)).collect();
        assert_eq!(
            problems(&r, 60),
            [Problem::NotResponding, Problem::MemoryGrowing, Problem::HighCpu, Problem::FromTemp]
        );
        let calm = row(6, 1, 0);
        assert!(problems(&calm, 60).is_empty());
        // A sawtooth (allocations freed again) isn't a leak.
        let mut saw = row(7, 1, 0);
        saw.mem_history = (0..30).map(|i| if i % 2 == 0 { 100 << 20 } else { 400 << 20 }).collect();
        assert!(!problems(&saw, 60).contains(&Problem::MemoryGrowing));
    }

    #[test]
    fn grouping_and_critical() {
        let w = r"C:\WINDOWS";
        assert_eq!(classify(1234, r"C:\Program Files\App\app.exe", true, w), Kind::App);
        assert_eq!(classify(1234, r"C:\Program Files\App\helper.exe", false, w), Kind::Background);
        assert_eq!(classify(800, r"C:\Windows\System32\svchost.exe", false, w), Kind::System);
        assert_eq!(classify(4, "", false, w), Kind::System);
        assert_eq!(classify(9000, r"C:\Windows\explorer.exe", true, w), Kind::System);
        assert!(critical("csrss.exe", 700) && critical("System", 4) && critical("launchd", 1));
        assert!(!critical("chrome.exe", 1234));
        assert!(anti_cheat("VALORANT-Win64-Shipping.exe") && anti_cheat("vgc.exe") && !anti_cheat("notepad.exe"));
    }
}

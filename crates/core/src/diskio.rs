//! Per-disk I/O (S1-D01): the OS gives cumulative counters; rates come from
//! two snapshots. Pure logic; the per-OS readers live in
//! `cpuzeez_platform::disk_io`.

/// Cumulative counters of one physical disk, as the OS reports them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Counters {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub reads: u64,
    pub writes: u64,
    /// Total time spent on reads / writes, summed over requests (ns).
    pub read_ns: u64,
    pub write_ns: u64,
    /// Time the disk had at least one request in flight (ns). None where the
    /// OS doesn't track it (macOS).
    pub busy_ns: Option<u64>,
    /// Requests in flight right now. None where the OS doesn't say.
    pub queue: Option<f64>,
}

/// What a page shows: rates over the interval between two snapshots.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rates {
    pub read_mbps: f64,
    pub write_mbps: f64,
    pub read_iops: f64,
    pub write_iops: f64,
    /// Share of the interval with I/O in flight, like the Task Manager's
    /// "Active time". Estimated from request time where there's no busy counter.
    pub active_pct: f64,
    /// Mean time per request completed in the interval; None without requests.
    pub read_ms: Option<f64>,
    pub write_ms: Option<f64>,
    pub queue: Option<f64>,
}

/// Rates between `a` and `b`, `secs` apart. A counter that went backwards (a
/// reset, a disk swapped under the same name) counts as no activity.
pub fn rates(a: &Counters, b: &Counters, secs: f64) -> Option<Rates> {
    if secs <= 0.0 {
        return None;
    }
    let d = |x: u64, y: u64| y.saturating_sub(x) as f64;
    let (reads, writes) = (d(a.reads, b.reads), d(a.writes, b.writes));
    let (rns, wns) = (d(a.read_ns, b.read_ns), d(a.write_ns, b.write_ns));
    let busy = match (a.busy_ns, b.busy_ns) {
        (Some(x), Some(y)) => d(x, y),
        // Overlapping requests count more than once; the clamp below caps it.
        _ => rns + wns,
    };
    let per = |ns: f64, n: f64| (n > 0.0).then(|| ns / n / 1e6);
    Some(Rates {
        read_mbps: d(a.read_bytes, b.read_bytes) / secs / 1e6,
        write_mbps: d(a.write_bytes, b.write_bytes) / secs / 1e6,
        read_iops: reads / secs,
        write_iops: writes / secs,
        active_pct: (busy / (secs * 1e9) * 100.0).clamp(0.0, 100.0),
        read_ms: per(rns, reads),
        write_ms: per(wns, writes),
        queue: b.queue,
    })
}

/// Whole disks from `/proc/diskstats` (partitions, loop, RAM and
/// device-mapper devices left out: their I/O is already in a disk's).
pub fn parse_diskstats(text: &str) -> Vec<(String, Counters)> {
    const SECTOR: u64 = 512; // diskstats always counts 512-byte sectors
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let name = *f.get(2)?;
            let n = |i: usize| f.get(i).and_then(|v| v.parse::<u64>().ok());
            if !is_whole_disk(name) {
                return None;
            }
            // 3 reads, 5 sectors read, 6 ms reading, 7 writes, 9 sectors
            // written, 10 ms writing, 11 in flight, 12 ms busy.
            Some((
                name.to_string(),
                Counters {
                    reads: n(3)?,
                    read_bytes: n(5)? * SECTOR,
                    read_ns: n(6)? * 1_000_000,
                    writes: n(7)?,
                    write_bytes: n(9)? * SECTOR,
                    write_ns: n(10)? * 1_000_000,
                    queue: n(11).map(|q| q as f64),
                    busy_ns: n(12).map(|ms| ms * 1_000_000),
                },
            ))
        })
        .collect()
}

fn is_whole_disk(name: &str) -> bool {
    let digits_tail = name.trim_end_matches(|c: char| c.is_ascii_digit());
    if ["loop", "ram", "dm-", "zram", "sr", "fd", "md"].iter().any(|p| name.starts_with(p)) {
        return false;
    }
    if name.starts_with("nvme") || name.starts_with("mmcblk") {
        // nvme0n1 / mmcblk0 are disks; nvme0n1p2 / mmcblk0p1 are partitions.
        return !name.contains('p') || !digits_tail.ends_with('p');
    }
    // sda, vdb, xvda, hdc: a trailing number means a partition.
    digits_tail.len() == name.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISKSTATS: &str = include_str!("../fixtures/linux/diskstats");

    #[test]
    fn diskstats_keeps_whole_disks_only() {
        let names: Vec<String> = parse_diskstats(DISKSTATS).into_iter().map(|d| d.0).collect();
        assert_eq!(names, ["nvme0n1", "sda", "mmcblk0", "vda"]);
    }

    #[test]
    fn diskstats_units() {
        let d = parse_diskstats(DISKSTATS);
        let sda = &d.iter().find(|d| d.0 == "sda").unwrap().1;
        assert_eq!(sda.reads, 51_234);
        assert_eq!(sda.read_bytes, 4_098_226 * 512);
        assert_eq!(sda.read_ns, 20_514 * 1_000_000);
        assert_eq!(sda.queue, Some(2.0));
        assert_eq!(sda.busy_ns, Some(61_000 * 1_000_000));
    }

    #[test]
    fn rates_over_one_second() {
        let a = Counters { read_bytes: 0, reads: 0, read_ns: 0, busy_ns: Some(0), ..Default::default() };
        let b = Counters {
            read_bytes: 250_000_000,
            reads: 1_000,
            read_ns: 500_000_000, // 0.5 ms each
            writes: 0,
            busy_ns: Some(400_000_000),
            queue: Some(3.0),
            ..Default::default()
        };
        let r = rates(&a, &b, 1.0).unwrap();
        assert_eq!((r.read_mbps, r.read_iops, r.read_ms, r.write_ms), (250.0, 1000.0, Some(0.5), None));
        assert!((r.active_pct - 40.0).abs() < 1e-9);
        assert_eq!(r.queue, Some(3.0));
    }

    #[test]
    fn a_reset_counter_is_no_activity_not_garbage() {
        let a = Counters { read_bytes: 1 << 40, reads: 9, ..Default::default() };
        let r = rates(&a, &Counters::default(), 1.0).unwrap();
        assert_eq!((r.read_mbps, r.read_iops, r.active_pct), (0.0, 0.0, 0.0));
        assert!(rates(&a, &a, 0.0).is_none());
    }

    #[test]
    fn without_a_busy_counter_active_time_is_estimated_and_capped() {
        let b = Counters { read_ns: 3_000_000_000, reads: 10, ..Default::default() };
        assert_eq!(rates(&Counters::default(), &b, 1.0).unwrap().active_pct, 100.0);
    }
}

//! Drive health tools (S1-K08): SMART self-tests and a read-only surface scan.
//!
//! | | Windows | Linux | macOS |
//! |---|---|---|---|
//! | NVMe self-test (short / extended) | 🔑 `IOCTL_STORAGE_PROTOCOL_COMMAND` (1903+) | 🔑 NVMe admin ioctl | ❌ |
//! | ATA self-test (short / extended / conveyance) | 🔑 ATA pass-through | 🔑 SG_IO ATA PASS-THROUGH(16) | ❌ |
//! | Progress and history | NVMe: no admin (log 06h); ATA: progress without admin, history 🔑 | 🔑 | ❌ |
//! | Surface scan (read-only) | 🔑 | 🔑 | 🔑 |
//!
//! A self-test runs inside the drive; reading the drive meanwhile is fine.
//! The surface scan only reads, 1 MiB at a time, uncached.

use crate::benchkit::Ctl;
use crate::storage::Drive;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Short,
    Extended,
    /// ATA only: checks for transport damage.
    Conveyance,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Short => "Short",
            Kind::Extended => "Extended",
            Kind::Conveyance => "Conveyance",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub kind: String,
    pub result: String,
    pub ok: bool,
    pub hours: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    /// (test, % done) while one runs.
    pub running: Option<(String, u8)>,
    /// Newest first.
    pub history: Vec<Entry>,
}

pub fn is_nvme(d: &Drive) -> bool {
    d.bus == 17
}

pub fn is_ata(d: &Drive) -> bool {
    matches!(d.bus, 3 | 11)
}

/// The tests this drive can run here.
pub fn kinds(d: &Drive) -> Vec<Kind> {
    if cfg!(target_os = "macos") {
        return Vec::new();
    }
    match (is_nvme(d), is_ata(d)) {
        // Identify Controller OACS bit 4: Device Self-test supported (not every drive has it).
        (true, _) if d.identify.as_ref().is_some_and(|id| id.len() >= 258 && id[256] & 0x10 == 0) => Vec::new(),
        (true, _) => vec![Kind::Short, Kind::Extended],
        (_, true) => vec![Kind::Short, Kind::Extended, Kind::Conveyance],
        _ => Vec::new(),
    }
}

fn nvme_kind(code: u8) -> String {
    match code {
        1 => "Short".into(),
        2 => "Extended".into(),
        0xE => "Vendor".into(),
        c => format!("Type {c:#x}"),
    }
}

/// NVMe Device Self-test log (log page 06h, 564 bytes).
pub fn parse_nvme(b: &[u8]) -> Option<Status> {
    if b.len() < 4 {
        return None;
    }
    let running = (b[0] & 0xF != 0).then(|| (nvme_kind(b[0] & 0xF), b[1] & 0x7F));
    let history = (0..20)
        .filter_map(|i| b.get(4 + 28 * i..4 + 28 * (i + 1)))
        .filter(|e| e[0] & 0xF != 0xF)
        .map(|e| {
            let r = e[0] & 0xF;
            let result = match r {
                0 => "Completed without error",
                1 => "Aborted by a Device Self-test command",
                2 => "Aborted by a controller reset",
                3 => "Aborted: namespace removed",
                4 => "Aborted by a Format NVM command",
                5 => "Fatal error",
                6 => "Failed (unknown segment)",
                7 => "Failed segments",
                8 => "Aborted (unknown reason)",
                9 => "Aborted by a sanitize operation",
                _ => "Unknown result",
            };
            Entry {
                kind: nvme_kind(e[0] >> 4),
                result: result.into(),
                ok: r == 0,
                hours: Some(u64::from_le_bytes(e[4..12].try_into().unwrap_or_default())),
            }
        })
        .collect();
    Some(Status { running, history })
}

fn ata_status_text(s: u8) -> (&'static str, bool) {
    match s {
        0 => ("Completed without error", true),
        1 => ("Aborted by the host", false),
        2 => ("Interrupted by a reset", false),
        3 => ("Fatal error", false),
        4 => ("Failed (unknown element)", false),
        5 => ("Failed (electrical element)", false),
        6 => ("Failed (servo or seek element)", false),
        7 => ("Failed (read element)", false),
        8 => ("Failed (handling damage)", false),
        0xF => ("In progress", true),
        _ => ("Unknown result", false),
    }
}

fn ata_kind(sub: u8) -> String {
    match sub & 0x7F {
        1 => "Short".into(),
        2 => "Extended".into(),
        3 => "Conveyance".into(),
        4 => "Selective".into(),
        0 => "Offline".into(),
        c => format!("Type {c:#x}"),
    }
}

/// ATA: progress from SMART READ DATA byte 363, history from the SMART
/// self-test log (address 06h) when it could be read.
pub fn parse_ata(data: &[u8], log: Option<&[u8]>) -> Status {
    let st = data.get(363).copied().unwrap_or(0);
    let running = (st >> 4 == 0xF).then(|| ("Self-test".to_string(), 100 - (st & 0xF).min(10) * 10));
    let mut history = Vec::new();
    if let Some(l) = log.filter(|l| l.len() >= 512) {
        let newest = l[508] as usize; // 1-based, 0 = no entries
        for k in 0..21 {
            if newest == 0 {
                break;
            }
            let i = (newest - 1 + 21 - k) % 21;
            let e = &l[2 + 24 * i..2 + 24 * (i + 1)];
            if e.iter().all(|&b| b == 0) {
                continue;
            }
            let (result, ok) = ata_status_text(e[1] >> 4);
            history.push(Entry {
                kind: ata_kind(e[0]),
                result: result.into(),
                ok,
                hours: Some(u16::from_le_bytes([e[2], e[3]]) as u64),
            });
        }
    }
    Status { running, history }
}

/// Starts (or, with None, aborts) a self-test. Needs admin / root.
pub fn start(d: &Drive, kind: Option<Kind>) -> Result<(), String> {
    if !cpuzeez_platform::elevation::is_elevated() {
        return Err("needs administrator rights".into());
    }
    let (nvme_code, ata_sub) = match kind {
        Some(Kind::Short) => (1, 1),
        Some(Kind::Extended) => (2, 2),
        Some(Kind::Conveyance) => (0, 3),
        None => (0xF, 0x7F),
    };
    if is_nvme(d) {
        if nvme_code == 0 {
            return Err("NVMe has no conveyance test".into());
        }
        return imp::nvme_selftest(d, nvme_code);
    }
    if is_ata(d) {
        return imp::ata_selftest(d, ata_sub);
    }
    Err("only NVMe and SATA drives run self-tests".into())
}

/// Progress and history.
pub fn status(d: &Drive) -> Result<Status, String> {
    if is_nvme(d) {
        return imp::nvme_log(d)
            .and_then(|b| parse_nvme(&b))
            .ok_or_else(|| "the self-test log can't be read here".into());
    }
    if is_ata(d) {
        let (data, log) = imp::ata_logs(d);
        return data.map(|b| parse_ata(&b, log.as_deref())).ok_or_else(|| "the SMART data can't be read here".into());
    }
    Err("only NVMe and SATA drives run self-tests".into())
}

#[cfg(windows)]
mod imp {
    use super::Drive;
    use crate::storage::imp::{ata_cmd_lba, ata_smart_nodata, nvme_admin_nodata, nvme_log_page};
    use crate::storage::win::Handle;

    pub fn nvme_selftest(d: &Drive, code: u32) -> Result<(), String> {
        let h = Handle::open_rw(&d.path).ok_or("can't open the drive (needs administrator rights)")?;
        // Opcode 14h, all namespaces, STC in CDW10 bits 3:0.
        nvme_admin_nodata(&h, 0x14, 0xFFFF_FFFF, code)
    }

    pub fn nvme_log(d: &Drive) -> Option<Vec<u8>> {
        nvme_log_page(&Handle::open(&d.path)?, 6, 564)
    }

    pub fn ata_selftest(d: &Drive, sub: u8) -> Result<(), String> {
        let h = Handle::open_rw(&d.path).ok_or("can't open the drive (needs administrator rights)")?;
        ata_smart_nodata(&h, 0xD4, sub)
    }

    /// SMART data (admin: fresh; else IOCTL_STORAGE_PREDICT_FAILURE) and the
    /// self-test log (admin).
    pub fn ata_logs(d: &Drive) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        const IOCTL_STORAGE_PREDICT_FAILURE: u32 = 0x2D1100;
        let rw = Handle::open_rw(&d.path);
        let data = rw.as_ref().and_then(|h| ata_cmd_lba(h, 0xB0, 0xD0, 0)).or_else(|| {
            let h = Handle::open(&d.path)?;
            h.ioctl(IOCTL_STORAGE_PREDICT_FAILURE, &[], 516).filter(|b| b.len() >= 516).map(|b| b[4..516].to_vec())
        });
        let log = rw.as_ref().and_then(|h| ata_cmd_lba(h, 0xB0, 0xD5, 6));
        (data, log)
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::Drive;
    use crate::storage::imp::{nvme_admin, nvme_log_page, sat_cmd};

    fn ctrl(d: &Drive) -> String {
        let name = d.path.trim_start_matches("/dev/");
        let n: String = name.trim_start_matches("nvme").chars().take_while(char::is_ascii_digit).collect();
        format!("nvme{n}")
    }

    pub fn nvme_selftest(d: &Drive, code: u32) -> Result<(), String> {
        nvme_admin(&ctrl(d), 0x14, code, None).map(|_| ())
    }

    pub fn nvme_log(d: &Drive) -> Option<Vec<u8>> {
        nvme_log_page(&ctrl(d), 6, 564)
    }

    fn open(d: &Drive) -> Option<std::fs::File> {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new().read(true).write(true).custom_flags(libc::O_NONBLOCK).open(&d.path).ok()
    }

    pub fn ata_selftest(d: &Drive, sub: u8) -> Result<(), String> {
        let f = open(d).ok_or("can't open the drive (needs root)")?;
        sat_cmd(&f, 0xD4, sub, false).map(|_| ()).ok_or_else(|| "the drive rejected the command".into())
    }

    pub fn ata_logs(d: &Drive) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        let Some(f) = open(d) else { return (None, None) };
        (sat_cmd(&f, 0xD0, 0, true), sat_cmd(&f, 0xD5, 6, true))
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    use super::Drive;
    const NO: &str = "this OS has no way to send SMART commands without a third-party driver";
    pub fn nvme_selftest(_: &Drive, _: u32) -> Result<(), String> {
        Err(NO.into())
    }
    pub fn nvme_log(_: &Drive) -> Option<Vec<u8>> {
        None
    }
    pub fn ata_selftest(_: &Drive, _: u8) -> Result<(), String> {
        Err(NO.into())
    }
    pub fn ata_logs(_: &Drive) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
        (None, None)
    }
}

// ---- surface scan ----

/// Read-latency buckets (upper bounds, ms), like HDD surface-scan tools.
pub const BUCKETS: [f64; 5] = [5.0, 20.0, 50.0, 150.0, 500.0];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scan {
    /// Blocks per bucket: <5, <20, <50, <150, <500, ≥500 ms.
    pub buckets: [u64; 6],
    /// Byte offsets of the 1 MiB blocks that couldn't be read.
    pub unreadable: Vec<u64>,
    pub blocks: u64,
    pub mbps: f64,
}

impl Scan {
    pub fn add(&mut self, ms: f64) {
        let i = BUCKETS.iter().position(|&b| ms < b).unwrap_or(BUCKETS.len());
        self.buckets[i] += 1;
        self.blocks += 1;
    }
}

/// Where a scan reads: every block, or `n` blocks spread over the drive.
pub fn offsets(capacity: u64, sample: Option<u64>) -> Vec<u64> {
    const MIB: u64 = 1 << 20;
    let blocks = capacity / MIB;
    match sample {
        Some(n) if n < blocks => (0..n).map(|i| i * blocks / n * MIB).collect(),
        _ => (0..blocks).map(|i| i * MIB).collect(),
    }
}

/// Reads the drive block by block (read-only, uncached). Needs admin / root.
pub fn scan(d: &Drive, sample: Option<u64>, ctl: &Ctl) -> Result<Scan, String> {
    use cpuzeez_platform::direct_io::{Buffer, read_at};
    let f = open_raw(&d.path).map_err(|e| format!("{e} (needs administrator rights)"))?;
    let list = offsets(d.capacity, sample);
    let mut buf = Buffer::new(1 << 20);
    let (mut s, t0, mut bytes) = (Scan::default(), Instant::now(), 0u64);
    for (i, &off) in list.iter().enumerate() {
        ctl.check()?;
        let t = Instant::now();
        match read_at(&f, buf.bytes(), off) {
            Ok(()) => {
                s.add(t.elapsed().as_secs_f64() * 1000.0);
                bytes += 1 << 20;
            }
            Err(_) => {
                s.unreadable.push(off);
                s.blocks += 1;
            }
        }
        if i % 64 == 0 {
            ctl.set("Surface scan", i as f32 / list.len().max(1) as f32);
        }
    }
    s.mbps = bytes as f64 / t0.elapsed().as_secs_f64().max(1e-9) / 1e6;
    Ok(s)
}

/// The whole device, read-only and uncached.
fn open_raw(path: &str) -> std::io::Result<std::fs::File> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new().read(true).share_mode(3).custom_flags(0x2000_0000).open(path)
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new().read(true).custom_flags(libc::O_DIRECT).open(path)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        // macOS: the raw node (/dev/rdiskN) skips the buffer cache.
        std::fs::File::open(path.replace("/dev/disk", "/dev/rdisk"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvme_log_parses() {
        let mut b = vec![0u8; 564];
        b[0] = 0x2; // extended running
        b[1] = 40;
        for i in 0..20 {
            b[4 + 28 * i] = 0xF; // unused
        }
        b[4] = 0x10; // short, completed OK
        b[8..16].copy_from_slice(&1234u64.to_le_bytes());
        b[32] = 0x27; // extended, failed segments
        let s = parse_nvme(&b).unwrap();
        assert_eq!(s.running, Some(("Extended".into(), 40)));
        assert_eq!(s.history.len(), 2);
        assert_eq!(
            s.history[0],
            Entry { kind: "Short".into(), result: "Completed without error".into(), ok: true, hours: Some(1234) }
        );
        assert!(!s.history[1].ok && s.history[1].kind == "Extended");
    }

    #[test]
    fn ata_log_parses_newest_first() {
        let mut data = vec![0u8; 512];
        data[363] = 0xF7; // in progress, 70 % remaining
        let mut log = vec![0u8; 512];
        // Entry 1: short OK at 100 h; entry 2: extended read failure at 200 h.
        log[2..6].copy_from_slice(&[1, 0x00, 100, 0]);
        log[26..30].copy_from_slice(&[2, 0x70, 200, 0]);
        log[508] = 2;
        let s = parse_ata(&data, Some(&log));
        assert_eq!(s.running, Some(("Self-test".into(), 30)));
        assert_eq!(s.history[0].kind, "Extended");
        assert_eq!((s.history[0].ok, s.history[0].hours), (false, Some(200)));
        assert_eq!(s.history[1].result, "Completed without error");
        assert_eq!(parse_ata(&[0; 512], None), Status::default());
    }

    #[test]
    fn scan_plan_and_buckets() {
        assert_eq!(offsets(4 << 20, None), vec![0, 1 << 20, 2 << 20, 3 << 20]);
        let o = offsets(1000 << 20, Some(10));
        assert_eq!(o.len(), 10);
        assert_eq!((o[0], o[9]), (0, 900 << 20));
        let mut s = Scan::default();
        for ms in [1.0, 6.0, 30.0, 100.0, 300.0, 900.0] {
            s.add(ms);
        }
        assert_eq!(s.buckets, [1; 6]);
    }

    #[test]
    fn nvme_without_self_test_offers_none() {
        let mut d = Drive { bus: 17, ..Default::default() };
        let mut id = vec![0u8; 4096];
        d.identify = Some(id.clone());
        assert!(kinds(&d).is_empty() || cfg!(target_os = "macos"));
        id[256] = 0x10;
        d.identify = Some(id);
        assert_eq!(kinds(&d).len(), if cfg!(target_os = "macos") { 0 } else { 2 });
    }
}

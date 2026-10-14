//! Local benchmark history (settings folder, `history.json`): every CPU and
//! memory benchmark result with date, label/notes and the system it ran on,
//! so runs before/after an overclock, paste swap or BIOS update can be compared.
//!
//! The file is a JSON array of `Entry`, oldest first, rewritten atomically
//! (temp file + rename). A file that fails to parse is kept as
//! `history.json.bad` instead of being overwritten.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// RFC 3339 UTC, e.g. "2026-10-14T10:20:00Z".
    pub when: String,
    /// User label ("before BIOS 1.A3", "PBO on"…).
    pub label: String,
    /// "CPU Single Thread", "CPU Multi Thread", "Memory".
    pub kind: String,
    /// Main score (points, or GB/s / ns for memory: see `unit`).
    pub score: f64,
    pub unit: String,
    pub threads: usize,
    /// "AVX-512", "Scalar"…
    pub variant: String,
    pub cpu: String,
    pub board: String,
    pub bios: String,
    pub memory: String,
    pub os: String,
    pub app_version: String,
    /// Full result (e.g. memory latency curve / bandwidth table).
    #[serde(default)]
    pub details: serde_json::Value,
}

fn path() -> Option<PathBuf> {
    crate::settings::dir().map(|d| d.join("history.json"))
}

pub fn load() -> Vec<Entry> {
    path().and_then(|p| load_from(&p).ok()).unwrap_or_default()
}

pub fn add(e: Entry) {
    update(|v| v.push(e))
}

pub fn remove(index: usize) {
    update(|v| {
        if index < v.len() {
            v.remove(index);
        }
    })
}

pub fn clear() {
    update(Vec::clear)
}

fn update(f: impl FnOnce(&mut Vec<Entry>)) {
    match path() {
        Some(p) => {
            if let Err(e) = update_at(&p, f) {
                eprintln!("{}: {e}", p.display());
            }
        }
        None => eprintln!("no settings folder for the benchmark history"),
    }
}

/// Missing file = empty history.
fn load_from(p: &Path) -> Result<Vec<Entry>, String> {
    match std::fs::read(p) {
        Ok(b) => serde_json::from_slice(&b).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}

fn update_at(p: &Path, f: impl FnOnce(&mut Vec<Entry>)) -> std::io::Result<()> {
    let mut v = match load_from(p) {
        Ok(v) => v,
        Err(_) => {
            std::fs::rename(p, p.with_extension("json.bad"))?; // keep what we can't read
            Vec::new()
        }
    };
    f(&mut v);
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&v)?)?;
    std::fs::rename(&tmp, p)
}

/// RFC 3339 UTC from Unix seconds (civil-from-days, proleptic Gregorian).
fn rfc3339(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem / 60 % 60, rem % 60)
}

pub fn now_rfc3339() -> String {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    rfc3339(s)
}

fn join(parts: &[Option<String>]) -> String {
    let v: Vec<&str> = parts.iter().flatten().map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    v.join(" ")
}

/// "32 GB DDR5-6000, 2 x 32-bit" from the memory controller, else SMBIOS.
fn memory_line(ctx: &crate::Ctx) -> String {
    let m = crate::chipset::summary(ctx).memory;
    let devs = crate::smbios::memory_devices(ctx);
    let devs: Vec<_> = devs.iter().filter(|d| d.size_mb > 0).collect();
    let mb = if m.size_mb > 0 { m.size_mb } else { devs.iter().map(|d| d.size_mb).sum() };
    let ty = m.mem_type.or_else(|| devs.first().map(|d| d.mem_type.to_string())).filter(|t| t != "unknown");
    let mts = m.freq_mhz.map(|f| (f * 2.0).round() as u32).or_else(|| devs.first().map(|d| d.configured_speed));
    let size = match mb {
        0 => None,
        mb if mb >= 1024 => Some(format!("{} GB", mb / 1024)),
        mb => Some(format!("{mb} MB")),
    };
    let mut s = join(&[size, ty]);
    if let Some(r) = mts.filter(|&r| r > 0) {
        s += &format!("-{r}");
    }
    if let Some(c) = m.channels {
        s += &format!(", {c}");
    }
    s
}

/// Fill the system fields (cpu/board/bios/memory/os/version/when) of a new entry.
pub fn system_entry(ctx: &crate::Ctx, kind: &str, score: f64, unit: &str) -> Entry {
    let board = crate::smbios::board(ctx)
        .map(|b| join(&[b.vendor, b.model]))
        .filter(|s| !s.is_empty())
        .or_else(|| crate::sensors::board_model(ctx))
        .unwrap_or_default();
    let bios =
        crate::smbios::bios(ctx).map(|b| join(&[b.version, b.date.map(|d| format!("({d})"))])).unwrap_or_default();
    Entry {
        when: now_rfc3339(),
        label: String::new(),
        kind: kind.into(),
        score,
        unit: unit.into(),
        threads: 0,
        variant: String::new(),
        cpu: crate::cpu::get(ctx).name.clone(),
        board,
        bios,
        memory: memory_line(ctx),
        os: crate::software::os_line(ctx),
        app_version: crate::VERSION.into(),
        details: serde_json::Value::Null,
    }
}

/// Entry for a CPU benchmark score (CLI and the GUI's Bench tab).
pub fn score_entry(ctx: &crate::Ctx, s: &crate::bench::Score, label: &str) -> Entry {
    let kind = if s.threads == 1 { "CPU Single Thread" } else { "CPU Multi Thread" };
    let mut e = system_entry(ctx, kind, s.points, "points");
    e.label = label.into();
    e.threads = s.threads;
    e.variant = s.variant.label().into();
    e.details = serde_json::json!({ "seconds": s.seconds });
    e
}

/// Save a CPU benchmark score to the history.
pub fn record(ctx: &crate::Ctx, score: &crate::bench::Score, label: &str) {
    add(score_entry(ctx, score, label))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(label: &str) -> Entry {
        Entry {
            when: rfc3339(0),
            label: label.into(),
            kind: "Memory".into(),
            score: 71.5,
            unit: "ns".into(),
            threads: 1,
            variant: "AVX2".into(),
            cpu: "cpu".into(),
            board: String::new(),
            bios: String::new(),
            memory: String::new(),
            os: String::new(),
            app_version: crate::VERSION.into(),
            details: serde_json::json!({"latency": [1, 2]}),
        }
    }

    #[test]
    fn dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(rfc3339(4_102_444_799), "2099-12-31T23:59:59Z");
        assert!(now_rfc3339().starts_with("20"));
    }

    #[test]
    fn roundtrip_in_temp_dir() {
        let dir = std::env::temp_dir().join(format!("cpuzeez-history-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("history.json");
        let _ = std::fs::remove_file(&p);
        assert!(load_from(&p).unwrap().is_empty());
        update_at(&p, |v| v.push(entry("a"))).unwrap();
        update_at(&p, |v| v.push(entry("b"))).unwrap();
        let v = load_from(&p).unwrap();
        assert_eq!(v.iter().map(|e| e.label.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(v[1].details["latency"][1], 2);
        update_at(&p, |v| {
            v.remove(0);
        })
        .unwrap();
        assert_eq!(load_from(&p).unwrap()[0].label, "b");
        assert!(!p.with_extension("json.tmp").exists());
        // A corrupt file is set aside, not lost.
        std::fs::write(&p, "not json").unwrap();
        update_at(&p, Vec::clear).unwrap();
        assert_eq!(std::fs::read_to_string(p.with_extension("json.bad")).unwrap(), "not json");
        assert!(load_from(&p).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

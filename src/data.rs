//! What CpuZeeZ keeps on this computer (docs/v1/README.md P1): every category,
//! where it lives, how much space it takes, and deleting or expiring it. All of
//! it is local; nothing is uploaded anywhere.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// CSV logs started from the Sensors page or `-log=`.
    SensorLogs,
    /// Saved benchmark results (`history.json`).
    BenchmarkHistory,
    /// Actions the app ran, such as loading a kernel module (`audit.log`).
    AuditLog,
    /// Preferences (`settings.json`); deleting them resets the app.
    Settings,
}

impl Category {
    pub const ALL: [Category; 4] =
        [Category::SensorLogs, Category::BenchmarkHistory, Category::AuditLog, Category::Settings];

    /// English key, translated by the GUI.
    pub fn label(self) -> &'static str {
        match self {
            Category::SensorLogs => "Sensor logs",
            Category::BenchmarkHistory => "Benchmark history",
            Category::AuditLog => "Action log",
            Category::Settings => "Settings",
        }
    }

    /// Files of this category in data folder `dir`.
    fn files(self, dir: &Path) -> Vec<PathBuf> {
        match self {
            Category::SensorLogs => std::fs::read_dir(dir.join("logs"))
                .map(|rd| {
                    rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "csv")).collect()
                })
                .unwrap_or_default(),
            Category::BenchmarkHistory => existing(&[dir.join("history.json"), dir.join("history.json.bad")]),
            Category::AuditLog => existing(&[dir.join("audit.log")]),
            Category::Settings => existing(&[dir.join("settings.json")]),
        }
    }
}

fn existing(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths.iter().filter(|p| p.is_file()).cloned().collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub cat: Category,
    pub files: usize,
    pub bytes: u64,
}

pub fn usage_in(dir: &Path) -> Vec<Usage> {
    Category::ALL
        .iter()
        .map(|&cat| {
            let f = cat.files(dir);
            let bytes = f.iter().filter_map(|p| std::fs::metadata(p).ok()).map(|m| m.len()).sum();
            Usage { cat, files: f.len(), bytes }
        })
        .collect()
}

pub fn delete_in(dir: &Path, cat: Category) -> std::io::Result<usize> {
    let f = cat.files(dir);
    for p in &f {
        std::fs::remove_file(p)?;
    }
    Ok(f.len())
}

/// Delete sensor logs older than `days` (0 keeps everything). Returns how many went.
pub fn expire_logs_in(dir: &Path, days: u32, now: SystemTime) -> usize {
    if days == 0 {
        return 0;
    }
    let cutoff = now.checked_sub(Duration::from_secs(days as u64 * 86_400)).unwrap_or(SystemTime::UNIX_EPOCH);
    Category::SensorLogs
        .files(dir)
        .into_iter()
        .filter(|p| std::fs::metadata(p).and_then(|m| m.modified()).is_ok_and(|t| t < cutoff))
        .filter(|p| std::fs::remove_file(p).is_ok())
        .count()
}

/// The app's own data folder, the same one Settings shows.
pub fn usage() -> Vec<Usage> {
    crate::settings::dir().map(|d| usage_in(&d)).unwrap_or_default()
}

pub fn delete(cat: Category) -> std::io::Result<usize> {
    crate::settings::dir().map_or(Ok(0), |d| delete_in(&d, cat))
}

/// Apply the retention setting at startup.
pub fn expire_logs(days: u32) {
    if let Some(d) = crate::settings::dir() {
        expire_logs_in(&d, days, SystemTime::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cz-data-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("logs")).unwrap();
        d
    }

    #[test]
    fn categories_sizes_and_delete() {
        let d = tmp("usage");
        std::fs::write(d.join("logs/a.csv"), b"12345").unwrap();
        std::fs::write(d.join("logs/notes.txt"), b"not a log").unwrap();
        std::fs::write(d.join("history.json"), b"[]").unwrap();
        std::fs::write(d.join("settings.json"), b"{}").unwrap();
        let u = usage_in(&d);
        let of = |c: Category| u.iter().find(|x| x.cat == c).copied().unwrap();
        assert_eq!((of(Category::SensorLogs).files, of(Category::SensorLogs).bytes), (1, 5));
        assert_eq!(of(Category::AuditLog).files, 0);
        assert_eq!(delete_in(&d, Category::SensorLogs).unwrap(), 1);
        assert!(d.join("logs/notes.txt").exists(), "only what the app wrote");
        assert!(d.join("settings.json").exists(), "other categories untouched");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn retention_expires_old_logs_only() {
        let d = tmp("retention");
        std::fs::write(d.join("logs/old.csv"), b"x").unwrap();
        let later = SystemTime::now() + Duration::from_secs(40 * 86_400);
        assert_eq!(expire_logs_in(&d, 0, later), 0, "0 keeps everything");
        assert_eq!(expire_logs_in(&d, 60, later), 0, "younger than 60 days");
        assert_eq!(expire_logs_in(&d, 30, later), 1);
        assert!(!d.join("logs/old.csv").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}

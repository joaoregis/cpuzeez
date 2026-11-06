//! ZeeZ Live capture sessions (S2b-09, S2b-10; docs/v1/zeez-live.md §6):
//! what `session.json` holds, folder names, and the checks a comparison
//! needs (same game, resolution and driver; repeat variance).

use crate::frames::{LowConvention, Summary};
use serde::{Deserialize, Serialize};

/// The machine a session ran on, as far as CpuZeeZ knows it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct System {
    pub cpu: String,
    pub gpu: String,
    pub gpu_driver: String,
    pub ram_gb: f64,
    pub os: String,
    /// "2560x1440" when known.
    pub resolution: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub t_s: f64,
    pub label: String,
}

/// A span with a CPU or GPU throttling flag set.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Band {
    pub from_s: f64,
    pub to_s: f64,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    #[default]
    Manual,
    Timed,
    Automatic,
    /// Frames imported from another tool's CSV.
    Imported,
}

/// `session.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    /// File format version.
    pub version: u32,
    pub game: String,
    pub pid: u32,
    /// Start, ISO 8601 local time.
    pub started: String,
    pub duration_s: f64,
    pub mode: Mode,
    /// Where the frames came from: "PresentMon 2.6.0", "MangoHud", "CSV import".
    pub source: String,
    pub system: System,
    pub markers: Vec<Marker>,
    pub throttling: Vec<Band>,
    pub summary: Summary,
    pub convention: LowConvention,
    /// The user's note.
    pub note: String,
}

pub const VERSION: u32 = 1;

/// `<yyyy-mm-dd_hhmm>_<game>` with the game's name made safe for a folder.
pub fn folder_name(started_ymd_hm: &str, game: &str) -> String {
    let stem = game.rsplit(['/', '\\']).next().unwrap_or(game);
    let stem = stem.strip_suffix(".exe").or_else(|| stem.strip_suffix(".EXE")).unwrap_or(stem);
    let safe: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(48)
        .collect();
    let safe = if safe.trim_matches('_').is_empty() { "session".to_string() } else { safe };
    format!("{started_ymd_hm}_{safe}")
}

/// Why two sessions shouldn't be compared as like for like (empty: fine).
pub fn comparability(a: &Session, b: &Session) -> Vec<&'static str> {
    let mut v = Vec::new();
    if !a.game.eq_ignore_ascii_case(&b.game) {
        v.push("Different games");
    }
    if !a.system.resolution.is_empty() && !b.system.resolution.is_empty() && a.system.resolution != b.system.resolution
    {
        v.push("Different resolutions");
    }
    if !a.system.gpu_driver.is_empty() && !b.system.gpu_driver.is_empty() && a.system.gpu_driver != b.system.gpu_driver
    {
        v.push("Different GPU drivers");
    }
    if a.convention != b.convention {
        v.push("Different 1% low conventions");
    }
    v
}

/// Repeats of one test vary too much when the average FPS spreads more than
/// 5 % around the median: "run it 3x and use the median".
pub fn high_variance(avg_fps: &[f64]) -> bool {
    if avg_fps.len() < 2 {
        return false;
    }
    let mut s = avg_fps.to_vec();
    s.sort_by(f64::total_cmp);
    let median = crate::frames::percentile(&s, 50.0);
    median > 0.0 && (s[s.len() - 1] - s[0]) / median > 0.05
}

/// The median of each summary figure across repeats (avg, 1 %, 0.1 %).
pub fn median_of(runs: &[Summary]) -> (f64, f64, f64) {
    let med = |f: fn(&Summary) -> f64| {
        let mut v: Vec<f64> = runs.iter().map(f).collect();
        v.sort_by(f64::total_cmp);
        crate::frames::percentile(&v, 50.0)
    };
    (med(|s| s.avg_fps), med(|s| s.low1_fps), med(|s| s.low01_fps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names_are_safe() {
        assert_eq!(
            folder_name("2026-11-05_2140", "VALORANT-Win64-Shipping.exe"),
            "2026-11-05_2140_VALORANT-Win64-Shipping"
        );
        assert_eq!(folder_name("2026-11-05_2140", r"C:\Games\My Game!.exe"), "2026-11-05_2140_My_Game_");
        assert_eq!(folder_name("2026-11-05_2140", "../.."), "2026-11-05_2140_session");
        assert_eq!(folder_name("x", ""), "x_session");
    }

    #[test]
    fn comparisons_and_repeats() {
        let a = Session { game: "cs2.exe".into(), ..Default::default() };
        let mut b = a.clone();
        assert!(comparability(&a, &b).is_empty());
        b.game = "game.exe".into();
        b.system.resolution = "1920x1080".into();
        assert_eq!(comparability(&a, &b), ["Different games"], "an unknown resolution isn't a difference");
        assert!(!high_variance(&[100.0, 101.0, 102.0]));
        assert!(high_variance(&[100.0, 110.0, 101.0]));
        let s = |avg| Summary { avg_fps: avg, ..Default::default() };
        assert_eq!(median_of(&[s(90.0), s(100.0), s(140.0)]).0, 100.0);
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Session>(&json).unwrap(), a);
    }
}

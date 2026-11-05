//! Per-user preferences and data folder. `cpuzeez.ini` (next to the exe, same
//! keys as cpuz.ini) stays the hardware-probe config; this file holds what the
//! GUI remembers between runs. It's JSON in the OS's per-user config folder:
//! %APPDATA%\CpuZeeZ, ~/Library/Application Support/CpuZeeZ, $XDG_CONFIG_HOME/cpuzeez.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Schema version of settings.json. Bump when a field changes meaning.
pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Schema version of the file (absent in 0.3.0 files = 1).
    pub schema: u32,
    /// The file came from a newer CpuZeeZ: never write over it (fields this
    /// version doesn't know would be lost).
    #[serde(skip)]
    pub read_only: bool,
    /// "en", "pt-BR", "es"; None = follow the OS language.
    pub language: Option<String>,
    pub theme: Theme,
    /// Show the tray / menu-bar mini monitor.
    pub tray: bool,
    /// Closing the window keeps running in the tray.
    pub close_to_tray: bool,
    /// Sensor sampling period.
    pub sample_ms: u64,
    /// History kept per sensor in the graphs.
    pub graph_seconds: u64,
    /// Temperature (°C) at which the stress monitor warns, if the CPU's Tjmax is unknown.
    pub warn_temp_c: f64,
    /// Show the always-on-top mini monitor window.
    pub mini: bool,
    /// Ask GitHub for a newer release at startup. None = not asked yet: the
    /// first-run screen asks, and nothing is checked until then (P1).
    pub update_check: Option<bool>,
    /// Version of the first-run privacy screen the user has seen (0 = never).
    pub privacy_seen: u32,
    /// Delete sensor logs older than this many days (0 = keep them).
    pub log_retention_days: u32,
    /// °C/°F, MHz/GHz, MB/MiB and decimals for every displayed value.
    pub units: cpuzeez_core::units::Units,
    /// Detail level per page ("Processor" → Basic/Advanced/Expert).
    pub detail: std::collections::BTreeMap<String, cpuzeez_core::detail::Level>,
    /// Rows pinned to the Overview, in pin order.
    pub pins: Vec<Pin>,
    /// The Overview's tiles; None = the default set.
    pub overview_tiles: Option<Vec<cpuzeez_core::layout::Widget>>,
    /// Local alerts are opt-in (P1): off until turned on in Settings.
    pub alerts_on: bool,
    pub alerts: Vec<cpuzeez_core::alerts::Rule>,
    /// Rated endurance (TB written) per drive serial, typed by the user.
    pub tbw: std::collections::BTreeMap<String, f64>,
    /// Tray icons, one per metric (S1-M01).
    pub tray_items: Vec<cpuzeez_core::layout::tray::Item>,
    pub tray_click: cpuzeez_core::layout::tray::Click,
    /// Mini monitors (S1-M02). Empty = one default, open when `mini` is set.
    pub minis: Vec<cpuzeez_core::layout::mini::Mini>,
}

/// A row pinned to the Overview (S1-U03).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Pin {
    /// Card and row as shown when pinned.
    pub card: String,
    pub label: String,
    /// Its text when pinned; shown until its page draws it again.
    pub value: String,
    /// A sensor's metric id: then the Overview shows it live.
    pub metric: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema: SCHEMA,
            read_only: false,
            language: None,
            theme: Theme::System,
            tray: false,
            close_to_tray: false,
            sample_ms: 1000,
            graph_seconds: 600,
            warn_temp_c: 95.0,
            mini: false,
            update_check: None,
            privacy_seen: 0,
            log_retention_days: 0,
            units: Default::default(),
            detail: Default::default(),
            pins: Vec::new(),
            overview_tiles: None,
            alerts_on: false,
            alerts: cpuzeez_core::alerts::defaults(),
            tbw: Default::default(),
            tray_items: vec![Default::default()],
            tray_click: Default::default(),
            minis: Vec::new(),
        }
    }
}

/// Folder for settings, benchmark history and default logs (created on demand).
pub fn dir() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from).map(|p| p.join(crate::DISPLAY_NAME))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support").join(crate::DISPLAY_NAME))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|p| p.join(crate::NAME))
    }?;
    std::fs::create_dir_all(&base).ok()?;
    // Logs and history are private: only this user may read the folder (P1).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o700));
    }
    Some(base)
}

fn file() -> Option<PathBuf> {
    dir().map(|d| d.join("settings.json"))
}

impl Settings {
    pub fn load() -> Settings {
        file().map(|p| Settings::load_from(&p)).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = file()
            && let Err(e) = self.save_to(&p)
        {
            eprintln!("{}: {e}", p.display());
        }
    }

    /// Missing file = defaults. An unreadable file is kept as
    /// `settings.json.bad` instead of being overwritten later.
    fn load_from(p: &Path) -> Settings {
        let Ok(bytes) = std::fs::read(p) else { return Settings::default() };
        match serde_json::from_slice::<Settings>(&bytes) {
            Ok(mut s) => {
                s.read_only = s.schema > SCHEMA;
                s
            }
            Err(_) => {
                let _ = std::fs::rename(p, p.with_extension("json.bad"));
                Settings::default()
            }
        }
    }

    /// Atomic write (temp file + rename); refuses to replace a newer file.
    fn save_to(&self, p: &Path) -> std::io::Result<()> {
        if self.read_only {
            return Err(std::io::Error::other("written by a newer CpuZeeZ; not overwriting it"));
        }
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&Settings { schema: SCHEMA, ..self.clone() })?)?;
        std::fs::rename(&tmp, p)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn roundtrip_and_defaults_for_missing_keys() {
        let s: super::Settings = serde_json::from_str(r#"{"theme":"Dark","tray":true}"#).unwrap();
        assert_eq!(s.theme, super::Theme::Dark);
        assert!(s.tray && s.sample_ms == 1000);
        assert!(!s.mini, "settings files from 0.3.0 have no mini key");
        assert_eq!(s.update_check, None, "0.3.0 users are asked before any update check");
        let back: super::Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.theme, super::Theme::Dark);
    }

    #[test]
    fn newer_files_are_never_overwritten_and_bad_ones_are_kept() {
        use super::*;
        let dir = std::env::temp_dir().join(format!("cz-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings.json");
        // A file from a future version, with a field this one doesn't know.
        std::fs::write(&p, br#"{"schema":99,"theme":"Light","future_field":42}"#).unwrap();
        let s = Settings::load_from(&p);
        assert!(s.read_only && s.theme == Theme::Light);
        assert!(s.save_to(&p).is_err());
        assert!(std::fs::read_to_string(&p).unwrap().contains("future_field"), "left untouched");
        // A corrupt file is set aside, not lost.
        std::fs::write(&p, b"{ not json").unwrap();
        let s = Settings::load_from(&p);
        assert!(!s.read_only && dir.join("settings.json.bad").exists());
        s.save_to(&p).unwrap();
        assert_eq!(Settings::load_from(&p).schema, SCHEMA);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

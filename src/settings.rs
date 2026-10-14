//! Per-user preferences and data folder. `cpuzeez.ini` (next to the exe, same
//! keys as cpuz.ini) stays the hardware-probe config; this file holds what the
//! GUI remembers between runs. It's JSON in the OS's per-user config folder:
//! %APPDATA%\CpuZeeZ, ~/Library/Application Support/CpuZeeZ, $XDG_CONFIG_HOME/cpuzeez.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: None,
            theme: Theme::System,
            tray: false,
            close_to_tray: false,
            sample_ms: 1000,
            graph_seconds: 600,
            warn_temp_c: 95.0,
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
    Some(base)
}

fn file() -> Option<PathBuf> {
    dir().map(|d| d.join("settings.json"))
}

impl Settings {
    pub fn load() -> Settings {
        file().and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        if let (Some(p), Ok(json)) = (file(), serde_json::to_vec_pretty(self)) {
            let _ = std::fs::write(p, json);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn roundtrip_and_defaults_for_missing_keys() {
        let s: super::Settings = serde_json::from_str(r#"{"theme":"Dark","tray":true}"#).unwrap();
        assert_eq!(s.theme, super::Theme::Dark);
        assert!(s.tray && s.sample_ms == 1000);
        let back: super::Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.theme, super::Theme::Dark);
    }
}

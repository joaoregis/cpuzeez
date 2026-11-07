//! MangoHud on Linux (S2b-06, docs/v1/zeez-live.md §3.1 Q4): detect it,
//! install it (the distro's package manager through pkexec, after the user
//! confirms), enable it for every game through `environment.d` (opt-in,
//! applies from the next login), generate its config from the overlay
//! layout, and read its per-frame logs into the frame hub.
//!
//! Steam's own config files are never edited (Steam rewrites them while it
//! runs); per game, the Live tab copies `mangohud %command%`.

use cpuzeez_core::mangohud as mh;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Install {
    Distro,
    Flatpak,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    pub installed: Vec<Install>,
    pub enabled_all: bool,
    /// The package manager an Install button would use.
    pub manager: Option<&'static str>,
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| home().map(|h| h.join(".config")))
}

pub fn env_path() -> Option<PathBuf> {
    config_home().map(|c| c.join("environment.d").join(mh::ENV_FILE))
}

pub fn config_path() -> Option<PathBuf> {
    config_home().map(|c| c.join("MangoHud").join(mh::CONFIG_FILE))
}

/// Where MangoHud writes the logs we read (inside the sessions folder, so
/// Privacy & data counts and deletes them).
pub fn log_dir() -> Option<PathBuf> {
    crate::settings::dir().map(|d| d.join("sessions").join(".mangohud"))
}

fn on_path(bin: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
}

pub fn status() -> Status {
    let mut installed = Vec::new();
    if on_path("mangohud")
        || ["/usr/lib/mangohud", "/usr/lib64/mangohud", "/usr/lib/x86_64-linux-gnu/mangohud"]
            .iter()
            .any(|d| std::path::Path::new(d).exists())
    {
        installed.push(Install::Distro);
    }
    let flatpak =
        [home().map(|h| h.join(".local/share/flatpak/runtime")), Some(PathBuf::from("/var/lib/flatpak/runtime"))];
    if flatpak.iter().flatten().any(|d| d.join(mh::FLATPAK_ID).exists()) {
        installed.push(Install::Flatpak);
    }
    let manager = ["apt-get", "dnf", "pacman", "zypper"]
        .into_iter()
        .find(|m| on_path(m))
        .map(|m| if m == "apt-get" { "apt" } else { m });
    Status { installed, enabled_all: env_path().is_some_and(|p| p.exists()), manager }
}

/// Write our config (from the layout) and, with `all`, the environment.d file;
/// `all` false removes that file.
pub fn configure(
    all: bool,
    layout: &cpuzeez_core::layout::Layout,
    anchor: cpuzeez_core::live::Anchor,
) -> Result<(), String> {
    let cfg = config_path().ok_or("no config folder")?;
    let logs = log_dir().ok_or("no data folder")?;
    std::fs::create_dir_all(&logs).map_err(|e| e.to_string())?;
    if let Some(d) = cfg.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&cfg, mh::config(layout, anchor, &logs.display().to_string())).map_err(|e| e.to_string())?;
    let env = env_path().ok_or("no config folder")?;
    if all {
        if let Some(d) = env.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(&env, mh::env_file(&cfg.display().to_string())).map_err(|e| e.to_string())
    } else {
        match std::fs::remove_file(&env) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

/// Install the distro package through pkexec (its own password prompt).
pub fn install(manager: &str) -> Result<(), String> {
    let cmd = mh::install_command(manager).ok_or("no supported package manager")?;
    let out = std::process::Command::new("pkexec").args(&cmd).output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) }
}

/// Follow the newest MangoHud log while frames are wanted (the frame hub's
/// Linux source). MangoHud names logs `<game>_<date>.csv`.
pub fn tail() {
    use std::io::{Read, Seek, SeekFrom};
    let Some(dir) = log_dir() else { return };
    let (mut file, mut pos, mut cols, mut buf): (Option<PathBuf>, u64, Option<(usize, Option<usize>)>, String) =
        (None, 0, None, String::new());
    while crate::live_frames::wanted() {
        std::thread::sleep(std::time::Duration::from_millis(250));
        // The newest log written in the last 10 s is the game being played.
        let newest = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .filter(|(t, p)| p.extension().is_some_and(|x| x == "csv") && t.elapsed().is_ok_and(|e| e.as_secs() < 10))
            .max_by_key(|(t, _)| *t)
            .map(|(_, p)| p);
        let Some(path) = newest else { continue };
        if file.as_ref() != Some(&path) {
            (file, pos, cols, buf) = (Some(path.clone()), 0, None, String::new());
        }
        let Ok(mut f) = std::fs::File::open(&path) else { continue };
        if f.seek(SeekFrom::Start(pos)).is_err() {
            continue;
        }
        let mut chunk = String::new();
        let Ok(n) = f.read_to_string(&mut chunk) else { continue };
        pos += n as u64;
        buf += &chunk;
        let game =
            path.file_stem().map_or(String::new(), |s| s.to_string_lossy().split('_').next().unwrap_or("").to_string());
        let mut frames = Vec::new();
        while let Some(i) = buf.find('\n') {
            let line: String = buf.drain(..=i).collect();
            let f: Vec<&str> = line.trim().split(',').collect();
            match cols {
                None => {
                    if let Some(ft) = f.iter().position(|c| c.eq_ignore_ascii_case("frametime")) {
                        cols = Some((ft, f.iter().position(|c| c.eq_ignore_ascii_case("elapsed"))));
                    }
                }
                Some((ft, el)) => {
                    let Some(ms) = f.get(ft).and_then(|v| v.parse::<f64>().ok()) else { continue };
                    let t_s = el.and_then(|i| f.get(i)?.parse::<f64>().ok()).map_or(0.0, |ns| ns / 1e9);
                    frames.push(cpuzeez_core::frames::Frame {
                        t_s,
                        ms,
                        app: game.clone(),
                        pid: 1,
                        ..Default::default()
                    });
                }
            }
        }
        if !frames.is_empty() {
            crate::live_frames::push(frames, None);
            crate::live_frames::set_running();
        }
    }
}

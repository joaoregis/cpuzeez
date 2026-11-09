//! cpuzeez: clean-room, cross-platform reimplementation of CPU-Z's features.
//! The spec is FEATURES.md; no CPU-Z code was used.

// Style-only lints; the tuple/type shapes are local to one function each.
#![allow(clippy::type_complexity, clippy::field_reassign_with_default)]

mod access;
mod applog;
mod battery;
mod bench;
mod benchkit;
mod bios;
mod chipset;
mod cpu;
mod cpusuite;
mod data;
mod diff;
mod diskbench;
mod drivetest;
mod executor;
mod gpu;
#[cfg(feature = "gui")]
mod gpubench;
#[cfg(feature = "gui")]
mod gui;
mod health;
mod history;
mod latency;
mod live_frames;
#[cfg(target_os = "linux")]
mod live_mangohud;
mod membench;
mod memplus;
mod memtest;
mod network;
mod npu;
mod pci;
mod peripherals;
mod procs;
#[cfg(test)]
mod robustness;
mod sensors;
mod settings;
mod smbios;
#[cfg(feature = "soak")]
mod soak;
mod software;
mod spd;
mod storage;
mod telemetry;
mod timers;
mod update;
mod usb;

use report::{Section, f, section};
use std::sync::OnceLock;

// Pure logic lives in cpuzeez-core (docs/adr/0011); these keep the old `crate::` paths.
pub use cpuzeez_core::{DISPLAY_NAME, NAME, config, report};
pub use cpuzeez_platform::{hw, os};

/// Tests that load every core (benchmarks, stress) or time themselves take
/// turns: run in parallel they starve each other and fail on a busy runner.
#[cfg(test)]
pub fn heavy_test() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

/// Shared state for one run: config, hardware backend and lazily-built caches
/// that several sections need (SMBIOS, PCI list, CPU model).
pub struct Ctx {
    pub cfg: config::Config,
    pub hw: Box<dyn hw::Hw>,
    pub smbios: OnceLock<Option<smbios::Table>>,
    pub pci: OnceLock<Vec<pci::Device>>,
    pub cpu: OnceLock<cpu::Cpu>,
}

impl Ctx {
    pub fn new(cfg: config::Config) -> Ctx {
        Ctx { cfg, hw: hw::detect(), smbios: OnceLock::new(), pci: OnceLock::new(), cpu: OnceLock::new() }
    }
}

/// Every report section, in CPU-Z's order.
/// Writes to stdout. A reader that stops early (`cpuzeez -sensors | head`)
/// closes the pipe: that's a normal end of the output, not a crash.
pub fn out(s: &str) {
    use std::io::Write;
    let mut o = std::io::stdout().lock();
    if let Err(e) = o.write_all(s.as_bytes()).and_then(|()| o.flush())
        && e.kind() == std::io::ErrorKind::BrokenPipe
    {
        std::process::exit(0);
    }
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let mut r = vec![section(
        "Binaries",
        vec![f(format!("{DISPLAY_NAME} version"), VERSION), f("Hardware access", ctx.hw.name())],
    )];
    r.extend(cpu::collect(ctx)); // Processors, APICs, Timers, Processors Information, Thread dumps
    r.push(bios::section(ctx));
    r.extend(chipset::collect(ctx)); // Chipset
    r.extend(spd::collect(ctx)); // Memory SPD
    r.extend(sensors::collect(ctx)); // Monitoring, LPCIO, Hardware Monitors
    r.extend(pci::collect(ctx)); // PCI Devices
    r.extend(smbios::collect(ctx)); // DMI
    r.extend(storage::collect(ctx));
    r.extend(battery::collect(ctx));
    r.extend(network::collect(ctx));
    r.extend(usb::collect(ctx));
    r.extend(gpu::collect(ctx)); // Graphics, Graphic APIs, Display Adapters (+ monitors)
    r.extend(npu::collect(ctx)); // AI Devices, NPU Devices
    r.extend(software::collect(ctx));
    r.extend(pci::register_spaces(ctx));
    // CPU-Z keeps these headers even when empty.
    r.retain(|s| !s.nodes.is_empty() || matches!(s.title.as_str(), "Binaries" | "AI Devices" | "NPU Devices"));
    r
}

enum Out {
    Txt(String),
    Html(String),
    Json(String),
    JsonTxt(String),
}

const USAGE: &str = "usage: cpuzeez [-txt=FILE] [-html=FILE] [-json=FILE] [-jsontxt=FILE] [-console] \
[-bench] [-membench] [-core=ID] [-coremask=HEX] [-sensors] [-log=FILE[,SECONDS]] [-diff=OLD.json,NEW.json|now] [-checkupdate[=stable|snapshot|beta|alpha]] [-screenshots=DIR]\nWith no output option the GUI opens.";

/// One copy per user session and data folder (Windows): a second launch
/// brings the first to the front instead (from the tray too). `None`: not
/// claimed (screenshots); `Some(None)`: another copy answered, quit;
/// `Some(Some(_))`: this copy runs.
#[cfg(feature = "gui")]
fn claim_instance() -> Option<Option<cpuzeez_platform::instance::Instance>> {
    use cpuzeez_platform::instance::{Claim, claim, show_running};
    // Per data folder, so a test profile (the soak's) never meets the user's copy.
    let dir = settings::dir().map(|d| d.to_string_lossy().to_lowercase()).unwrap_or_default();
    let key = format!(
        "{:016x}",
        dir.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3))
    );
    match claim(&key) {
        Claim::First(i) => Some(Some(i)),
        Claim::Running if show_running(&key, std::time::Duration::from_secs(3)) => Some(None),
        Claim::Running => {
            applog::warn("another copy holds the instance but did not answer; starting anyway");
            None
        }
    }
}

fn main() {
    applog::install_panic_hook();
    let mut outs = Vec::new();
    let (mut console, mut bench, mut core, mut coremask) = (false, false, None, None);
    let mut shots: Option<std::path::PathBuf> = None;
    let mut handoff = None;
    let (mut apply_msi, mut wait_pid, mut relaunch, mut replaces) = (None, None, None, None);
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').map_or((a.as_str(), ""), |(k, v)| (k, v));
        match k.to_ascii_lowercase().trim_start_matches('-') {
            // Per-action elevation: runs one allow-listed action and exits (executor.rs).
            "elevated-op" => std::process::exit(executor::elevated_main(v)),
            "txt" => outs.push(Out::Txt(v.into())),
            "html" => outs.push(Out::Html(v.into())),
            "json" => outs.push(Out::Json(v.into())),
            "jsontxt" => outs.push(Out::JsonTxt(v.into())),
            "console" => console = true,
            "checkupdate" => return update::cli(v),
            "sensors" => return telemetry::cli_once(&Ctx::new(config::Config::load())),
            "membench" => return membench::cli(&Ctx::new(config::Config::load())),
            "diff" => return diff::cli(&Ctx::new(config::Config::load()), v),
            "log" => {
                let (file, secs) = v.split_once(',').map_or((v, None), |(f, s)| (f, s.parse().ok()));
                return telemetry::cli_log(Ctx::new(config::Config::load()), file, secs);
            }
            "bench" => bench = true,
            "core" => core = v.parse::<usize>().ok(),
            "coremask" => coremask = u64::from_str_radix(v.trim_start_matches("0x"), 16).ok(),
            "screenshots" => shots = Some(if v.is_empty() { "screenshots" } else { v }.into()),
            // Set by "Restart as administrator" on Linux; see platform::elevation.
            "elevated-handoff" => handoff = Some(v.to_string()),
            // The update helper (update::apply), started by the app from its download folder.
            "apply-update" => apply_msi = Some(std::path::PathBuf::from(v)),
            "wait-pid" => wait_pid = v.parse::<u32>().ok(),
            "relaunch" => relaunch = Some(std::path::PathBuf::from(v)),
            // Set by the update helper after a failed install: the app says why.
            "update-failed" => {
                let _ = update::FAILED.set(v.parse().unwrap_or(-1));
            }
            // Set by "Restart as administrator": wait for the copy being replaced.
            "replaces" => replaces = v.parse::<u32>().ok(),
            "h" | "help" | "?" => return println!("{USAGE}"),
            _ => return eprintln!("unknown option {a}\n{USAGE}"),
        }
    }

    if let Some(msi) = apply_msi {
        std::process::exit(update::apply(&msi, wait_pid, relaunch.as_deref()));
    }

    let ctx = Ctx::new(config::Config::load());
    if bench {
        return bench::cli(&ctx, coremask);
    }
    if let Some(id) = core {
        return cpu::print_core_clock(&ctx, id);
    }
    if outs.is_empty() && !console {
        #[cfg(feature = "gui")]
        {
            os::hide_own_console();
            applog::info(&format!(
                "start {DISPLAY_NAME} {VERSION} on {} {}, hardware access: {}, elevated: {}",
                std::env::consts::OS,
                std::env::consts::ARCH,
                ctx.hw.name(),
                cpuzeez_platform::elevation::is_elevated()
            ));
            if let Some(m) = &handoff {
                cpuzeez_platform::elevation::signal_handoff(m);
            }
            if let Some(pid) = replaces {
                cpuzeez_platform::instance::wait_exit(pid, std::time::Duration::from_secs(15));
            }
            let instance = if shots.is_some() { None } else { claim_instance() };
            if matches!(instance, Some(None)) {
                return; // the running copy came to the front
            }
            return gui::run(ctx, shots, instance.flatten());
        }
        #[cfg(not(feature = "gui"))]
        {
            let _ = shots;
            console = true;
        }
    }

    let rep = collect(&ctx);
    if console {
        out(&report::to_txt(&rep));
    }
    for o in outs {
        let (name, ext, body) = match o {
            Out::Txt(n) => (n, "txt", report::to_txt(&rep)),
            Out::Html(n) => (n, "html", report::to_html(&rep)),
            Out::Json(n) => (n, "json", report::to_json(&rep)),
            Out::JsonTxt(n) => (n, "txt", report::to_json(&rep)),
        };
        let path = report_path(&name, ext);
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(&path, body) {
            Ok(()) => eprintln!("wrote {}", path.display()),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }
}

/// CPU-Z writes next to the exe; we write relative to the current dir, which
/// works without admin rights. The extension is added when missing.
fn report_path(name: &str, ext: &str) -> std::path::PathBuf {
    let name = if name.is_empty() { NAME } else { name };
    let mut p = std::path::PathBuf::from(name);
    if p.extension().is_none() {
        p.set_extension(ext);
    }
    p
}

//! cpuzeez: clean-room, cross-platform reimplementation of CPU-Z's features.
//! The spec is FEATURES.md; no CPU-Z code was used.

// Style-only lints; the tuple/type shapes are local to one function each.
#![allow(clippy::type_complexity, clippy::field_reassign_with_default)]

mod battery;
mod bench;
mod bios;
mod chipset;
mod config;
mod cpu;
mod diff;
mod gpu;
#[cfg(feature = "gui")]
mod gui;
mod history;
mod hw;
mod membench;
mod network;
mod npu;
mod os;
mod pci;
mod report;
mod sensors;
mod settings;
mod smbios;
mod software;
mod spd;
mod storage;
mod telemetry;
mod timers;
mod update;
mod usb;

use report::{Section, f, section};
use std::sync::OnceLock;

/// Binary, package and file-name stem.
pub const NAME: &str = "cpuzeez";
/// Name shown to people (window title, About, report headers).
pub const DISPLAY_NAME: &str = "CpuZeeZ";
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
[-bench] [-membench] [-core=ID] [-coremask=HEX] [-sensors] [-log=FILE[,SECONDS]] [-diff=OLD.json,NEW.json|now] [-checkupdate] [-screenshots=DIR]\nWith no output option the GUI opens.";

fn main() {
    let mut outs = Vec::new();
    let (mut console, mut bench, mut core, mut coremask) = (false, false, None, None);
    let mut shots: Option<std::path::PathBuf> = None;
    for a in std::env::args().skip(1) {
        let (k, v) = a.split_once('=').map_or((a.as_str(), ""), |(k, v)| (k, v));
        match k.to_ascii_lowercase().trim_start_matches('-') {
            "txt" => outs.push(Out::Txt(v.into())),
            "html" => outs.push(Out::Html(v.into())),
            "json" => outs.push(Out::Json(v.into())),
            "jsontxt" => outs.push(Out::JsonTxt(v.into())),
            "console" => console = true,
            "checkupdate" => return update::cli(),
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
            "h" | "help" | "?" => return println!("{USAGE}"),
            _ => return eprintln!("unknown option {a}\n{USAGE}"),
        }
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
            return gui::run(ctx, shots);
        }
        #[cfg(not(feature = "gui"))]
        {
            let _ = shots;
            console = true;
        }
    }

    let rep = collect(&ctx);
    if console {
        print!("{}", report::to_txt(&rep));
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

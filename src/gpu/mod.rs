//! Graphics, Graphic APIs, Display Adapters, Monitors (FEATURES.md 3.17-3.19, 9).
//! Each OS backend fills a `Probe`; this file owns the model and the report
//! layout (CPU-Z's field names and value formats).

pub mod db;
#[cfg(any(windows, target_os = "linux"))]
pub(crate) mod dl;
pub mod edid;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(windows, target_os = "linux"))]
mod nvml;
#[cfg(windows)]
pub(crate) mod win;

use crate::Ctx;
use crate::hw::Bdf;
use crate::report::{Node, Section, f, g, gv, section};
use std::sync::OnceLock;

pub use edid::Monitor;

/// One performance level ("Base", "Boost", "Current"), clocks in MHz.
#[derive(Debug, Clone, Default)]
pub struct Perf {
    pub level: &'static str,
    pub core: Option<f64>,
    pub shader: Option<f64>,
    pub mem: Option<f64>,
}

/// PCIe link: (lanes, GT/s).
#[derive(Debug, Clone, Copy, Default)]
pub struct Link {
    pub width: Option<u32>,
    pub speed: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct Adapter {
    pub name: String,
    pub vendor_id: u16,
    pub device_id: u16,
    /// (subsystem vendor, subsystem device)
    pub subsys: (u16, u16),
    pub rev: u8,
    pub board_vendor: Option<String>,
    pub codename: Option<String>,
    /// Silicon revision as printed ("A1"), NVIDIA only.
    pub revision: Option<String>,
    pub core_family: Option<String>,
    pub cores: Option<u32>,
    pub rops: Option<u32>,
    pub tmus: Option<u32>,
    pub rt_cores: Option<u32>,
    pub tensor_cores: Option<u32>,
    pub technology_nm: Option<u32>,
    pub mem_type: Option<String>,
    pub mem_vendor: Option<String>,
    pub mem_size_mb: Option<u64>,
    pub bus_width: Option<u32>,
    /// W: current, default, max
    pub power_limits: [Option<f64>; 3],
    /// °C: max operating, slowdown, shutdown
    pub thermal_limits: [Option<f64>; 3],
    pub link: Link,
    pub link_max: Link,
    pub pci: Option<Bdf>,
    pub root: Option<Bdf>,
    pub perf: Vec<Perf>,
    pub driver_version: Option<String>,
    /// Windows: WDDM version ("3.2"); elsewhere the kernel driver name.
    pub driver_model: Option<String>,
    pub primary: bool,
    /// OS-specific extra lines (e.g. Metal family on macOS).
    pub extra: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Probe {
    pub apis: Vec<Node>,
    pub adapters: Vec<Adapter>,
    /// Top-level lines between adapters and monitors (Win32_VideoController).
    pub extra: Vec<Node>,
    pub monitors: Vec<Monitor>,
}

/// (adapter index, core MHz, memory MHz, temperature °C, power W, load %)
pub type LiveRow = (usize, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>);

static PROBE: OnceLock<Probe> = OnceLock::new();

fn probe(ctx: &Ctx) -> &'static Probe {
    PROBE.get_or_init(|| {
        if !ctx.cfg.display {
            return Probe::default();
        }
        let mut p = backend(ctx.cfg.use_display_api);
        for a in &mut p.adapters {
            fill_from_db(a);
        }
        p
    })
}

#[cfg(windows)]
fn backend(api: bool) -> Probe {
    win::probe(api)
}
#[cfg(target_os = "linux")]
fn backend(api: bool) -> Probe {
    linux::probe(api)
}
#[cfg(target_os = "macos")]
fn backend(_api: bool) -> Probe {
    macos::probe()
}
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn backend(_api: bool) -> Probe {
    Probe::default()
}

/// Static table values fill whatever the live APIs did not report.
fn fill_from_db(a: &mut Adapter) {
    if a.board_vendor.is_none() && a.subsys.0 != 0 {
        a.board_vendor = db::board_vendor(a.subsys.0).map(str::to_string);
    }
    if a.vendor_id == 0x10DE && a.revision.is_none() && a.rev != 0 {
        a.revision = Some(format!("{:X}", a.rev));
    }
    let Some(s) = db::lookup(a.vendor_id, a.device_id, a.rev) else { return };
    let nz = |v: u32| (v != 0).then_some(v);
    a.codename.get_or_insert_with(|| s.codename.to_string());
    a.cores = a.cores.or(nz(s.cores));
    a.rops = a.rops.or(nz(s.rops));
    a.tmus = a.tmus.or(nz(s.tmus));
    a.rt_cores = a.rt_cores.or(nz(s.rt));
    a.tensor_cores = a.tensor_cores.or(nz(s.tensor));
    a.technology_nm = a.technology_nm.or(nz(s.nm));
    a.bus_width = a.bus_width.or(nz(s.bus));
    if a.mem_type.is_none() && !s.mem.is_empty() {
        a.mem_type = Some(s.mem.into());
    }
}

/// Every display adapter (GUI Graphics tab).
pub fn adapters(ctx: &Ctx) -> &'static [Adapter] {
    &probe(ctx).adapters
}

/// Every attached monitor with a readable EDID.
pub fn monitors(ctx: &Ctx) -> &'static [Monitor] {
    &probe(ctx).monitors
}

/// Live clocks / temperature / power / load per adapter (Tools > Clocks).
pub fn live(ctx: &Ctx) -> Vec<LiveRow> {
    let ads = adapters(ctx);
    if ads.is_empty() {
        return Vec::new();
    }
    #[cfg(windows)]
    return win::live(ads, ctx.cfg.use_display_api);
    #[cfg(target_os = "linux")]
    return linux::live(ads, ctx.cfg.use_display_api);
    #[allow(unreachable_code)]
    Vec::new()
}

/// Map NVML per-bus readings onto adapter indices (shared by Windows/Linux).
#[cfg(any(windows, target_os = "linux"))]
fn nvml_live(ads: &[Adapter], out: &mut Vec<LiveRow>) {
    let Some(n) = nvml::get() else { return };
    for ((bus, dev), v) in n.live() {
        if let Some(i) = ads.iter().position(|a| a.pci.is_some_and(|p| p.bus as u32 == bus && p.dev as u32 == dev)) {
            out.push((i, v[0], v[1], v[2], v[3], v[4]));
        }
    }
}

/// Apply NVML static info to the matching NVIDIA adapter.
#[cfg(any(windows, target_os = "linux"))]
fn apply_nvml(ads: &mut Vec<Adapter>, apis: &mut Vec<Node>) {
    let Some(n) = nvml::get() else { return };
    apis.push(f("API", "NVIDIA NVML"));
    for i in n.info() {
        let at = |a: &Adapter| a.pci.is_some_and(|p| p.bus as u32 == i.bus && p.dev as u32 == i.dev);
        if !ads.iter().any(at) {
            // GPU without an OS display device (WSL's /dev/dxg, headless): NVML alone.
            ads.push(Adapter {
                vendor_id: i.ids.0 as u16,
                device_id: (i.ids.0 >> 16) as u16,
                subsys: (i.ids.1 as u16, (i.ids.1 >> 16) as u16),
                pci: Some(Bdf { seg: i.domain as u16, bus: i.bus as u8, dev: i.dev as u8, func: 0 }),
                ..Default::default()
            });
        }
        let Some(a) = ads.iter_mut().find(|a| at(a)) else { continue };
        if let Some(nm) = &i.name {
            a.name = nm.clone(); // NVML's marketing name beats pci.ids
        }
        a.cores = a.cores.or(i.cores);
        a.bus_width = a.bus_width.or(i.bus_width);
        a.mem_size_mb = i.mem_total.map(|b| b >> 20).or(a.mem_size_mb);
        for k in 0..3 {
            a.power_limits[k] = a.power_limits[k].or(i.power[k]);
            a.thermal_limits[k] = a.thermal_limits[k].or(i.thermal[k]);
        }
        let gts = |g: Option<u32>| g.and_then(gen_speed);
        a.link.width = a.link.width.or(i.link[0]);
        a.link.speed = a.link.speed.or(gts(i.link[1]));
        a.link_max.width = a.link_max.width.or(i.link[2]);
        a.link_max.speed = a.link_max.speed.or(gts(i.link[3]));
        if !a.perf.iter().any(|p| p.level == "Current") {
            a.perf.push(Perf { level: "Current", core: i.cur_clock[0], shader: None, mem: i.cur_clock[1] });
        }
        if !a.perf.iter().any(|p| p.level == "Boost") && a.perf.iter().all(|p| p.level == "Current") {
            // No base/boost from NVAPI: NVML's max clocks are the best we have.
            a.perf.insert(0, Perf { level: "Max", core: i.max_clock[0], shader: None, mem: i.max_clock[1] });
        }
    }
}

/// PCIe generation → GT/s.
pub fn gen_speed(g: u32) -> Option<f64> {
    [2.5, 5.0, 8.0, 16.0, 32.0, 64.0].get(g.checked_sub(1)? as usize).copied()
}

fn loc(b: Bdf) -> String {
    format!("bus {0} (0x{0:X}), device {1} (0x{1:X}), function {2} (0x{2:X})", b.bus, b.dev, b.func)
}

fn mhz(v: f64) -> String {
    format!("{v:.1} MHz")
}

fn adapter_nodes(a: &Adapter) -> Vec<Node> {
    let mut n = vec![f("Name", &a.name)];
    fn opt(n: &mut Vec<Node>, k: &str, v: Option<String>) {
        if let Some(v) = v {
            n.push(f(k, v));
        }
    }
    opt(&mut n, "Board Manufacturer", a.board_vendor.clone());
    opt(&mut n, "Revision", a.revision.clone());
    opt(&mut n, "Codename", a.codename.clone());
    opt(&mut n, "Core family", a.core_family.clone());
    let triple = |v: &[Option<f64>; 3], unit: &str, names: [&str; 3]| {
        let parts: Vec<String> =
            v.iter().zip(names).filter_map(|(x, nm)| x.map(|x| format!("{x:.1} {unit} ({nm})"))).collect();
        (!parts.is_empty()).then(|| parts.join(", "))
    };
    opt(&mut n, "Power Limit", triple(&a.power_limits, "W", ["current", "default", "max"]));
    opt(&mut n, "Thermal Limit", triple(&a.thermal_limits, "°C", ["max", "slowdown", "shutdown"]));
    let s = |v: Option<u32>| v.map(|v| v.to_string());
    opt(&mut n, "Cores", s(a.cores));
    opt(&mut n, "ROP Units", s(a.rops));
    opt(&mut n, "TM Units", s(a.tmus));
    opt(&mut n, "RayTracing Cores", s(a.rt_cores));
    opt(&mut n, "Tensor Cores", s(a.tensor_cores));
    opt(&mut n, "Technology", a.technology_nm.map(|v| format!("{v} nm")));
    opt(&mut n, "Memory type", a.mem_type.clone());
    opt(&mut n, "Memory vendor", a.mem_vendor.clone());
    opt(&mut n, "Memory size", a.mem_size_mb.map(|v| format!("{v} MB")));
    opt(&mut n, "Memory bus width", a.bus_width.map(|v| format!("{v} bits")));
    opt(&mut n, "Current Link Width", a.link.width.map(|v| format!("x{v}")));
    opt(&mut n, "Current Link Speed", a.link.speed.map(|v| format!("{v:.1} GT/s")));
    opt(&mut n, "Max Link Width", a.link_max.width.filter(|&w| Some(w) != a.link.width).map(|v| format!("x{v}")));
    opt(
        &mut n,
        "Max Link Speed",
        a.link_max.speed.filter(|&w| Some(w) != a.link.speed).map(|v| format!("{v:.1} GT/s")),
    );
    if let Some(p) = a.pci {
        let mut ids = vec![
            f("Vendor ID", format!("0x{:04X} (0x{:04X})", a.vendor_id, a.subsys.0)),
            f("Model ID", format!("0x{:04X} (0x{:04X})", a.device_id, a.subsys.1)),
        ];
        if a.rev != 0 {
            ids.push(f("Revision ID", format!("0x{:02X}", a.rev)));
        }
        n.push(gv("PCI device", loc(p), ids));
    }
    if let Some(r) = a.root {
        n.push(f("Root device", loc(r)));
    }
    for p in &a.perf {
        let mut c = Vec::new();
        for (k, v) in [("Core clock", p.core), ("Shader clock", p.shader), ("Memory clock", p.mem)] {
            if let Some(v) = v {
                c.push(f(k, mhz(v)));
            }
        }
        if !c.is_empty() {
            n.push(gv("Performance Level", p.level, c));
        }
    }
    for (k, v) in &a.extra {
        n.push(f(k, v));
    }
    opt(&mut n, "Driver version", a.driver_version.clone());
    let model_key = if cfg!(windows) { "WDDM Model" } else { "Driver" };
    opt(&mut n, model_key, a.driver_model.clone());
    n
}

fn monitor_nodes(m: &Monitor) -> Vec<Node> {
    let mut n = Vec::new();
    let model = m.model.as_ref().map_or_else(|| m.vendor.clone(), |md| format!("{md} ({})", m.vendor));
    n.push(f("Model", model));
    if !m.id.is_empty() {
        n.push(f("ID", &m.id));
    }
    if let Some(s) = &m.serial {
        n.push(f("Serial", s));
    }
    if m.year != 0 {
        n.push(f(
            "Manufacturing Date",
            if m.week == 0xFF { format!("Model Year {}", m.year) } else { format!("Week {}, Year {}", m.week, m.year) },
        ));
    }
    if let Some(s) = m.size_in {
        n.push(f("Size", format!("{s:.1} inches")));
    }
    if let Some((w, h, hz)) = m.max_res {
        n.push(f("Max Resolution", format!("{w} x {h} @ {hz} Hz")));
    }
    if let Some((a, b)) = m.h_range_khz {
        n.push(f("Horizontal Freq. Range", format!("{a}-{b} kHz")));
    }
    if let Some((a, b)) = m.v_range_hz {
        n.push(f("Vertical Freq. Range", format!("{a}-{b} Hz")));
    }
    if let Some(p) = m.max_pclk_mhz {
        n.push(f("Max Pixel Clock", format!("{p} MHz")));
    }
    if let Some(gm) = m.gamma {
        n.push(f("Gamma Factor", format!("{gm:.1}")));
    }
    if !m.input.is_empty() {
        n.push(f("Video Input", &m.input));
    }
    n
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    if !ctx.cfg.display {
        return Vec::new();
    }
    let p = probe(ctx);
    let mut da: Vec<Node> = p
        .adapters
        .iter()
        .enumerate()
        .map(|(i, a)| g(format!("Display adapter {i}{}", if a.primary { " (primary)" } else { "" }), adapter_nodes(a)))
        .collect();
    da.extend(p.extra.iter().cloned());
    da.extend(p.monitors.iter().enumerate().map(|(i, m)| g(format!("Monitor {i}"), monitor_nodes(m))));
    vec![
        section("Graphics", vec![f("Number of adapters", p.adapters.len())]),
        section("Graphic APIs", p.apis.clone()),
        section("Display Adapters", da),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layout() {
        let mut a = Adapter {
            name: "NVIDIA GeForce RTX 4060".into(),
            vendor_id: 0x10DE,
            device_id: 0x2882,
            subsys: (0x1462, 0x5161),
            rev: 0xA1,
            power_limits: [Some(115.0), Some(115.0), Some(115.0)],
            pci: Some(Bdf { seg: 0, bus: 1, dev: 0, func: 0 }),
            perf: vec![Perf { level: "Base", core: Some(1830.0), shader: None, mem: Some(8501.0) }],
            ..Default::default()
        };
        fill_from_db(&mut a);
        let r = vec![section("Display Adapters", vec![g("Display adapter 0", adapter_nodes(&a))])];
        let t = crate::report::to_txt(&r);
        assert!(t.contains("\tBoard Manufacturer\tMicro-Star International Co., Ltd. (MSI)\n"));
        assert!(t.contains("\tRevision\t\tA1\n\tCodename\t\tAD107-400\n"));
        assert!(t.contains("\tPower Limit\t\t115.0 W (current), 115.0 W (default), 115.0 W (max)\n"));
        assert!(t.contains(
            "\tPCI device\t\tbus 1 (0x1), device 0 (0x0), function 0 (0x0)\n\t\tVendor ID\t0x10DE (0x1462)\n"
        ));
        assert!(t.contains("\tPerformance Level\tBase\n\t\tCore clock\t1830.0 MHz\n"));
        assert!(t.contains("\tMemory bus width\t128 bits\n"));
        assert_eq!(gen_speed(4), Some(16.0));
    }

    #[test]
    fn live_rows_point_at_adapters() {
        // Real hardware when present; trivially passes on GPU-less machines.
        let ctx = crate::Ctx::new(crate::config::Config::default());
        let n = adapters(&ctx).len();
        for r in live(&ctx) {
            println!("{r:?}");
            assert!(r.0 < n);
        }
    }
}

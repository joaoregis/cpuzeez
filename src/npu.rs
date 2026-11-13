//! AI Devices and NPU Devices sections (FEATURES.md 3.20).
//! Windows: DXCore compute/ML adapters without graphics. Linux: the kernel's
//! compute accelerator class (`/sys/class/accel`). macOS: the Apple Neural Engine.
use crate::Ctx;
use crate::report::{Node, Section, f, g, section};

#[derive(Debug, Clone, Default)]
pub struct Npu {
    pub name: String,
    pub vendor_id: u16,
    pub device_id: u16,
    pub driver: Option<String>,
    pub driver_version: Option<String>,
    pub mem_mb: Option<u64>,
    pub cores: Option<u32>,
}

pub fn list(ctx: &Ctx) -> Vec<Npu> {
    if ctx.cfg.display { devices() } else { Vec::new() }
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let list = list(ctx);
    let ai = list.iter().map(|n| f("AI Device", &n.name)).collect();
    let npu = list
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mut c = vec![f("Name", &n.name)];
            if n.vendor_id != 0 {
                c.push(f("Vendor", crate::gpu::db::vendor_name(n.vendor_id)));
                c.push(f("Vendor ID", format!("0x{:04X}", n.vendor_id)));
                c.push(f("Model ID", format!("0x{:04X}", n.device_id)));
            }
            let opt = [
                ("Cores", n.cores.map(|v| v.to_string())),
                ("Dedicated memory", n.mem_mb.map(|v| format!("{v} MB"))),
                ("Driver", n.driver.clone()),
                ("Driver version", n.driver_version.clone()),
            ];
            c.extend(opt.into_iter().filter_map(|(k, v)| v.map(|v| f(k, v))));
            g(format!("NPU {i}"), c)
        })
        .collect::<Vec<Node>>();
    vec![section("AI Devices", ai), section("NPU Devices", npu)]
}

#[cfg(windows)]
fn devices() -> Vec<Npu> {
    win::devices()
}

#[cfg(target_os = "linux")]
fn devices() -> Vec<Npu> {
    let Ok(rd) = std::fs::read_dir("/sys/class/accel") else { return Vec::new() };
    let mut v: Vec<Npu> = rd
        .flatten()
        .filter_map(|e| {
            let d = e.path().join("device");
            let id = |n: &str| crate::os::read_u64(d.join(n)).unwrap_or(0) as u16;
            let driver = std::fs::read_link(d.join("driver")).ok()?.file_name()?.to_string_lossy().into_owned();
            let name = match driver.as_str() {
                "intel_vpu" => "Intel AI Boost (NPU)".to_string(),
                "amdxdna" => "AMD XDNA NPU".into(),
                "qaic" => "Qualcomm Cloud AI".into(),
                "habanalabs" => "Intel Gaudi".into(),
                "ethosu" | "rocket" => "Arm/Rockchip NPU".into(),
                d => format!("{d} accelerator"),
            };
            let driver_version = crate::os::read_trim(format!("/sys/module/{driver}/version"));
            Some(Npu {
                name,
                vendor_id: id("vendor"),
                device_id: id("device"),
                driver: Some(driver),
                driver_version,
                ..Default::default()
            })
        })
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

#[cfg(target_os = "macos")]
fn devices() -> Vec<Npu> {
    if !cfg!(target_arch = "aarch64") {
        return Vec::new(); // Intel Macs have no Neural Engine
    }
    // Every Apple M-series SoC has a 16-core Neural Engine; Ultra (two dies) has 32.
    let brand = crate::os::cmd("sysctl", &["-n", "machdep.cpu.brand_string"]).unwrap_or_default();
    let cores = if brand.contains("Ultra") { 32 } else { 16 };
    vec![Npu { name: "Apple Neural Engine".into(), vendor_id: 0x106B, cores: Some(cores), ..Default::default() }]
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn devices() -> Vec<Npu> {
    Vec::new()
}

#[cfg(windows)]
mod win {
    use super::Npu;

    pub fn devices() -> Vec<Npu> {
        let mut out: Vec<Npu> = Vec::new();
        // IsHardware and no graphics: an NPU / compute-only accelerator.
        for a in cpuzeez_platform::dxcore::compute_adapters().into_iter().filter(|a| a.hardware && !a.graphics) {
            // DXCoreHardwareID {vendor, device, subsys, rev}
            let id = |i: usize| a.hardware_id.get(i * 4..i * 4 + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]));
            let n = Npu {
                name: a.name.clone().unwrap_or_else(|| "NPU".into()),
                vendor_id: id(0),
                device_id: id(1),
                driver_version: a
                    .driver_version
                    .map(|v| format!("{}.{}.{}.{}", v >> 48, (v >> 32) & 0xFFFF, (v >> 16) & 0xFFFF, v & 0xFFFF)),
                mem_mb: a.dedicated_memory.map(|v| v >> 20).filter(|&m| m > 0),
                ..Default::default()
            };
            if !out.iter().any(|o| (&o.name, o.vendor_id, o.device_id) == (&n.name, n.vendor_id, n.device_id)) {
                out.push(n);
            }
        }
        out
    }
}

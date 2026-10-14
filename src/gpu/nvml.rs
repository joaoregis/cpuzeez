//! NVIDIA Management Library, loaded at run time (public `nvml.h` API).

use super::dl::{Lib, sym};
use std::ffi::{c_char, c_uint, c_void};
use std::sync::OnceLock;

type Dev = *mut c_void;
type R = i32; // nvmlReturn_t, 0 = success

#[repr(C)]
struct PciInfo {
    bus_id_legacy: [c_char; 16],
    domain: c_uint,
    bus: c_uint,
    device: c_uint,
    pci_device_id: c_uint,
    pci_subsystem_id: c_uint,
    bus_id: [c_char; 32],
}

#[repr(C)]
#[derive(Default)]
struct Memory {
    total: u64,
    free: u64,
    used: u64,
}

#[repr(C)]
#[derive(Default)]
struct Util {
    gpu: c_uint,
    memory: c_uint,
}

pub struct Nvml {
    _lib: Lib,
    count: unsafe extern "C" fn(*mut c_uint) -> R,
    handle: unsafe extern "C" fn(c_uint, *mut Dev) -> R,
    pci: unsafe extern "C" fn(Dev, *mut PciInfo) -> R,
    u1: Vec<(&'static str, unsafe extern "C" fn(Dev, *mut c_uint) -> R)>,
    u2: Vec<(&'static str, unsafe extern "C" fn(Dev, c_uint, *mut c_uint) -> R)>,
    limits: Option<unsafe extern "C" fn(Dev, *mut c_uint, *mut c_uint) -> R>,
    mem: Option<unsafe extern "C" fn(Dev, *mut Memory) -> R>,
    util: Option<unsafe extern "C" fn(Dev, *mut Util) -> R>,
    name: Option<unsafe extern "C" fn(Dev, *mut c_char, c_uint) -> R>,
    pub driver: Option<String>,
}
// SAFETY: NVML is documented as thread safe.
unsafe impl Send for Nvml {}
unsafe impl Sync for Nvml {}

/// Static data for one NVIDIA GPU. Clocks in MHz, power in W, temps in °C.
#[derive(Debug, Default, Clone)]
pub struct Info {
    pub domain: u32,
    /// device << 16 | vendor, subdevice << 16 | subvendor
    pub ids: (u32, u32),
    pub bus: u32,
    pub dev: u32,
    pub name: Option<String>,
    pub cores: Option<u32>,
    pub bus_width: Option<u32>,
    pub mem_total: Option<u64>,
    pub power: [Option<f64>; 3],     // current, default, max
    pub thermal: [Option<f64>; 3],   // max operating, slowdown, shutdown
    pub link: [Option<u32>; 4],      // cur width, cur gen, max width, max gen
    pub max_clock: [Option<f64>; 2], // graphics, memory
    pub cur_clock: [Option<f64>; 2],
}

pub fn get() -> Option<&'static Nvml> {
    static N: OnceLock<Option<Nvml>> = OnceLock::new();
    N.get_or_init(load).as_ref()
}

fn load() -> Option<Nvml> {
    #[cfg(windows)]
    let names = ["nvml.dll", "C:\\Program Files\\NVIDIA Corporation\\NVSMI\\nvml.dll"];
    #[cfg(unix)]
    let names = ["libnvidia-ml.so.1", "libnvidia-ml.so"];
    let lib = Lib::open(&names)?;
    let init: unsafe extern "C" fn() -> R = sym!(lib, "nvmlInit_v2")?;
    if unsafe { init() } != 0 {
        return None;
    }
    type F1 = unsafe extern "C" fn(Dev, *mut c_uint) -> R;
    type F2 = unsafe extern "C" fn(Dev, c_uint, *mut c_uint) -> R;
    let one = |n: &'static str, k: &'static str| -> Option<(&'static str, F1)> { sym!(lib, n).map(|f| (k, f)) };
    let two = |n: &'static str, k: &'static str| -> Option<(&'static str, F2)> { sym!(lib, n).map(|f| (k, f)) };
    let u1 = [
        one("nvmlDeviceGetNumGpuCores", "cores"),
        one("nvmlDeviceGetMemoryBusWidth", "buswidth"),
        one("nvmlDeviceGetPowerManagementLimit", "plimit"),
        one("nvmlDeviceGetPowerManagementDefaultLimit", "pdefault"),
        one("nvmlDeviceGetCurrPcieLinkWidth", "lw"),
        one("nvmlDeviceGetCurrPcieLinkGeneration", "lg"),
        one("nvmlDeviceGetMaxPcieLinkWidth", "mlw"),
        one("nvmlDeviceGetMaxPcieLinkGeneration", "mlg"),
        one("nvmlDeviceGetPowerUsage", "power"),
    ];
    let u2 = [
        two("nvmlDeviceGetClockInfo", "clock"),
        two("nvmlDeviceGetMaxClockInfo", "maxclock"),
        two("nvmlDeviceGetTemperatureThreshold", "threshold"),
        two("nvmlDeviceGetTemperature", "temp"),
    ];
    let driver =
        sym!(lib, "nvmlSystemGetDriverVersion").and_then(|f: unsafe extern "C" fn(*mut c_char, c_uint) -> R| {
            let mut b = [0 as c_char; 96];
            (unsafe { f(b.as_mut_ptr(), b.len() as c_uint) } == 0).then(|| cstr(&b))
        });
    Some(Nvml {
        count: sym!(lib, "nvmlDeviceGetCount_v2")?,
        handle: sym!(lib, "nvmlDeviceGetHandleByIndex_v2")?,
        pci: sym!(lib, "nvmlDeviceGetPciInfo_v3")?,
        u1: u1.into_iter().flatten().collect(),
        u2: u2.into_iter().flatten().collect(),
        limits: sym!(lib, "nvmlDeviceGetPowerManagementLimitConstraints"),
        mem: sym!(lib, "nvmlDeviceGetMemoryInfo"),
        util: sym!(lib, "nvmlDeviceGetUtilizationRates"),
        name: sym!(lib, "nvmlDeviceGetName"),
        driver,
        _lib: lib,
    })
}

fn cstr(b: &[c_char]) -> String {
    // c_char is i8 on x86 and u8 on ARM Linux, so the cast is needed on some targets.
    #[allow(clippy::unnecessary_cast)]
    let v: Vec<u8> = b.iter().take_while(|&&c| c != 0).map(|&c| c as u8).collect();
    String::from_utf8_lossy(&v).trim().to_string()
}

impl Nvml {
    fn devices(&self) -> Vec<(Dev, PciInfo)> {
        let mut n = 0;
        if unsafe { (self.count)(&mut n) } != 0 {
            return Vec::new();
        }
        (0..n)
            .filter_map(|i| {
                let mut d: Dev = std::ptr::null_mut();
                // SAFETY: PciInfo is plain data, NVML fills it.
                let mut p: PciInfo = unsafe { std::mem::zeroed() };
                (unsafe { (self.handle)(i, &mut d) } == 0 && unsafe { (self.pci)(d, &mut p) } == 0).then_some((d, p))
            })
            .collect()
    }

    fn g1(&self, d: Dev, k: &str) -> Option<u32> {
        let f = self.u1.iter().find(|(n, _)| *n == k)?.1;
        let mut v = 0;
        (unsafe { f(d, &mut v) } == 0).then_some(v)
    }

    fn g2(&self, d: Dev, k: &str, a: u32) -> Option<u32> {
        let f = self.u2.iter().find(|(n, _)| *n == k)?.1;
        let mut v = 0;
        (unsafe { f(d, a, &mut v) } == 0).then_some(v)
    }

    pub fn info(&self) -> Vec<Info> {
        self.devices()
            .into_iter()
            .map(|(d, p)| {
                let w = |mw: Option<u32>| mw.map(|v| v as f64 / 1000.0);
                let mut lim = (0, 0);
                let max = self
                    .limits
                    .and_then(|f| (unsafe { f(d, &mut lim.0, &mut lim.1) } == 0).then(|| lim.1 as f64 / 1000.0));
                let mem = self.mem.and_then(|f| {
                    let mut m = Memory::default();
                    (unsafe { f(d, &mut m) } == 0).then_some(m.total)
                });
                let name = self.name.and_then(|f| {
                    let mut b = [0 as c_char; 96];
                    (unsafe { f(d, b.as_mut_ptr(), 96) } == 0).then(|| cstr(&b))
                });
                let t = |k| self.g2(d, "threshold", k).map(f64::from);
                let mhz = |k, a| self.g2(d, k, a).map(f64::from);
                Info {
                    domain: p.domain,
                    ids: (p.pci_device_id, p.pci_subsystem_id),
                    bus: p.bus,
                    dev: p.device,
                    name,
                    cores: self.g1(d, "cores"),
                    bus_width: self.g1(d, "buswidth"),
                    mem_total: mem,
                    power: [w(self.g1(d, "plimit")), w(self.g1(d, "pdefault")), max],
                    // NVML_TEMPERATURE_THRESHOLD_GPU_MAX = 3, SLOWDOWN = 1, SHUTDOWN = 0
                    thermal: [t(3), t(1), t(0)],
                    link: ["lw", "lg", "mlw", "mlg"].map(|k| self.g1(d, k)),
                    // NVML_CLOCK_GRAPHICS = 0, NVML_CLOCK_MEM = 2
                    max_clock: [mhz("maxclock", 0), mhz("maxclock", 2)],
                    cur_clock: [mhz("clock", 0), mhz("clock", 2)],
                }
            })
            .collect()
    }

    /// (bus, device) → (core MHz, mem MHz, temp °C, power W, load %).
    pub fn live(&self) -> Vec<((u32, u32), [Option<f64>; 5])> {
        self.devices()
            .into_iter()
            .map(|(d, p)| {
                let load = self.util.and_then(|f| {
                    let mut u = Util::default();
                    (unsafe { f(d, &mut u) } == 0).then_some(u.gpu as f64)
                });
                let v = [
                    self.g2(d, "clock", 0).map(f64::from),
                    self.g2(d, "clock", 2).map(f64::from),
                    self.g2(d, "temp", 0).map(f64::from),
                    self.g1(d, "power").map(|mw| mw as f64 / 1000.0),
                    load,
                ];
                ((p.bus, p.device), v)
            })
            .collect()
    }
}

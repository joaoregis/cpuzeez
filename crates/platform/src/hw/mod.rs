//! Privileged hardware access. CPU-Z does all of this through its own kernel
//! driver; we use whatever the OS offers: PawnIO on Windows, the kernel's
//! msr / pci sysfs / port / i2c-dev interfaces on Linux, nothing on macOS.
//! Every method returns None/false when the backend can't do it, and probes
//! must degrade instead of failing.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

pub use cpuzeez_core::pci::Bdf;

static PAUSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// ZeeZ Live's anti-cheat safe mode (S2b-12): while on, no call reaches the
/// PawnIO driver; readings that need it come back empty and the app shows
/// what the OS and the GPU driver give. The driver itself stays loaded.
pub fn pause(on: bool) {
    PAUSED.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn paused() -> bool {
    PAUSED.load(std::sync::atomic::Ordering::Relaxed)
}

/// One SMBus/I2C adapter that can talk to DIMM SPD/thermal/PMIC devices.
pub trait Smbus: Send + Sync {
    fn name(&self) -> String;
    /// SMBus "read byte data".
    fn read_byte(&self, addr: u8, cmd: u8) -> Option<u8>;
    /// SMBus "write byte data" (needed for DDR4 page select / DDR5 MR11 paging).
    fn write_byte(&self, addr: u8, cmd: u8, val: u8) -> bool;
    /// SMBus "quick write"/"send byte" with no data (DDR4 SPA0/SPA1 page select).
    fn send_byte(&self, addr: u8, val: u8) -> bool;
    /// SMBus "read word data", low byte first (JC42.4 DDR4 TSOD temperature).
    fn read_word(&self, _addr: u8, _cmd: u8) -> Option<u16> {
        None
    }
    /// Whole EEPROM if the OS already decoded it for us (Linux ee1004/spd5118).
    fn eeprom(&self, _addr: u8) -> Option<Vec<u8>> {
        None
    }
}

/// What the privileged backend can do here; drives the capability banners.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Caps {
    /// Model-specific registers (per-core clocks, temperatures, power, P-states).
    pub msr: bool,
    /// Super I/O port access (mainboard voltages, fans, temperatures).
    pub superio: bool,
    /// SMBus to the DIMMs (SPD, DIMM temperatures).
    pub smbus: bool,
    /// AMD SMU metrics table.
    pub smu: bool,
    /// Memory-controller registers (live timings): AMD SMN/UMC or Intel MCHBAR.
    pub imc: bool,
    /// Intel PCH (chipset) temperature.
    pub pch: bool,
}

pub trait Hw: Send + Sync {
    fn name(&self) -> String;
    /// What this backend can do on this machine.
    fn caps(&self) -> Caps {
        Caps::default()
    }
    fn rdmsr(&self, _cpu: usize, _msr: u32) -> Option<u64> {
        None
    }
    /// Several MSRs of one CPU, read together: on Windows that's one move to
    /// the CPU instead of one per register, and the values are close in time.
    fn rdmsr_many(&self, cpu: usize, msrs: &[u32]) -> Vec<Option<u64>> {
        msrs.iter().map(|&m| self.rdmsr(cpu, m)).collect()
    }
    fn pci_read(&self, _bdf: Bdf, _off: u16, _buf: &mut [u8]) -> bool {
        false
    }
    fn io_in8(&self, _port: u16) -> Option<u8> {
        None
    }
    fn io_out8(&self, _port: u16, _val: u8) -> bool {
        false
    }
    fn io_in32(&self, _port: u16) -> Option<u32> {
        None
    }
    fn phys_read(&self, _addr: u64, _buf: &mut [u8]) -> bool {
        false
    }
    /// AMD System Management Network (Zen SMU/UMC registers).
    fn amd_smn_read(&self, _addr: u32) -> Option<u32> {
        None
    }
    fn smbus(&self) -> Vec<Box<dyn Smbus>> {
        Vec::new()
    }
    /// Freshly updated AMD SMU PM (metrics) table: (table version, f32 cells).
    /// Layout depends on the version; trailing cells may be padding.
    fn amd_smu_pm_table(&self) -> Option<(u32, Vec<f32>)> {
        None
    }
    /// AMD SMU firmware version, bytes [program.]major.minor.patch (e.g. 0x2E3600 = 46.54.0).
    fn amd_smu_version(&self) -> Option<u32> {
        None
    }
    /// Intel MCHBAR (host bridge memory-controller MMIO) register at `off`.
    fn mchbar_read32(&self, _off: u32) -> Option<u32> {
        None
    }
    fn mchbar_read64(&self, _off: u32) -> Option<u64> {
        None
    }
    /// Intel PCH (chipset) die temperature in °C. Linux gets it from the
    /// kernel's intel_pch_thermal hwmon zone instead (sensors/hwmon.rs).
    fn pch_temp_c(&self) -> Option<f64> {
        None
    }

    fn pci_u32(&self, bdf: Bdf, off: u16) -> Option<u32> {
        let mut b = [0u8; 4];
        self.pci_read(bdf, off, &mut b).then(|| u32::from_le_bytes(b))
    }
}

struct NoHw;
impl Hw for NoHw {
    fn name(&self) -> String {
        "none (user mode only)".into()
    }
}

/// The privileged driver is installed (Windows: PawnIO). Unix needs none.
pub fn driver_installed() -> bool {
    #[cfg(windows)]
    return windows::installed();
    #[allow(unreachable_code)]
    false
}

/// Windows: PawnIO's module blobs are present (`pawnio-modules` next to the exe).
pub fn driver_modules_found() -> bool {
    #[cfg(windows)]
    return windows::modules_found();
    #[allow(unreachable_code)]
    false
}

/// Best backend available on this machine right now.
pub fn detect() -> Box<dyn Hw> {
    #[cfg(target_os = "linux")]
    return linux::open();
    #[cfg(windows)]
    return windows::open();
    #[allow(unreachable_code)]
    Box::new(NoHw)
}

#[allow(dead_code)]
pub(crate) fn none() -> Box<dyn Hw> {
    Box::new(NoHw)
}

#[cfg(test)]
mod pause_tests {
    /// Safe mode closes the PawnIO gate (Windows: `Module::exec` checks it).
    #[test]
    fn pause_flag_round_trips() {
        super::pause(true);
        assert!(super::paused());
        super::pause(false);
        assert!(!super::paused());
    }
}

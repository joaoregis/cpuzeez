//! Privileged hardware access. CPU-Z does all of this through its own kernel
//! driver; we use whatever the OS offers: PawnIO on Windows, the kernel's
//! msr / pci sysfs / port / i2c-dev interfaces on Linux, nothing on macOS.
//! Every method returns None/false when the backend can't do it, and probes
//! must degrade instead of failing.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

/// PCI function address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bdf {
    pub seg: u16,
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
}

impl std::fmt::Display for Bdf {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:04x}:{:02x}:{:02x}.{}", self.seg, self.bus, self.dev, self.func)
    }
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

pub trait Hw: Send + Sync {
    fn name(&self) -> String;
    fn rdmsr(&self, _cpu: usize, _msr: u32) -> Option<u64> {
        None
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
    /// Intel PCH (chipset) die temperature, °C.
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

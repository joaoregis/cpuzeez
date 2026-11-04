use super::{Bdf, Hw, Smbus};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

pub fn open() -> Box<dyn Hw> {
    // msr needs `modprobe msr` and root; probe once so name() is honest.
    let msr = File::open("/dev/cpu/0/msr").is_ok();
    let port = OpenOptions::new().read(true).write(true).open("/dev/port").ok();
    Box::new(Linux { msr, port, smn: Mutex::new(()) })
}

struct Linux {
    msr: bool,
    port: Option<File>,
    smn: Mutex<()>,
}

fn cfg_path(b: Bdf) -> PathBuf {
    format!("/sys/bus/pci/devices/{b}/config").into()
}

const RYZEN_SMU: &str = "/sys/kernel/ryzen_smu_drv";

/// ryzen_smu prints "major.minor.patch" or "program.major.minor.patch"; pack one byte each.
fn parse_smu_version(s: &str) -> Option<u32> {
    let parts: Vec<u32> = s.trim().split('.').map(|p| p.parse().ok().filter(|&n| n < 256)).collect::<Option<_>>()?;
    (3..=4).contains(&parts.len()).then(|| parts.iter().fold(0, |a, &p| a << 8 | p))
}

/// Uncached volatile read of one aligned MMIO register through /dev/mem
/// (needs root; CONFIG_IO_STRICT_DEVMEM blocks ranges a driver has claimed).
fn mmio_read<T: Copy>(addr: u64) -> Option<T> {
    use std::os::unix::fs::OpenOptionsExt;
    let n = std::mem::size_of::<T>() as u64;
    let page = addr & !0xFFF;
    if !addr.is_multiple_of(n) || addr - page + n > 0x1000 {
        return None;
    }
    let f = OpenOptions::new().read(true).custom_flags(libc::O_SYNC).open("/dev/mem").ok()?;
    // SAFETY: maps one page read-only, reads inside it, unmaps.
    unsafe {
        let p = libc::mmap(
            std::ptr::null_mut(),
            0x1000,
            libc::PROT_READ,
            libc::MAP_SHARED,
            f.as_raw_fd(),
            page as libc::off_t,
        );
        if p == libc::MAP_FAILED {
            return None;
        }
        let v = std::ptr::read_volatile(p.cast::<u8>().add((addr - page) as usize).cast::<T>());
        libc::munmap(p, 0x1000);
        Some(v)
    }
}

impl Linux {
    /// Intel host bridge 00:00.0 MCHBAR (0x48/0x4C, bit 0 = enable), same mask as PawnIO's IntelMCHBAR.
    fn mchbar(&self) -> Option<u64> {
        let root = Bdf { seg: 0, bus: 0, dev: 0, func: 0 };
        if self.pci_u32(root, 0)? & 0xFFFF != 0x8086 {
            return None;
        }
        let (lo, hi) = (self.pci_u32(root, 0x48)?, self.pci_u32(root, 0x4C)?);
        let base = ((hi as u64) << 32 | lo as u64) & 0x3FF_FFFF_8000;
        (lo & 1 == 1 && base != 0).then_some(base)
    }

    fn pci_write(&self, b: Bdf, off: u16, data: &[u8]) -> bool {
        OpenOptions::new().write(true).open(cfg_path(b)).and_then(|f| f.write_all_at(data, off as u64)).is_ok()
    }
}

impl Hw for Linux {
    fn name(&self) -> String {
        let mut v = vec!["linux sysfs"];
        if self.msr {
            v.push("msr");
        }
        if self.port.is_some() {
            v.push("/dev/port");
        }
        v.join(" + ")
    }

    fn rdmsr(&self, cpu: usize, msr: u32) -> Option<u64> {
        let f = File::open(format!("/dev/cpu/{cpu}/msr")).ok()?;
        let mut b = [0u8; 8];
        f.read_exact_at(&mut b, msr as u64).ok()?;
        Some(u64::from_le_bytes(b))
    }

    fn pci_read(&self, b: Bdf, off: u16, buf: &mut [u8]) -> bool {
        // Unprivileged reads of sysfs config return only the first 64 bytes.
        File::open(cfg_path(b)).and_then(|f| f.read_exact_at(buf, off as u64)).is_ok()
    }

    fn io_in8(&self, port: u16) -> Option<u8> {
        let mut b = [0u8];
        self.port.as_ref()?.read_exact_at(&mut b, port as u64).ok()?;
        Some(b[0])
    }

    fn io_out8(&self, port: u16, val: u8) -> bool {
        self.port.as_ref().is_some_and(|f| f.write_all_at(&[val], port as u64).is_ok())
    }

    fn io_in32(&self, port: u16) -> Option<u32> {
        let mut b = [0u8; 4];
        self.port.as_ref()?.read_exact_at(&mut b, port as u64).ok()?;
        Some(u32::from_le_bytes(b))
    }

    fn phys_read(&self, addr: u64, buf: &mut [u8]) -> bool {
        // Works only for ranges STRICT_DEVMEM allows (e.g. BIOS area).
        File::open("/dev/mem").and_then(|f| f.read_exact_at(buf, addr)).is_ok()
    }

    fn amd_smn_read(&self, addr: u32) -> Option<u32> {
        // SMN index/data pair in the root complex config space (00:00.0, 0x60/0x64).
        let root = Bdf { seg: 0, bus: 0, dev: 0, func: 0 };
        let _g = self.smn.lock().ok()?;
        if !self.pci_write(root, 0x60, &addr.to_le_bytes()) {
            return None;
        }
        self.pci_u32(root, 0x64)
    }

    fn amd_smu_pm_table(&self) -> Option<(u32, Vec<f32>)> {
        // Out-of-tree ryzen_smu driver; files are root-only binary (LE u32, f32 array).
        let v = std::fs::read(format!("{RYZEN_SMU}/pm_table_version")).ok()?;
        let ver = u32::from_le_bytes(v.get(..4)?.try_into().ok()?);
        let t = std::fs::read(format!("{RYZEN_SMU}/pm_table")).ok()?;
        let cells: Vec<f32> =
            t.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        (!cells.is_empty()).then_some((ver, cells))
    }

    fn amd_smu_version(&self) -> Option<u32> {
        parse_smu_version(&std::fs::read_to_string(format!("{RYZEN_SMU}/version")).ok()?)
    }

    fn mchbar_read32(&self, off: u32) -> Option<u32> {
        mmio_read(self.mchbar()? + off as u64)
    }

    fn mchbar_read64(&self, off: u32) -> Option<u64> {
        mmio_read(self.mchbar()? + off as u64)
    }

    fn smbus(&self) -> Vec<Box<dyn Smbus>> {
        let mut v: Vec<Box<dyn Smbus>> = Vec::new();
        let Ok(rd) = std::fs::read_dir("/sys/class/i2c-adapter") else {
            return v;
        };
        let mut ents: Vec<_> = rd.flatten().collect();
        ents.sort_by_key(|e| e.file_name());
        for e in ents {
            let name = std::fs::read_to_string(e.path().join("name")).unwrap_or_default();
            let name = name.trim().to_string();
            // Only real SMBus host controllers have DIMMs behind them.
            let lname = name.to_lowercase();
            if !(lname.contains("smbus") || lname.contains("piix4") || lname.contains("i801")) {
                continue;
            }
            let dev = format!("/dev/{}", e.file_name().to_string_lossy());
            let file = OpenOptions::new().read(true).write(true).open(&dev).ok();
            v.push(Box::new(I2cDev { name, sys: e.path(), file: Mutex::new(file) }));
        }
        v
    }
}

// ---- i2c-dev SMBus ----

const I2C_SLAVE_FORCE: libc::c_ulong = 0x0706;
const I2C_SMBUS: libc::c_ulong = 0x0720;
const I2C_SMBUS_READ: u8 = 1;
const I2C_SMBUS_WRITE: u8 = 0;
const I2C_SMBUS_BYTE: u32 = 1;
const I2C_SMBUS_BYTE_DATA: u32 = 2;
const I2C_SMBUS_WORD_DATA: u32 = 3;

#[repr(C)]
struct SmbusIoctl {
    read_write: u8,
    command: u8,
    size: u32,
    data: *mut [u8; 34],
}

struct I2cDev {
    name: String,
    sys: PathBuf,
    file: Mutex<Option<File>>,
}

impl I2cDev {
    fn xfer(&self, addr: u8, rw: u8, cmd: u8, size: u32, val: u8) -> Option<u16> {
        let g = self.file.lock().ok()?;
        let fd = g.as_ref()?.as_raw_fd();
        let mut data = [0u8; 34];
        data[0] = val;
        let mut req = SmbusIoctl { read_write: rw, command: cmd, size, data: &mut data };
        // SAFETY: fd is an open i2c-dev node; both ioctls take the documented structs.
        unsafe {
            if libc::ioctl(fd, I2C_SLAVE_FORCE as _, addr as libc::c_ulong) < 0 {
                return None;
            }
            if libc::ioctl(fd, I2C_SMBUS as _, &mut req) < 0 {
                return None;
            }
        }
        Some(u16::from_le_bytes([data[0], data[1]]))
    }
}

impl Smbus for I2cDev {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn read_byte(&self, addr: u8, cmd: u8) -> Option<u8> {
        self.xfer(addr, I2C_SMBUS_READ, cmd, I2C_SMBUS_BYTE_DATA, 0).map(|v| v as u8)
    }
    fn write_byte(&self, addr: u8, cmd: u8, val: u8) -> bool {
        self.xfer(addr, I2C_SMBUS_WRITE, cmd, I2C_SMBUS_BYTE_DATA, val).is_some()
    }
    fn send_byte(&self, addr: u8, val: u8) -> bool {
        self.xfer(addr, I2C_SMBUS_WRITE, val, I2C_SMBUS_BYTE, 0).is_some()
    }
    fn read_word(&self, addr: u8, cmd: u8) -> Option<u16> {
        self.xfer(addr, I2C_SMBUS_READ, cmd, I2C_SMBUS_WORD_DATA, 0)
    }
    fn eeprom(&self, addr: u8) -> Option<Vec<u8>> {
        // Bound by ee1004 (DDR4) or spd5118 (DDR5): <bus>-00<addr>/eeprom.
        let bus = self.sys.file_name()?.to_string_lossy().trim_start_matches("i2c-").to_string();
        let p = self.sys.join(format!("{bus}-{addr:04x}/eeprom"));
        std::fs::read(p).ok().filter(|b| !b.is_empty())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smu_version() {
        assert_eq!(super::parse_smu_version("46.54.0\n"), Some(0x2E3600));
        assert_eq!(super::parse_smu_version("4.76.83.0"), Some(0x044C_5300));
        assert_eq!(super::parse_smu_version("x"), None);
    }
}

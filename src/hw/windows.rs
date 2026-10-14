//! Windows backend over PawnIO (https://pawnio.eu), a signed open-source
//! driver that runs signed Pawn modules. Nothing privileged is possible from
//! user mode alone, so without PawnIO this falls back to `super::none()`.
//!
//! API: PawnIOLib.h in https://github.com/namazso/PawnIO (PawnIOLib/include).
//! Modules: https://github.com/namazso/PawnIO.Modules (function names and
//! in/out layouts below come from the `///` docs of each `.p` file).
//! One handle holds exactly one module, so each module gets its own handle.
//! Module blobs are searched as `<Name>.bin` in `<exe dir>\pawnio-modules`,
//! then the PawnIO install dir; see docs/windows-pawnio.md.
#![cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]

use super::{Hw, Smbus};
use std::ffi::{CString, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type Handle = *mut c_void;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> Handle;
    fn GetProcAddress(m: Handle, name: *const u8) -> *mut c_void;
    fn CreateMutexW(attr: *mut c_void, initial: i32, name: *const u16) -> Handle;
    fn OpenMutexW(access: u32, inherit: i32, name: *const u16) -> Handle;
    fn WaitForSingleObject(h: Handle, ms: u32) -> u32;
    fn ReleaseMutex(h: Handle) -> i32;
}

// PawnIOLib exports are HRESULT STDAPICALLTYPE (= "system").
type OpenFn = unsafe extern "system" fn(*mut Handle) -> i32;
type LoadFn = unsafe extern "system" fn(Handle, *const u8, usize) -> i32;
type ExecFn = unsafe extern "system" fn(Handle, *const u8, *const u64, usize, *mut u64, usize, *mut usize) -> i32;
type CloseFn = unsafe extern "system" fn(Handle) -> i32;
type VersionFn = unsafe extern "system" fn(*mut u32) -> i32;

struct Lib {
    open: OpenFn,
    load: LoadFn,
    exec: ExecFn,
    close: CloseFn,
    version: VersionFn,
}

const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO";

fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

/// The installer has no custom path option; registry first, then the default.
fn install_dir() -> Option<PathBuf> {
    crate::os::win::reg_str(UNINSTALL_KEY, "InstallLocation")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("ProgramFiles").map(|p| Path::new(&p).join("PawnIO")))
}

impl Lib {
    /// Loaded once and never freed: modules hold handles for the process lifetime.
    fn load(dir: &Path) -> Option<&'static Lib> {
        let dll = dir.join("PawnIOLib.dll");
        if !dll.is_file() {
            return None;
        }
        // SAFETY: plain Win32 calls; each pointer is checked before transmuting
        // to the signature declared in PawnIOLib.h.
        unsafe {
            let m = LoadLibraryW(wide(&dll).as_ptr());
            if m.is_null() {
                return None;
            }
            let sym = |n: &[u8]| {
                let p = GetProcAddress(m, n.as_ptr());
                (!p.is_null()).then_some(p)
            };
            Some(Box::leak(Box::new(Lib {
                open: std::mem::transmute::<*mut c_void, OpenFn>(sym(b"pawnio_open\0")?),
                load: std::mem::transmute::<*mut c_void, LoadFn>(sym(b"pawnio_load\0")?),
                exec: std::mem::transmute::<*mut c_void, ExecFn>(sym(b"pawnio_execute\0")?),
                close: std::mem::transmute::<*mut c_void, CloseFn>(sym(b"pawnio_close\0")?),
                version: std::mem::transmute::<*mut c_void, VersionFn>(sym(b"pawnio_version\0")?),
            })))
        }
    }
}

/// PawnIOLib copies the name into a 32-byte field and rejects len >= 32.
fn fn_name(name: &str) -> Option<CString> {
    (name.len() < 32).then(|| CString::new(name).ok()).flatten()
}

/// pawnio_version format: (major << 16) | (minor << 8) | patch.
fn version_str(v: u32) -> String {
    format!("{}.{}.{}", v >> 16, (v >> 8) & 0xFF, v & 0xFF)
}

/// One loaded module on its own handle; the Mutex serializes every call.
struct Module {
    name: &'static str,
    lib: &'static Lib,
    h: Mutex<Handle>,
}

// SAFETY: the handle is only used under the Mutex.
unsafe impl Send for Module {}
unsafe impl Sync for Module {}

impl Module {
    fn load(lib: &'static Lib, dirs: &[PathBuf], name: &'static str) -> Option<Module> {
        let blob = dirs.iter().find_map(|d| std::fs::read(d.join(format!("{name}.bin"))).ok())?;
        let mut h: Handle = std::ptr::null_mut();
        // SAFETY: calls match PawnIOLib.h; blob outlives pawnio_load.
        unsafe {
            if (lib.open)(&mut h) < 0 || h.is_null() {
                return None;
            }
            // Fails with NOT_SUPPORTED when the module's main() rejects this CPU/chipset.
            if (lib.load)(h, blob.as_ptr(), blob.len()) < 0 {
                (lib.close)(h);
                return None;
            }
        }
        Some(Module { name, lib, h: Mutex::new(h) })
    }

    /// Run `ioctl_*` with `input`; returns the cells the module wrote (≤ out_n).
    fn exec(&self, f: &str, input: &[u64], out_n: usize) -> Option<Vec<u64>> {
        let f = fn_name(f)?;
        let mut out = vec![0u64; out_n];
        let mut ret = 0usize;
        let h = self.h.lock().ok()?;
        // SAFETY: buffers sized as passed; handle valid while the guard lives.
        let hr = unsafe {
            (self.lib.exec)(*h, f.as_ptr().cast(), input.as_ptr(), input.len(), out.as_mut_ptr(), out.len(), &mut ret)
        };
        (hr >= 0).then(|| {
            out.truncate(ret);
            out
        })
    }

    fn exec1(&self, f: &str, input: &[u64]) -> Option<u64> {
        self.exec(f, input, 1)?.first().copied()
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        if let Ok(h) = self.h.get_mut() {
            // SAFETY: handle came from pawnio_open and is closed once.
            unsafe { (self.lib.close)(*h) };
        }
    }
}

/// Cross-process lock shared by hardware tools (HWiNFO, LHM, ...). The
/// modules document `\BaseNamedObjects\<name>`, i.e. `Global\<name>` here.
struct GlobalMutex(Handle);

unsafe impl Send for GlobalMutex {}
unsafe impl Sync for GlobalMutex {}

struct Held(Handle);

impl Drop for Held {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: we own the mutex on this thread.
            unsafe { ReleaseMutex(self.0) };
        }
    }
}

impl GlobalMutex {
    fn new(name: &str) -> GlobalMutex {
        let n = wide(format!(r"Global\{name}"));
        const SYNCHRONIZE_MODIFY: u32 = 0x0010_0001;
        // SAFETY: plain Win32 calls; a null result just means "no lock".
        unsafe {
            let mut h = CreateMutexW(std::ptr::null_mut(), 0, n.as_ptr());
            if h.is_null() {
                h = OpenMutexW(SYNCHRONIZE_MODIFY, 0, n.as_ptr());
            }
            GlobalMutex(h)
        }
    }

    /// None if another tool holds it too long. Missing mutex = best effort, proceed.
    fn lock(&self) -> Option<Held> {
        if self.0.is_null() {
            return Some(Held(std::ptr::null_mut()));
        }
        const WAIT_OBJECT_0: u32 = 0;
        const WAIT_ABANDONED: u32 = 0x80;
        // SAFETY: valid mutex handle owned by self.
        match unsafe { WaitForSingleObject(self.0, 500) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Some(Held(self.0)),
            _ => None,
        }
    }
}

/// MSR module candidates per CPUID vendor; each module's main() checks family.
fn msr_modules(vendor: &str) -> &'static [&'static str] {
    match vendor {
        "GenuineIntel" => &["IntelMSR"],
        "AuthenticAMD" => &["AMDFamily17", "AMDFamily10", "AMDFamily0F"],
        "CentaurHauls" | "  Shanghai  " => &["ZhaoxinMSR"],
        _ => &[],
    }
}

fn smbus_modules(vendor: &str) -> &'static [&'static str] {
    match vendor {
        "GenuineIntel" => &["SmbusI801"],
        "AuthenticAMD" | "HygonGenuine" => &["SmbusPIIX4"],
        _ => &[],
    }
}

/// LpcIO slot: 0 = 0x2E/0x2F, 1 = 0x4E/0x4F.
fn sio_slot(port: u16) -> Option<usize> {
    match port {
        0x2E | 0x2F => Some(0),
        0x4E | 0x4F => Some(1),
        _ => None,
    }
}

/// True if this write leaves Super I/O config mode: 0xAA to the index port
/// (Winbond/Nuvoton/Fintek), or bit 1 of reg 0x02 (ITE). 0x55 then 0xAA is
/// the tail of ITE's 0x4E entry key, not an exit.
fn exits_config(is_index: bool, last_index: u8, val: u8) -> bool {
    if is_index { val == 0xAA && last_index != 0x55 } else { last_index == 0x02 && val & 0x02 != 0 }
}

/// LpcIO only allows the Super I/O index/data ports plus the BARs it found
/// with `ioctl_find_bars`, which must run while the chip is in config mode.
/// So we harvest BARs right before the caller exits config mode, or on the
/// first denied access. One LpcIO handle per slot keeps each slot's BARs.
struct SioSlot {
    m: Module,
    st: Mutex<(bool, u8)>, // (bars found, last index register written)
}

impl SioSlot {
    fn find_bars(&self, st: &mut (bool, u8)) {
        if !st.0 {
            st.0 = self.m.exec("ioctl_find_bars", &[], 0).is_some();
        }
    }
}

struct PawnIo {
    version: String,
    msr: Option<Module>,
    smn: bool,
    sio: [Option<SioSlot>; 2],
    smbus: Vec<Arc<Module>>,
    smu: Option<Module>,
    mchbar: Option<Module>,
    pci_lock: GlobalMutex,
    isa_lock: GlobalMutex,
    smbus_lock: Arc<GlobalMutex>,
}

pub fn open() -> Box<dyn Hw> {
    open_pawnio().map_or_else(super::none, |p| Box::new(p) as Box<dyn Hw>)
}

#[cfg(not(target_arch = "x86_64"))]
fn open_pawnio() -> Option<PawnIo> {
    None // Every module we use is x64-only.
}

#[cfg(target_arch = "x86_64")]
fn open_pawnio() -> Option<PawnIo> {
    let dir = install_dir()?;
    let lib = Lib::load(&dir)?;
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let dirs: Vec<PathBuf> =
        exe_dir.map(|d| d.join("pawnio-modules")).into_iter().chain([dir.clone(), dir.join("modules")]).collect();

    let vendor = crate::cpu::x86::vendor();
    let msr = msr_modules(&vendor).iter().find_map(|n| Module::load(lib, &dirs, n));
    let smn = msr.as_ref().is_some_and(|m| m.name == "AMDFamily17");
    let sio = [0u64, 1].map(|slot| {
        let m = Module::load(lib, &dirs, "LpcIO")?;
        m.exec("ioctl_select_slot", &[slot], 0)?;
        Some(SioSlot { m, st: Mutex::new((false, 0)) })
    });
    let smbus: Vec<Arc<Module>> =
        smbus_modules(&vendor).iter().filter_map(|n| Module::load(lib, &dirs, n)).map(Arc::new).collect();
    let smu = (vendor == "AuthenticAMD").then(|| Module::load(lib, &dirs, "RyzenSMU")).flatten();
    let mchbar = (vendor == "GenuineIntel").then(|| Module::load(lib, &dirs, "IntelMCHBAR")).flatten();

    if msr.is_none() && sio.iter().all(Option::is_none) && smbus.is_empty() && smu.is_none() && mchbar.is_none() {
        return None;
    }
    let version = crate::os::win::reg_str(UNINSTALL_KEY, "DisplayVersion").unwrap_or_else(|| {
        let mut v = 0u32;
        // SAFETY: out-param per PawnIOLib.h.
        unsafe { (lib.version)(&mut v) };
        version_str(v)
    });
    Some(PawnIo {
        version,
        msr,
        smn,
        sio,
        smbus,
        smu,
        mchbar,
        pci_lock: GlobalMutex::new("Access_PCI"),
        isa_lock: GlobalMutex::new("Access_ISABUS.HTP.Method"),
        smbus_lock: Arc::new(GlobalMutex::new("Access_SMBUS.HTP.Method")),
    })
}

impl PawnIo {
    /// pio on the slot owning `port`, or on any slot whose BARs allow it.
    // ponytail: ISA lock is per byte, not per index/data sequence; widen it if
    // another tool's Super I/O traffic interleaves with ours in practice.
    fn pio(&self, port: u16, write: Option<u8>) -> Option<u64> {
        let _g = self.isa_lock.lock()?;
        let run = |s: &SioSlot| match write {
            Some(v) => s.m.exec("ioctl_pio_outb", &[port as u64, v as u64], 0).map(|_| 0),
            None => s.m.exec1("ioctl_pio_inb", &[port as u64]),
        };
        if let Some(i) = sio_slot(port) {
            let s = self.sio[i].as_ref()?;
            let mut st = s.st.lock().ok()?;
            if let Some(v) = write {
                let is_index = port & 1 == 0;
                if exits_config(is_index, st.1, v) {
                    s.find_bars(&mut st);
                }
                if is_index {
                    st.1 = v;
                }
            }
            return run(s);
        }
        self.sio.iter().flatten().find_map(|s| {
            let mut st = s.st.lock().ok()?;
            if let Some(v) = run(s) {
                return Some(v);
            }
            // Denied: maybe BARs weren't harvested yet (chip still in config mode).
            if st.0 {
                return None;
            }
            s.find_bars(&mut st);
            if st.0 { run(s) } else { None }
        })
    }
}

impl Hw for PawnIo {
    fn name(&self) -> String {
        let mut mods: Vec<&str> = self.msr.iter().map(|m| m.name).collect();
        if self.sio.iter().any(Option::is_some) {
            mods.push("LpcIO");
        }
        mods.extend(self.smbus.iter().map(|m| m.name));
        mods.extend(self.smu.iter().chain(&self.mchbar).map(|m| m.name));
        format!("PawnIO {} (modules: {})", self.version, mods.join(", "))
    }

    fn rdmsr(&self, cpu: usize, msr: u32) -> Option<u64> {
        let m = self.msr.as_ref()?;
        // Modules read the MSR on whatever CPU runs the ioctl, so pin first.
        // Unlisted MSRs are refused by the module's allow-list (ACCESS_DENIED).
        crate::os::on_cpu(cpu, || m.exec1("ioctl_read_msr", &[msr as u64]))?
    }

    fn io_in8(&self, port: u16) -> Option<u8> {
        self.pio(port, None).map(|v| v as u8)
    }

    fn io_out8(&self, port: u16, val: u8) -> bool {
        self.pio(port, Some(val)).is_some()
    }

    fn amd_smn_read(&self, addr: u32) -> Option<u32> {
        let m = self.msr.as_ref().filter(|_| self.smn)?;
        let _g = self.pci_lock.lock()?;
        m.exec1("ioctl_read_smn", &[addr as u64]).map(|v| v as u32)
    }

    fn smbus(&self) -> Vec<Box<dyn Smbus>> {
        self.smbus
            .iter()
            .map(|m| {
                // identity: [0] type, [1] I/O base, [2] vid | did << 16 | subsys << 32
                let id = m.exec("ioctl_identity", &[], 3).unwrap_or_default();
                let (base, ids) = (id.get(1).copied().unwrap_or(0), id.get(2).copied().unwrap_or(0));
                let name = format!("PawnIO {} {:04x}:{:04x} @ 0x{base:X}", m.name, ids & 0xFFFF, (ids >> 16) & 0xFFFF);
                Box::new(PawnSmbus { m: m.clone(), lock: self.smbus_lock.clone(), name }) as Box<dyn Smbus>
            })
            .collect()
    }

    fn amd_smu_pm_table(&self) -> Option<(u32, Vec<f32>)> {
        let m = self.smu.as_ref()?;
        let _g = self.pci_lock.lock()?;
        // resolve → [version, table base]; update asks the SMU to refresh DRAM;
        // read returns up to one 4 KiB page as u64 cells.
        let ver = *m.exec("ioctl_resolve_pm_table", &[], 2)?.first()? as u32;
        m.exec("ioctl_update_pm_table", &[], 0)?;
        let cells = m.exec("ioctl_read_pm_table", &[], 512)?;
        Some((ver, unpack_f32(&cells)))
    }

    fn amd_smu_version(&self) -> Option<u32> {
        let m = self.smu.as_ref()?;
        let _g = self.pci_lock.lock()?;
        m.exec1("ioctl_get_smu_version", &[]).map(|v| v as u32)
    }

    // The module bounds-checks against the generation's MCHBAR size and alignment.
    fn mchbar_read32(&self, off: u32) -> Option<u32> {
        self.mchbar.as_ref()?.exec1("ioctl_read_dword", &[off as u64]).map(|v| v as u32)
    }

    fn mchbar_read64(&self, off: u32) -> Option<u64> {
        self.mchbar.as_ref()?.exec1("ioctl_read_qword", &[off as u64])
    }
}

/// PM table cells are u64s each carrying two little-endian f32s (low first).
fn unpack_f32(cells: &[u64]) -> Vec<f32> {
    cells.iter().flat_map(|c| [f32::from_bits(*c as u32), f32::from_bits((c >> 32) as u32)]).collect()
}

/// `ioctl_smbus_xfer` input: [addr, read(1)/write(0), command, protocol, data...].
/// Protocol numbers follow Linux i2c: 1 = BYTE, 2 = BYTE_DATA.
enum Xfer {
    ReadByte(u8),
    WriteByte(u8, u8),
    SendByte(u8),
    ReadWord(u8),
}

fn xfer_args(addr: u8, x: Xfer) -> Vec<u64> {
    let a = addr as u64;
    match x {
        Xfer::ReadByte(cmd) => vec![a, 1, cmd as u64, 2],
        Xfer::WriteByte(cmd, v) => vec![a, 0, cmd as u64, 2, v as u64],
        // BYTE write sends `command`; the module still wants a data cell.
        Xfer::SendByte(v) => vec![a, 0, v as u64, 1, 0],
        // 3 = WORD_DATA; the word comes back in the low 16 bits of the first cell.
        Xfer::ReadWord(cmd) => vec![a, 1, cmd as u64, 3],
    }
}

struct PawnSmbus {
    m: Arc<Module>,
    lock: Arc<GlobalMutex>,
    name: String,
}

impl PawnSmbus {
    fn xfer(&self, addr: u8, x: Xfer, out_n: usize) -> Option<Vec<u64>> {
        let _g = self.lock.lock()?;
        self.m.exec("ioctl_smbus_xfer", &xfer_args(addr, x), out_n)
    }
}

impl Smbus for PawnSmbus {
    fn name(&self) -> String {
        self.name.clone()
    }
    fn read_byte(&self, addr: u8, cmd: u8) -> Option<u8> {
        self.xfer(addr, Xfer::ReadByte(cmd), 1)?.first().map(|&v| v as u8)
    }
    fn write_byte(&self, addr: u8, cmd: u8, val: u8) -> bool {
        self.xfer(addr, Xfer::WriteByte(cmd, val), 0).is_some()
    }
    fn send_byte(&self, addr: u8, val: u8) -> bool {
        self.xfer(addr, Xfer::SendByte(val), 0).is_some()
    }
    fn read_word(&self, addr: u8, cmd: u8) -> Option<u16> {
        self.xfer(addr, Xfer::ReadWord(cmd), 1)?.first().map(|&v| v as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing() {
        assert_eq!(xfer_args(0x50, Xfer::ReadByte(0x12)), [0x50, 1, 0x12, 2]);
        assert_eq!(xfer_args(0x37, Xfer::WriteByte(0x0B, 3)), [0x37, 0, 0x0B, 2, 3]);
        assert_eq!(xfer_args(0x36, Xfer::SendByte(0)), [0x36, 0, 0, 1, 0]);
        assert!(fn_name("ioctl_read_msr").is_some());
        assert!(fn_name(&"x".repeat(32)).is_none());
        assert!(fn_name("a\0b").is_none());
        assert_eq!(version_str(0x0002_0100), "2.1.0");
        assert_eq!(sio_slot(0x2F), Some(0));
        assert_eq!(sio_slot(0x4E), Some(1));
        assert_eq!(sio_slot(0x290), None);
        assert!(exits_config(true, 0, 0xAA));
        assert!(!exits_config(true, 0x55, 0xAA));
        assert!(exits_config(false, 0x02, 0x02));
        assert!(!exits_config(false, 0x07, 0x02));
        assert_eq!(msr_modules("AuthenticAMD")[0], "AMDFamily17");
        assert!(msr_modules("Apple").is_empty());
    }

    #[test]
    fn open_never_panics() {
        let _ = open().name();
    }

    #[test]
    fn pm_cells() {
        let c = (2.5f32.to_bits() as u64) << 32 | 1.0f32.to_bits() as u64;
        assert_eq!(unpack_f32(&[c]), [1.0, 2.5]);
    }

    /// Manual, needs PawnIO + admin: `cpuzeez-*.exe --ignored live_dump`.
    /// Writes %TEMP%\cpuzeez-pawnio-live.txt.
    #[test]
    #[ignore]
    fn live_dump() {
        use std::fmt::Write;
        let hw = open();
        let mut s = format!("{}\n", hw.name());
        for msr in [0xE7u32, 0xE8, 0xC000_00E7, 0xC000_00E8, 0xC001_0064, 0xCE, 0x198] {
            let _ = writeln!(s, "msr {msr:#x}: {:x?}", hw.rdmsr(0, msr));
        }
        let _ = writeln!(s, "smn 0x59800 (Tctl): {:x?}", hw.amd_smn_read(0x59800));
        let _ = writeln!(s, "smu version: {:x?}", hw.amd_smu_version());
        let t = hw.amd_smu_pm_table();
        let _ = writeln!(
            s,
            "pm table: {:x?} {:?}",
            t.as_ref().map(|t| (t.0, t.1.len())),
            t.as_ref().map(|t| &t.1[..16.min(t.1.len())])
        );
        let _ = writeln!(s, "mchbar 0x5000: {:x?}", hw.mchbar_read32(0x5000));
        let _ = writeln!(s, "superio 0x2E id: {:x?}", {
            hw.io_out8(0x2E, 0x87);
            hw.io_out8(0x2E, 0x87);
            hw.io_out8(0x2E, 0x20);
            let id = hw.io_in8(0x2F);
            hw.io_out8(0x2E, 0xAA);
            id
        });
        for b in hw.smbus() {
            let _ = writeln!(s, "smbus {}: spd 0x50[0]={:x?}", b.name(), b.read_byte(0x50, 0));
        }
        std::fs::write(std::env::temp_dir().join("cpuzeez-pawnio-live.txt"), s).unwrap();
    }
}

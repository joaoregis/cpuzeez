//! Slow probes kept off the UI thread. A page that drew `storage::drives()`
//! every frame spent ~150 ms per frame on SMART IOCTLs (the Overview, measured
//! with `CZ_FRAME_LOG=1`). Each value here is read on a background thread,
//! kept, and refreshed in the background once it's older than its max age;
//! the UI always gets the last value at once (None only before the first read).

use crate::Ctx;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Slot<T> {
    value: Option<Arc<T>>,
    at: Option<Instant>,
    busy: bool,
}

/// One cached probe.
pub struct Cached<T> {
    slot: Mutex<Slot<T>>,
    max_age: Duration,
}

impl<T: Send + Sync + 'static> Cached<T> {
    pub const fn new(max_age: Duration) -> Cached<T> {
        Cached { slot: Mutex::new(Slot { value: None, at: None, busy: false }), max_age }
    }

    /// The last value; starts a background refresh when there is none or it's stale.
    pub fn get(&'static self, ctx: &Arc<Ctx>, read: fn(&Ctx) -> T) -> Option<Arc<T>> {
        let mut s = self.slot.lock().unwrap_or_else(|e| e.into_inner());
        let stale = s.at.is_none_or(|t| t.elapsed() > self.max_age);
        if stale && !s.busy {
            s.busy = true;
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let v = Arc::new(read(&ctx));
                let mut s = self.slot.lock().unwrap_or_else(|e| e.into_inner());
                (s.value, s.at, s.busy) = (Some(v), Some(Instant::now()), false);
            });
        }
        s.value.clone()
    }
}

static DRIVES: Cached<Vec<crate::storage::Drive>> = Cached::new(Duration::from_secs(60));
static BATTERIES: Cached<Vec<crate::battery::Battery>> = Cached::new(Duration::from_secs(30));
static FIRMWARE: Cached<cpuzeez_platform::firmware::Firmware> = Cached::new(Duration::from_secs(3600));
static CPU_OS: Cached<CpuOs> = Cached::new(Duration::from_secs(10));
static GPUS: Cached<Vec<String>> = Cached::new(Duration::from_secs(3600));

/// What the OS says about the CPU (vulnerabilities, virtualization, power policy).
pub struct CpuOs {
    pub vulnerabilities: Vec<(String, String)>,
    pub virtualization: cpuzeez_platform::cpu_os::Virtualization,
    pub power: cpuzeez_platform::cpu_os::Power,
}

pub fn drives(ctx: &Arc<Ctx>) -> Arc<Vec<crate::storage::Drive>> {
    DRIVES.get(ctx, crate::storage::drives).unwrap_or_default()
}

pub fn batteries(ctx: &Arc<Ctx>) -> Arc<Vec<crate::battery::Battery>> {
    BATTERIES.get(ctx, crate::battery::batteries).unwrap_or_default()
}

pub fn firmware(ctx: &Arc<Ctx>) -> Option<Arc<cpuzeez_platform::firmware::Firmware>> {
    FIRMWARE.get(ctx, |_| cpuzeez_platform::firmware::read())
}

pub fn cpu_os(ctx: &Arc<Ctx>) -> Option<Arc<CpuOs>> {
    CPU_OS.get(ctx, |_| CpuOs {
        vulnerabilities: cpuzeez_platform::cpu_os::vulnerabilities(),
        virtualization: cpuzeez_platform::cpu_os::virtualization(),
        power: cpuzeez_platform::cpu_os::power(),
    })
}

/// GPU adapters for the benchmark (creating a wgpu instance takes ~0.9 s).
pub fn gpu_adapters(ctx: &Arc<Ctx>) -> Option<Arc<Vec<String>>> {
    GPUS.get(ctx, |_| crate::gpubench::adapters())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refreshes_in_the_background() {
        static C: Cached<u32> = Cached::new(Duration::from_millis(50));
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let ctx = Arc::new(Ctx::new(Default::default()));
        let read = |_: &Ctx| N.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let first = C.get(&ctx, read);
        let t0 = Instant::now();
        let mut v = first;
        while v.is_none() && t0.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(5));
            v = C.get(&ctx, read);
        }
        assert_eq!(v.as_deref(), Some(&1), "the first read lands");
        std::thread::sleep(Duration::from_millis(80));
        assert_eq!(C.get(&ctx, read).as_deref(), Some(&1), "a stale value is still served at once");
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(C.get(&ctx, read).as_deref(), Some(&2), "…while the refresh replaces it");
    }

    /// Slow probes never run on the UI thread: pages go through this cache
    /// (the Devices page reads them on its own background thread).
    #[test]
    fn pages_never_probe_in_a_frame() {
        let slow = [
            "crate::storage::drives(",
            "crate::battery::batteries(",
            "firmware::read(",
            "cpu_os::vulnerabilities(",
            "cpu_os::virtualization(",
            "cpu_os::power(",
            "gpubench::adapters(",
        ];
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gui");
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).collect();
        files.extend(std::fs::read_dir(dir.join("pages")).unwrap().flatten().map(|e| e.path()));
        for f in files.iter().filter(|f| f.extension().is_some_and(|e| e == "rs")) {
            let name = f.file_name().unwrap().to_string_lossy();
            if name == "cache.rs" || name == "devices.rs" {
                continue;
            }
            let text = std::fs::read_to_string(f).unwrap();
            for s in slow {
                assert!(!text.contains(s), "{name} calls {s}…) in a frame: use gui::cache");
            }
        }
    }
}

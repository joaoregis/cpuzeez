//! Facts about this machine for the capability banners; the decision itself is
//! `cpuzeez_core::availability`.

use crate::Ctx;
use cpuzeez_core::availability::{self, Facts, Gap, Os};

/// What can't be shown here right now, and what it would take.
pub fn gaps(ctx: &Ctx) -> Vec<Gap> {
    availability::gaps(Os::current(), &facts(ctx))
}

fn facts(ctx: &Ctx) -> Facts {
    let c = ctx.hw.caps();
    Facts {
        elevated: cpuzeez_platform::elevation::is_elevated(),
        driver_installed: crate::hw::driver_installed(),
        driver_modules: crate::hw::driver_modules_found(),
        intel: crate::cpu::get(ctx).vendor == "GenuineIntel",
        msr: c.msr,
        superio: c.superio,
        smbus: c.smbus,
        smu: c.smu,
        imc: c.imc,
        pch: c.pch,
        kernel_board_sensors: kernel_board_sensors(),
        msr_device: std::path::Path::new("/dev/cpu/0/msr").exists(),
    }
}

/// Linux: a kernel hwmon driver for a mainboard Super I/O / EC is loaded.
fn kernel_board_sensors() -> bool {
    #[cfg(target_os = "linux")]
    {
        const BOARD: [&str; 9] = [
            "nct6775",
            "nct6683",
            "nct6687",
            "it87",
            "f71882fg",
            "w83627ehf",
            "asus_ec_sensors",
            "dell_smm",
            "thinkpad",
        ];
        if let Ok(dir) = std::fs::read_dir("/sys/class/hwmon") {
            return dir.flatten().any(|e| {
                crate::os::read_trim(e.path().join("name")).is_some_and(|n| BOARD.iter().any(|b| n.starts_with(b)))
            });
        }
    }
    false
}

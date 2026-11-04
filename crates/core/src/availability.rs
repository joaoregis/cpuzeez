//! What can't be shown on this machine right now, and what it would take
//! (docs/v1/README.md P3). Pure: the binary gathers the facts, this decides the
//! gaps, and the GUI's single capability banner and the README's permissions
//! map both come from here, so they can't drift apart.

/// Data that needs more than user-mode access on some OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// Per-core temperatures, clocks and power, P-states, power limits.
    CpuSensors,
    /// Mainboard voltages, fans and temperatures (Super I/O).
    BoardSensors,
    /// Memory module details (SPD) and DIMM temperatures.
    Spd,
    /// Live memory timings from the memory controller.
    MemoryTimings,
    /// Chipset (PCH) temperature.
    ChipsetTemp,
}

impl Capability {
    pub const ALL: [Capability; 5] = [
        Capability::CpuSensors,
        Capability::BoardSensors,
        Capability::Spd,
        Capability::MemoryTimings,
        Capability::ChipsetTemp,
    ];

    /// What the user doesn't see, in plain words (English key, translated by the GUI).
    pub fn label(self) -> &'static str {
        match self {
            Capability::CpuSensors => "per-core temperatures, clocks and power",
            Capability::BoardSensors => "mainboard voltages, fans and temperatures",
            Capability::Spd => "memory module details (SPD) and DIMM temperatures",
            Capability::MemoryTimings => "live memory timings",
            Capability::ChipsetTemp => "chipset temperature",
        }
    }
}

/// What it would take to get a capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Need {
    /// Run as administrator (Windows) or root (Linux).
    Elevation,
    /// Install the PawnIO driver (Windows).
    Driver,
    /// Load a Linux kernel module (e.g. `msr`), as root.
    KernelModule(&'static str),
    /// Not possible on this machine, with the reason (English key).
    Unsupported(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gap {
    pub cap: Capability,
    pub need: Need,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
    MacOs,
    Other,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else {
            Os::Other
        }
    }
}

/// What the binary observed about this machine.
#[derive(Debug, Clone, Copy, Default)]
pub struct Facts {
    pub elevated: bool,
    /// Windows: PawnIO is installed (whether or not it could be opened).
    pub driver_installed: bool,
    /// Windows: PawnIO's modules were found (they ship in `pawnio-modules`).
    pub driver_modules: bool,
    pub intel: bool,
    /// The privileged backend's abilities (`hw::Caps`).
    pub msr: bool,
    pub superio: bool,
    pub smbus: bool,
    pub smu: bool,
    pub imc: bool,
    pub pch: bool,
    /// Linux: the kernel's own hwmon drivers already report mainboard sensors.
    pub kernel_board_sensors: bool,
    /// Linux: the `msr` device exists (`modprobe msr`).
    pub msr_device: bool,
}

/// Everything that's missing, in [`Capability::ALL`] order.
pub fn gaps(os: Os, f: &Facts) -> Vec<Gap> {
    Capability::ALL.iter().filter_map(|&cap| need(os, f, cap).map(|need| Gap { cap, need })).collect()
}

fn have(f: &Facts, cap: Capability) -> bool {
    match cap {
        Capability::CpuSensors => f.msr || f.smu,
        Capability::BoardSensors => f.superio,
        Capability::Spd => f.smbus,
        Capability::MemoryTimings => f.imc,
        Capability::ChipsetTemp => f.pch,
    }
}

fn need(os: Os, f: &Facts, cap: Capability) -> Option<Need> {
    if have(f, cap) {
        return None;
    }
    match os {
        Os::Windows => {
            if cap == Capability::ChipsetTemp && !f.intel {
                return None; // AMD boards route it to the Super I/O, if anywhere.
            }
            Some(if !f.driver_installed {
                Need::Driver
            } else if !f.driver_modules {
                // Restarting as admin wouldn't help.
                Need::Unsupported("PawnIO's modules are missing: put the pawnio-modules folder next to cpuzeez.exe")
            } else if !f.elevated {
                Need::Elevation
            } else {
                Need::Unsupported("PawnIO has no module for this CPU or chipset")
            })
        }
        Os::Linux => match cap {
            // The kernel's intel_pch_thermal / nct6775 / it87 drivers work without root.
            Capability::ChipsetTemp => None,
            Capability::BoardSensors if f.kernel_board_sensors => None,
            Capability::CpuSensors if f.elevated && !f.msr_device => Some(Need::KernelModule("msr")),
            _ if !f.elevated => Some(Need::Elevation),
            Capability::BoardSensors => Some(Need::Unsupported("no supported Super I/O chip or kernel driver")),
            _ => Some(Need::Unsupported("the kernel gives no access to it on this machine")),
        },
        Os::MacOs => match cap {
            // SMC already covers what macOS exposes.
            Capability::CpuSensors | Capability::ChipsetTemp => None,
            _ => Some(Need::Unsupported("macOS doesn't allow third-party register access")),
        },
        Os::Other => Some(Need::Unsupported("not supported on this OS")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(f: &Facts, os: Os) -> Vec<(Capability, Need)> {
        gaps(os, f).into_iter().map(|g| (g.cap, g.need)).collect()
    }

    #[test]
    fn windows_driver_then_elevation_then_modules() {
        let none = Facts { intel: true, ..Default::default() };
        assert!(caps(&none, Os::Windows).iter().all(|(_, n)| *n == Need::Driver), "no PawnIO: install it");
        let no_modules = Facts { driver_installed: true, ..none };
        assert!(
            caps(&no_modules, Os::Windows)
                .iter()
                .all(|(_, n)| matches!(n, Need::Unsupported(r) if r.contains("pawnio-modules"))),
            "installed without modules: say so, don't ask for admin"
        );
        let installed = Facts { driver_installed: true, driver_modules: true, ..none };
        assert!(caps(&installed, Os::Windows).iter().all(|(_, n)| *n == Need::Elevation), "installed: run as admin");
        let full = Facts { elevated: true, msr: true, superio: true, smbus: true, imc: true, pch: true, ..installed };
        assert!(gaps(Os::Windows, &full).is_empty());
        let no_spd = Facts { smbus: false, ..full };
        assert!(matches!(caps(&no_spd, Os::Windows)[..], [(Capability::Spd, Need::Unsupported(_))]));
        let amd = Facts { intel: false, pch: false, ..full };
        assert!(gaps(Os::Windows, &amd).is_empty(), "no PCH sensor to miss on AMD");
    }

    #[test]
    fn linux_root_kernel_module_and_hwmon() {
        let user = Facts { kernel_board_sensors: true, ..Default::default() };
        let v = caps(&user, Os::Linux);
        assert!(v.iter().all(|(_, n)| *n == Need::Elevation));
        assert!(!v.iter().any(|(c, _)| matches!(c, Capability::BoardSensors | Capability::ChipsetTemp)), "{v:?}");
        let root = Facts { elevated: true, smbus: true, imc: true, ..user };
        assert_eq!(caps(&root, Os::Linux), [(Capability::CpuSensors, Need::KernelModule("msr"))]);
        let all = Facts { msr: true, msr_device: true, ..root };
        assert!(gaps(Os::Linux, &all).is_empty());
    }

    #[test]
    fn macos_explains_what_is_impossible() {
        let v = caps(&Facts::default(), Os::MacOs);
        let lost: Vec<Capability> = v.iter().map(|g| g.0).collect();
        assert_eq!(lost, [Capability::BoardSensors, Capability::Spd, Capability::MemoryTimings]);
        assert!(v.iter().all(|(_, n)| matches!(n, Need::Unsupported(_))));
    }

    #[test]
    fn every_capability_is_described() {
        for c in Capability::ALL {
            assert!(!c.label().is_empty());
        }
    }
}

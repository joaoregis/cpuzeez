//! cpuzeez-platform: the per-OS layer (docs/adr/0011).
//!
//! OS helpers (`os`), CPUID (`cpu::x86`) and privileged hardware access
//! (`hw`: PawnIO on Windows, kernel interfaces on Linux). New per-OS sources
//! are written here; existing collection code in the binary moves here as
//! each page is rebuilt (docs/v1/engineering.md §10).

pub mod clock;
pub mod cpu_os;
pub mod cstates;
pub mod desktop;
pub mod direct_io;
pub mod disk_io;
pub mod display;
mod display_drm;
#[cfg(windows)]
pub mod dxcore;
pub mod elevation;
pub mod encryption;
pub mod firmware;
pub mod gpu_engines;
pub mod hw;
pub mod instance;
pub mod large_pages;
pub mod memory;
pub mod net_io;
pub mod os;
pub mod overlay;
pub mod partitions;
pub mod pdh;
pub mod proc_net;
pub mod proc_ops;
pub mod procs;
#[cfg(test)]
mod robustness;
pub mod services;
#[cfg(windows)]
pub mod setupapi;
#[cfg(target_os = "macos")]
pub mod smc;
pub mod startup;
pub mod tempdir;
pub mod users;

pub mod cpu {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub mod x86;
}

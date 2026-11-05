//! cpuzeez-platform: the per-OS layer (docs/adr/0011).
//!
//! OS helpers (`os`), CPUID (`cpu::x86`) and privileged hardware access
//! (`hw`: PawnIO on Windows, kernel interfaces on Linux). New per-OS sources
//! are written here; existing collection code in the binary moves here as
//! each page is rebuilt (docs/v1/engineering.md §10).

pub mod disk_io;
pub mod elevation;
pub mod hw;
pub mod net_io;
pub mod os;

pub mod cpu {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub mod x86;
}

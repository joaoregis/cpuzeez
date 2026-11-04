//! cpuzeez-core: domain types and pure logic (decoders, the report model).
//!
//! Nothing here calls the OS or the GUI, so every module is testable on any
//! platform with recorded data. Per-OS collection lives in the `cpuzeez`
//! binary crate (docs/adr/0011).

#![forbid(unsafe_code)]

/// Binary, package and file-name stem.
pub const NAME: &str = "cpuzeez";
/// Name shown to people (window title, About, report headers).
pub const DISPLAY_NAME: &str = "CpuZeeZ";

pub mod config;
pub mod pci;
pub mod report;
pub mod smbios;

pub mod cpu {
    pub mod db;
}
pub mod gpu {
    pub mod db;
    pub mod edid;
}
pub mod sensors {
    pub mod amd_pm;
}
pub mod spd {
    pub mod decode;
    pub mod jep106;
}
pub mod storage {
    pub mod smart;
}

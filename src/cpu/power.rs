//! Preferred cores (CPPC) and Intel package power limits (S1-G03), read once:
//! both are set by the firmware and don't change while the app runs.

use crate::Ctx;
use cpuzeez_core::cpu::limits::{self, Limit};
use std::sync::OnceLock;

/// CPPC highest performance of each physical core (the cores table's order):
/// Linux sysfs when present (no root), else the MSR (PawnIO / msr module).
pub fn cppc(ctx: &Ctx) -> &'static [Option<u32>] {
    static V: OnceLock<Vec<Option<u32>>> = OnceLock::new();
    V.get_or_init(|| {
        let c = super::get(ctx);
        let amd = c.vendor == "AuthenticAMD" || c.vendor == "HygonGenuine";
        c.logical
            .iter()
            .filter(|l| l.smt == 0)
            .map(|l| {
                let sysfs = format!("/sys/devices/system/cpu/cpu{}/acpi_cppc/highest_perf", l.cpu);
                crate::os::read_u64(&sysfs).map(|v| v as u32).or_else(|| {
                    if amd {
                        ctx.hw.rdmsr(l.cpu, 0xC001_02B0).map(limits::amd_cppc_highest)
                    } else {
                        ctx.hw.rdmsr(l.cpu, 0x771).map(limits::intel_hwp_highest)
                    }
                })
            })
            .collect()
    })
}

/// Intel PL1 / PL2 (None on AMD, or without MSR access).
pub fn intel_limits(ctx: &Ctx) -> Option<(Limit, Limit)> {
    static V: OnceLock<Option<(Limit, Limit)>> = OnceLock::new();
    *V.get_or_init(|| {
        let c = super::get(ctx);
        if c.vendor != "GenuineIntel" {
            return None;
        }
        let cpu = c.logical.first().map_or(0, |l| l.cpu);
        let [units, limit] = ctx.hw.rdmsr_many(cpu, &[0x606, 0x610])[..] else { return None };
        Some(limits::intel_pkg_limits(units?, limit?))
    })
}

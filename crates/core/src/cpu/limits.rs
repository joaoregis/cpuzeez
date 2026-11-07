//! CPU power limits and preferred cores from their registers (S1-G03).
//!
//! - Intel package power limits: `MSR_PKG_POWER_LIMIT` (610h) in the units of
//!   `MSR_RAPL_POWER_UNIT` (606h): PL1 (long-term, with its time window τ) and
//!   PL2 (short-term).
//! - CPPC "highest performance" per core (AMD `MSR_AMD_CPPC_CAP1`
//!   C00102B0h bits 31:24, Intel `IA32_HWP_CAPABILITIES` 771h bits 7:0, or
//!   Linux `acpi_cppc/highest_perf`): the OS prefers the cores with the
//!   highest value, so ranking them shows the "best" cores.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limit {
    pub watts: f64,
    pub enabled: bool,
    /// Time window, seconds (PL1 only).
    pub seconds: Option<f64>,
}

/// (PL1, PL2) from MSRs 606h (`units`) and 610h (`limit`).
pub fn intel_pkg_limits(units: u64, limit: u64) -> (Limit, Limit) {
    let power_unit = 1.0 / (1u64 << (units & 0xF)) as f64;
    let time_unit = 1.0 / (1u64 << ((units >> 16) & 0xF)) as f64;
    let pl = |bits: u64| (bits & 0x7FFF) as f64 * power_unit;
    let window = |bits: u64| {
        let (y, z) = ((bits >> 17) & 0x1F, (bits >> 22) & 0x3);
        (1u64 << y) as f64 * (1.0 + z as f64 / 4.0) * time_unit
    };
    let lo = limit & 0xFFFF_FFFF;
    let hi = limit >> 32;
    (
        Limit { watts: pl(lo), enabled: lo & (1 << 15) != 0, seconds: Some(window(lo)) },
        Limit { watts: pl(hi), enabled: hi & (1 << 15) != 0, seconds: None },
    )
}

/// AMD `MSR_AMD_CPPC_CAP1`: the highest performance, bits 31:24.
pub fn amd_cppc_highest(cap1: u64) -> u32 {
    ((cap1 >> 24) & 0xFF) as u32
}

/// Intel `IA32_HWP_CAPABILITIES`: the highest performance, bits 7:0.
pub fn intel_hwp_highest(cap: u64) -> u32 {
    (cap & 0xFF) as u32
}

/// Rank per core (1 = the most preferred), from each core's highest
/// performance; equal values share a rank. None where the value is unknown,
/// and all None when every core reports the same (nothing to prefer).
pub fn ranks(highest: &[Option<u32>]) -> Vec<Option<usize>> {
    let mut distinct: Vec<u32> = highest.iter().flatten().copied().collect();
    distinct.sort_unstable_by(|a, b| b.cmp(a));
    distinct.dedup();
    if distinct.len() < 2 {
        return vec![None; highest.len()];
    }
    highest.iter().map(|h| h.and_then(|v| distinct.iter().position(|&d| d == v)).map(|p| p + 1)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intel_limits_decode() {
        // Units 0x000A0E03: power 1/8 W, time 1/1024 s. PL1 = 125 W enabled
        // with Y = 13, Z = 2: τ = 2^13 × 1.5 / 1024 = 12 s. PL2 = 253 W.
        let units = 0x000A_0E03;
        let pl1 = (125 * 8) | (1 << 15) | (13 << 17) | (2 << 22);
        let pl2 = (253 * 8) | (1 << 15);
        let (a, b) = intel_pkg_limits(units, (pl2 << 32) | pl1);
        assert_eq!((a.watts, a.enabled, a.seconds), (125.0, true, Some(12.0)));
        assert_eq!((b.watts, b.enabled), (253.0, true));
    }

    #[test]
    fn cppc_ranks() {
        assert_eq!(amd_cppc_highest(0xA6_7A_4E_1E), 0xA6);
        assert_eq!(intel_hwp_highest(0x0001_2B46), 0x46);
        let r = ranks(&[Some(166), Some(171), Some(166), None, Some(161)]);
        assert_eq!(r, vec![Some(2), Some(1), Some(2), None, Some(3)]);
        assert_eq!(ranks(&[Some(5), Some(5)]), vec![None, None], "all equal: no preferred core");
    }
}

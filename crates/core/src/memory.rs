//! What RAM is used for (S1-D03), in MB. Every field is optional: each OS
//! tracks a different subset, and a missing value is shown as missing, never
//! as 0. The per-OS readers live in `cpuzeez_platform::memory`.

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Composition {
    pub total: Option<f64>,
    /// In use: what can't be handed out without paging something out.
    pub used: Option<f64>,
    pub available: Option<f64>,
    /// Not used for anything.
    pub free: Option<f64>,
    /// File cache that can be dropped at once (Windows "standby", Linux
    /// "cached", macOS file-backed pages).
    pub cached: Option<f64>,
    /// Changed data waiting to be written out (Windows "modified", Linux "dirty").
    pub modified: Option<f64>,
    /// Held compressed in RAM (Windows memory compression, macOS compressor, Linux zswap).
    pub compressed: Option<f64>,
    /// Kernel memory that can never be paged (Windows non-paged pool, macOS wired, Linux unreclaimable slab).
    pub wired: Option<f64>,
    /// Kernel memory that can be paged (Windows paged pool, Linux reclaimable slab).
    pub paged_pool: Option<f64>,
    /// Commit charge and limit (Windows; Linux `Committed_AS` / `CommitLimit`).
    pub commit: Option<f64>,
    pub commit_limit: Option<f64>,
    pub swap_used: Option<f64>,
    pub swap_total: Option<f64>,
}

/// `/proc/meminfo` (kB) → composition.
pub fn parse_meminfo(text: &str) -> Composition {
    let kb = |k: &str| {
        text.lines().find_map(|l| {
            let (key, v) = l.split_once(':')?;
            (key == k).then(|| v.split_whitespace().next()?.parse::<f64>().ok()).flatten().map(|kb| kb / 1024.0)
        })
    };
    let (total, available) = (kb("MemTotal"), kb("MemAvailable"));
    let swap_total = kb("SwapTotal");
    Composition {
        total,
        available,
        used: total.zip(available).map(|(t, a)| t - a),
        free: kb("MemFree"),
        // Page cache + reclaimable slab is what the kernel drops first.
        cached: kb("Cached").map(|c| c + kb("SReclaimable").unwrap_or(0.0) + kb("Buffers").unwrap_or(0.0)),
        modified: kb("Dirty").map(|d| d + kb("Writeback").unwrap_or(0.0)),
        compressed: kb("Zswap"),
        wired: kb("SUnreclaim"),
        paged_pool: kb("SReclaimable"),
        commit: kb("Committed_AS"),
        commit_limit: kb("CommitLimit"),
        swap_used: swap_total.zip(kb("SwapFree")).map(|(t, f)| t - f),
        swap_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMINFO: &str = include_str!("../fixtures/linux/meminfo");

    #[test]
    fn meminfo() {
        let c = parse_meminfo(MEMINFO);
        let mb = |kb: f64| Some(kb / 1024.0);
        assert_eq!(c.total, mb(32_768_000.0));
        assert_eq!(c.available, mb(20_480_000.0));
        assert_eq!(c.used, mb(32_768_000.0 - 20_480_000.0));
        assert_eq!(c.cached, mb(12_000_000.0 + 500_000.0 + 300_000.0));
        assert_eq!(c.modified, mb(2_048.0 + 0.0));
        assert_eq!((c.wired, c.paged_pool), (mb(200_000.0), mb(500_000.0)));
        assert_eq!((c.swap_total, c.swap_used), (mb(8_388_604.0), mb(8_388_604.0 - 8_000_000.0)));
        assert_eq!(c.compressed, None, "no zswap line: missing, not 0");
    }
}

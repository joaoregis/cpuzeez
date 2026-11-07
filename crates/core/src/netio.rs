//! Per-adapter network traffic (S1-D02): cumulative OS counters, rates from
//! two snapshots. Pure logic; the per-OS readers live in
//! `cpuzeez_platform::net_io`.

/// Cumulative counters of one network adapter.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Counters {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_drops: u64,
    pub tx_drops: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rates {
    /// Megabits per second, as adapters and ISPs are rated.
    pub rx_mbps: f64,
    pub tx_mbps: f64,
    pub rx_pps: f64,
    pub tx_pps: f64,
    /// Errors and drops per second, both directions.
    pub errors_ps: f64,
    pub drops_ps: f64,
}

/// Rates between `a` and `b`, `secs` apart. A counter that went backwards
/// (adapter reset, 32-bit wrap) counts as no activity.
pub fn rates(a: &Counters, b: &Counters, secs: f64) -> Option<Rates> {
    if secs <= 0.0 {
        return None;
    }
    let d = |x: u64, y: u64| y.saturating_sub(x) as f64 / secs;
    Some(Rates {
        rx_mbps: d(a.rx_bytes, b.rx_bytes) * 8.0 / 1e6,
        tx_mbps: d(a.tx_bytes, b.tx_bytes) * 8.0 / 1e6,
        rx_pps: d(a.rx_packets, b.rx_packets),
        tx_pps: d(a.tx_packets, b.tx_packets),
        errors_ps: d(a.rx_errors, b.rx_errors) + d(a.tx_errors, b.tx_errors),
        drops_ps: d(a.rx_drops, b.rx_drops) + d(a.tx_drops, b.tx_drops),
    })
}

/// Adapters from `/proc/net/dev`, loopback left out.
pub fn parse_proc_net_dev(text: &str) -> Vec<(String, Counters)> {
    text.lines()
        .filter_map(|l| {
            let (name, rest) = l.split_once(':')?;
            let name = name.trim();
            if name == "lo" || name.is_empty() {
                return None;
            }
            let f: Vec<u64> = rest.split_whitespace().map(|v| v.parse().ok()).collect::<Option<_>>()?;
            // Receive: bytes packets errs drop fifo frame compressed multicast;
            // transmit: bytes packets errs drop fifo colls carrier compressed.
            (f.len() >= 16).then(|| {
                (
                    name.to_string(),
                    Counters {
                        rx_bytes: f[0],
                        rx_packets: f[1],
                        rx_errors: f[2],
                        rx_drops: f[3],
                        tx_bytes: f[8],
                        tx_packets: f[9],
                        tx_errors: f[10],
                        tx_drops: f[11],
                    },
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NET_DEV: &str = include_str!("../fixtures/linux/net_dev");

    #[test]
    fn proc_net_dev() {
        let v = parse_proc_net_dev(NET_DEV);
        let names: Vec<&str> = v.iter().map(|a| a.0.as_str()).collect();
        assert_eq!(names, ["enp5s0", "wlp4s0", "docker0"]);
        let e = &v[0].1;
        assert_eq!((e.rx_bytes, e.rx_packets, e.rx_errors, e.rx_drops), (9_876_543_210, 7_654_321, 3, 120));
        assert_eq!((e.tx_bytes, e.tx_packets, e.tx_errors, e.tx_drops), (1_234_567_890, 2_345_678, 1, 7));
    }

    #[test]
    fn rates_in_megabits() {
        let a = Counters::default();
        let b = Counters { rx_bytes: 125_000_000, tx_bytes: 12_500_000, rx_errors: 2, tx_drops: 3, ..a };
        let r = rates(&a, &b, 1.0).unwrap();
        assert_eq!((r.rx_mbps, r.tx_mbps, r.errors_ps, r.drops_ps), (1000.0, 100.0, 2.0, 3.0));
        assert_eq!(rates(&b, &a, 1.0).unwrap().rx_mbps, 0.0, "a reset is no traffic");
    }
}

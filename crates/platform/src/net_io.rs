//! Cumulative traffic counters per network adapter (S1-D02), no privileges
//! needed: Windows `GetIfTable2`, Linux `/proc/net/dev`, macOS the routing
//! socket's 64-bit `if_data64`. Rates are `cpuzeez_core::netio::rates`.
//! Loopback and per-container/tunnel interfaces are left out.

pub use cpuzeez_core::netio::Counters;

/// (adapter name, counters) for every adapter that is up.
pub fn read() -> Vec<(String, Counters)> {
    imp::read()
}

/// Interfaces that are plumbing rather than adapters a person would pick.
#[cfg_attr(windows, allow(dead_code))]
fn virtual_name(name: &str) -> bool {
    ["lo", "veth", "utun", "awdl", "llw", "gif", "stf", "anpi", "ap", "bridge", "tap", "tun"]
        .iter()
        .any(|p| name.starts_with(p) && name[p.len()..].chars().next().is_none_or(|c| c.is_ascii_digit()))
}

#[cfg(target_os = "linux")]
mod imp {
    pub fn read() -> Vec<(String, super::Counters)> {
        let text = std::fs::read_to_string("/proc/net/dev").unwrap_or_default();
        cpuzeez_core::netio::parse_proc_net_dev(&text).into_iter().filter(|(n, _)| !super::virtual_name(n)).collect()
    }
}

#[cfg(windows)]
mod imp {
    use super::Counters;
    use windows_sys::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
    use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;

    /// MIB_IF_ROW2.InterfaceAndOperStatusFlags bits.
    const HARDWARE: u8 = 1 << 0;
    const FILTER: u8 = 1 << 1;

    pub fn read() -> Vec<(String, Counters)> {
        let mut v = Vec::new();
        let mut t: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        // SAFETY: GetIfTable2 allocates the table; it's freed with FreeMibTable.
        unsafe {
            if GetIfTable2(&mut t) != 0 || t.is_null() {
                return v;
            }
            let rows = std::slice::from_raw_parts((*t).Table.as_ptr(), (*t).NumEntries as usize);
            for r in rows {
                let flags = r.InterfaceAndOperStatusFlags._bitfield;
                if flags & HARDWARE == 0 || flags & FILTER != 0 || r.OperStatus != IfOperStatusUp {
                    continue;
                }
                let n = r.Alias.iter().position(|&c| c == 0).unwrap_or(r.Alias.len());
                v.push((
                    String::from_utf16_lossy(&r.Alias[..n]),
                    Counters {
                        rx_bytes: r.InOctets,
                        tx_bytes: r.OutOctets,
                        rx_packets: r.InUcastPkts + r.InNUcastPkts,
                        tx_packets: r.OutUcastPkts + r.OutNUcastPkts,
                        rx_errors: r.InErrors,
                        tx_errors: r.OutErrors,
                        rx_drops: r.InDiscards,
                        tx_drops: r.OutDiscards,
                    },
                ));
            }
            FreeMibTable(t.cast());
        }
        v
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::Counters;

    const RTM_IFINFO2: u8 = 0x12;

    pub fn read() -> Vec<(String, Counters)> {
        let mut mib = [libc::CTL_NET, libc::PF_ROUTE, 0, 0, libc::NET_RT_IFLIST2, 0];
        let mut len = 0usize;
        // SAFETY: the first sysctl sizes the buffer, the second fills it;
        // messages are walked by their own length fields inside it.
        unsafe {
            if libc::sysctl(mib.as_mut_ptr(), 6, std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) != 0 {
                return Vec::new();
            }
            let mut buf = vec![0u8; len];
            if libc::sysctl(mib.as_mut_ptr(), 6, buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) != 0 {
                return Vec::new();
            }
            let mut v = Vec::new();
            let mut off = 0;
            while off + size_of::<libc::if_msghdr2>() <= len {
                let m = std::ptr::read_unaligned(buf.as_ptr().add(off).cast::<libc::if_msghdr2>());
                if m.ifm_msglen == 0 {
                    break;
                }
                off += m.ifm_msglen as usize;
                let up = m.ifm_flags & libc::IFF_UP != 0 && m.ifm_flags & libc::IFF_LOOPBACK == 0;
                if m.ifm_type != RTM_IFINFO2 || !up {
                    continue;
                }
                let mut name = [0 as libc::c_char; libc::IF_NAMESIZE];
                if libc::if_indextoname(m.ifm_index as u32, name.as_mut_ptr()).is_null() {
                    continue;
                }
                let name = std::ffi::CStr::from_ptr(name.as_ptr()).to_string_lossy().into_owned();
                if super::virtual_name(&name) {
                    continue;
                }
                let d = m.ifm_data;
                v.push((
                    name,
                    Counters {
                        rx_bytes: d.ifi_ibytes,
                        tx_bytes: d.ifi_obytes,
                        rx_packets: d.ifi_ipackets,
                        tx_packets: d.ifi_opackets,
                        rx_errors: d.ifi_ierrors,
                        tx_errors: d.ifi_oerrors,
                        rx_drops: d.ifi_iqdrops,
                        tx_drops: m.ifm_snd_drops.max(0) as u64,
                    },
                ));
            }
            v
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn read() -> Vec<(String, super::Counters)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn plumbing_is_left_out() {
        for n in ["lo", "lo0", "veth3a1b", "utun2", "awdl0", "bridge0"] {
            assert!(super::virtual_name(n), "{n}");
        }
        for n in ["en0", "enp5s0", "wlan0", "eth0", "apple-ish", "tailscale0"] {
            assert!(!super::virtual_name(n), "{n}");
        }
    }

    #[test]
    fn counters_only_grow_between_two_reads() {
        let a = super::read();
        std::thread::sleep(std::time::Duration::from_millis(50));
        for (n, b) in super::read() {
            if let Some((_, a)) = a.iter().find(|(m, _)| *m == n) {
                assert!(b.rx_bytes >= a.rx_bytes && b.tx_bytes >= a.tx_bytes, "{n}");
            }
        }
    }
}

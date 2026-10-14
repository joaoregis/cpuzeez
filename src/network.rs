//! Network adapters: name, description, type (Ethernet / Wi-Fi / virtual),
//! MAC, MTU, link speed, state, IPv4/IPv6, driver and version, PCI/USB device,
//! Wi-Fi SSID/band/signal when the OS exposes them.
//! Windows: IP Helper GetAdaptersAddresses, network class registry keys, WLAN
//! API (loaded at run time). Linux: getifaddrs, /sys/class/net, /proc/net,
//! ethtool GDRVINFO ioctl, `iw`. macOS: getifaddrs, `networksetup`, `route`,
//! `system_profiler SPAirPortDataType`. Nothing here needs privileges.
// Parsers stay OS-neutral so their unit tests run on every host.
#![allow(dead_code)]

use crate::Ctx;
use crate::report::{Section, f, gv, section};
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Adapter {
    pub name: String,
    pub description: String,
    /// "Ethernet", "Wi-Fi", "Bluetooth", "Virtual", "Loopback", "Cellular", "Other".
    pub kind: String,
    pub mac: String,
    pub mtu: Option<u32>,
    pub link_mbps: Option<u64>,
    pub up: bool,
    /// "192.168.1.10/24".
    pub ipv4: Vec<String>,
    /// "fe80::1/64".
    pub ipv6: Vec<String>,
    pub gateway: Vec<String>,
    pub dns: Vec<String>,
    pub driver: String,
    pub driver_version: String,
    pub driver_provider: String,
    /// "PCI bus 6, device 0, function 0" / "USB 0x0bda:0x8153".
    pub device: String,
    pub wifi_ssid: Option<String>,
    pub wifi_band: Option<String>,
    pub wifi_signal_pct: Option<u32>,
    pub wifi_channel: Option<u32>,
    pub wifi_rssi_dbm: Option<i32>,
    /// "802.11ax"…
    pub wifi_phy: Option<String>,
}

pub fn adapters(_ctx: &Ctx) -> Vec<Adapter> {
    imp::adapters()
}

/// "Network" report section. Loopback and IPv4-less virtual adapters are
/// skipped unless nothing else is left.
pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let all = adapters(ctx);
    let shown: Vec<&Adapter> =
        all.iter().filter(|a| a.kind != "Loopback" && (a.kind != "Virtual" || !a.ipv4.is_empty())).collect();
    let list = if shown.is_empty() { all.iter().collect() } else { shown };
    if list.is_empty() {
        return Vec::new();
    }
    let nodes = list.into_iter().map(node).collect();
    vec![section("Network", nodes)]
}

fn node(a: &Adapter) -> crate::report::Node {
    let mut c = Vec::new();
    let mut add = |k: &str, v: Option<String>| {
        if let Some(v) = v.filter(|v| !v.is_empty()) {
            c.push(f(k, v));
        }
    };
    add("Description", Some(a.description.clone()));
    add("Type", Some(a.kind.clone()));
    add("Status", Some(if a.up { "Up" } else { "Down" }.into()));
    add("MAC Address", Some(a.mac.clone()));
    add("MTU", a.mtu.map(|m| m.to_string()));
    add("Link Speed", a.link_mbps.map(speed_text));
    for ip in &a.ipv4 {
        add("IPv4 Address", Some(ip.clone()));
    }
    for ip in &a.ipv6 {
        add("IPv6 Address", Some(ip.clone()));
    }
    for ip in &a.gateway {
        add("Gateway", Some(ip.clone()));
    }
    for ip in &a.dns {
        add("DNS Server", Some(ip.clone()));
    }
    add("SSID", a.wifi_ssid.clone());
    add("Wi-Fi Standard", a.wifi_phy.clone());
    add("Band", a.wifi_band.clone().map(|b| a.wifi_channel.map_or(b.clone(), |ch| format!("{b}, channel {ch}"))));
    add("Signal", a.wifi_signal_pct.map(|p| a.wifi_rssi_dbm.map_or(format!("{p}%"), |d| format!("{p}% ({d} dBm)"))));
    add("Driver", Some(a.driver.clone()));
    add("Driver Version", Some(a.driver_version.clone()));
    add("Driver Provider", Some(a.driver_provider.clone()));
    add("Device", Some(a.device.clone()));
    gv("Adapter", &a.name, c)
}

pub fn speed_text(mbps: u64) -> String {
    if mbps >= 1000 && mbps.is_multiple_of(1000) {
        format!("{} Gbps", mbps / 1000)
    } else if mbps >= 1000 {
        format!("{:.1} Gbps", mbps as f64 / 1000.0)
    } else {
        format!("{mbps} Mbps")
    }
}

/// "00:1A:2B:3C:4D:5E"; empty for an all-zero or empty address.
pub fn fmt_mac(b: &[u8]) -> String {
    if b.iter().all(|&x| x == 0) {
        return String::new();
    }
    b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(":")
}

/// Linear RSSI → quality mapping (-100 dBm = 0 %, -50 dBm = 100 %).
pub fn dbm_to_pct(dbm: i32) -> u32 {
    (2 * (dbm + 100)).clamp(0, 100) as u32
}

pub fn band_from_mhz(mhz: u32) -> Option<&'static str> {
    Some(match mhz {
        2400..=2500 => "2.4 GHz",
        4900..=5899 => "5 GHz",
        5925..=7125 => "6 GHz",
        57000..=71000 => "60 GHz",
        _ => return None,
    })
}

pub fn channel_from_mhz(mhz: u32) -> Option<u32> {
    Some(match mhz {
        2484 => 14,
        2412..=2472 => (mhz - 2407) / 5,
        5955..=7115 => (mhz - 5950) / 5,
        5000..=5900 => (mhz - 5000) / 5,
        _ => return None,
    })
}

/// IEEE 802.11 PHY type (Windows DOT11_PHY_TYPE numbering).
pub fn phy_name(t: u32) -> Option<&'static str> {
    Some(match t {
        4 => "802.11a",
        5 => "802.11b",
        6 => "802.11g",
        7 => "802.11n (Wi-Fi 4)",
        8 => "802.11ac (Wi-Fi 5)",
        9 => "802.11ad",
        10 => "802.11ax (Wi-Fi 6)",
        11 => "802.11be (Wi-Fi 7)",
        _ => return None,
    })
}

fn prefix_len(mask: &[u8]) -> u32 {
    mask.iter().map(|b| b.count_ones()).sum()
}

/// IPv4 default routes from /proc/net/route: (interface, gateway).
pub fn parse_proc_route(text: &str) -> Vec<(String, Ipv4Addr)> {
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let c: Vec<&str> = l.split_whitespace().collect();
            let hex = |i: usize| c.get(i).and_then(|s| u32::from_str_radix(s, 16).ok());
            // Destination 0, RTF_GATEWAY; addresses are printed in host byte order.
            let gw = hex(2)?;
            (hex(1)? == 0 && hex(3)? & 2 != 0).then(|| (c[0].to_string(), Ipv4Addr::from(gw.to_ne_bytes())))
        })
        .collect()
}

/// IPv6 default routes from /proc/net/ipv6_route: (interface, next hop).
pub fn parse_ipv6_route(text: &str) -> Vec<(String, Ipv6Addr)> {
    text.lines()
        .filter_map(|l| {
            let c: Vec<&str> = l.split_whitespace().collect();
            if c.len() < 10 || c[0] != "0".repeat(32) || c[1] != "00" {
                return None;
            }
            let hop = u128::from_str_radix(c[4], 16).ok().filter(|&h| h != 0)?;
            Some((c[9].to_string(), Ipv6Addr::from(hop)))
        })
        .collect()
}

/// `nameserver` lines of resolv.conf.
pub fn parse_resolv(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("nameserver"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// /proc/net/wireless: (interface, signal level dBm).
pub fn parse_proc_wireless(text: &str) -> Vec<(String, i32)> {
    text.lines()
        .skip(2)
        .filter_map(|l| {
            let (name, rest) = l.split_once(':')?;
            let level: f64 = rest.split_whitespace().nth(2)?.trim_end_matches('.').parse().ok()?;
            Some((name.trim().to_string(), level as i32))
        })
        .collect()
}

/// `iw dev X link`: (SSID, frequency MHz, signal dBm).
pub fn parse_iw_link(text: &str) -> (Option<String>, Option<u32>, Option<i32>) {
    let get = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string()));
    let num = |s: Option<String>| s.and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok());
    (get("SSID:"), num(get("freq:")).map(|f| f as u32), num(get("signal:")).map(|s| s as i32))
}

/// `networksetup -listallhardwareports`: (port name, device).
pub fn parse_hardware_ports(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut port = None;
    for l in text.lines() {
        if let Some(p) = l.strip_prefix("Hardware Port:") {
            port = Some(p.trim().to_string());
        } else if let (Some(d), Some(p)) = (l.strip_prefix("Device:"), port.take()) {
            out.push((p, d.trim().to_string()));
        }
    }
    out
}

/// Wi-Fi from `system_profiler SPAirPortDataType -json`, per interface:
/// (device, SSID, channel, band, RSSI dBm, PHY mode).
pub fn parse_airport(
    v: &serde_json::Value,
) -> Vec<(String, Option<String>, Option<u32>, Option<String>, Option<i32>, Option<String>)> {
    let mut out = Vec::new();
    let ifs = v.pointer("/SPAirPortDataType/0/spairport_airport_interfaces").and_then(|x| x.as_array());
    for i in ifs.into_iter().flatten() {
        let Some(dev) = i.get("_name").and_then(|x| x.as_str()) else { continue };
        let Some(cur) = i.get("spairport_current_network_information") else { continue };
        let s = |k: &str| cur.get(k).map(|x| x.as_str().map_or(x.to_string(), str::to_string));
        // "149 (5GHz, 80MHz)" or a bare number.
        let ch = s("spairport_network_channel");
        let channel = ch.as_deref().and_then(|c| c.split([' ', '(']).next()?.parse().ok());
        let band = ch.as_deref().and_then(|c| {
            let g = c.split(['(', ',', ')']).map(str::trim).find(|p| p.ends_with("GHz"))?;
            Some(g.replace("GHz", " GHz"))
        });
        let rssi = s("spairport_signal_noise").and_then(|x| x.split_whitespace().next()?.parse().ok());
        out.push((dev.to_string(), s("_name"), channel, band, rssi, s("spairport_network_phymode")));
    }
    out
}

// ---------------------------------------------------------------- Unix (getifaddrs)

#[cfg(unix)]
#[derive(Default)]
struct IfAddrs {
    flags: u32,
    v4: Vec<String>,
    v6: Vec<String>,
    /// macOS AF_LINK only: (MAC, MTU, baud rate, IFT_* type).
    link: Option<(String, u32, u64, u8)>,
}

#[cfg(unix)]
fn ifaddrs() -> std::collections::BTreeMap<String, IfAddrs> {
    let mut map: std::collections::BTreeMap<String, IfAddrs> = Default::default();
    let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs allocates a list we walk read-only and free once.
    unsafe {
        if libc::getifaddrs(&mut head) != 0 {
            return map;
        }
        let mut p = head;
        while !p.is_null() {
            let ifa = &*p;
            p = ifa.ifa_next;
            let name = std::ffi::CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned();
            let e = map.entry(name).or_default();
            e.flags = ifa.ifa_flags;
            if ifa.ifa_addr.is_null() {
                continue;
            }
            let mask = |len: usize, off: usize| -> u32 {
                if ifa.ifa_netmask.is_null() {
                    return 0;
                }
                prefix_len(std::slice::from_raw_parts((ifa.ifa_netmask as *const u8).add(off), len))
            };
            match (*ifa.ifa_addr).sa_family as i32 {
                libc::AF_INET => {
                    let a = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                    let ip = Ipv4Addr::from(u32::from_be(a.sin_addr.s_addr));
                    e.v4.push(format!("{ip}/{}", mask(4, 4)));
                }
                libc::AF_INET6 => {
                    let a = &*(ifa.ifa_addr as *const libc::sockaddr_in6);
                    let ip = Ipv6Addr::from(a.sin6_addr.s6_addr);
                    e.v6.push(format!("{ip}/{}", mask(16, 8)));
                }
                #[cfg(target_os = "macos")]
                libc::AF_LINK => {
                    // sockaddr_dl: nlen at 5, alen at 6, name+address from 8.
                    let b = ifa.ifa_addr as *const u8;
                    let (nlen, alen) = (*b.add(5) as usize, *b.add(6) as usize);
                    let mac = fmt_mac(std::slice::from_raw_parts(b.add(8 + nlen), alen));
                    let (mtu, baud, ty) = if ifa.ifa_data.is_null() {
                        (0, 0, 0)
                    } else {
                        let d = &*(ifa.ifa_data as *const libc::if_data);
                        (d.ifi_mtu, d.ifi_baudrate as u64, d.ifi_type)
                    };
                    e.link = Some((mac, mtu, baud, ty));
                }
                _ => {}
            }
        }
        libc::freeifaddrs(head);
    }
    map
}

#[cfg(unix)]
fn resolv_dns() -> Vec<String> {
    // systemd-resolved keeps the real upstream servers here; resolv.conf has the 127.0.0.53 stub.
    ["/run/systemd/resolve/resolv.conf", "/etc/resolv.conf"]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok().map(|t| parse_resolv(&t)).filter(|v| !v.is_empty()))
        .unwrap_or_default()
}

// ---------------------------------------------------------------- Linux

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use crate::os::{read_trim, read_u64};
    use std::path::Path;

    /// ETHTOOL_GDRVINFO: (driver, version, firmware, bus info). No privileges needed.
    fn drvinfo(ifname: &str) -> Option<(String, String, String, String)> {
        #[repr(C)]
        struct IfReq {
            name: [u8; 16],
            data: *mut libc::c_void,
            _pad: [u8; 24],
        }
        let mut info = [0u8; 196];
        info[0..4].copy_from_slice(&3u32.to_ne_bytes()); // ETHTOOL_GDRVINFO
        let mut rq = IfReq { name: [0; 16], data: info.as_mut_ptr().cast(), _pad: [0; 24] };
        let n = ifname.as_bytes();
        rq.name[..n.len().min(15)].copy_from_slice(&n[..n.len().min(15)]);
        // SAFETY: plain socket + ioctl on local buffers that outlive the call.
        let ok = unsafe {
            let s = libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0);
            if s < 0 {
                return None;
            }
            let r = libc::ioctl(s, 0x8946 as _, &mut rq); // SIOCETHTOOL
            libc::close(s);
            r == 0
        };
        let s = |o: usize| {
            let b = &info[o..o + 32];
            String::from_utf8_lossy(&b[..b.iter().position(|&c| c == 0).unwrap_or(32)]).trim().to_string()
        };
        ok.then(|| (s(4), s(36), s(68), s(100)))
    }

    /// USB device directory above a sysfs interface path (has idVendor).
    fn usb_dev(mut p: &Path) -> Option<&Path> {
        while let Some(par) = p.parent() {
            if p.join("idVendor").exists() {
                return Some(p);
            }
            p = par;
        }
        None
    }

    pub fn adapters() -> Vec<Adapter> {
        let addrs = ifaddrs();
        let routes = std::fs::read_to_string("/proc/net/route").map(|t| parse_proc_route(&t)).unwrap_or_default();
        let routes6 = std::fs::read_to_string("/proc/net/ipv6_route").map(|t| parse_ipv6_route(&t)).unwrap_or_default();
        let wireless =
            std::fs::read_to_string("/proc/net/wireless").map(|t| parse_proc_wireless(&t)).unwrap_or_default();
        let dns = resolv_dns();
        let mut names: Vec<String> = std::fs::read_dir("/sys/class/net")
            .map(|r| r.flatten().filter_map(|e| e.file_name().into_string().ok()).collect())
            .unwrap_or_default();
        names.sort();
        let mut out = Vec::new();
        for name in names {
            let p = Path::new("/sys/class/net").join(&name);
            let real = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone()).to_string_lossy().into_owned();
            let ty = read_u64(p.join("type")).unwrap_or(0);
            let mut a = Adapter { name: name.clone(), ..Adapter::default() };
            a.kind = if ty == 772 {
                "Loopback"
            } else if p.join("wireless").exists() || p.join("phy80211").exists() {
                "Wi-Fi"
            } else if name.starts_with("wwan") || p.join("device/wwan").exists() {
                "Cellular"
            } else if name.starts_with("bnep") || real.contains("/bluetooth/") {
                "Bluetooth"
            } else if real.contains("/virtual/") {
                "Virtual"
            } else if ty == 1 {
                "Ethernet"
            } else {
                "Other"
            }
            .into();
            a.mac = read_trim(p.join("address")).map(|m| m.to_ascii_uppercase()).unwrap_or_default();
            if a.mac.chars().all(|c| c == '0' || c == ':') {
                a.mac.clear();
            }
            a.mtu = read_u64(p.join("mtu")).map(|m| m as u32);
            a.link_mbps =
                read_trim(p.join("speed")).and_then(|s| s.parse::<i64>().ok()).filter(|&s| s > 0).map(|s| s as u64);
            let oper = read_trim(p.join("operstate")).unwrap_or_default();
            a.up = oper == "up" || (oper == "unknown" && read_u64(p.join("carrier")) == Some(1));
            if let Some(i) = addrs.get(&name) {
                a.ipv4 = i.v4.clone();
                a.ipv6 = i.v6.clone();
            }
            a.gateway = routes.iter().filter(|r| r.0 == name).map(|r| r.1.to_string()).collect();
            a.gateway.extend(routes6.iter().filter(|r| r.0 == name).map(|r| r.1.to_string()));
            if !a.gateway.is_empty() {
                a.dns = dns.clone();
            }
            // Driver: ethtool, else the device/driver symlink.
            if let Some((drv, ver, fw, _bus)) = drvinfo(&name) {
                a.driver = drv;
                a.driver_version = if fw.is_empty() || fw == "N/A" { ver } else { format!("{ver} (firmware {fw})") };
            }
            if a.driver.is_empty() {
                a.driver = std::fs::read_link(p.join("device/driver"))
                    .ok()
                    .and_then(|l| l.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_default();
            }
            if a.driver_version.is_empty() && !a.driver.is_empty() {
                a.driver_version = read_trim(format!("/sys/module/{}/version", a.driver)).unwrap_or_default();
            }
            // Device: PCI slot + IDs, or the USB device's IDs and product string.
            let ue = std::fs::read_to_string(p.join("device/uevent")).unwrap_or_default();
            let kv = |k: &str| ue.lines().find_map(|l| l.strip_prefix(k)).map(str::to_string);
            let dev_real = std::fs::canonicalize(p.join("device")).ok();
            if let Some(slot) = kv("PCI_SLOT_NAME=") {
                let n: Vec<u32> = slot
                    .get(5..)
                    .unwrap_or("")
                    .split([':', '.'])
                    .filter_map(|x| u32::from_str_radix(x, 16).ok())
                    .collect();
                let id = kv("PCI_ID=").unwrap_or_default();
                if n.len() == 3 {
                    a.device = format!("PCI bus {}, device {}, function {} ({id})", n[0], n[1], n[2]);
                }
                let mut ids = id.split(':').map(|h| u16::from_str_radix(h, 16).ok());
                if let (Some(Some(v)), Some(Some(d))) = (ids.next(), ids.next()) {
                    a.description = crate::pci::ids_name(v, d).unwrap_or_default();
                }
            } else if let Some(u) = dev_real.as_deref().and_then(usb_dev) {
                let id = |f: &str| read_trim(u.join(f)).unwrap_or_default();
                a.device = format!("USB 0x{}:0x{}", id("idVendor"), id("idProduct"));
                a.description = [id("manufacturer"), id("product")].join(" ").trim().to_string();
            }
            if a.description.is_empty() {
                a.description = a.driver.clone();
            }
            if a.kind == "Wi-Fi" {
                let link = crate::os::cmd("iw", &["dev", &name, "link"]).map(|t| parse_iw_link(&t)).unwrap_or_default();
                a.wifi_ssid = link.0;
                a.wifi_band = link.1.and_then(band_from_mhz).map(str::to_string);
                a.wifi_channel = link.1.and_then(channel_from_mhz);
                a.wifi_rssi_dbm = link.2.or_else(|| wireless.iter().find(|w| w.0 == name).map(|w| w.1));
                a.wifi_signal_pct = a.wifi_rssi_dbm.filter(|&d| d < 0).map(dbm_to_pct);
            }
            out.push(a);
        }
        out
    }
}

// ---------------------------------------------------------------- macOS

#[cfg(target_os = "macos")]
mod imp {
    use super::*;

    pub fn adapters() -> Vec<Adapter> {
        let addrs = ifaddrs();
        let ports = crate::os::cmd("networksetup", &["-listallhardwareports"])
            .map(|t| parse_hardware_ports(&t))
            .unwrap_or_default();
        let route = |args: &[&str]| -> Option<(String, String)> {
            let t = crate::os::cmd("route", args)?;
            let get = |k: &str| t.lines().find_map(|l| l.trim().strip_prefix(k).map(|v| v.trim().to_string()));
            Some((get("interface:")?, get("gateway:")?))
        };
        let gws: Vec<(String, String)> = [route(&["-n", "get", "default"]), route(&["-n", "get", "-inet6", "default"])]
            .into_iter()
            .flatten()
            .collect();
        let dns = resolv_dns();
        let wifi = if ports.iter().any(|(p, _)| p.contains("Wi-Fi") || p.contains("AirPort")) {
            crate::os::cmd("system_profiler", &["SPAirPortDataType", "-json"])
                .and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok())
                .map(|v| parse_airport(&v))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let mut out = Vec::new();
        for (name, i) in &addrs {
            let port = ports.iter().find(|(_, d)| d == name).map(|(p, _)| p.clone());
            // Skip the many service interfaces (awdl, llw, utun…) unless they carry IPv4 (VPN).
            if port.is_none() && name != "lo0" && i.v4.is_empty() {
                continue;
            }
            let p = port.clone().unwrap_or_default();
            let kind = if name.starts_with("lo") {
                "Loopback"
            } else if p.contains("Wi-Fi") || p.contains("AirPort") {
                "Wi-Fi"
            } else if p.contains("Bluetooth") {
                "Bluetooth"
            } else if p.contains("Bridge") || port.is_none() {
                "Virtual"
            } else if p.contains("iPhone") || p.contains("Cellular") {
                "Cellular"
            } else {
                "Ethernet"
            };
            let (mac, mtu, baud, _) = i.link.clone().unwrap_or_default();
            let up = i.flags & (libc::IFF_UP | libc::IFF_RUNNING) as u32 == (libc::IFF_UP | libc::IFF_RUNNING) as u32;
            let mut a = Adapter {
                name: name.clone(),
                description: port.unwrap_or_else(|| name.clone()),
                kind: kind.into(),
                mac,
                mtu: Some(mtu).filter(|&m| m > 0),
                link_mbps: Some(baud / 1_000_000).filter(|&b| b > 0 && up),
                up,
                ipv4: i.v4.clone(),
                ipv6: i.v6.clone(),
                gateway: gws.iter().filter(|g| &g.0 == name).map(|g| g.1.clone()).collect(),
                ..Adapter::default()
            };
            if !a.gateway.is_empty() {
                a.dns = dns.clone();
            }
            if let Some(w) = wifi.iter().find(|w| &w.0 == name) {
                a.wifi_ssid = w.1.clone();
                a.wifi_channel = w.2;
                a.wifi_band = w.3.clone();
                a.wifi_rssi_dbm = w.4;
                a.wifi_signal_pct = w.4.map(dbm_to_pct);
                a.wifi_phy = w.5.clone();
            }
            out.push(a);
        }
        out
    }
}

// ---------------------------------------------------------------- Windows

#[cfg(windows)]
mod imp {
    #![allow(non_snake_case)]
    use super::*;
    use crate::storage::win as cm;
    use std::ffi::c_void;

    /// Shared head of IP_ADAPTER_{UNICAST,GATEWAY,DNS_SERVER}_ADDRESS (+ unicast tail).
    #[repr(C)]
    struct AddrNode {
        length: u32,
        flags: u32,
        next: *const AddrNode,
        address: SocketAddress,
        // Unicast only below.
        prefix_origin: u32,
        suffix_origin: u32,
        dad_state: u32,
        valid_lifetime: u32,
        preferred_lifetime: u32,
        lease_lifetime: u32,
        on_link_prefix_length: u8,
    }

    /// SOCKET_ADDRESS (pointer-aligned, so 16 bytes on x64).
    #[repr(C)]
    struct SocketAddress {
        sockaddr: *const u8,
        len: i32,
    }

    /// IP_ADAPTER_ADDRESSES_LH up to FirstGatewayAddress.
    #[repr(C)]
    struct IpAdapterAddresses {
        length: u32,
        if_index: u32,
        next: *const IpAdapterAddresses,
        adapter_name: *const u8,
        first_unicast: *const AddrNode,
        first_anycast: *const c_void,
        first_multicast: *const c_void,
        first_dns: *const AddrNode,
        dns_suffix: *const u16,
        description: *const u16,
        friendly_name: *const u16,
        physical_address: [u8; 8],
        physical_address_length: u32,
        flags: u32,
        mtu: u32,
        if_type: u32,
        oper_status: u32,
        ipv6_if_index: u32,
        zone_indices: [u32; 16],
        first_prefix: *const c_void,
        transmit_link_speed: u64,
        receive_link_speed: u64,
        first_wins: *const c_void,
        first_gateway: *const AddrNode,
    }

    #[link(name = "iphlpapi")]
    unsafe extern "system" {
        fn GetAdaptersAddresses(family: u32, flags: u32, res: *mut c_void, out: *mut c_void, size: *mut u32) -> u32;
    }

    unsafe fn wstr(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut n = 0;
        // SAFETY: NUL-terminated string owned by the adapter list.
        unsafe {
            while *p.add(n) != 0 {
                n += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
        }
    }

    /// SOCKADDR (AF_INET 2 / AF_INET6 23) to text.
    fn sockaddr(b: &[u8]) -> Option<String> {
        match u16::from_le_bytes([*b.first()?, *b.get(1)?]) {
            2 => Some(Ipv4Addr::from(<[u8; 4]>::try_from(b.get(4..8)?).ok()?).to_string()),
            23 => Some(Ipv6Addr::from(<[u8; 16]>::try_from(b.get(8..24)?).ok()?).to_string()),
            _ => None,
        }
    }

    unsafe fn list(mut n: *const AddrNode, prefix: bool) -> Vec<String> {
        let mut out = Vec::new();
        // SAFETY: linked list inside the GetAdaptersAddresses buffer.
        unsafe {
            while !n.is_null() {
                let a = &*n;
                if !a.address.sockaddr.is_null()
                    && let Some(ip) =
                        sockaddr(std::slice::from_raw_parts(a.address.sockaddr, a.address.len.max(0) as usize))
                {
                    out.push(if prefix { format!("{ip}/{}", a.on_link_prefix_length) } else { ip });
                }
                n = a.next;
            }
        }
        out
    }

    const NET_CLASS: &str = "SYSTEM\\CurrentControlSet\\Control\\Network\\{4D36E972-E325-11CE-BFC1-08002BE10318}";
    const DRP_SERVICE: u32 = 0x05;
    const DRP_DRIVER: u32 = 0x0A;

    /// Driver (service, version, provider) and device location from the PnP instance.
    fn driver_info(a: &mut Adapter, guid: &str) -> Option<()> {
        let inst = crate::os::win::reg_str(&format!("{NET_CLASS}\\{guid}\\Connection"), "PnPInstanceId")?;
        let up = inst.to_ascii_uppercase();
        let dn = cm::locate(&inst)?;
        a.driver = cm::reg_str(dn, DRP_SERVICE).unwrap_or_default();
        if let Some(key) = cm::reg_str(dn, DRP_DRIVER) {
            let k = format!("SYSTEM\\CurrentControlSet\\Control\\Class\\{key}");
            a.driver_version = crate::os::win::reg_str(&k, "DriverVersion").unwrap_or_default();
            a.driver_provider = crate::os::win::reg_str(&k, "ProviderName").unwrap_or_default();
        }
        let hex = |k: &str| up.find(k).and_then(|i| up.get(i + k.len()..i + k.len() + 4)).map(str::to_string);
        if up.starts_with("PCI\\") {
            let (bus, addr) = (cm::reg_u32(dn, cm::DRP_BUSNUMBER)?, cm::reg_u32(dn, cm::DRP_ADDRESS)?);
            a.device = format!(
                "PCI bus {bus}, device {}, function {} ({}:{})",
                addr >> 16,
                addr & 0xFFFF,
                hex("VEN_").unwrap_or_default(),
                hex("DEV_").unwrap_or_default()
            );
        } else if up.starts_with("USB\\") {
            let l = |s: Option<String>| s.unwrap_or_default().to_ascii_lowercase();
            a.device = format!("USB 0x{}:0x{}", l(hex("VID_")), l(hex("PID_")));
        } else if matches!(a.kind.as_str(), "Ethernet" | "Other") {
            a.kind = "Virtual".into(); // ROOT\, SWD\, {…}\ software devices
        }
        Some(())
    }

    pub fn adapters() -> Vec<Adapter> {
        const GAA_FLAG_INCLUDE_GATEWAYS: u32 = 0x80;
        let mut size = 32 * 1024u32;
        let mut buf: Vec<u64> = Vec::new();
        for _ in 0..3 {
            buf = vec![0u64; size as usize / 8 + 1];
            let r = unsafe {
                GetAdaptersAddresses(
                    0,
                    GAA_FLAG_INCLUDE_GATEWAYS,
                    std::ptr::null_mut(),
                    buf.as_mut_ptr().cast(),
                    &mut size,
                )
            };
            match r {
                0 => break,
                111 => continue, // ERROR_BUFFER_OVERFLOW: size updated
                _ => return Vec::new(),
            }
        }
        let wifi = wlan::connections();
        let mut out = Vec::new();
        let mut p = buf.as_ptr() as *const IpAdapterAddresses;
        // SAFETY: the list lives in `buf` for this whole loop.
        unsafe {
            while !p.is_null() {
                let x = &*p;
                p = x.next;
                let guid = std::ffi::CStr::from_ptr(x.adapter_name.cast()).to_string_lossy().into_owned();
                let mut a = Adapter {
                    name: wstr(x.friendly_name),
                    description: wstr(x.description),
                    mac: fmt_mac(&x.physical_address[..(x.physical_address_length as usize).min(8)]),
                    mtu: Some(x.mtu).filter(|&m| m != u32::MAX),
                    up: x.oper_status == 1,
                    ipv4: Vec::new(),
                    gateway: list(x.first_gateway, false),
                    dns: list(x.first_dns, false),
                    ..Adapter::default()
                };
                for ip in list(x.first_unicast, true) {
                    if ip.contains(':') { &mut a.ipv6 } else { &mut a.ipv4 }.push(ip);
                }
                let speed = x.receive_link_speed.max(x.transmit_link_speed);
                a.link_mbps = Some(speed / 1_000_000).filter(|&s| s > 0 && speed != u64::MAX && a.up);
                let d = a.description.to_ascii_lowercase();
                a.kind = match x.if_type {
                    24 => "Loopback",
                    71 => "Wi-Fi",
                    243 | 244 => "Cellular",
                    _ if d.contains("bluetooth") => "Bluetooth",
                    131 | 23 => "Virtual",
                    6 => "Ethernet",
                    _ => "Other",
                }
                .into();
                if driver_info(&mut a, &guid).is_none() && matches!(a.kind.as_str(), "Ethernet" | "Other") {
                    a.kind = "Virtual".into(); // no PnP device behind it
                }
                if let Some(w) = wifi.iter().find(|w| w.guid.eq_ignore_ascii_case(&guid)) {
                    a.wifi_ssid = Some(w.ssid.clone()).filter(|s| !s.is_empty());
                    a.wifi_signal_pct = Some(w.quality);
                    a.wifi_phy = phy_name(w.phy).map(str::to_string);
                    a.wifi_rssi_dbm = w.rssi;
                    a.wifi_band = w.mhz.and_then(band_from_mhz).map(str::to_string);
                    a.wifi_channel = w.mhz.and_then(channel_from_mhz);
                }
                out.push(a);
            }
        }
        out
    }

    /// WLAN API (wlanapi.dll), loaded at run time: absent on Server Core.
    mod wlan {
        use crate::gpu::dl::{Lib, sym};
        use std::ffi::c_void;

        pub struct Conn {
            pub guid: String,
            pub ssid: String,
            pub quality: u32,
            pub phy: u32,
            pub rssi: Option<i32>,
            pub mhz: Option<u32>,
        }

        type Open = unsafe extern "system" fn(u32, *mut c_void, *mut u32, *mut isize) -> u32;
        type Enum = unsafe extern "system" fn(isize, *mut c_void, *mut *mut u8) -> u32;
        type Query =
            unsafe extern "system" fn(isize, *const u8, u32, *mut c_void, *mut u32, *mut *mut u8, *mut u32) -> u32;
        type BssList =
            unsafe extern "system" fn(isize, *const u8, *const u8, u32, i32, *mut c_void, *mut *mut u8) -> u32;
        type Free = unsafe extern "system" fn(*mut c_void);
        type Close = unsafe extern "system" fn(isize, *mut c_void) -> u32;

        fn guid_str(g: &[u8]) -> String {
            let d1 = u32::from_le_bytes([g[0], g[1], g[2], g[3]]);
            let (d2, d3) = (u16::from_le_bytes([g[4], g[5]]), u16::from_le_bytes([g[6], g[7]]));
            let tail: String = g[10..16].iter().map(|b| format!("{b:02X}")).collect();
            format!("{{{d1:08X}-{d2:04X}-{d3:04X}-{:02X}{:02X}-{tail}}}", g[8], g[9])
        }

        fn le32(b: &[u8], o: usize) -> u32 {
            u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
        }

        pub fn connections() -> Vec<Conn> {
            imp().unwrap_or_default()
        }

        fn imp() -> Option<Vec<Conn>> {
            let lib = Lib::open(&["wlanapi.dll"])?;
            let open: Open = sym!(lib, "WlanOpenHandle")?;
            let enum_if: Enum = sym!(lib, "WlanEnumInterfaces")?;
            let query: Query = sym!(lib, "WlanQueryInterface")?;
            let bss: BssList = sym!(lib, "WlanGetNetworkBssList")?;
            let free: Free = sym!(lib, "WlanFreeMemory")?;
            let close: Close = sym!(lib, "WlanCloseHandle")?;
            let mut out = Vec::new();
            // SAFETY: documented WLAN API calls; every returned buffer is read
            // within its reported size and released with WlanFreeMemory.
            unsafe {
                let (mut ver, mut h) = (0u32, 0isize);
                if open(2, std::ptr::null_mut(), &mut ver, &mut h) != 0 {
                    return None;
                }
                let mut list: *mut u8 = std::ptr::null_mut();
                if enum_if(h, std::ptr::null_mut(), &mut list) == 0 && !list.is_null() {
                    // WLAN_INTERFACE_INFO_LIST: count, index, then 532-byte entries.
                    let n = u32::from_le_bytes(*(list as *const [u8; 4])) as usize;
                    for i in 0..n {
                        let e = list.add(8 + i * 532);
                        let guid = std::slice::from_raw_parts(e, 16);
                        let (mut size, mut data) = (0u32, std::ptr::null_mut::<u8>());
                        // wlan_intf_opcode_current_connection → WLAN_CONNECTION_ATTRIBUTES.
                        if query(h, e, 7, std::ptr::null_mut(), &mut size, &mut data, std::ptr::null_mut()) != 0
                            || data.is_null()
                        {
                            continue;
                        }
                        let c = std::slice::from_raw_parts(data, size as usize);
                        if c.len() >= 588 && le32(c, 0) == 1 {
                            // wlan_interface_state_connected; association attributes at 520.
                            let len = (le32(c, 520) as usize).min(32);
                            let bssid = c[560..566].to_vec();
                            let mut conn = Conn {
                                guid: guid_str(guid),
                                ssid: String::from_utf8_lossy(&c[524..524 + len]).into_owned(),
                                phy: le32(c, 568),
                                quality: le32(c, 576).min(100),
                                rssi: None,
                                mhz: None,
                            };
                            // BSS list (all networks): match our BSSID for RSSI and frequency.
                            let mut bl: *mut u8 = std::ptr::null_mut();
                            if bss(h, e, std::ptr::null(), 3, 0, std::ptr::null_mut(), &mut bl) == 0 && !bl.is_null() {
                                let total = u32::from_le_bytes(*(bl as *const [u8; 4])) as usize;
                                let b = std::slice::from_raw_parts(bl, total);
                                let count = le32(b, 4) as usize;
                                // WLAN_BSS_ENTRY is 360 bytes: BSSID at 40, lRssi at 56, frequency (kHz) at 92.
                                for k in 0..count {
                                    let o = 8 + k * 360;
                                    if b.get(o + 40..o + 46) == Some(&bssid[..]) {
                                        conn.rssi = Some(le32(b, o + 56) as i32);
                                        conn.mhz = Some(le32(b, o + 92) / 1000);
                                    }
                                }
                                free(bl.cast());
                            }
                            out.push(conn);
                        }
                        free(data.cast());
                    }
                    free(list.cast());
                }
                close(h, std::ptr::null_mut());
            }
            Some(out)
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn adapters() -> Vec<super::Adapter> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting() {
        assert_eq!(fmt_mac(&[0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]), "00:1A:2B:3C:4D:5E");
        assert_eq!(fmt_mac(&[0; 6]), "");
        assert_eq!(fmt_mac(&[]), "");
        assert_eq!(speed_text(2500), "2.5 Gbps");
        assert_eq!(speed_text(1000), "1 Gbps");
        assert_eq!(speed_text(100), "100 Mbps");
        assert_eq!(dbm_to_pct(-50), 100);
        assert_eq!(dbm_to_pct(-75), 50);
        assert_eq!(dbm_to_pct(-110), 0);
        assert_eq!((band_from_mhz(2437), channel_from_mhz(2437)), (Some("2.4 GHz"), Some(6)));
        assert_eq!((band_from_mhz(5180), channel_from_mhz(5180)), (Some("5 GHz"), Some(36)));
        assert_eq!((band_from_mhz(5975), channel_from_mhz(5975)), (Some("6 GHz"), Some(5)));
        assert_eq!(prefix_len(&[255, 255, 255, 0]), 24);
    }

    #[test]
    fn proc_net() {
        let r = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n\
eth0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0\n\
eth0\t0001A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0\n";
        let gw = parse_proc_route(r);
        assert_eq!(gw.len(), 1);
        assert_eq!(gw[0].0, "eth0");
        // Host byte order: true for every little-endian target we build.
        if cfg!(target_endian = "little") {
            assert_eq!(gw[0].1, Ipv4Addr::new(192, 168, 1, 1));
        }
        let r6 = "00000000000000000000000000000000 00 00000000000000000000000000000000 00 fe800000000000000000000000000001 00000400 00000001 00000000 00000003  wlan0\n\
fe800000000000000000000000000000 40 00000000000000000000000000000000 00 00000000000000000000000000000000 00000100 00000001 00000000 00000001  wlan0\n";
        assert_eq!(parse_ipv6_route(r6), vec![("wlan0".to_string(), "fe80::1".parse().unwrap())]);
        assert_eq!(parse_resolv("# x\nnameserver 1.1.1.1\nsearch lan\nnameserver  ::1\n"), vec!["1.1.1.1", "::1"]);
        let w = "Inter-| sta-|   Quality        |   Discarded packets\n face | tus | link level noise |  nwid  crypt\n wlan0: 0000   54.  -56.  -256        0      0\n";
        assert_eq!(parse_proc_wireless(w), vec![("wlan0".to_string(), -56)]);
        let iw = "Connected to aa:bb:cc:dd:ee:ff (on wlan0)\n\tSSID: Home Net\n\tfreq: 5180.0\n\tsignal: -45 dBm\n";
        assert_eq!(parse_iw_link(iw), (Some("Home Net".into()), Some(5180), Some(-45)));
    }

    #[test]
    fn macos_tools() {
        let t = "\nHardware Port: Ethernet\nDevice: en0\nEthernet Address: aa:bb\n\nHardware Port: Wi-Fi\nDevice: en1\nEthernet Address: cc:dd\n";
        assert_eq!(parse_hardware_ports(t), vec![("Ethernet".into(), "en0".into()), ("Wi-Fi".into(), "en1".into())]);
        let v: serde_json::Value = serde_json::from_str(
            r#"{"SPAirPortDataType":[{"spairport_airport_interfaces":[{"_name":"en1",
            "spairport_current_network_information":{"_name":"Cafe","spairport_network_channel":"149 (5GHz, 80MHz)",
            "spairport_network_phymode":"802.11ac","spairport_signal_noise":"-61 dBm / -94 dBm"}},{"_name":"awdl0"}]}]}"#,
        )
        .unwrap();
        let a = parse_airport(&v);
        assert_eq!(a.len(), 1);
        assert_eq!(
            a[0],
            ("en1".into(), Some("Cafe".into()), Some(149), Some("5 GHz".into()), Some(-61), Some("802.11ac".into()))
        );
    }
}

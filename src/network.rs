//! Network adapters: name, description, type (Ethernet / Wi-Fi / virtual),
//! MAC, MTU, link speed, state, IPv4/IPv6, driver and version, PCI/USB device,
//! Wi-Fi SSID/band/signal when the OS exposes them.

use crate::Ctx;
use crate::report::Section;

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
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
    pub gateway: Vec<String>,
    pub dns: Vec<String>,
    pub driver: String,
    pub driver_version: String,
    /// "PCI bus 6, device 0, function 0" / "USB 0x0bda:0x8153".
    pub device: String,
    pub wifi_ssid: Option<String>,
    pub wifi_band: Option<String>,
    pub wifi_signal_pct: Option<u32>,
}

pub fn adapters(_ctx: &Ctx) -> Vec<Adapter> {
    Vec::new()
}

/// "Network" report section.
pub fn collect(_ctx: &Ctx) -> Vec<Section> {
    Vec::new()
}

//! macOS: `system_profiler SPDisplaysDataType -json` for GPUs and displays,
//! EDIDs from the IOKit registry (`ioreg`: IODisplayEDID / IODisplayEDIDOriginal).

use super::{Adapter, Monitor, Probe, edid};
use crate::os::cmd;
use crate::report::f;
use serde_json::Value;

fn hex_id(v: &Value, k: &str) -> Option<u16> {
    let s = v.get(k)?.as_str()?;
    let h = s.trim().trim_start_matches("0x");
    u16::from_str_radix(h.split_whitespace().next()?, 16).ok()
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}

/// "spdisplays_metal3" → "Metal 3"
fn metal(v: &str) -> String {
    v.strip_prefix("spdisplays_metal").map_or_else(|| v.to_string(), |n| format!("Metal {n}").trim().to_string())
}

/// Apple GPU process node by SoC generation (Apple's announcements).
fn apple_nm(model: &str) -> Option<u32> {
    let soc = model.split_whitespace().find(|w| w.starts_with('M') && w.len() >= 2)?;
    match soc.get(1..2)? {
        "1" | "2" => Some(5),
        "3" | "4" | "5" => Some(3),
        _ => None,
    }
}

fn adapter(g: &Value) -> Adapter {
    let name = s(g, "sppci_model").or_else(|| s(g, "_name")).unwrap_or_default();
    let vendor = s(g, "spdisplays_vendor").unwrap_or_default().to_ascii_lowercase();
    let vendor_id = hex_id(g, "spdisplays_vendor-id").unwrap_or(if vendor.contains("apple") {
        0x106B
    } else if vendor.contains("intel") {
        0x8086
    } else if vendor.contains("amd") || vendor.contains("ati") {
        0x1002
    } else if vendor.contains("nvidia") {
        0x10DE
    } else {
        0
    });
    let mut a = Adapter {
        vendor_id,
        device_id: hex_id(g, "spdisplays_device-id").unwrap_or(0),
        rev: hex_id(g, "spdisplays_revision-id").unwrap_or(0) as u8,
        cores: s(g, "sppci_cores").and_then(|c| c.parse().ok()),
        ..Default::default()
    };
    if vendor_id == 0x106B {
        a.technology_nm = apple_nm(&name);
        a.mem_type = Some("Unified (LPDDR)".into());
    }
    // "8 GB" / "1536 MB"
    a.mem_size_mb = s(g, "spdisplays_vram").or_else(|| s(g, "_spdisplays_vram")).and_then(|v| {
        let mut it = v.split_whitespace();
        let n: f64 = it.next()?.parse().ok()?;
        Some(if it.next()?.eq_ignore_ascii_case("GB") { n * 1024.0 } else { n } as u64)
    });
    if let Some(m) = s(g, "spdisplays_mtlgpufamilysupport") {
        a.extra.push(("Metal family".into(), metal(&m)));
    }
    if let Some(b) = s(g, "sppci_bus") {
        a.extra.push(("Bus".into(), b.trim_start_matches("spdisplays_").into()));
    }
    a.name = name;
    a
}

/// Every EDID blob anywhere in the IOKit registry dump.
fn ioreg_edids() -> Vec<Vec<u8>> {
    let Some(txt) = cmd("ioreg", &["-l", "-w0"]) else { return Vec::new() };
    let low = txt.to_ascii_lowercase();
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut at = 0;
    while let Some(i) = low[at..].find("<00ffffffffffff00") {
        let start = at + i + 1;
        let end = low[start..].find('>').map_or(low.len(), |e| start + e);
        let hex = &low[start..end];
        let b: Vec<u8> =
            (0..hex.len() / 2).filter_map(|k| u8::from_str_radix(&hex[2 * k..2 * k + 2], 16).ok()).collect();
        if b.len() >= 128 && !out.contains(&b) {
            out.push(b);
        }
        at = end;
    }
    out
}

pub fn probe() -> Probe {
    let mut p = Probe::default();
    let json = cmd("system_profiler", &["SPDisplaysDataType", "-json"])
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .unwrap_or(Value::Null);
    let gpus = json.get("SPDisplaysDataType").and_then(Value::as_array).cloned().unwrap_or_default();
    if !gpus.is_empty() {
        p.apis.push(f("API", "Metal"));
    }
    let mut fallback = Vec::new();
    for g in &gpus {
        p.adapters.push(adapter(g));
        for d in g.get("spdisplays_ndrvs").and_then(Value::as_array).into_iter().flatten() {
            // Used only when IOKit has no EDID for this display (e.g. built-in panels).
            let res = s(d, "_spdisplays_pixels").or_else(|| s(d, "_spdisplays_resolution")).and_then(|r| {
                let n: Vec<u32> = r.split(|c: char| !c.is_ascii_digit()).filter_map(|x| x.parse().ok()).collect();
                (n.len() >= 2).then(|| (n[0], n[1], n.get(2).copied().unwrap_or(0)))
            });
            if s(d, "spdisplays_main").as_deref() == Some("spdisplays_yes")
                && let Some(a) = p.adapters.last_mut()
            {
                a.primary = true;
            }
            fallback.push(Monitor {
                model: s(d, "_name"),
                vendor: "Apple".into(),
                id: s(d, "_spdisplays_display-product-id").unwrap_or_default(),
                max_res: res,
                input: s(d, "spdisplays_connection_type").unwrap_or_else(|| "Internal".into()),
                ..Default::default()
            });
        }
    }
    p.monitors = ioreg_edids().iter().filter_map(|e| edid::parse(e)).collect();
    if p.monitors.is_empty() {
        p.monitors = fallback;
    }
    p
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_profiler() {
        let g: serde_json::Value = serde_json::from_str(
            r#"{"sppci_model":"Apple M2 Pro","spdisplays_vendor":"sppci_vendor_Apple","sppci_cores":"19",
               "spdisplays_mtlgpufamilysupport":"spdisplays_metal3"}"#,
        )
        .unwrap();
        let a = super::adapter(&g);
        assert_eq!((a.vendor_id, a.cores, a.technology_nm), (0x106B, Some(19), Some(5)));
        assert_eq!(a.extra[0].1, "Metal 3");
    }
}

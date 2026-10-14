//! macOS: IOPCIDevice registry entries via `ioreg -a` (XML plist). No raw
//! config space: IOKit only exposes the IDs and the PCIe link registers.
use super::{Device, Link};
use crate::hw::Bdf;

#[derive(Debug, Clone, PartialEq)]
pub enum Pv {
    Dict(Vec<(String, Pv)>),
    Arr(Vec<Pv>),
    Str(String),
    Data(Vec<u8>),
    Int(i64),
    Other,
}

impl Pv {
    fn get(&self, k: &str) -> Option<&Pv> {
        match self {
            Pv::Dict(v) => v.iter().find(|e| e.0 == k).map(|e| &e.1),
            _ => None,
        }
    }
    /// Little-endian integer from <data> or <integer>.
    fn num(&self, k: &str) -> Option<u64> {
        match self.get(k)? {
            Pv::Data(b) => Some(b.iter().take(8).rev().fold(0u64, |a, &x| a << 8 | x as u64)),
            Pv::Int(i) => Some(*i as u64),
            _ => None,
        }
    }
    fn text(&self, k: &str) -> Option<String> {
        match self.get(k)? {
            Pv::Str(s) => Some(s.clone()),
            Pv::Data(b) => Some(String::from_utf8_lossy(b).trim_end_matches('\0').to_string()),
            _ => None,
        }
    }
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

fn b64(s: &str) -> Vec<u8> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let (mut out, mut acc, mut bits) = (Vec::new(), 0u32, 0);
    for v in s.bytes().filter_map(val) {
        acc = acc << 6 | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/// Minimal XML plist reader (dict/array/string/data/integer; everything else = Other).
pub fn parse_plist(x: &str) -> Option<Pv> {
    let mut p = x.find("<plist").and_then(|i| x[i..].find('>').map(|j| i + j + 1))?;
    value(x, &mut p)
}

fn next_tag<'a>(x: &'a str, p: &mut usize) -> Option<&'a str> {
    let i = *p + x[*p..].find('<')?;
    let j = i + x[i..].find('>')?;
    *p = j + 1;
    Some(&x[i + 1..j])
}

fn value(x: &str, p: &mut usize) -> Option<Pv> {
    let tag = next_tag(x, p)?;
    let body = |p: &mut usize, name: &str| -> Option<String> {
        let end = format!("</{name}>");
        let i = x[*p..].find(&end)? + *p;
        let s = x[*p..i].to_string();
        *p = i + end.len();
        Some(s)
    };
    Some(match tag {
        "dict" => {
            let mut v = Vec::new();
            loop {
                let save = *p;
                match next_tag(x, p)? {
                    "/dict" => break,
                    "key" => {
                        let k = unescape(&body(p, "key")?);
                        v.push((k, value(x, p)?));
                    }
                    _ => {
                        *p = save;
                        value(x, p)?;
                    }
                }
            }
            Pv::Dict(v)
        }
        "array" => {
            let mut v = Vec::new();
            loop {
                let save = *p;
                if next_tag(x, p)? == "/array" {
                    break;
                }
                *p = save;
                v.push(value(x, p)?);
            }
            Pv::Arr(v)
        }
        "dict/" => Pv::Dict(Vec::new()),
        "array/" => Pv::Arr(Vec::new()),
        "string" => Pv::Str(unescape(&body(p, "string")?)),
        "string/" => Pv::Str(String::new()),
        "data" => Pv::Data(b64(&body(p, "data")?)),
        "integer" => {
            let s = body(p, "integer")?;
            let s = s.trim();
            Pv::Int(s.strip_prefix("0x").map_or_else(|| s.parse().ok(), |h| i64::from_str_radix(h, 16).ok())?)
        }
        t if t.ends_with('/') => Pv::Other, // <true/>, <false/>
        t => {
            body(p, t)?; // <real>, <date>
            Pv::Other
        }
    })
}

fn device(e: &Pv) -> Option<Device> {
    // "pcidebug" = "bus:dev:func" (optionally followed by "(...)").
    let dbg = e.text("pcidebug")?;
    let mut it = dbg.split(['(', ' ']).next()?.split(':').map(|n| n.parse::<u8>().ok());
    let (bus, dev, func) = (it.next()??, it.next()??, it.next()??);
    let mut d = Device::new(Bdf { seg: 0, bus, dev, func });
    d.vendor = e.num("vendor-id")? as u16;
    d.device = e.num("device-id").unwrap_or(0) as u16;
    d.subvendor = e.num("subsystem-vendor-id").unwrap_or(0) as u16;
    d.subdevice = e.num("subsystem-id").unwrap_or(0) as u16;
    d.revision = e.num("revision-id").unwrap_or(0) as u8;
    d.class = e.num("class-code").unwrap_or(0) as u32 & 0xFF_FFFF;
    d.name = e.text("model").or_else(|| e.text("IOName")).filter(|s| !s.is_empty());
    if let (Some(ls), Some(lc)) = (e.num("IOPCIExpressLinkStatus"), e.num("IOPCIExpressLinkCapabilities")) {
        d.link = Some(Link {
            speed: (ls & 0xF) as u8,
            width: ((ls >> 4) & 0x3F) as u8,
            max_speed: (lc & 0xF) as u8,
            max_width: ((lc >> 4) & 0x3F) as u8,
        });
    }
    Some(d)
}

pub fn list() -> Vec<Device> {
    let Some(xml) = crate::os::cmd("ioreg", &["-a", "-r", "-d", "1", "-c", "IOPCIDevice"]) else { return Vec::new() };
    match parse_plist(&xml) {
        Some(Pv::Arr(v)) => v.iter().filter_map(device).collect(),
        Some(d @ Pv::Dict(_)) => device(&d).into_iter().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plist() {
        let x = r#"<?xml version="1.0"?><!DOCTYPE plist><plist version="1.0"><array><dict>
<key>IOName</key><string>pci106b,1a07</string><key>vendor-id</key><data>axAAAA==</data>
<key>device-id</key><data>BxoAAA==</data><key>class-code</key><data>AAQGAA==</data>
<key>pcidebug</key><string>0:28:0(1:1)</string><key>IOPCIExpressLinkStatus</key><integer>0x2012</integer>
<key>IOPCIExpressLinkCapabilities</key><integer>4195362</integer><key>x</key><true/><key>y</key><dict><key>z</key><array/></dict>
</dict></array></plist>"#;
        let v = parse_plist(x).unwrap();
        let Pv::Arr(a) = v else { panic!() };
        let d = device(&a[0]).unwrap();
        assert_eq!((d.vendor, d.device, d.class), (0x106B, 0x1A07, 0x060400));
        assert_eq!((d.bdf.bus, d.bdf.dev, d.bdf.func), (0, 28, 0));
        let l = d.link.unwrap();
        assert_eq!((l.speed, l.width), (2, 1));
        assert_eq!(d.name.as_deref(), Some("pci106b,1a07"));
    }
}

//! Memory SPD section (FEATURES.md 3.9, 8): read DIMM SPD EEPROMs over SMBus
//! and decode them. Decoders are pure (`decode`), reading is in this file.

pub mod decode;
mod jep106;

pub use decode::{DecodedModule, Timing};

use crate::Ctx;
use crate::hw::Smbus;
use crate::report::{Node, Section, f, gv, hex, section};
use std::sync::OnceLock;

/// Raw SPD image of one module. `hub` holds SPD5118 registers MR0..MR63
/// (DDR5 only, empty when unreadable).
#[derive(Debug, Clone)]
pub struct Dimm {
    pub bus: String,
    pub addr: u8,
    pub bytes: Vec<u8>,
    pub hub: Vec<u8>,
}

const SPD_ADDRS: std::ops::RangeInclusive<u8> = 0x50..=0x57;

/// SPD5118 hub: MR0/MR1 read back the device type 0x5118.
fn is_spd5118(bus: &dyn Smbus, addr: u8) -> bool {
    bus.read_byte(addr, 0x00) == Some(0x51) && bus.read_byte(addr, 0x01) == Some(0x18)
}

fn read_range(bus: &dyn Smbus, addr: u8, cmds: std::ops::Range<u16>) -> Option<Vec<u8>> {
    cmds.map(|c| bus.read_byte(addr, c as u8)).collect()
}

/// DDR5: 8 pages of 128 bytes selected through MR11, read at 0x80..0xFF.
/// Only MR11 is ever written; page 0 is restored on every exit path.
fn read_spd5118(bus: &dyn Smbus, addr: u8) -> Option<Vec<u8>> {
    let mr11 = bus.read_byte(addr, 0x0B)?;
    if mr11 & 0x08 != 0 {
        return None; // 2-byte addressing mode: not ours to change
    }
    let mut out = Vec::with_capacity(1024);
    let mut ok = true;
    for page in 0..8u8 {
        if !bus.write_byte(addr, 0x0B, (mr11 & !7) | page) {
            ok = false;
            break;
        }
        match read_range(bus, addr, 0x80..0x100) {
            Some(p) => out.extend(p),
            None => {
                ok = false;
                break;
            }
        }
    }
    bus.write_byte(addr, 0x0B, mr11 & !7);
    ok.then_some(out)
}

/// SDR..DDR4 EEPROMs. DDR4/LPDDR4 EE1004 has two 256-byte pages selected by
/// a send-byte to 0x36 (page 0) / 0x37 (page 1); we always end on page 0.
fn read_legacy(bus: &dyn Smbus, addr: u8) -> Option<Vec<u8>> {
    let _ = bus.send_byte(0x36, 0);
    let mut out = read_range(bus, addr, 0..0x100)?;
    if matches!(out[2], 0x0C | 0x0E | 0x10 | 0x11 | 0x13 | 0x15) {
        let page1 = bus.send_byte(0x37, 0).then(|| read_range(bus, addr, 0..0x100)).flatten();
        let _ = bus.send_byte(0x36, 0);
        out.extend(page1?);
    }
    Some(out)
}

fn read_dimm(bus: &dyn Smbus, addr: u8) -> Option<Dimm> {
    let present = bus.read_byte(addr, 0x00);
    let os = bus.eeprom(addr);
    if present.is_none() && os.is_none() {
        return None;
    }
    let hub5118 = is_spd5118(bus, addr);
    let hub = if hub5118 { read_range(bus, addr, 0..64).unwrap_or_default() } else { Vec::new() };
    let bytes = match os {
        Some(b) => b,
        None if hub5118 => read_spd5118(bus, addr)?,
        None => read_legacy(bus, addr)?,
    };
    if bytes.len() < 128 || bytes[2] == 0 || bytes[2] == 0xFF {
        return None;
    }
    Some(Dimm { bus: bus.name(), addr, bytes, hub })
}

/// Every SPD EEPROM on every SMBus adapter (read once per run).
pub fn read_all(ctx: &Ctx) -> Vec<Dimm> {
    static CACHE: OnceLock<Vec<Dimm>> = OnceLock::new();
    if cfg!(target_os = "macos") || !ctx.cfg.spd || !ctx.cfg.smbus {
        return Vec::new();
    }
    CACHE
        .get_or_init(|| {
            let mut v = Vec::new();
            for bus in ctx.hw.smbus() {
                v.extend(SPD_ADDRS.filter_map(|a| read_dimm(bus.as_ref(), a)));
            }
            v
        })
        .clone()
}

/// Live DIMM temperatures in degC, keyed by the module's SPD address
/// (0x50-0x57): DDR5 from the SPD5118 sensor (MR49/MR50), DDR4 from the
/// JC42.4 TSOD at 0x18-0x1F.
#[allow(dead_code)] // used by the sensors module
pub fn dimm_temps(ctx: &Ctx) -> Vec<(u8, f64)> {
    if cfg!(target_os = "macos") || !ctx.cfg.smbus {
        return Vec::new();
    }
    let ddr4: Vec<u8> = read_all(ctx).iter().filter(|d| d.bytes.get(2) == Some(&0x0C)).map(|d| d.addr).collect();
    let mut v = Vec::new();
    for bus in ctx.hw.smbus() {
        let bus = bus.as_ref();
        for a in SPD_ADDRS {
            if is_spd5118(bus, a) {
                // MR26 bit 0 = sensor disabled.
                if bus.read_byte(a, 0x1A).is_some_and(|x| x & 1 == 0)
                    && let (Some(lo), Some(hi)) = (bus.read_byte(a, 0x31), bus.read_byte(a, 0x32))
                {
                    v.push((a, ts5118(lo, hi)));
                }
            } else if ddr4.contains(&a)
                && let Some(w) = bus.read_word(0x18 + (a - 0x50), 0x05)
            {
                v.push((a, tsod(w.swap_bytes())));
            }
        }
    }
    v
}

/// SPD5118 temperature: 0.25 degC LSB in bits 12:2, two's complement.
fn ts5118(lo: u8, hi: u8) -> f64 {
    let raw = ((hi as u16) << 8 | lo as u16) >> 2;
    ((raw << 5) as i16 >> 5) as f64 * 0.25
}

/// JC42.4 temperature register (big-endian on the wire): 13-bit, 1/16 degC.
fn tsod(w: u16) -> f64 {
    ((w << 3) as i16 >> 3) as f64 / 16.0
}

/// Decoded modules for the SPD tab.
pub fn modules(ctx: &Ctx) -> Vec<DecodedModule> {
    read_all(ctx).iter().filter_map(decode_dimm).collect()
}

fn decode_dimm(d: &Dimm) -> Option<DecodedModule> {
    let mut m = decode::decode(&d.bytes, &d.hub)?;
    m.bus = d.bus.clone();
    m.addr = d.addr;
    Some(m)
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let mods = modules(ctx);
    if mods.is_empty() {
        return Vec::new();
    }
    vec![section("Memory SPD", nodes(&mods))]
}

fn yes(rev: &Option<String>) -> String {
    match rev {
        Some(r) if r.contains('.') => format!("yes, rev. {r}"),
        Some(_) => "yes".into(),
        None => "no".into(),
    }
}

fn row_text(t: &Timing, cr: bool, volts: bool) -> String {
    let mut s = format!("{:.1}-{}-{}-{}-{}", t.cl, t.rcd, t.rp, t.ras, t.rc);
    if cr {
        s += &t.cr.map_or("-n.a".to_string(), |c| format!("-{c}T"));
    }
    s += &format!(" @ {} MHz", (t.freq + 1e-6).floor() as u64);
    if let (true, Some(v)) = (volts, t.volts) {
        s += &format!(" ({v:.3} Volts)");
    }
    s
}

/// CPU-Z's layout: one "DIMM #" block per module (plus its timing tables),
/// then the raw dumps.
pub fn nodes(mods: &[DecodedModule]) -> Vec<Node> {
    let mut out = Vec::new();
    for (i, m) in mods.iter().enumerate() {
        let mut c = vec![
            f("SMBus address", format!("0x{:02X}", m.addr)),
            f("Memory type", &m.mem_type),
            f("Module format", &m.format),
        ];
        if !m.module_mfr_id.is_empty() {
            c.push(f("Module Manufacturer(ID)", format!("{} ({})", m.module_mfr, m.module_mfr_id)));
        }
        if !m.dram_mfr_id.is_empty() {
            c.push(f("SDRAM Manufacturer (ID)", format!("{} ({})", m.dram_mfr, m.dram_mfr_id)));
        }
        c.push(f("Size", format!("{} MBytes", m.size_mb)));
        for (n, (on, name)) in m.pmic.iter().enumerate() {
            c.push(f(format!("PMIC #{n}"), if *on { "Enabled" } else { "Disabled" }));
            if *on && !name.is_empty() {
                c.push(f(format!("PMIC #{n} Manufacturer"), name));
            }
        }
        if let Some((on, name)) = &m.ckd {
            c.push(f("Clock Driver (CKD)", if *on { "Enabled" } else { "Disabled" }));
            if *on && !name.is_empty() {
                c.push(f("Clock Driver Manufacturer", name));
            }
        }
        c.push(f("Max bandwidth", &m.max_bw));
        if let Some(j) = &m.max_jedec {
            c.push(f("Max JEDEC", j));
        }
        if let Some(ch) = m.channels {
            c.push(f("Channels", ch));
        }
        if !m.part.is_empty() {
            c.push(f("Part number", &m.part));
        }
        if let Some(s) = &m.serial {
            c.push(f("Serial number", s));
        }
        if let (Some(w), Some(y)) = (m.week, m.year) {
            c.push(f("Manufacturing date", format!("Week {w}/Year {y:02}")));
        }
        if let Some(v) = m.voltage {
            c.push(f("Nominal Voltage", format!("{v:.2} Volts")));
        }
        if let Some(t) = &m.temp_limit {
            c.push(f("Temperature Limit", t));
        }
        c.push(f("EPP", yes(&m.epp)));
        c.push(f("XMP", yes(&m.xmp)));
        c.push(f("AMP", yes(&m.amp)));
        c.push(f("EXPO", yes(&m.expo)));
        out.push(gv("DIMM #", i + 1, c));

        if !m.jedec.is_empty() {
            let rows = m.jedec.iter().map(|t| f(&t.label, row_text(t, false, false))).collect();
            out.push(gv("JEDEC timings table", "CL-tRCD-tRP-tRAS-tRC @ frequency", rows));
        }
        for p in &m.profiles {
            let fields = p.fields.iter().map(|(k, v)| f(k, v)).collect();
            out.push(gv(format!("{} profile", p.kind), &p.name, fields));
            let (head, cr) = match p.kind {
                "XMP" => ("CL-tRCD-tRP-tRAS-tRC-CR @ frequency (voltage)", true),
                _ => ("CL-tRCD-tRP-tRAS-tRC @ frequency", false),
            };
            let r = f(&p.timing.label, row_text(&p.timing, cr, cr));
            out.push(gv(format!("{} timings table", p.kind), head, vec![r]));
        }
    }
    for (i, m) in mods.iter().enumerate() {
        out.push(gv("DIMM #", i + 1, vec![]));
        out.push(hex("SPD registers", m.raw.clone()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUMP: &str = include_str!("testdata/cpuz_ddr5_teamgroup.txt");

    /// The SPD hex dumps from CPU-Z's report, one Vec per "SPD registers" block.
    fn dumps() -> Vec<Vec<u8>> {
        let mut out: Vec<Vec<u8>> = Vec::new();
        let mut cur: Option<Vec<u8>> = None;
        for l in DUMP.lines() {
            if l.starts_with("SPD registers") {
                out.extend(cur.take());
                cur = Some(Vec::new());
                continue;
            }
            let Some(buf) = cur.as_mut() else { continue };
            let mut it = l.trim().split('\t');
            let (Some(off), Some(data)) = (it.next(), it.next()) else { continue };
            if u16::from_str_radix(off, 16).is_err() {
                continue;
            }
            buf.extend(data.split_whitespace().map(|h| u8::from_str_radix(h, 16).unwrap()));
        }
        out.extend(cur);
        out
    }

    /// SPD5118 power-on register image as the hub reported it: 55/85 degC limits.
    fn hub() -> Vec<u8> {
        let mut h = vec![0u8; 64];
        h[0] = 0x51;
        h[1] = 0x18;
        (h[0x1C], h[0x1D]) = (0x70, 0x03);
        (h[0x20], h[0x21]) = (0x50, 0x05);
        h
    }

    fn norm(s: &str) -> Vec<String> {
        s.lines().map(|l| l.trim_end().to_string()).filter(|l| !l.is_empty()).collect()
    }

    #[test]
    fn golden_ddr5_matches_cpuz() {
        let d = dumps();
        assert_eq!(d.len(), 2);
        assert!(d.iter().all(|b| b.len() == 1024));
        let mods: Vec<DecodedModule> = d
            .iter()
            .zip([0x51, 0x53])
            .map(|(b, a)| {
                let mut m = decode::decode(b, &hub()).unwrap();
                m.addr = a;
                m
            })
            .collect();
        assert_eq!(mods[0].crc_ok, Some(true));
        assert_eq!((mods[0].ranks, mods[0].banks), (1, 32));
        assert_eq!(mods[0].die_density, "16 Gbit");
        assert_eq!(mods[0].extensions(), ["XMP 3.0", "EXPO 1.0"]);

        let txt = crate::report::to_txt(&[section("Memory SPD", nodes(&mods))]);
        let body: Vec<String> = norm(&txt).into_iter().skip_while(|l| !l.starts_with("DIMM #")).collect();
        let want = norm(DUMP);
        for (i, (g, w)) in body.iter().zip(&want).enumerate() {
            assert_eq!(g, w, "line {i}");
        }
        assert_eq!(body.len(), want.len());
    }

    #[test]
    fn temperatures() {
        assert_eq!(ts5118(0x6C, 0x02), 38.75); // CPU-Z "39 degC [0x26C]"
        assert_eq!(ts5118(0xC0, 0x1F), -4.0);
        assert_eq!(tsod(0x0190), 25.0);
        assert_eq!(tsod(0x1FF0), -1.0);
    }

    /// Fake SPD5118 hub with 1024 NVM bytes behind MR11 paging.
    struct FakeHub {
        nvm: Vec<u8>,
        mr11: std::sync::Mutex<u8>,
    }
    impl Smbus for FakeHub {
        fn name(&self) -> String {
            "fake".into()
        }
        fn read_byte(&self, addr: u8, cmd: u8) -> Option<u8> {
            if addr != 0x51 {
                return None;
            }
            let page = *self.mr11.lock().unwrap() as usize & 7;
            Some(match cmd {
                0x00 => 0x51,
                0x01 => 0x18,
                0x0B => *self.mr11.lock().unwrap(),
                0x80.. => self.nvm[page * 128 + (cmd as usize - 0x80)],
                _ => 0,
            })
        }
        fn write_byte(&self, addr: u8, cmd: u8, val: u8) -> bool {
            assert_eq!((addr, cmd), (0x51, 0x0B), "only MR11 may be written");
            *self.mr11.lock().unwrap() = val;
            true
        }
        fn send_byte(&self, _: u8, _: u8) -> bool {
            panic!("no EE1004 page select on DDR5")
        }
    }

    #[test]
    fn spd5118_paged_read_restores_page0() {
        let nvm = dumps().remove(0);
        let hub = FakeHub { nvm: nvm.clone(), mr11: std::sync::Mutex::new(0) };
        assert!(read_dimm(&hub, 0x50).is_none());
        let d = read_dimm(&hub, 0x51).unwrap();
        assert_eq!(d.bytes, nvm);
        assert_eq!(d.hub.len(), 64);
        assert_eq!(*hub.mr11.lock().unwrap(), 0);
    }
}

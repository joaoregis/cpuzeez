//! Pure SPD decoders: raw EEPROM bytes in, `DecodedModule` out. No I/O.
//! Layouts from the public JEDEC SPD annexes (JESD21-C 4.1.2.x, JESD400-5),
//! Intel XMP 1.x/2.0/3.0, AMD EXPO and NVIDIA EPP public descriptions.

use super::jep106;

/// One row of a timings table. Cycle counts are at `freq`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timing {
    pub label: String,
    pub freq: f64,
    pub cl: f64,
    pub rcd: u32,
    pub rp: u32,
    pub ras: u32,
    pub rc: u32,
    pub cr: Option<u32>,
    pub volts: Option<f64>,
}

/// XMP / EXPO / EPP profile: CPU-Z's key/value lines plus its timing row.
#[derive(Debug, Clone, Default)]
pub struct Profile {
    pub kind: &'static str,
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub timing: Timing,
}

#[derive(Debug, Clone, Default)]
pub struct DecodedModule {
    pub bus: String,
    pub addr: u8,
    pub mem_type: String,
    pub format: String,
    pub module_mfr: String,
    pub module_mfr_id: String,
    pub dram_mfr: String,
    pub dram_mfr_id: String,
    pub size_mb: u64,
    pub ranks: u32,
    pub banks: u32,
    /// Per-die density, e.g. "16 Gbit".
    pub die_density: String,
    pub registered: bool,
    /// (enabled, manufacturer) per PMIC slot (DDR5).
    pub pmic: Vec<(bool, String)>,
    /// (enabled, manufacturer) clock driver / register (DDR5).
    pub ckd: Option<(bool, String)>,
    pub max_bw: String,
    pub max_jedec: Option<String>,
    pub channels: Option<u32>,
    pub part: String,
    pub serial: Option<String>,
    pub week: Option<u8>,
    pub year: Option<u8>,
    pub voltage: Option<f64>,
    pub temp_limit: Option<String>,
    pub epp: Option<String>,
    pub xmp: Option<String>,
    pub amp: Option<String>,
    pub expo: Option<String>,
    pub jedec: Vec<Timing>,
    pub profiles: Vec<Profile>,
    pub crc_ok: Option<bool>,
    pub raw: Vec<u8>,
}

impl DecodedModule {
    /// "XMP 3.0", "EXPO 1.0", ... for the SPD tab's extension box.
    pub fn extensions(&self) -> Vec<String> {
        [("EPP", &self.epp), ("XMP", &self.xmp), ("AMP", &self.amp), ("EXPO", &self.expo)]
            .iter()
            .filter_map(|(k, v)| v.as_ref().map(|r| format!("{k} {r}").trim().to_string()))
            .collect()
    }
}

/// Decode any supported SPD image. `hub` is the SPD5118 register file
/// (MR0..MR63) when it could be read, else empty.
pub fn decode(b: &[u8], hub: &[u8]) -> Option<DecodedModule> {
    let mut m = match *b.get(2)? {
        0x01..=0x04 if b.len() >= 128 => legacy(b, Gen::Sdr),
        0x07 if b.len() >= 128 => legacy(b, Gen::Ddr),
        0x08 if b.len() >= 128 => legacy(b, Gen::Ddr2),
        0x09 if b.len() >= 256 => fbdimm(b),
        0x0B if b.len() >= 256 => ddr3(b),
        0x0C | 0x0E | 0x10 | 0x11 | 0x13 | 0x15 if b.len() >= 384 => ddr4(b),
        0x12 | 0x14 if b.len() >= 640 => ddr5(b, hub),
        _ => None,
    }?;
    m.raw = b.to_vec();
    Some(m)
}

// ---------------------------------------------------------------- helpers

fn u16le(b: &[u8], i: usize) -> u32 {
    b.get(i).copied().unwrap_or(0) as u32 | (b.get(i + 1).copied().unwrap_or(0) as u32) << 8
}

fn bcd(x: u8) -> u8 {
    (x >> 4) * 10 + (x & 0xF)
}

fn ascii(b: &[u8]) -> String {
    b.iter()
        .take_while(|&&c| c != 0)
        .map(|&c| if (0x20..0x7F).contains(&c) { c as char } else { ' ' })
        .collect::<String>()
        .trim()
        .to_string()
}

/// CRC-16/XMODEM, the SPD CRC since DDR3.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

/// Manufacturer from a 1-based bank and code: (name, CPU-Z style 16-byte hex id).
fn mfr(bank: u8, code: u8) -> (String, String) {
    if code == 0 || code == 0xFF {
        return (String::new(), String::new());
    }
    let mut id = "7F".repeat(bank.saturating_sub(1) as usize);
    id.push_str(&format!("{code:02X}"));
    while id.len() < 32 {
        id.push('0');
    }
    let name = jep106::name(bank, code).unwrap_or("Unknown").to_string();
    (name, id)
}

/// DDR3+ two-byte ID: continuation count (with parity) then code.
fn mfr2(cont: u8, code: u8) -> (String, String) {
    mfr((cont & 0x7F) + 1, code)
}

/// SDR..DDR2 / FB-DIMM eight-byte ID: 0x7F continuation bytes then the code.
fn mfr8(b: &[u8]) -> (String, String) {
    let n = b.iter().take_while(|&&x| x == 0x7F).count();
    mfr(n as u8 + 1, b.get(n).copied().unwrap_or(0))
}

/// Clock in multiples of 100/3 MHz, the grid every SDRAM speed grade sits on.
fn quantize(tck_ps: f64) -> f64 {
    let k = (30000.0 / tck_ps).round().max(1.0);
    30000.0 / k
}

fn mhz(tck_ps: f64) -> f64 {
    1e6 / tck_ps
}

fn nck(t_ps: f64, tck_ps: f64) -> u32 {
    (t_ps / tck_ps - 1e-6).ceil().max(0.0) as u32
}

fn ifloor(x: f64) -> u64 {
    (x + 1e-6).floor() as u64
}

#[derive(Clone, Copy)]
struct Core {
    taa: f64,
    trcd: f64,
    trp: f64,
    tras: f64,
    trc: f64,
}

fn row(label: String, tck: f64, cl: f64, t: &Core) -> Timing {
    Timing {
        label,
        freq: mhz(tck),
        cl,
        rcd: nck(t.trcd, tck),
        rp: nck(t.trp, tck),
        ras: nck(t.tras, tck),
        rc: nck(t.trc, tck),
        cr: None,
        volts: None,
    }
}

fn number(rows: &mut [Timing], prefix: &str) {
    for (i, r) in rows.iter_mut().enumerate() {
        r.label = format!("{prefix} #{}", i + 1);
    }
}

/// "DDR5-6000 (3000 MHz)", "PC3-12800 (800 MHz)", ...
fn bandwidth(fam: &str, tck: f64) -> String {
    let f = mhz(tck);
    let rate = 2.0 * f;
    let pc = |round: bool| {
        let x = rate * 8.0 / 100.0;
        (if round { x.round() } else { (x + 1e-6).floor() }) as u64 * 100
    };
    match fam {
        "SDRAM" => format!("PC{} ({} MHz)", ifloor(f), ifloor(f)),
        "DDR" => format!("PC{} ({} MHz)", pc(true), ifloor(f)),
        "DDR2" | "DDR2 FB-DIMM" => format!("PC2-{} ({} MHz)", pc(false), ifloor(f)),
        "DDR3" => format!("PC3-{} ({} MHz)", pc(false), ifloor(f)),
        _ => format!("{fam}-{} ({} MHz)", ifloor(rate), ifloor(f)),
    }
}

fn ns(ps: f64) -> String {
    format!("{:.2} ns", ps / 1000.0)
}

fn cycle(tck: f64) -> String {
    format!("{:.3} ns ({} MHz)", tck / 1000.0, ifloor(mhz(tck)))
}

fn volts3(v: f64) -> String {
    format!("{v:.3} Volts")
}

fn kv(k: &str, v: String) -> (String, String) {
    (k.to_string(), v)
}

/// Common tail of every decoder: max bandwidth over JEDEC and profiles.
fn finish(m: &mut DecodedModule, fam: &str, jedec_tck: f64, profile_tcks: &[f64]) {
    let best = profile_tcks.iter().copied().fold(jedec_tck, f64::min);
    m.max_bw = bandwidth(fam, best);
    if !profile_tcks.is_empty() {
        m.max_jedec = Some(bandwidth(fam, jedec_tck));
    }
}

// ---------------------------------------------------------------- DDR5

fn xmp3_volts(x: u8) -> f64 {
    ((x >> 5) & 3) as f64 + (x & 0x1F) as f64 * 0.05
}

fn ddr5(b: &[u8], hub: &[u8]) -> Option<DecodedModule> {
    let mut m =
        DecodedModule { mem_type: if b[2] == 0x14 { "DDR5 NVDIMM-P" } else { "DDR5" }.into(), ..Default::default() };
    let kind = b[3] & 0xF;
    m.format = match kind {
        1 => "RDIMM",
        2 => "UDIMM",
        3 => "SO-DIMM",
        4 => "LRDIMM",
        5 => "CUDIMM",
        6 => "CSODIMM",
        7 => "MRDIMM",
        8 => "CAMM2",
        0xA => "DDIMM",
        0xB => "Soldered",
        _ => "Unknown",
    }
    .into();
    m.registered = matches!(kind, 1 | 4 | 7);

    // First/second SDRAM: density (Gbit), dies per package, I/O width.
    let dens = |x: u8| [0u64, 4, 8, 12, 16, 24, 32, 48, 64].get((x & 0x1F) as usize).copied().unwrap_or(0);
    let dies = |x: u8| match (x >> 5) & 7 {
        2 => 2,
        3 => 4,
        4 => 8,
        5 => 16,
        _ => 1u64,
    };
    let width = |x: u8| 4u64 << ((x >> 5) & 3);
    let ranks = ((b[234] >> 3) & 7) as u64 + 1;
    let asym = b[234] & 0x40 != 0;
    let channels = ((b[235] >> 5) & 3) as u64 + 1;
    let bus = 8u64 << (b[235] & 7);
    let mut gbit = 0;
    for r in 0..ranks {
        let o = if asym && r % 2 == 1 { 4 } else { 0 };
        gbit += bus / width(b[6 + o]) * dies(b[4 + o]) * dens(b[4 + o]);
    }
    m.size_mb = channels * gbit * 1024 / 8;
    m.ranks = ranks as u32;
    m.banks = (1 << ((b[7] >> 5) & 7)) * (1 << (b[7] & 7));
    m.die_density = format!("{} Gbit", dens(b[4]));
    m.channels = Some(channels as u32);

    for i in 0..3 {
        let o = 198 + 4 * i;
        let on = b[o + 2] & 0x80 != 0;
        m.pmic.push((on, if on { mfr2(b[o], b[o + 1]).0 } else { String::new() }));
    }
    // Clock driver / RCD: first bytes of the module-specific block.
    let has_ckd = matches!(kind, 1 | 4 | 5 | 6 | 7) && (b[240] | b[241]) != 0;
    m.ckd = Some((has_ckd, if has_ckd { mfr2(b[240], b[241]).0 } else { String::new() }));

    let tck = u16le(b, 20) as f64;
    let tckmax = u16le(b, 22) as f64;
    if tck == 0.0 {
        return None;
    }
    let t = Core {
        taa: u16le(b, 30) as f64,
        trcd: u16le(b, 32) as f64,
        trp: u16le(b, 34) as f64,
        tras: u16le(b, 36) as f64,
        trc: u16le(b, 38) as f64,
    };
    let jtck = quantize(tck);
    // One row per supported CL, at the speed-grid clock where it meets tAA.
    for i in 0..40 {
        if b[24 + i / 8] >> (i % 8) & 1 == 0 || t.taa == 0.0 {
            continue;
        }
        let cl = 20.0 + 2.0 * i as f64;
        let ck = quantize(t.taa / cl);
        if ck + 1e-6 < jtck || (tckmax > 0.0 && ck > quantize(tckmax) + 1e-6) {
            continue;
        }
        m.jedec.push(row(String::new(), ck, cl, &t));
    }
    number(&mut m.jedec, "JEDEC");

    m.voltage = (b[16] >> 4 == 0).then_some(1.1);
    if hub.len() >= 0x22 {
        let ts = |lo: u8, hi: u8| {
            let raw = ((hi as u16) << 8 | lo as u16) >> 2;
            ((raw << 5) as i16 >> 5) as f64 * 0.25
        };
        m.temp_limit = Some(format!(
            "{:.1} degC (high limit), {:.1} degC (critical limit)",
            ts(hub[0x1C], hub[0x1D]),
            ts(hub[0x20], hub[0x21])
        ));
    }

    (m.module_mfr, m.module_mfr_id) = mfr2(b[512], b[513]);
    (m.dram_mfr, m.dram_mfr_id) = mfr2(b[552], b[553]);
    if b[515] | b[516] != 0 {
        (m.year, m.week) = (Some(bcd(b[515])), Some(bcd(b[516])));
    }
    m.serial = Some(b[517..521].iter().map(|x| format!("{x:02X}")).collect());
    m.part = ascii(&b[521..551]);
    m.crc_ok = Some(crc16(&b[..510]) == u16le(b, 510) as u16);

    let mut ptck = Vec::new();
    if b.len() >= 1024 && b[640] == 0x0C && b[641] == 0x4A {
        m.xmp = Some(format!("{}.{}", b[642] >> 4, b[642] & 0xF));
        for p in 0..3 {
            if b[643] >> p & 1 == 0 {
                continue;
            }
            let o = 0x2C0 + 0x40 * p;
            let raw = u16le(b, o + 5) as f64;
            if raw == 0.0 {
                continue;
            }
            let tck = quantize(raw);
            let t = Core {
                taa: u16le(b, o + 13) as f64,
                trcd: u16le(b, o + 15) as f64,
                trp: u16le(b, o + 17) as f64,
                tras: u16le(b, o + 19) as f64,
                trc: u16le(b, o + 21) as f64,
            };
            let (vpp, vdd, vddq, vmc) =
                (xmp3_volts(b[o]), xmp3_volts(b[o + 1]), xmp3_volts(b[o + 2]), xmp3_volts(b[o + 4]));
            let cl = nck(t.taa, tck) as f64;
            // CPU-Z shows 15 of the 16 name bytes.
            let spec = ascii(&b[0x28E + 16 * p..0x28E + 16 * p + 15]);
            let mut fields = Vec::new();
            if !spec.is_empty() {
                fields.push(kv("Specification", spec));
            }
            fields.extend([
                kv("VDD Voltage", volts3(vdd)),
                kv("VDDQ Voltage", volts3(vddq)),
                kv("VPP Voltage", volts3(vpp)),
                kv("Memory Controller Volt.", volts3(vmc)),
                kv("Min Cycle time", cycle(tck)),
                kv("Max CL", format!("{cl:.1}")),
                kv("Min tRP", ns(t.trp)),
                kv("Min tRCD", ns(t.trcd)),
                kv("Min tRAS", ns(t.tras)),
                kv("Min tRC", ns(t.trc)),
            ]);
            let mut timing = row("XMP #1".into(), tck, cl, &t);
            timing.volts = Some(vdd);
            m.profiles.push(Profile { kind: "XMP", name: format!("XMP-{}", ifloor(2.0 * mhz(tck))), fields, timing });
            ptck.push(tck);
        }
    }
    if b.len() >= 0x3C0 && &b[0x340..0x344] == b"EXPO" {
        m.expo = Some(format!("{}.{}", b[0x344] >> 4, b[0x344] & 0xF));
        for p in 0..2 {
            let o = 0x34A + 40 * p;
            let raw = u16le(b, o + 4) as f64;
            if b[0x345] >> (4 * p) & 1 == 0 || raw == 0.0 {
                continue;
            }
            let tck = quantize(raw);
            let t = Core {
                taa: u16le(b, o + 6) as f64,
                trcd: u16le(b, o + 8) as f64,
                trp: u16le(b, o + 10) as f64,
                tras: u16le(b, o + 12) as f64,
                trc: u16le(b, o + 14) as f64,
            };
            let cl = nck(t.taa, tck) as f64;
            let fields = vec![
                kv("VDD Voltage", volts3(xmp3_volts(b[o]))),
                kv("VDDQ Voltage", volts3(xmp3_volts(b[o + 1]))),
                kv("VPP Voltage", volts3(xmp3_volts(b[o + 2]))),
                kv("Min Cycle time", cycle(tck)),
                kv("Max CL", format!("{cl:.1}")),
                kv("Min tRP", ns(t.trp)),
                kv("Min tRCD", ns(t.trcd)),
                kv("Min tRAS", ns(t.tras)),
                // CPU-Z quirk: tRFC1 is stored in ns but printed as if ps.
                kv("Min tRFC", ns(u16le(b, o + 18) as f64)),
            ];
            let timing = row("EXPO #1".into(), tck, cl, &t);
            m.profiles.push(Profile { kind: "EXPO", name: String::new(), fields, timing });
            ptck.push(tck);
        }
    }
    finish(&mut m, "DDR5", jtck, &ptck);
    m.max_jedec = Some(bandwidth("DDR5", jtck));
    Some(m)
}

// ---------------------------------------------------------------- DDR4 / LPDDR4 / LPDDR5

fn mtb_ftb(mtb: u32, ftb: u8) -> f64 {
    (mtb as i64 * 125 + ftb as i8 as i64) as f64
}

/// JEDEC rows for DDR3/DDR4: each CL at the exact clock where it meets tAA,
/// capped at tCKmin (CPU-Z truncates the shown MHz).
fn cl_rows(cls: &[f64], t: &Core, tck: f64, tckmax: f64) -> Vec<Timing> {
    let mut v = Vec::new();
    for &cl in cls {
        let ck = t.taa / cl;
        if tckmax > 0.0 && ck > tckmax + 1e-6 {
            continue;
        }
        if ck <= tck + 1e-6 {
            v.push(row(String::new(), quantize(tck), cl, t));
            break;
        }
        v.push(row(String::new(), ck, cl, t));
    }
    number(&mut v, "JEDEC");
    v
}

fn ddr4(b: &[u8]) -> Option<DecodedModule> {
    let lp = b[2] != 0x0C && b[2] != 0x0E;
    let fam = match b[2] {
        0x10 => "LPDDR4",
        0x11 => "LPDDR4X",
        0x13 => "LPDDR5",
        0x15 => "LPDDR5X",
        _ => "DDR4",
    };
    let mut m = DecodedModule { mem_type: fam.into(), ..Default::default() };
    let kind = b[3] & 0xF;
    m.format = match kind {
        1 => "RDIMM",
        2 => "UDIMM",
        3 => "SO-DIMM",
        4 => "LRDIMM",
        5 => "Mini-RDIMM",
        6 => "Mini-UDIMM",
        8 => "72b-SO-RDIMM",
        9 => "72b-SO-UDIMM",
        0xC => "16b-SO-DIMM",
        0xD => "32b-SO-DIMM",
        0xE => "Soldered",
        _ => "Unknown",
    }
    .into();
    m.registered = !lp && matches!(kind, 1 | 4 | 5 | 8);
    let dens_mb = [256u64, 512, 1024, 2048, 4096, 8192, 16384, 32768, 12288, 24576]
        .get((b[4] & 0xF) as usize)
        .copied()
        .unwrap_or(0);
    let bg = [1u32, 2, 4, 1][(b[4] >> 6) as usize];
    m.banks = (4 << ((b[4] >> 4) & 3)) * bg;
    let dies = if b[6] & 0x80 != 0 { ((b[6] >> 4) & 7) as u64 + 1 } else { 1 };
    let width = 4u64 << (b[12] & 7);
    let ranks = ((b[12] >> 3) & 7) as u64 + 1;
    let bus = 8u64 << (b[13] & 7);
    // ponytail: LPDDR channel count per package isn't folded in; size can be low for LP parts.
    m.size_mb = dens_mb / 8 * (bus / width) * ranks * dies;
    m.ranks = ranks as u32;
    m.die_density = if dens_mb >= 1024 { format!("{} Gbit", dens_mb / 1024) } else { format!("{dens_mb} Mbit") };
    m.voltage = Some(match fam {
        "DDR4" => 1.2,
        "LPDDR5" | "LPDDR5X" => 1.05,
        _ => 1.1,
    });

    let tck = mtb_ftb(b[18] as u32, b[125]);
    let tckmax = mtb_ftb(b[19] as u32, b[124]);
    if tck <= 0.0 {
        return None;
    }
    let t = Core {
        taa: mtb_ftb(b[24] as u32, b[123]),
        trcd: mtb_ftb(b[25] as u32, b[122]),
        trp: mtb_ftb(b[26] as u32, b[121]),
        tras: ((((b[27] & 0xF) as u32) << 8 | b[28] as u32) * 125) as f64,
        trc: mtb_ftb(((b[27] >> 4) as u32) << 8 | b[29] as u32, b[120]),
    };
    let jtck = quantize(tck);
    if lp {
        // LPDDR CL bitmaps are not a plain range: one row at max speed.
        m.jedec = vec![row("JEDEC #1".into(), jtck, nck(t.taa, jtck) as f64, &t)];
    } else {
        let bits = u32::from_le_bytes([b[20], b[21], b[22], b[23]]);
        let base = if bits & 0x8000_0000 != 0 { 23.0 } else { 7.0 };
        let cls: Vec<f64> = (0..30).filter(|i| bits >> i & 1 == 1).map(|i| base + i as f64).collect();
        m.jedec = cl_rows(&cls, &t, tck, tckmax);
    }

    (m.module_mfr, m.module_mfr_id) = mfr2(b[320], b[321]);
    (m.dram_mfr, m.dram_mfr_id) = mfr2(b[350], b[351]);
    if b[323] | b[324] != 0 {
        (m.year, m.week) = (Some(bcd(b[323])), Some(bcd(b[324])));
    }
    m.serial = Some(b[325..329].iter().map(|x| format!("{x:02X}")).collect());
    m.part = ascii(&b[329..349]);
    let mut ok = crc16(&b[..126]) == u16le(b, 126) as u16;
    ok &= crc16(&b[128..254]) == u16le(b, 254) as u16;
    m.crc_ok = Some(ok);

    let mut ptck = Vec::new();
    if !lp && b.len() >= 512 && b[384] == 0x0C && b[385] == 0x4A {
        m.xmp = Some(format!("{}.{}", b[387] >> 4, b[387] & 0xF));
        for p in 0..2 {
            let o = 393 + 47 * p;
            let raw = mtb_ftb(b[o + 3] as u32, b[o + 38]);
            if b[386] >> p & 1 == 0 || raw <= 0.0 {
                continue;
            }
            let tck = quantize(raw);
            let vdd = (b[o] >> 7) as f64 + (b[o] & 0x7F) as f64 / 100.0;
            let t = Core {
                taa: mtb_ftb(b[o + 8] as u32, b[o + 37]),
                trcd: mtb_ftb(b[o + 9] as u32, b[o + 36]),
                trp: mtb_ftb(b[o + 10] as u32, b[o + 35]),
                tras: ((((b[o + 11] & 0xF) as u32) << 8 | b[o + 12] as u32) * 125) as f64,
                trc: mtb_ftb(((b[o + 11] >> 4) as u32) << 8 | b[o + 13] as u32, b[o + 34]),
            };
            m.profiles.push(xmp_profile(fam, tck, vdd, &t, None));
            ptck.push(tck);
        }
    }
    finish(&mut m, fam, jtck, &ptck);
    Some(m)
}

/// XMP 1.x/2.0 profile in CPU-Z's layout.
fn xmp_profile(fam: &str, tck: f64, vdd: f64, t: &Core, cr: Option<u32>) -> Profile {
    let cl = nck(t.taa, tck) as f64;
    let rate = ifloor(2.0 * mhz(tck));
    let fields = vec![
        kv("Specification", format!("{fam}-{rate}")),
        kv("VDD Voltage", volts3(vdd)),
        kv("Min Cycle time", cycle(tck)),
        kv("Max CL", format!("{cl:.1}")),
        kv("Min tRP", ns(t.trp)),
        kv("Min tRCD", ns(t.trcd)),
        kv("Min tRAS", ns(t.tras)),
        kv("Min tRC", ns(t.trc)),
    ];
    let mut timing = row("XMP #1".into(), tck, cl, t);
    timing.volts = Some(vdd);
    timing.cr = cr;
    Profile { kind: "XMP", name: format!("XMP-{rate}"), fields, timing }
}

// ---------------------------------------------------------------- DDR3

fn ddr3(b: &[u8]) -> Option<DecodedModule> {
    let mut m = DecodedModule { mem_type: "DDR3".into(), ..Default::default() };
    let kind = b[3] & 0xF;
    m.format = match kind {
        1 => "RDIMM",
        2 => "UDIMM",
        3 => "SO-DIMM",
        4 => "Micro-DIMM",
        5 => "Mini-RDIMM",
        6 => "Mini-UDIMM",
        7 => "Mini-CDIMM",
        8 => "72b-SO-UDIMM",
        9 => "72b-SO-RDIMM",
        0xA => "72b-SO-CDIMM",
        0xB => "LRDIMM",
        0xC => "16b-SO-DIMM",
        0xD => "32b-SO-DIMM",
        _ => "Unknown",
    }
    .into();
    m.registered = matches!(kind, 1 | 5 | 9 | 0xB);
    let dens_mb = 256u64 << (b[4] & 0xF).min(7);
    m.banks = 8 << ((b[4] >> 4) & 7);
    let width = 4u64 << (b[7] & 7);
    let ranks = ((b[7] >> 3) & 7) as u64 + 1;
    let bus = 8u64 << (b[8] & 7);
    m.size_mb = dens_mb / 8 * (bus / width) * ranks;
    m.ranks = ranks as u32;
    m.die_density = if dens_mb >= 1024 { format!("{} Gbit", dens_mb / 1024) } else { format!("{dens_mb} Mbit") };
    m.voltage = Some(if b[6] & 1 == 0 {
        1.5
    } else if b[6] & 2 != 0 {
        1.35
    } else {
        1.25
    });

    let mtb = if b[11] != 0 { 1000.0 * b[10] as f64 / b[11] as f64 } else { 125.0 };
    let ftb = if b[9] & 0xF != 0 { (b[9] >> 4) as f64 / (b[9] & 0xF) as f64 } else { 1.0 };
    let tm = |x: u32, f: u8| x as f64 * mtb + f as i8 as f64 * ftb;
    let tck = tm(b[12] as u32, b[34]);
    if tck <= 0.0 {
        return None;
    }
    let t = Core {
        taa: tm(b[16] as u32, b[35]),
        trcd: tm(b[18] as u32, b[36]),
        trp: tm(b[20] as u32, b[37]),
        tras: tm(((b[21] & 0xF) as u32) << 8 | b[22] as u32, 0),
        trc: tm(((b[21] >> 4) as u32) << 8 | b[23] as u32, b[38]),
    };
    let bits = u16le(b, 14);
    let cls: Vec<f64> = (0..16).filter(|i| bits >> i & 1 == 1).map(|i| 4.0 + i as f64).collect();
    m.jedec = cl_rows(&cls, &t, tck, 0.0);
    let jtck = quantize(tck);

    (m.module_mfr, m.module_mfr_id) = mfr2(b[117], b[118]);
    (m.dram_mfr, m.dram_mfr_id) = mfr2(b[148], b[149]);
    if b[120] | b[121] != 0 {
        (m.year, m.week) = (Some(bcd(b[120])), Some(bcd(b[121])));
    }
    m.serial = Some(b[122..126].iter().map(|x| format!("{x:02X}")).collect());
    m.part = ascii(&b[128..146]);
    let end = if b[0] & 0x80 != 0 { 117 } else { 126 };
    m.crc_ok = Some(crc16(&b[..end]) == u16le(b, 126) as u16);

    let mut ptck = Vec::new();
    if b[176] == 0x0C && b[177] == 0x4A {
        m.xmp = Some(format!("{}.{}", b[179] >> 4, b[179] & 0xF));
        for p in 0..2 {
            let (div, dvs) = (b[180 + 2 * p] as f64, b[181 + 2 * p] as f64);
            let o = 185 + 35 * p;
            if b[178] >> p & 1 == 0 || dvs == 0.0 || b[o + 1] == 0 {
                continue;
            }
            let pm = 1000.0 * div / dvs;
            let vdd = ((b[o] >> 5) & 3) as f64 + (b[o] & 0x1F) as f64 * 0.05;
            let raw = b[o + 1] as f64 * pm;
            let t = Core {
                taa: b[o + 2] as f64 * pm,
                trp: b[o + 6] as f64 * pm,
                trcd: b[o + 7] as f64 * pm,
                tras: (((b[o + 9] & 0xF) as u32) << 8 | b[o + 10] as u32) as f64 * pm,
                trc: (((b[o + 9] >> 4) as u32) << 8 | b[o + 11] as u32) as f64 * pm,
            };
            // Command rate is stored in MTB units; express it in clocks.
            let cr = (b[o + 24] != 0).then(|| (b[o + 24] as f64 / b[o + 1] as f64).round() as u32);
            let tck = quantize(raw);
            m.profiles.push(xmp_profile("DDR3", tck, vdd, &t, cr));
            ptck.push(tck);
        }
    }
    finish(&mut m, "DDR3", jtck, &ptck);
    Some(m)
}

// ---------------------------------------------------------------- SDR / DDR / DDR2

#[derive(Clone, Copy, PartialEq)]
enum Gen {
    Sdr,
    Ddr,
    Ddr2,
}

/// tCK byte: ns in the high nibble, tenths (or .25/.33/.66/.75) in the low one.
fn tck_byte(x: u8) -> f64 {
    let frac = match x & 0xF {
        n @ 0..=9 => n as f64 / 10.0,
        0xA => 0.25,
        0xB => 1.0 / 3.0,
        0xC => 2.0 / 3.0,
        0xD => 0.75,
        _ => 0.0,
    };
    ((x >> 4) as f64 + frac) * 1000.0
}

fn quarter_ns(x: u8) -> f64 {
    ((x >> 2) as f64 + (x & 3) as f64 * 0.25) * 1000.0
}

fn legacy(b: &[u8], fam: Gen) -> Option<DecodedModule> {
    let name = match fam {
        Gen::Sdr => "SDRAM",
        Gen::Ddr => "DDR",
        Gen::Ddr2 => "DDR2",
    };
    let mut m = DecodedModule { mem_type: name.into(), ..Default::default() };
    let (rows, cols) = ((b[3] & 0x1F) as u32, (b[4] & 0xF) as u32);
    let ranks = if fam == Gen::Ddr2 { (b[5] & 7) as u64 + 1 } else { b[5].max(1) as u64 };
    let bus = if b[6] >= 64 { 64 } else { b[6] as u64 };
    m.banks = b[17] as u32;
    m.ranks = ranks as u32;
    m.size_mb = ((1u64 << (rows + cols).min(40)) * b[17] as u64 * bus / 8 * ranks) >> 20;
    if b[13] != 0 {
        let mbit = ((1u64 << (rows + cols).min(40)) * b[17] as u64 * b[13] as u64) >> 20;
        m.die_density = format!("{mbit} Mbit");
    }
    if fam == Gen::Ddr2 {
        m.registered = b[20] & 0x11 != 0;
        m.format = match b[20] {
            x if x & 1 != 0 => "RDIMM",
            x if x & 2 != 0 => "UDIMM",
            x if x & 4 != 0 => "SO-DIMM",
            x if x & 8 != 0 => "Micro-DIMM",
            x if x & 0x10 != 0 => "Mini-RDIMM",
            x if x & 0x20 != 0 => "Mini-UDIMM",
            _ => "Unknown",
        }
        .into();
    } else {
        m.registered = b[21] & 0x02 != 0;
        m.format = if m.registered { "Registered" } else { "Unbuffered" }.into();
    }
    m.voltage = Some(match fam {
        Gen::Sdr => 3.3,
        Gen::Ddr => 2.5,
        Gen::Ddr2 => 1.8,
    });

    // Supported CLs, highest first, each with its own tCK byte (9, 23, 25).
    let cl_of = |i: u32| match fam {
        Gen::Sdr => i as f64 + 1.0,
        Gen::Ddr => 1.0 + i as f64 * 0.5,
        Gen::Ddr2 => i as f64,
    };
    let mut cls: Vec<f64> = (0..8).rev().filter(|i| b[18] >> i & 1 == 1).map(cl_of).collect();
    cls.truncate(3);
    let q = |x: u8| if fam == Gen::Sdr { x as f64 * 1000.0 } else { quarter_ns(x) };
    let mut t = Core { taa: 0.0, trcd: q(b[29]), trp: q(b[27]), tras: b[30] as f64 * 1000.0, trc: 0.0 };
    t.trc = match fam {
        Gen::Sdr => t.tras + t.trp,
        Gen::Ddr => b[41] as f64 * 1000.0,
        Gen::Ddr2 => {
            let fr = [0.0, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 0.0, 0.0][((b[40] >> 4) & 7) as usize];
            (b[41] as f64 + fr) * 1000.0
        }
    };
    let mut rows_v = Vec::new();
    for (cl, at) in cls.iter().zip([9usize, 23, 25]) {
        let tck = tck_byte(b[at]);
        if tck > 0.0 {
            rows_v.push(row(String::new(), quantize(tck), *cl, &t));
        }
    }
    rows_v.reverse();
    number(&mut rows_v, "JEDEC");
    m.jedec = rows_v;
    let tck = tck_byte(b[9]);
    if tck <= 0.0 {
        return None;
    }

    (m.module_mfr, m.module_mfr_id) = mfr8(&b[64..72]);
    m.part = ascii(&b[73..91]);
    if b[93] | b[94] != 0 {
        (m.year, m.week) = (Some(bcd(b[93])), Some(bcd(b[94])));
    }
    m.serial = Some(b[95..99].iter().map(|x| format!("{x:02X}")).collect());
    let sum = b[..63].iter().fold(0u8, |a, &x| a.wrapping_add(x));
    m.crc_ok = Some(sum == b[63]);

    // NVIDIA EPP lives in the DDR2 manufacturer area: "NVm" + profile type.
    if fam == Gen::Ddr2 && &b[99..102] == b"NVm" {
        m.epp = Some(if b[102] == 0xB1 { "full".into() } else { "abbreviated".into() });
        // ponytail: EPP profile timings not decoded, add when an EPP dump is at hand.
    }
    finish(&mut m, name, quantize(tck), &[]);
    Some(m)
}

// ---------------------------------------------------------------- FB-DIMM

fn fbdimm(b: &[u8]) -> Option<DecodedModule> {
    // ponytail: identity, size and speed only; FB-DIMM timings table not decoded.
    let mut m = DecodedModule {
        mem_type: "DDR2 FB-DIMM".into(),
        format: "FB-DIMM".into(),
        registered: true,
        ..Default::default()
    };
    let dens_mb = 256u64 << ((b[4] >> 4) & 7);
    m.banks = 4 << (b[4] & 3);
    let width = 4u64 << (b[7] & 7);
    let ranks = ((b[7] >> 3) & 7) as u64 + 1;
    m.size_mb = dens_mb / 8 * (64 / width) * ranks;
    m.ranks = ranks as u32;
    m.die_density = format!("{dens_mb} Mbit");
    m.voltage = Some(1.8);
    let mtb = if b[10] != 0 { 1000.0 * b[9] as f64 / b[10] as f64 } else { 0.0 };
    let tck = b[11] as f64 * mtb;
    if tck <= 0.0 {
        return None;
    }
    (m.module_mfr, m.module_mfr_id) = mfr2(b[117], b[118]);
    (m.dram_mfr, m.dram_mfr_id) = mfr2(b[148], b[149]);
    if b[120] | b[121] != 0 {
        (m.year, m.week) = (Some(bcd(b[120])), Some(bcd(b[121])));
    }
    m.serial = Some(b[122..126].iter().map(|x| format!("{x:02X}")).collect());
    m.part = ascii(&b[128..146]);
    let end = if b[0] & 0x80 != 0 { 117 } else { 126 };
    m.crc_ok = Some(crc16(&b[..end]) == u16le(b, 126) as u16);
    finish(&mut m, "DDR2 FB-DIMM", quantize(tck), &[]);
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_crc(b: &mut [u8], lo: usize, hi: usize) {
        let c = crc16(&b[lo..hi]);
        b[hi] = c as u8;
        b[hi + 1] = (c >> 8) as u8;
    }

    #[test]
    fn crc_known_value() {
        assert_eq!(crc16(b"123456789"), 0x31C3);
    }

    #[test]
    fn ddr4_synthetic() {
        // DDR4-2400 17-17-17, 8 GB single rank x8 8Gb, Samsung module.
        let mut b = vec![0u8; 512];
        b[0] = 0x23;
        b[1] = 0x11;
        b[2] = 0x0C;
        b[3] = 0x02;
        b[4] = 0x85; // 8 Gb, 4 banks/BG, 4 BG
        b[6] = 0x00;
        b[12] = 0x01; // x8, 1 rank
        b[13] = 0x03; // 64-bit
        b[18] = 7; // 875 ps
        b[125] = (-42i8) as u8; // 833 ps
        b[19] = 13;
        b[20] = 0xFC; // CL 9..
        b[21] = 0xFF;
        b[22] = 0x03;
        b[24] = 0x6F; // 13.875 ns
        b[123] = 0; // tAA 13875
        b[25] = 0x6F;
        b[26] = 0x6F;
        b[27] = 0x11;
        b[28] = 0x00; // tRAS 256+0 = 256 MTB = 32 ns
        b[29] = 0x6F; // tRC (1<<8 | 0x6F) * 125 = 45.875 ns
        b[320] = 0x80;
        b[321] = 0xCE;
        b[323] = 0x19;
        b[324] = 0x12;
        b[329..337].copy_from_slice(b"M378A1K4");
        set_crc(&mut b, 0, 126);
        set_crc(&mut b, 128, 254);
        let m = decode(&b, &[]).unwrap();
        assert_eq!(m.mem_type, "DDR4");
        assert_eq!(m.format, "UDIMM");
        assert_eq!(m.size_mb, 8192);
        assert_eq!(m.banks, 16);
        assert_eq!(m.module_mfr, "Samsung");
        assert_eq!(m.max_bw, "DDR4-2400 (1200 MHz)");
        assert_eq!((m.week, m.year), (Some(12), Some(19)));
        assert_eq!(m.crc_ok, Some(true));
        let last = m.jedec.last().unwrap();
        assert_eq!((last.cl, last.rcd, ifloor(last.freq)), (17.0, 17, 1200));
        assert_eq!(m.jedec[0].cl, 9.0);
    }

    #[test]
    fn ddr3_synthetic() {
        // DDR3-1600 11-11-11, 4 GB, 2Gb x8 2 ranks.
        let mut b = vec![0u8; 256];
        b[0] = 0x92;
        b[2] = 0x0B;
        b[3] = 0x02;
        b[4] = 0x03;
        b[7] = 0x09;
        b[8] = 0x03;
        b[9] = 0x11;
        b[10] = 1;
        b[11] = 8;
        b[12] = 10; // 1.25 ns
        b[14] = 0xFC; // CL 6..11
        b[15] = 0x00;
        b[16] = 105; // 13.125 ns
        b[18] = 105;
        b[20] = 105;
        b[21] = 0x11;
        b[22] = 0x18; // tRAS 0x118 = 280 MTB = 35 ns
        b[23] = 0x89; // tRC 0x189 = 393 MTB = 49.125 ns
        b[117] = 0x01;
        b[118] = 0x98;
        let c = crc16(&b[..117]);
        (b[126], b[127]) = (c as u8, (c >> 8) as u8);
        let m = decode(&b, &[]).unwrap();
        assert_eq!(m.size_mb, 4096);
        assert_eq!(m.module_mfr, "Kingston");
        assert_eq!(m.max_bw, "PC3-12800 (800 MHz)");
        // 8 @ 13.125 ns -> 609 MHz, the classic CPU-Z DDR3 row.
        let r = m.jedec.iter().find(|r| r.cl == 8.0).unwrap();
        assert_eq!((ifloor(r.freq), r.rcd, r.ras, r.rc), (609, 8, 22, 30));
        assert_eq!(m.crc_ok, Some(true));
    }

    #[test]
    fn ddr2_and_sdr_synthetic() {
        let mut b = vec![0u8; 128];
        b[2] = 0x08;
        b[3] = 14;
        b[4] = 10;
        b[5] = 0x01; // 2 ranks
        b[6] = 64;
        b[9] = 0x25; // 2.5 ns
        b[13] = 8;
        b[17] = 8;
        b[18] = 0x38; // CL 3,4,5
        b[20] = 0x02;
        b[23] = 0x30;
        b[25] = 0x3D;
        b[27] = 0x3C;
        b[29] = 0x3C;
        b[30] = 45;
        b[41] = 60;
        b[63] = b[..63].iter().fold(0u8, |a, &x| a.wrapping_add(x));
        let m = decode(&b, &[]).unwrap();
        assert_eq!(m.size_mb, 2048);
        assert_eq!(m.max_bw, "PC2-6400 (400 MHz)");
        assert_eq!(m.jedec.len(), 3);
        assert_eq!(m.jedec[2].cl, 5.0);
        assert_eq!(m.crc_ok, Some(true));

        let mut s = vec![0u8; 128];
        s[2] = 0x04;
        s[3] = 12;
        s[4] = 10;
        s[5] = 1;
        s[6] = 64;
        s[9] = 0x75;
        s[17] = 4;
        s[18] = 0x06;
        s[64] = 0x2C;
        let m = decode(&s, &[]).unwrap();
        assert_eq!(m.size_mb, 128);
        assert_eq!(m.max_bw, "PC133 (133 MHz)");
        assert_eq!(m.module_mfr, "Micron Technology");
    }
}

//! LPC Super I/O detection and hardware-monitor readout over port I/O.
//! Register facts are from the public Nuvoton/Winbond, ITE and Fintek
//! datasheets. Writes are limited to: the vendor's config-mode key/exit
//! sequences, the LDN select register (0x07), and the index/bank/page
//! registers the HWM blocks need for *reading* (bank is restored to 0).

use super::{Kind, Monitor};
use crate::hw::Hw;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Family {
    /// NCT6683/6686/6687: EC-style page/index/data at base+4/5/6.
    NctEc,
    /// NCT6779 and NCT679x: banked ISA port, 0x480 voltages, RPM registers.
    NctNew,
    /// NCT6775/6776 and Winbond W836xx: banked ISA port, 0x20 voltages.
    /// `rpm13` = NCT6776 13-bit fan counts, else 8-bit counts with divisors.
    NctOld {
        rpm13: bool,
    },
    /// ITE IT87xx/IT86xx: ADC step in volts, number of temperature inputs.
    Ite {
        lsb: f64,
        temps: u8,
    },
    Fintek,
}

#[derive(Debug, Clone)]
pub struct Chip {
    pub vendor: &'static str,
    pub model: &'static str,
    pub family: Family,
    pub vendor_id: u16,
    pub chip_id: u8,
    pub rev: u8,
    pub port: u16,
    pub ldn: u8,
    /// Config registers 0x00-0x7F with the HWM logical device selected.
    pub regs: Vec<u8>,
    pub base: u16,
}

type Id = (u16, u16, &'static str, &'static str, Family); // (id, mask, vendor, model, family)

const WINBOND_NUVOTON: &[Id] = &[
    (0x8850, 0xFFF0, "Winbond", "W83627EHF", Family::NctOld { rpm13: false }),
    (0x8860, 0xFFF0, "Winbond", "W83627EHG", Family::NctOld { rpm13: false }),
    (0xA020, 0xFFF0, "Winbond", "W83627DHG", Family::NctOld { rpm13: false }),
    (0xB070, 0xFFF0, "Winbond", "W83627DHG-P", Family::NctOld { rpm13: false }),
    (0xA230, 0xFFF0, "Winbond", "W83627UHG", Family::NctOld { rpm13: false }),
    (0xA510, 0xFFF0, "Winbond", "W83667HG", Family::NctOld { rpm13: false }),
    (0xB350, 0xFFF0, "Winbond", "W83667HG-B", Family::NctOld { rpm13: false }),
    (0xB470, 0xFFF0, "Nuvoton", "NCT6775", Family::NctOld { rpm13: false }),
    (0xC330, 0xFFF0, "Nuvoton", "NCT6776", Family::NctOld { rpm13: true }),
    (0xC560, 0xFFF0, "Nuvoton", "NCT6779", Family::NctNew),
    (0xC800, 0xFFF0, "Nuvoton", "NCT6791", Family::NctNew),
    (0xC910, 0xFFF0, "Nuvoton", "NCT6792", Family::NctNew),
    (0xD120, 0xFFF0, "Nuvoton", "NCT6793", Family::NctNew),
    (0xD350, 0xFFF0, "Nuvoton", "NCT6795", Family::NctNew),
    (0xD420, 0xFFF8, "Nuvoton", "NCT6796", Family::NctNew),
    (0xD428, 0xFFF8, "Nuvoton", "NCT6798", Family::NctNew),
    (0xD450, 0xFFF0, "Nuvoton", "NCT6797", Family::NctNew),
    // NCT6701D (ASUS 800-series AM5) shares the NCT6799's register map.
    (0xD806, 0xFFFF, "Nuvoton", "NCT6701", Family::NctNew),
    (0xD800, 0xFFF0, "Nuvoton", "NCT6799", Family::NctNew),
    (0xC730, 0xFFF0, "Nuvoton", "NCT6683", Family::NctEc),
    (0xD440, 0xFFF0, "Nuvoton", "NCT6686", Family::NctEc),
    (0xD590, 0xFFF0, "Nuvoton", "NCT6687", Family::NctEc),
];

const FINTEK: &[Id] = &[
    (0x0507, 0xFFFF, "Fintek", "F71858", Family::Fintek),
    (0x0601, 0xFFFF, "Fintek", "F71862", Family::Fintek),
    (0x0814, 0xFFFF, "Fintek", "F71869", Family::Fintek),
    (0x1007, 0xFFFF, "Fintek", "F71869A", Family::Fintek),
    (0x1106, 0xFFFF, "Fintek", "F71868A", Family::Fintek),
    (0x0541, 0xFFFF, "Fintek", "F71882", Family::Fintek),
    (0x0723, 0xFFFF, "Fintek", "F71889", Family::Fintek),
    (0x0909, 0xFFFF, "Fintek", "F71889E", Family::Fintek),
    (0x1005, 0xFFFF, "Fintek", "F71889A", Family::Fintek),
    (0x0901, 0xFFFF, "Fintek", "F71808E", Family::Fintek),
];

// ADC LSB: 16 mV on the original IT87 parts, 12 mV from IT8721 on and on the
// IT8686/8688/8689/8696 Gigabyte parts, 11 mV on the IT8625 and the IT879x
// second chips, 10.9 mV on IT8655/8665 (the values board readings agree with,
// as used by open-source monitors). ponytail: per-model, not per-stepping;
// the board map's multiplier is the calibration knob when a board disagrees.
const OLD: Family = Family::Ite { lsb: 0.016, temps: 3 };
const MV12: Family = Family::Ite { lsb: 0.012, temps: 3 };
const MV12_6: Family = Family::Ite { lsb: 0.012, temps: 6 };
const MV11: Family = Family::Ite { lsb: 0.011, temps: 3 };
const MV11_6: Family = Family::Ite { lsb: 0.011, temps: 6 };
const MV10_6: Family = Family::Ite { lsb: 0.0109, temps: 6 };
const ITE: &[Id] = &[
    (0x8620, 0xFFFF, "ITE", "IT8620", MV12_6),
    (0x8625, 0xFFFF, "ITE", "IT8625", MV11_6),
    (0x8628, 0xFFFF, "ITE", "IT8628", MV12_6),
    (0x8655, 0xFFFF, "ITE", "IT8655", MV10_6),
    (0x8665, 0xFFFF, "ITE", "IT8665", MV10_6),
    (0x8686, 0xFFFF, "ITE", "IT8686", MV12_6),
    (0x8688, 0xFFFF, "ITE", "IT8688", MV12_6),
    (0x8689, 0xFFFF, "ITE", "IT8689", MV12_6),
    (0x8696, 0xFFFF, "ITE", "IT8696", MV12_6),
    (0x8695, 0xFFFF, "ITE", "IT87952", MV11),
    (0x8705, 0xFFFF, "ITE", "IT8705", OLD),
    (0x8712, 0xFFFF, "ITE", "IT8712", OLD),
    (0x8716, 0xFFFF, "ITE", "IT8716", OLD),
    (0x8718, 0xFFFF, "ITE", "IT8718", OLD),
    (0x8720, 0xFFFF, "ITE", "IT8720", OLD),
    (0x8726, 0xFFFF, "ITE", "IT8726", OLD),
    (0x8721, 0xFFFF, "ITE", "IT8721", MV12),
    (0x8728, 0xFFFF, "ITE", "IT8728", MV12),
    (0x8771, 0xFFFF, "ITE", "IT8771", MV12),
    (0x8772, 0xFFFF, "ITE", "IT8772", MV12),
    (0x8790, 0xFFFF, "ITE", "IT8790", MV12),
    (0x8733, 0xFFFF, "ITE", "IT8792", MV11),
    (0x8792, 0xFFFF, "ITE", "IT8792", MV11),
];

fn lookup(t: &[Id], id: u16) -> Option<&Id> {
    t.iter().find(|e| id & e.1 == e.0)
}

pub fn is_superio_name(name: &str) -> bool {
    ["Nuvoton ", "Winbond ", "ITE ", "Fintek "].iter().any(|p| name.starts_with(p))
}

// ---- config mode ----

fn cfg_rd(hw: &dyn Hw, port: u16, reg: u8) -> u8 {
    hw.io_out8(port, reg);
    hw.io_in8(port + 1).unwrap_or(0xFF)
}

fn cfg_rd16(hw: &dyn Hw, port: u16, reg: u8) -> u16 {
    (cfg_rd(hw, port, reg) as u16) << 8 | cfg_rd(hw, port, reg + 1) as u16
}

fn cfg_wr(hw: &dyn Hw, port: u16, reg: u8, val: u8) {
    hw.io_out8(port, reg);
    hw.io_out8(port + 1, val);
}

/// Super I/O chips at the two standard config ports.
pub fn detect(hw: &dyn Hw) -> Vec<Chip> {
    [0x2Eu16, 0x4E].into_iter().filter_map(|p| probe(hw, p)).collect()
}

fn probe(hw: &dyn Hw, port: u16) -> Option<Chip> {
    hw.io_in8(port)?; // backend without port I/O: don't even try
    // Winbond / Nuvoton / Fintek: key 0x87 0x87, exit 0xAA.
    hw.io_out8(port, 0x87);
    hw.io_out8(port, 0x87);
    let id = cfg_rd16(hw, port, 0x20);
    let hit = if cfg_rd16(hw, port, 0x23) == 0x1934 {
        lookup(FINTEK, id).map(|e| (e, 0x04, 0x1934))
    } else {
        lookup(WINBOND_NUVOTON, id).map(|e| (e, 0x0B, 0x5CA3))
    };
    let chip = hit.map(|(e, ldn, vid)| finish(hw, port, e, ldn, vid));
    hw.io_out8(port, 0xAA);
    if chip.is_some() {
        return chip;
    }
    // ITE: key 0x87 0x01 0x55 0x55 (0x2E) / 0xAA (0x4E), exit = CR02 bit 1.
    for b in [0x87, 0x01, 0x55, if port == 0x2E { 0x55 } else { 0xAA }] {
        hw.io_out8(port, b);
    }
    let id = cfg_rd16(hw, port, 0x20);
    let chip = lookup(ITE, id).map(|e| {
        let mut c = finish(hw, port, e, 0x04, 0x90);
        c.rev = c.regs[0x22] & 0x0F;
        c
    });
    cfg_wr(hw, port, 0x02, 0x02);
    chip
}

fn finish(hw: &dyn Hw, port: u16, e: &Id, ldn: u8, vendor_id: u16) -> Chip {
    cfg_wr(hw, port, 0x07, ldn);
    let regs: Vec<u8> = (0..0x80).map(|r| cfg_rd(hw, port, r)).collect();
    Chip {
        vendor: e.2,
        model: e.3,
        family: e.4,
        vendor_id,
        chip_id: regs[0x20],
        rev: regs[0x21],
        port,
        ldn,
        base: ((regs[0x60] as u16) << 8 | regs[0x61] as u16) & 0xFFF8,
        regs,
    }
}

// ---- hardware monitor blocks ----

struct Io<'a> {
    hw: &'a dyn Hw,
    base: u16,
}

impl Io<'_> {
    fn isa(&self, reg: u8) -> u8 {
        self.hw.io_out8(self.base + 5, reg);
        self.hw.io_in8(self.base + 6).unwrap_or(0xFF)
    }
    /// Winbond/Nuvoton banked register: bank in 0x4E, then the low byte.
    fn bank(&self, reg: u16) -> u8 {
        self.hw.io_out8(self.base + 5, 0x4E);
        self.hw.io_out8(self.base + 6, (reg >> 8) as u8);
        self.isa(reg as u8)
    }
    fn ec(&self, reg: u16) -> u8 {
        self.hw.io_out8(self.base + 4, 0xFF);
        self.hw.io_out8(self.base + 4, (reg >> 8) as u8);
        self.hw.io_out8(self.base + 5, reg as u8);
        self.hw.io_in8(self.base + 6).unwrap_or(0xFF)
    }
}

/// Values straight off the chip, before any board knowledge:
/// (input, value, raw code). Volts are at the pin (ADC count × LSB), so a
/// board divider or the chip's internal /2 is still to be applied.
#[derive(Default)]
struct Raw {
    volts: Vec<(u8, f64, u32)>,
    temps: Vec<(u8, f64, u32)>,
    fans: Vec<(u8, f64, u32)>,
}

impl Raw {
    fn volt(&mut self, i: u8, v: f64, raw: u32) {
        self.volts.push((i, v, raw));
    }
    fn temp(&mut self, i: u8, t: f64, raw: u32) {
        if (1.0..=125.0).contains(&t) {
            self.temps.push((i, t, raw));
        }
    }
    fn fan(&mut self, i: u8, rpm: f64, raw: u32) {
        if (50.0..=20000.0).contains(&rpm) {
            self.fans.push((i, rpm, raw));
        }
    }
}

pub fn read(hw: &dyn Hw, c: &Chip, board_vendor: &str, board_model: &str) -> Option<Monitor> {
    if c.base == 0 || c.base == 0xFFF8 {
        return None;
    }
    let io = Io { hw, base: c.base };
    let name = format!("{} {}", c.vendor, c.model);
    let mut raw = Raw::default();
    let mut regs = Vec::new();
    let generic = match c.family {
        Family::NctEc => {
            nct_ec(&io, &mut raw, &mut regs);
            &EC_GENERIC
        }
        Family::NctNew => {
            nct_new(&io, &mut raw);
            io.bank(0); // leave bank 0 selected
            &NCT_GENERIC
        }
        Family::NctOld { rpm13 } => {
            nct_old(&io, rpm13, &mut raw);
            io.bank(0);
            &NCT_OLD_GENERIC
        }
        Family::Ite { lsb, temps } => {
            ite(&io, lsb, temps, &mut raw);
            if lsb < 0.016 { &ITE_GENERIC } else { &ITE_OLD_GENERIC }
        }
        Family::Fintek => {
            let mut m = Monitor::new(name);
            fintek(&io, c.model == "F71858", &mut m);
            return Some(m);
        }
    };
    let board = board_map(board_vendor, board_model, c.model);
    let show_all = std::env::var_os("CPUZEEZ_EC_DUMP").is_some();
    let mut m = apply(name, &raw, generic, board, show_all);
    m.registers = regs;
    Some(m)
}

/// Label, scale and filter raw inputs with the board's map (falling back to
/// the chip's generic list per kind). `show_all` (the contributor dump) also
/// lists inputs the map hides, under their datasheet names.
fn apply(name: String, raw: &Raw, generic: &Map, board: Option<&Map>, show_all: bool) -> Monitor {
    let mut m = Monitor::new(name);
    let pick = |b: fn(&Map) -> &'static [V]| board.map(b).filter(|l| !l.is_empty()).unwrap_or(b(generic));
    let kinds: [(Kind, &[(u8, f64, u32)], &'static [V], &'static [V]); 3] = [
        (Kind::Voltage, &raw.volts, pick(|m| m.volts), generic.volts),
        (Kind::Temperature, &raw.temps, pick(|m| m.temps), generic.temps),
        (Kind::Fan, &raw.fans, pick(|m| m.fans), generic.fans),
    ];
    for (kind, vals, list, gen_list) in kinds {
        let get = |i: u8| vals.iter().find(|r| r.0 == i);
        for e in list {
            if let Some(&(_, v, code)) = get(e.input) {
                m.push_ch(kind, e.row as usize, e.label, v * e.k, code);
            }
        }
        if show_all {
            for &(i, v, code) in vals.iter().filter(|r| !list.iter().any(|e| e.input == r.0)) {
                let g = gen_list.iter().find(|e| e.input == i);
                let label = format!("{} (unmapped)", g.map_or("", |g| g.label));
                m.push_ch(kind, i as usize, label, v * g.map_or(1.0, |g| g.k), code);
            }
        }
    }
    m
}

fn nct_ec(io: &Io, r: &mut Raw, regs: &mut Vec<crate::report::Node>) {
    let w = |reg: u16| (io.ec(reg) as u32) << 8 | io.ec(reg + 1) as u32;
    if std::env::var_os("CPUZEEZ_EC_DUMP").is_some() {
        regs.push(crate::report::hex("EC 0x100", (0x100..0x200).map(|reg| io.ec(reg)).collect()));
    }
    // 16 voltage words at 0x120, value in mV << 4.
    for i in 0..16u8 {
        let raw = w(0x120 + 2 * i as u16);
        if raw != 0 && raw != 0xFFFF {
            r.volt(i, (raw >> 4) as f64 / 1000.0, raw);
        }
    }
    for i in 0..8u16 {
        let hi = io.ec(0x100 + 2 * i) as i8;
        let half = (io.ec(0x101 + 2 * i) >> 7) as f64 * 0.5;
        r.temp(i as u8, hi as f64 + half, hi as u8 as u32);
    }
    // 16 tachometer words at 0x140, numbered from 1 ("Fan 15", "Fan 16" for the last two).
    for i in 0..16u16 {
        let rpm = w(0x140 + 2 * i);
        if rpm != 0xFFFF {
            r.fan(i as u8 + 1, rpm as f64, rpm);
        }
    }
}

fn nct_volt(r: &mut Raw, i: u8, raw: u8) {
    if raw != 0 && raw != 0xFF {
        r.volt(i, raw as f64 * 0.008, raw as u32);
    }
}

fn nct_new(io: &Io, r: &mut Raw) {
    for i in 0..15u8 {
        nct_volt(r, i, io.bank(0x480 + i as u16));
    }
    // SYSTIN, CPUTIN, AUXTIN0-3 readings.
    for i in 0..6u8 {
        let t = io.bank(0x490 + i as u16);
        r.temp(i, t as i8 as f64, t as u32);
    }
    for (i, reg) in [0x4C0u16, 0x4C2, 0x4C4, 0x4C6, 0x4C8, 0x4CA, 0x4CE].into_iter().enumerate() {
        let rpm = (io.bank(reg) as u32) << 8 | io.bank(reg + 1) as u32;
        r.fan(i as u8, rpm as f64, rpm);
    }
}

fn nct_old(io: &Io, rpm13: bool, r: &mut Raw) {
    for (i, reg) in [0x20u16, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x550, 0x551].into_iter().enumerate() {
        nct_volt(r, i as u8, io.bank(reg));
    }
    let sys = io.bank(0x027);
    r.temp(0, sys as i8 as f64, sys as u32);
    for (j, reg) in [0x150u16, 0x250].into_iter().enumerate() {
        let hi = io.bank(reg);
        r.temp(j as u8 + 1, hi as i8 as f64 + (io.bank(reg + 1) >> 7) as f64 * 0.5, hi as u32);
    }
    if rpm13 {
        for i in 0..5u16 {
            let count = (io.bank(0x630 + 2 * i) as u32) << 5 | (io.bank(0x631 + 2 * i) & 0x1F) as u32;
            if count != 0 && count != 0x1FFF {
                r.fan(i as u8, 1_350_000.0 / count as f64, count);
            }
        }
    } else {
        // 8-bit counts; divisor = 2^(2 bits in 0x47/0x4B + 1 bit in 0x5D).
        let (r47, r4b, r5d) = (io.bank(0x47), io.bank(0x4B), io.bank(0x5D));
        let div = [
            (r47 >> 4) & 3 | ((r5d >> 5) & 1) << 2,
            (r47 >> 6) & 3 | ((r5d >> 6) & 1) << 2,
            (r4b >> 6) & 3 | ((r5d >> 7) & 1) << 2,
        ];
        for (i, reg) in [0x28u16, 0x29, 0x2A].into_iter().enumerate() {
            let count = io.bank(reg) as u32;
            if count != 0 && count != 0xFF {
                r.fan(i as u8, 1_350_000.0 / (count << div[i]) as f64, count);
            }
        }
    }
}

fn ite(io: &Io, lsb: f64, temps: u8, r: &mut Raw) {
    for i in 0..9u8 {
        let raw = io.isa(0x20 + i);
        if raw != 0 && raw != 0xFF {
            r.volt(i, raw as f64 * lsb, raw as u32);
        }
    }
    for i in 0..temps {
        let t = io.isa(0x29 + i);
        r.temp(i, t as i8 as f64, t as u32);
    }
    for (i, (lo, hi)) in
        [(0x0D, 0x18), (0x0E, 0x19), (0x0F, 0x1A), (0x80, 0x81), (0x82, 0x83), (0x93, 0x94)].into_iter().enumerate()
    {
        let count = io.isa(lo) as u32 | (io.isa(hi) as u32) << 8;
        if count != 0 && count != 0xFFFF {
            r.fan(i as u8, 1_350_000.0 / (2 * count) as f64, count);
        }
    }
}

fn fintek(io: &Io, f71858: bool, m: &mut Monitor) {
    // (register, label, internal divider)
    let volts: &[(u8, &str, f64)] = if f71858 {
        &[(0x20, "+3.3V", 2.0), (0x21, "+3.3V Standby", 2.0), (0x22, "VBAT", 2.0)]
    } else {
        &[
            (0x20, "+3.3V", 2.0),
            (0x21, "CPU", 1.0),
            (0x22, "", 1.0),
            (0x23, "", 1.0),
            (0x24, "", 1.0),
            (0x25, "", 1.0),
            (0x26, "", 1.0),
            (0x27, "+3.3V Standby", 2.0),
            (0x28, "VBAT", 2.0),
        ]
    };
    for &(r, l, k) in volts {
        let raw = io.isa(r);
        if raw != 0 && raw != 0xFF {
            m.push_ch(Kind::Voltage, (r - 0x20) as usize, l, raw as f64 * 0.008 * k, raw as u32);
        }
    }
    if !f71858 {
        for i in 0..3u8 {
            let t = io.isa(0x72 + 2 * i);
            if (1.0..=125.0).contains(&(t as i8 as f64)) {
                m.push_ch(Kind::Temperature, i as usize, format!("Temperature {}", i + 1), t as i8 as f64, t as u32);
            }
        }
    }
    for i in 0..4u8 {
        let count = (io.isa(0xA0 + 16 * i) as u32) << 8 | io.isa(0xA1 + 16 * i) as u32;
        let rpm = 1_500_000.0 / count as f64;
        if count != 0 && count != 0xFFFF && (50.0..=20000.0).contains(&rpm) {
            m.push_ch(Kind::Fan, i as usize, format!("Fan {}", i + 1), rpm, count);
        }
    }
}

// ---- board sensor maps ----
//
// Which header/rail each chip input is wired to is a board design choice, so
// it lives in data: one `Board` per (SMBIOS vendor, model pattern, chip). Facts
// come from boards checked against vendor tools/BIOS readings, this project's
// own machines, and open-source monitors' published board tables (re-stated
// here, not copied). docs/boards.md explains how to contribute one.

/// One input: chip input number, report row ("Voltage 6"), label, and the
/// multiplier applied to the pin value. For voltages `k` folds in both the
/// board's resistor divider (1 + Ri/Rf) and the chip's internal /2 on
/// AVCC/3VCC/3VSB/VBAT-style inputs. Temperatures and fans use k = 1.
#[derive(Debug, Clone, Copy)]
pub struct V {
    pub input: u8,
    pub row: u8,
    pub label: &'static str,
    pub k: f64,
}

const fn v(input: u8, label: &'static str, k: f64) -> V {
    V { input, row: input, label, k }
}

/// Voltage reported on a different row than its input (CPU-Z's numbering).
const fn vr(input: u8, row: u8, label: &'static str, k: f64) -> V {
    V { input, row, label, k }
}

/// Temperature or fan input.
const fn t(input: u8, label: &'static str) -> V {
    V { input, row: input, label, k: 1.0 }
}

/// Per-kind input lists. In a board map an empty list keeps the chip's
/// generic list for that kind; a non-empty one shows only what it names
/// (anything unlisted is hidden).
#[derive(Debug, Clone, Copy)]
pub struct Map {
    pub volts: &'static [V],
    pub temps: &'static [V],
    pub fans: &'static [V],
}

pub struct Board {
    /// Normalized substring of the SMBIOS board vendor ("ASUS", "MICROSTAR", ...).
    pub vendor: &'static str,
    /// Model patterns: normalized substring of the SMBIOS board product; "" = any.
    pub models: &'static [&'static str],
    /// Chip models (`Chip::model`) this wiring applies to.
    pub chips: &'static [&'static str],
    pub map: Map,
}

/// Upper-case alphanumerics only: "PRO X870-P WIFI (MS-7E47)" → "PROX870PWIFIMS7E47".
fn norm(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect()
}

/// First board entry matching this machine; entries are ordered specific first.
pub fn board_map(vendor: &str, model: &str, chip: &str) -> Option<&'static Map> {
    let (nv, nm) = (norm(vendor), norm(model));
    let vendor_ok = |k: &str| nv.contains(k) || (k == "MICROSTAR" && nv == "MSI");
    BOARDS
        .iter()
        .find(|b| vendor_ok(b.vendor) && b.chips.contains(&chip) && b.models.iter().any(|p| nm.contains(&norm(p))))
        .map(|b| &b.map)
}

// Generic fallbacks: datasheet input names, internal dividers only.

const NCT_GENERIC: Map = Map {
    volts: &[
        v(0, "CPUVCORE", 1.0),
        v(1, "VIN1", 1.0),
        v(2, "AVCC", 2.0),
        v(3, "3VCC", 2.0),
        v(4, "VIN0", 1.0),
        v(5, "VIN8", 1.0),
        v(6, "VIN4", 1.0),
        v(7, "3VSB", 2.0),
        v(8, "VBAT", 2.0),
        v(9, "VTT", 1.0),
        v(10, "VIN5", 1.0),
        v(11, "VIN6", 1.0),
        v(12, "VIN2", 1.0),
        v(13, "VIN3", 1.0),
        v(14, "VIN7", 1.0),
    ],
    temps: &[t(0, "SYSTIN"), t(1, "CPUTIN"), t(2, "AUXTIN0"), t(3, "AUXTIN1"), t(4, "AUXTIN2"), t(5, "AUXTIN3")],
    fans: &[
        t(0, "SYSFAN"),
        t(1, "CPUFAN"),
        t(2, "AUXFAN0"),
        t(3, "AUXFAN1"),
        t(4, "AUXFAN2"),
        t(5, "AUXFAN3"),
        t(6, "AUXFAN4"),
    ],
};

const NCT_OLD_GENERIC: Map = Map {
    volts: &[
        v(0, "CPUVCORE", 1.0),
        v(1, "VIN1", 1.0),
        v(2, "AVCC", 2.0),
        v(3, "3VCC", 2.0),
        v(4, "VIN0", 1.0),
        v(5, "VIN2", 1.0),
        v(6, "VIN3", 1.0),
        v(7, "3VSB", 2.0),
        v(8, "VBAT", 2.0),
    ],
    temps: &[t(0, "SYSTIN"), t(1, "CPUTIN"), t(2, "AUXTIN")],
    fans: &[t(0, "SYSFAN"), t(1, "CPUFAN"), t(2, "AUXFAN0"), t(3, "AUXFAN1"), t(4, "AUXFAN2")],
};

const ITE_TEMPS: &[V] =
    &[t(0, "TMPIN1"), t(1, "TMPIN2"), t(2, "TMPIN3"), t(3, "TMPIN4"), t(4, "TMPIN5"), t(5, "TMPIN6")];
const ITE_FANS: &[V] = &[t(0, "FAN1"), t(1, "FAN2"), t(2, "FAN3"), t(3, "FAN4"), t(4, "FAN5"), t(5, "FAN6")];

/// 12/11/10.9 mV parts divide 3VSB and VBAT by two internally.
const ITE_GENERIC: Map = Map {
    volts: &[
        v(0, "VIN0", 1.0),
        v(1, "VIN1", 1.0),
        v(2, "VIN2", 1.0),
        v(3, "VIN3", 1.0),
        v(4, "VIN4", 1.0),
        v(5, "VIN5", 1.0),
        v(6, "VIN6", 1.0),
        v(7, "3VSB", 2.0),
        v(8, "VBAT", 2.0),
    ],
    temps: ITE_TEMPS,
    fans: ITE_FANS,
};

const ITE_OLD_GENERIC: Map = Map {
    volts: &[
        v(0, "VIN0", 1.0),
        v(1, "VIN1", 1.0),
        v(2, "VIN2", 1.0),
        v(3, "VIN3", 1.0),
        v(4, "VIN4", 1.0),
        v(5, "VIN5", 1.0),
        v(6, "VIN6", 1.0),
        v(7, "VIN7", 1.0),
        v(8, "VBAT", 1.0),
    ],
    temps: ITE_TEMPS,
    fans: ITE_FANS,
};

/// NCT6683/6686/6687: the EC already scales most channels; what each one is
/// depends on the board firmware, so generic names are just numbers.
const EC_GENERIC: Map = Map {
    volts: &[
        v(0, "VIN0", 1.0),
        v(1, "VIN1", 1.0),
        v(2, "VIN2", 1.0),
        v(3, "VIN3", 1.0),
        v(4, "VIN4", 1.0),
        v(5, "VIN5", 1.0),
        v(6, "VIN6", 1.0),
        v(7, "VIN7", 1.0),
        v(8, "VIN8", 1.0),
        v(9, "VIN9", 1.0),
        v(10, "VIN10", 1.0),
        v(11, "VIN11", 1.0),
        v(12, "VIN12", 1.0),
        v(13, "VIN13", 1.0),
        v(14, "VIN14", 1.0),
        v(15, "VIN15", 1.0),
    ],
    temps: &[
        t(0, "TEMP0"),
        t(1, "TEMP1"),
        t(2, "TEMP2"),
        t(3, "TEMP3"),
        t(4, "TEMP4"),
        t(5, "TEMP5"),
        t(6, "TEMP6"),
        t(7, "TEMP7"),
    ],
    fans: &[
        t(1, "FAN1"),
        t(2, "FAN2"),
        t(3, "FAN3"),
        t(4, "FAN4"),
        t(5, "FAN5"),
        t(6, "FAN6"),
        t(7, "FAN7"),
        t(8, "FAN8"),
        t(9, "FAN9"),
        t(10, "FAN10"),
        t(11, "FAN11"),
        t(12, "FAN12"),
        t(13, "FAN13"),
        t(14, "FAN14"),
        t(15, "FAN15"),
        t(16, "FAN16"),
    ],
};

// Divider shorthands (1 + Ri/Rf).
const X2: f64 = 2.0;
const X5: f64 = 5.0;
const X6: f64 = 6.0;
const X12: f64 = 12.0;

// ---- MSI, NCT6687 (EC) ----

// Inputs are EC words 0x120 + 2n. MSI's EC already scales +3.3V/3VSB/AVSB/VBAT.
// MSI PRO X870-P WIFI: checked against this project's own board and CPU-Z's
// labels, including CPU-Z's row numbers; the other inputs are rails CPU-Z
// doesn't show there.
const MSI_X870P: Map = Map {
    volts: &[
        vr(0, 0, "+12V", X12),
        vr(1, 1, "+5V", X5),
        vr(2, 2, "CPU NB/SoC", 1.0),
        vr(4, 3, "CPU", 1.0),
        vr(8, 6, "+3.3V", 1.0),
        vr(3, 9, "CPU VDDIO", X2),
    ],
    temps: &[t(0, "CPU Core"), t(1, "System"), t(2, "MOS"), t(3, "Chipset A")],
    fans: MSI_800_FANS,
};

/// MSI 800-series (AM5 and LGA1851) tach layout: SYS fans count down from 0x15E.
const MSI_800_FANS: &[V] = &[
    t(1, "CPU"),
    t(2, "PUMP"),
    t(3, "Chipset"),
    t(4, "EZ-Conn"),
    t(10, "SYS 7"),
    t(11, "SYS 6"),
    t(12, "SYS 5"),
    t(13, "SYS 4"),
    t(14, "SYS 3"),
    t(15, "SYS 2"),
    t(16, "SYS 1"),
];

/// Older MSI tach layout (500/600/700 series).
const MSI_FANS: &[V] = &[
    t(1, "CPU"),
    t(2, "PUMP"),
    t(3, "SYS 1"),
    t(4, "SYS 2"),
    t(5, "SYS 3"),
    t(6, "SYS 4"),
    t(7, "SYS 5"),
    t(8, "SYS 6"),
];

const MSI_EC_TEMPS_AMD: &[V] =
    &[t(0, "CPU"), t(1, "System"), t(2, "VRM MOS"), t(3, "Chipset"), t(4, "CPU Socket"), t(5, "PCIe x1"), t(6, "M.2")];

const MSI_EC_TEMPS_INTEL: &[V] =
    &[t(0, "CPU"), t(1, "System"), t(2, "VRM MOS"), t(3, "PCH"), t(4, "CPU Socket"), t(5, "PCIe x1"), t(6, "M.2")];

const MSI_EC_VOLTS_AMD: &[V] = &[
    v(0, "+12V", X12),
    v(1, "+5V", X5),
    v(2, "CPU NB/SoC", 1.0),
    v(3, "CPU VDDIO", X2),
    v(4, "CPU", 1.0),
    v(5, "Chipset", 1.0),
    v(8, "+3.3V", 1.0),
    v(13, "VTT", 1.0),
    v(15, "+1.8V", 1.0),
    v(11, "+3.3V Standby", 1.0),
    v(12, "AVSB", 1.0),
    v(14, "VBAT", 1.0),
];

const MSI_EC_VOLTS_INTEL: &[V] = &[
    v(0, "+12V", X12),
    v(1, "+5V", X5),
    v(2, "CPU", 1.0),
    v(4, "DRAM", X2),
    v(5, "CPU VCCIO", 1.0),
    v(6, "CPU SA", 1.0),
    v(8, "+3.3V", 1.0),
    v(13, "VTT", 1.0),
    v(11, "+3.3V Standby", 1.0),
    v(12, "AVSB", 1.0),
    v(14, "VBAT", 1.0),
];

const MSI_EC_800_INTEL_VOLTS: &[V] = &[
    v(0, "+12V", X12),
    v(1, "+5V", X5),
    v(2, "CPU", 1.0),
    v(4, "DRAM", X2),
    v(5, "Chipset", 1.0),
    v(8, "+3.3V", 1.0),
    v(13, "VTT", 1.0),
    v(15, "+1.8V", 1.0),
    v(11, "+3.3V Standby", 1.0),
    v(12, "AVSB", 1.0),
    v(14, "VBAT", 1.0),
];

// ---- MSI, NCT679x ----

const MSI_NCT_FANS: &[V] =
    &[t(0, "PUMP"), t(1, "CPU"), t(2, "SYS 1"), t(3, "SYS 2"), t(4, "SYS 3"), t(5, "SYS 4"), t(6, "Chipset")];

const MSI_X570_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+5V", X5),
    v(2, "AVCC", X2),
    v(3, "+3.3V", X2),
    v(4, "+12V", X12),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(9, "VTT", 1.0),
    v(12, "CPU NB/SoC", 1.0),
    v(13, "DRAM", X2),
];

// ---- Gigabyte, ITE ----

const GB_AM4_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+3.3V", 1.65),
    v(2, "+12V", X6),
    v(3, "+5V", 2.5),
    v(4, "CPU SoC", 1.0),
    v(5, "CPU VDDP", 1.0),
    v(6, "DRAM", 1.0),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
];

const GB_AM5_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+3.3V", 1.649),
    v(2, "+12V", X6),
    v(3, "+5V", 2.5),
    v(4, "CPU SoC", 1.0),
    v(5, "CPU Misc", 1.0),
    v(6, "DDR5 5V", 2.5),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
];

const GB_AM5_HI_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+3.3V", 1.649),
    v(2, "+12V", X6),
    v(3, "+5V", 2.5),
    v(4, "CPU SoC", 1.0),
    v(5, "CPU Misc", 1.0),
    v(6, "CPU VDDIO", 1.0),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
];

const GB_INTEL_DDR5_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+3.3V", 1.649),
    v(2, "+12V", X6),
    v(3, "+5V", 2.5),
    v(4, "CPU GT", 1.0),
    v(5, "CPU AUX", 1.0),
    v(6, "DDR5 5V", 2.5),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
];

const GB_FANS5: &[V] = &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2"), t(3, "SYS 3"), t(4, "CPU OPT")];

const GB_AM4_TEMPS: &[V] =
    &[t(0, "System"), t(1, "System 2"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS"), t(5, "Chipset")];

const GB_INTEL_TEMPS: &[V] =
    &[t(0, "System"), t(1, "PCH"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS"), t(5, "System 2")];

// ---- ASUS ----

/// Vcore on a doubled input (several ASUS AM4/AM5 boards).
const ASUS_AMD_X2_VOLTS: &[V] = &[
    v(0, "CPU", X2),
    v(1, "+5V", X5),
    v(2, "AVCC", X2),
    v(3, "+3.3V", X2),
    v(4, "+12V", X12),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(9, "VTT", 1.0),
];

const ASUS_AM5_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+5V", X5),
    v(2, "AVSB", X2),
    v(3, "+3.3V", X2),
    v(4, "+12V", X12),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(10, "CPU VDDIO", X2),
];

/// ASUS 800-series (NCT6701D) boards without a VTT input...
const ASUS_800_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+5V", 5.02),
    v(2, "AVSB", X2),
    v(3, "+3.3V", X2),
    v(4, "+12V", 11.98),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(10, "CPU VDDIO", X2),
    v(11, "CPU Misc", X2),
    v(12, "+1.8V Standby", 1.766),
];

/// ...and with one.
const ASUS_800_VTT_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+5V", 5.02),
    v(2, "AVSB", X2),
    v(3, "+3.3V", X2),
    v(4, "+12V", 11.98),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(9, "VTT", X2),
    v(10, "CPU VDDIO", X2),
    v(11, "CPU Misc", X2),
    v(12, "+1.8V Standby", 1.766),
];

const ASUS_INTEL_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+5V", X5),
    v(2, "AVSB", X2),
    v(3, "+3.3V", X2),
    v(4, "+12V", X12),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(9, "VTT", X2),
    v(12, "PCH", 1.0),
];

/// CPUTIN and SYSTIN: the two inputs every ASUS NCT board wires the same.
const ASUS_TEMPS: &[V] = &[t(1, "CPU"), t(0, "Motherboard")];
const MB_TEMP: &[V] = &[t(0, "Motherboard")];

const ASUS_FANS7: &[V] =
    &[t(0, "CHA 1"), t(1, "CPU"), t(2, "CHA 2"), t(3, "CHA 3"), t(4, "CHA 4"), t(5, "W_PUMP"), t(6, "AIO PUMP")];

const ASUS_FANS7_CHA5: &[V] =
    &[t(0, "CHA 1"), t(1, "CPU"), t(2, "CHA 2"), t(3, "CHA 3"), t(4, "CHA 4"), t(5, "CHA 5"), t(6, "AIO PUMP")];

const ASUS_ITE_VOLTS: &[V] =
    &[v(0, "CPU", 1.0), v(1, "SB 2.5V", 1.0), v(2, "+12V", X6), v(3, "+5V", 2.5), v(7, "+3.3V", X2), v(8, "VBAT", X2)];

// ---- ASRock ----

const ASR_AM4_VOLTS: &[V] = &[
    v(0, "CPU", X2),
    v(2, "AVCC", X2),
    v(3, "+3.3V", X2),
    v(4, "+12V", 6.6),
    v(5, "CPU VREF", 1.0),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(11, "Chipset 1.05V", 1.0),
    v(12, "+5V", 3.0),
    v(14, "+1.8V", 1.0),
];

const ASR_INTEL_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+5V", 3.0),
    v(2, "AVCC", X2),
    v(3, "+3.3V", X2),
    v(4, "+12V", X12),
    v(5, "CPU AUX", X2),
    v(6, "DRAM", 1.0),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(9, "VTT", X2),
    v(10, "CPU 1.05V", X2),
    v(11, "Chipset 0.82V", X2),
    v(12, "Chipset 1.0V", 1.0),
    v(13, "CPU SA", X2),
    v(14, "+5V Standby", 3.35),
];

const ASR_AM5_VOLTS: &[V] = &[
    v(0, "CPU", 1.0),
    v(1, "+12V", 6.6),
    v(2, "AVCC", X2),
    v(3, "+3.3V", X2),
    v(4, "+5V", 3.0),
    v(5, "+1.05V", 1.0),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(9, "VTT", X2),
    v(10, "CPU SoC", X2),
    v(11, "CPU Misc", X2),
    v(13, "+1.8V", X2),
    v(14, "DRAM", 1.0),
];

const ASR_X2_VOLTS: &[V] = &[
    v(0, "CPU", X2),
    v(2, "AVCC", X2),
    v(3, "+3.3V", X2),
    v(7, "+3.3V Standby", X2),
    v(8, "VBAT", X2),
    v(9, "VTT", 1.0),
];

const ASR_INTEL_FANS: &[V] =
    &[t(1, "CPU 1"), t(2, "CPU 2"), t(3, "CHA 1"), t(4, "CHA 2"), t(0, "CHA 3"), t(5, "CHA 4"), t(6, "CHA 5")];

const ASR_AM4_FANS: &[V] =
    &[t(0, "CHA 3"), t(1, "CPU 1"), t(2, "CPU 2"), t(3, "CHA 1"), t(4, "CHA 2"), t(5, "Chipset"), t(6, "CHA 4")];

const NCT679X: &[&str] = &["NCT6791", "NCT6792", "NCT6793", "NCT6795", "NCT6796", "NCT6797", "NCT6798", "NCT6799"];
const NCT6701: &[&str] = &["NCT6701", "NCT6799"];

const fn b(vendor: &'static str, models: &'static [&'static str], chips: &'static [&'static str], map: Map) -> Board {
    Board { vendor, models, chips, map }
}

const fn m(volts: &'static [V], temps: &'static [V], fans: &'static [V]) -> Map {
    Map { volts, temps, fans }
}

/// Named boards first, then chipset-series defaults (the shared reference
/// wiring a vendor uses across a generation; unverified per board, marked
/// "series"), so a named entry always wins.
pub const BOARDS: &[Board] = &[
    // ---------------- MSI ----------------
    b("MICROSTAR", &["PRO X870-P WIFI"], &["NCT6687"], MSI_X870P),
    b(
        "MICROSTAR",
        &["MS-7C35"], // X570 board on the NCT6797
        &["NCT6797"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVCC", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "CPU 1.8V", 1.0),
                v(10, "CPU VDDP", 1.0),
                v(12, "CPU NB/SoC", 1.0),
                v(13, "DRAM", X2),
                v(14, "+5V Standby", 3.327),
            ],
            &[t(1, "CPU Socket"), t(0, "System"), t(2, "VRM MOS"), t(4, "Chipset")],
            MSI_NCT_FANS,
        ),
    ),
    b(
        "MICROSTAR",
        &["X570 GAMING PLUS"],
        &["NCT6797"],
        m(MSI_X570_VOLTS, &[t(0, "System"), t(2, "VRM MOS"), t(4, "Chipset")], MSI_NCT_FANS),
    ),
    b(
        "MICROSTAR",
        &["B450 A-PRO"],
        &["NCT6797"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVCC", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(7, "+3.3V Standby", X2),
                v(9, "VTT", 1.0),
                v(12, "CPU NB/SoC", 1.0),
                v(13, "DRAM", X2),
            ],
            &[t(1, "CPU"), t(0, "System"), t(2, "VRM MOS"), t(4, "Chipset")],
            &[t(0, "PUMP"), t(1, "CPU"), t(2, "SYS 1"), t(3, "SYS 2"), t(4, "SYS 3"), t(5, "SYS 4")],
        ),
    ),
    b(
        "MICROSTAR",
        &["Z390 GAMING EDGE AC"],
        &["NCT6797"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVCC", X2),
                v(3, "+3.3V", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", 1.0),
            ],
            &[t(0, "System"), t(1, "VRM MOS"), t(2, "M.2"), t(3, "PCIe"), t(4, "CPU Socket")],
            &[t(0, "PUMP"), t(1, "CPU"), t(5, "SYS 1"), t(2, "SYS 2"), t(4, "SYS 3"), t(3, "SYS 4"), t(6, "SYS 5")],
        ),
    ),
    b(
        "MICROSTAR",
        &["B360M PRO-VDH", "Z270 PC MATE"],
        &["NCT6795", "NCT6797"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVCC", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(6, "CPU VCCIO", 1.0),
                v(7, "+3.3V Standby", X2),
                v(9, "VTT", 1.0),
                v(10, "CPU SA", 1.0),
                v(13, "DRAM", X2),
            ],
            &[t(0, "Motherboard"), t(1, "Auxiliary")],
            &[t(0, "PUMP"), t(1, "CPU"), t(2, "SYS 1"), t(3, "SYS 2"), t(4, "SYS 3"), t(5, "SYS 4")],
        ),
    ),
    b(
        "MICROSTAR",
        &["B650M GAMING PLUS WIFI", "B550-A PRO"],
        &["NCT6687"],
        m(MSI_EC_VOLTS_AMD, MSI_EC_TEMPS_AMD, MSI_FANS),
    ),
    b(
        "MICROSTAR",
        &["Z790 GODLIKE MAX"],
        &["NCT6687"],
        m(
            MSI_EC_VOLTS_INTEL,
            MSI_EC_TEMPS_INTEL,
            &[
                t(1, "CPU"),
                t(2, "PUMP 1"),
                t(9, "PUMP 2"),
                t(3, "SYS 1"),
                t(4, "SYS 2"),
                t(5, "SYS 3"),
                t(6, "SYS 4"),
                t(7, "SYS 5"),
                t(8, "SYS 6"),
                t(10, "EZ-Conn"),
            ],
        ),
    ),
    // series: MSI moved the SYS tachs on its 800-series EC firmware
    b("MICROSTAR", &["B840", "B850", "X870"], &["NCT6687"], m(MSI_EC_VOLTS_AMD, MSI_EC_TEMPS_AMD, MSI_800_FANS)),
    b(
        "MICROSTAR",
        &["Z890", "B860", "H810"],
        &["NCT6687"],
        m(MSI_EC_800_INTEL_VOLTS, MSI_EC_TEMPS_INTEL, MSI_800_FANS),
    ),
    // series
    b(
        "MICROSTAR",
        &["B550", "A520", "X570", "B650", "X670", "A620"],
        &["NCT6687"],
        m(MSI_EC_VOLTS_AMD, MSI_EC_TEMPS_AMD, MSI_FANS),
    ),
    // series
    b(
        "MICROSTAR",
        &["Z790", "B760", "H770", "Z690", "B660", "H670", "H610", "Z590", "B560", "H510", "Z490", "B460", "H410"],
        &["NCT6687"],
        m(MSI_EC_VOLTS_INTEL, MSI_EC_TEMPS_INTEL, MSI_FANS),
    ),
    // ---------------- Gigabyte ----------------
    b(
        "GIGABYTE",
        &["X570 AORUS MASTER", "X570 AORUS ULTRA", "X570 AORUS PRO", "X570 GAMING X"],
        &["IT8688"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+3.3V", 1.649),
                v(2, "+12V", X6),
                v(3, "+5V", 2.5),
                v(4, "CPU SoC", 1.0),
                v(5, "CPU VDDP", 1.0),
                v(6, "DRAM", 1.0),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
            ],
            GB_AM4_TEMPS,
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2"), t(3, "Chipset"), t(4, "CPU OPT")],
        ),
    ),
    b(
        "GIGABYTE",
        &[
            "B550 AORUS MASTER",
            "B550 AORUS PRO",
            "B550 VISION D",
            "B550 AORUS ELITE",
            "B550 GAMING X",
            "B550 UD AC",
            "B550M AORUS PRO",
        ],
        &["IT8688"],
        m(GB_AM4_VOLTS, GB_AM4_TEMPS, GB_FANS5),
    ),
    b(
        "GIGABYTE",
        &["B550I AORUS PRO AX", "B550M AORUS ELITE", "B550M GAMING", "B550M DS3H", "B550M S2H", "B550M H"],
        &["IT8688"],
        m(
            GB_AM4_VOLTS,
            &[t(0, "System"), t(1, "SoC MOS"), t(2, "CPU"), t(4, "VRM MOS"), t(5, "Chipset")],
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2")],
        ),
    ),
    b(
        "GIGABYTE",
        &["B450 AORUS PRO", "B450 AORUS ELITE", "B450 GAMING X", "B450M AORUS ELITE"],
        &["IT8686"],
        m(
            GB_AM4_VOLTS,
            &[t(0, "System"), t(1, "Chipset"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS"), t(5, "SoC MOS")],
            GB_FANS5,
        ),
    ),
    b(
        "GIGABYTE",
        &["B450M GAMING", "B450 AORUS M", "B450 I AORUS PRO WIFI", "B450M DS3H", "B450M S2H", "B450M H", "B450M K"],
        &["IT8686"],
        m(
            GB_AM4_VOLTS,
            &[t(0, "System"), t(1, "Chipset"), t(2, "CPU"), t(4, "VRM MOS"), t(5, "SoC MOS")],
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2")],
        ),
    ),
    b(
        "GIGABYTE",
        &["X470 AORUS GAMING 7"],
        &["IT8686"],
        m(GB_AM4_VOLTS, &[t(0, "System"), t(1, "Chipset"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS")], GB_FANS5),
    ),
    b(
        "GIGABYTE",
        &["B650I AX"],
        &["IT8689"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+3.3V", 1.649),
                v(2, "+12V", X6),
                v(3, "+5V", 2.5),
                v(4, "CPU SoC", 1.0),
                v(5, "CPU Misc", 1.0),
            ],
            &[t(0, "System"), t(1, "Chipset"), t(2, "CPU"), t(4, "VRM MOS"), t(5, "SoC MOS")],
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2")],
        ),
    ),
    b(
        "GIGABYTE",
        &["X670E AORUS XTREME", "X870E AORUS PRO", "X870E AORUS XTREME"],
        &["IT8689", "IT8696"],
        m(
            GB_AM5_HI_VOLTS,
            &[t(0, "System"), t(1, "Chipset"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS"), t(5, "External")],
            GB_FANS5,
        ),
    ),
    b(
        "GIGABYTE",
        &["X670 AORUS ELITE AX"],
        &["IT8689"],
        m(
            GB_AM5_HI_VOLTS,
            &[t(0, "System"), t(1, "Chipset"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS")],
            GB_FANS5,
        ),
    ),
    b(
        "GIGABYTE",
        &["X870 AORUS ELITE WIFI7"],
        &["IT8696"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+3.3V", 1.649),
                v(2, "+12V", X6),
                v(3, "+5V", 2.5),
                v(4, "CPU SoC", 1.0),
                v(5, "CPU Misc", 1.0),
                v(6, "CPU VDDIO", 1.0),
                v(7, "DRAM VDD", 1.0),
                v(8, "DRAM VDDQ", 1.0),
            ],
            &[t(0, "System"), t(1, "Chipset"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS"), t(5, "EC")],
            GB_FANS5,
        ),
    ),
    b(
        "GIGABYTE",
        &["Z390 M GAMING", "Z390 AORUS ULTRA", "Z390 UD", "Z390 AORUS PRO"],
        &["IT8688"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+3.3V", 1.649),
                v(2, "+12V", X6),
                v(3, "+5V", 2.5),
                v(4, "CPU GT", 1.0),
                v(5, "CPU SA", 1.0),
                v(6, "DRAM", 1.0),
            ],
            GB_INTEL_TEMPS,
            GB_FANS5,
        ),
    ),
    b(
        "GIGABYTE",
        &["Z790 GAMING X"],
        &["IT8689"],
        m(
            GB_INTEL_DDR5_VOLTS,
            GB_INTEL_TEMPS,
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2"), t(3, "SYS 3"), t(4, "SYS 4 / PUMP"), t(5, "CPU OPT")],
        ),
    ),
    b(
        "GIGABYTE",
        &["Z790 UD"],
        &["IT8689"],
        m(
            GB_INTEL_DDR5_VOLTS,
            GB_INTEL_TEMPS,
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2"), t(3, "SYS 3 / PUMP"), t(4, "CPU OPT")],
        ),
    ),
    b(
        "GIGABYTE",
        &["Z790 AORUS PRO X", "Z690 AORUS PRO", "Z690 AORUS ULTRA", "Z690 AORUS MASTER"],
        &["IT8689"],
        m(
            GB_INTEL_DDR5_VOLTS,
            &[t(0, "System"), t(1, "PCH"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS"), t(5, "External")],
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2"), t(3, "SYS 3 / PUMP"), t(4, "CPU OPT")],
        ),
    ),
    b(
        "GIGABYTE",
        &["Z690 GAMING X DDR4"],
        &["IT8689"],
        m(
            &[],
            GB_INTEL_TEMPS,
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2"), t(3, "SYS 3"), t(4, "CPU OPT"), t(5, "SYS 4 / PUMP")],
        ),
    ),
    b(
        "GIGABYTE",
        &["B660 DS3H DDR4", "B660 DS3H AC DDR4"],
        &["IT8689"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+3.3V", 1.65),
                v(2, "+12V", X6),
                v(3, "+5V", 2.5),
                v(4, "CPU GT", 1.0),
                v(5, "CPU AUX", 1.0),
                v(6, "DRAM", 1.0),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
            ],
            GB_INTEL_TEMPS,
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2"), t(3, "SYS 3 / PUMP"), t(4, "CPU OPT")],
        ),
    ),
    b(
        "GIGABYTE",
        &["B560M AORUS ELITE", "B560M AORUS PRO", "B560I AORUS PRO AX"],
        &["IT8689"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+3.3V", 1.649),
                v(2, "+12V", X6),
                v(3, "+5V", 2.5),
                v(4, "CPU GT", 1.0),
                v(5, "CPU SA", 1.0),
                v(6, "DRAM", 1.0),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
            ],
            GB_INTEL_TEMPS,
            GB_FANS5,
        ),
    ),
    // series: Gigabyte's AM5 B650 reference wiring (listed B650 boards all share it)
    b(
        "GIGABYTE",
        &["B650"],
        &["IT8689"],
        m(
            GB_AM5_VOLTS,
            &[t(0, "System"), t(1, "Chipset"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS"), t(5, "SoC MOS")],
            &[t(0, "CPU"), t(1, "SYS 1"), t(2, "SYS 2"), t(3, "SYS 3"), t(4, "SYS 4 / PUMP"), t(5, "CPU OPT")],
        ),
    ),
    // series
    b("GIGABYTE", &["B550", "X570"], &["IT8688"], m(GB_AM4_VOLTS, GB_AM4_TEMPS, GB_FANS5)),
    b(
        "GIGABYTE",
        &["X870", "B850"],
        &["IT8696"],
        m(
            GB_AM5_HI_VOLTS,
            &[t(0, "System"), t(1, "Chipset"), t(2, "CPU"), t(3, "PCIe x16"), t(4, "VRM MOS")],
            GB_FANS5,
        ),
    ),
    b("GIGABYTE", &["Z790", "Z690"], &["IT8689"], m(GB_INTEL_DDR5_VOLTS, GB_INTEL_TEMPS, GB_FANS5)),
    // ---------------- ASUS ----------------
    b(
        "ASUS",
        &["TUF GAMING X570-PLUS"],
        NCT679X,
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVSB", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", 1.0),
            ],
            MB_TEMP,
            &[
                t(0, "CHA 1"),
                t(1, "CPU"),
                t(2, "CHA 2"),
                t(3, "CHA 3"),
                t(4, "Chipset"),
                t(5, "AIO PUMP"),
                t(6, "CPU OPT"),
            ],
        ),
    ),
    b(
        "ASUS",
        &["TUF GAMING B550M-PLUS"],
        NCT679X,
        m(
            &[
                v(0, "CPU", 1.0),
                v(2, "AVCC", X2),
                v(3, "+3.3V", X2),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", 1.0),
            ],
            ASUS_TEMPS,
            &[],
        ),
    ),
    b(
        "ASUS",
        &["ROG CROSSHAIR VIII HERO", "ROG CROSSHAIR VIII DARK HERO", "ROG CROSSHAIR VIII FORMULA"],
        NCT679X,
        m(
            &[
                v(0, "CPU", 1.0),
                v(2, "AVCC", X2),
                v(3, "+3.3V", X2),
                v(6, "CPU SoC", 1.0),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", 1.0),
                v(13, "DRAM", 1.0),
            ],
            ASUS_TEMPS,
            &[t(0, "CHA 1"), t(1, "CPU"), t(2, "CHA 2"), t(3, "CHA 3"), t(4, "HAMP"), t(5, "W_PUMP"), t(6, "AIO PUMP")],
        ),
    ),
    b(
        "ASUS",
        &["ROG STRIX X570-I GAMING", "ROG STRIX B550-I GAMING"],
        NCT679X,
        m(ASUS_AMD_X2_VOLTS, ASUS_TEMPS, &[t(0, "CHA"), t(1, "CPU"), t(4, "AIO PUMP")]),
    ),
    b("ASUS", &["ROG STRIX B550-F GAMING"], NCT679X, m(ASUS_AMD_X2_VOLTS, &[], &[])),
    b("ASUS", &["ROG CROSSHAIR X670E HERO", "ROG CROSSHAIR X670E GENE"], NCT679X, m(ASUS_AMD_X2_VOLTS, MB_TEMP, &[])),
    b(
        "ASUS",
        &["ROG STRIX X670E-A GAMING", "ROG STRIX X670E-E GAMING", "ROG STRIX X670E-F GAMING"],
        NCT679X,
        m(ASUS_AMD_X2_VOLTS, &[t(1, "VRM"), t(0, "Motherboard")], &[]),
    ),
    b(
        "ASUS",
        &["PRIME B650-PLUS"],
        NCT679X,
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVSB", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", 1.0),
                v(10, "CPU VDDIO", X2),
            ],
            MB_TEMP,
            &[t(0, "CHA 1"), t(1, "CPU"), t(2, "CHA 2"), t(3, "CHA 3"), t(4, "CPU OPT"), t(5, "AIO PUMP")],
        ),
    ),
    b(
        "ASUS",
        &["PROART X670E-CREATOR"],
        NCT679X,
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVCC", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
            ],
            MB_TEMP,
            &[],
        ),
    ),
    b(
        "ASUS",
        &["PRIME X870-P"],
        NCT6701,
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVSB", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", X2),
                v(10, "CPU VDDIO", X2),
            ],
            MB_TEMP,
            &[
                t(0, "CHA 1"),
                t(1, "CPU"),
                t(2, "CHA 2"),
                t(3, "CHA 3"),
                t(4, "CPU OPT"),
                t(5, "CHA 4"),
                t(6, "AIO PUMP"),
            ],
        ),
    ),
    b(
        "ASUS",
        &["ROG CROSSHAIR X870E APEX"],
        NCT6701,
        m(
            &[
                v(0, "CPU", 1.110),
                v(1, "+5V", X5),
                v(2, "AVSB", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(5, "CPU Misc", 1.110),
                v(6, "CPU SoC", 1.110),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", X2),
                v(10, "Chipset 0", X2),
                v(11, "Chipset 1", X2),
                v(13, "CPU VDDIO", 1.110),
                v(14, "+1.8V PLL", 1.472),
            ],
            &[t(1, "VRM"), t(0, "Motherboard")],
            &[t(0, "CHA 1"), t(1, "CPU"), t(2, "CHA 2"), t(3, "Extra Flow"), t(5, "W_PUMP"), t(6, "AIO PUMP")],
        ),
    ),
    b("ASUS", &["ROG CROSSHAIR X870E HERO"], NCT6701, m(ASUS_AM5_VOLTS, MB_TEMP, ASUS_FANS7)),
    b("ASUS", &["ROG STRIX X870E-E GAMING"], NCT6701, m(ASUS_AM5_VOLTS, MB_TEMP, ASUS_FANS7_CHA5)),
    b(
        "ASUS",
        &["ROG STRIX X870-I GAMING"],
        NCT6701,
        m(ASUS_800_VTT_VOLTS, MB_TEMP, &[t(0, "CHA"), t(1, "CPU"), t(5, "Chipset / Disk"), t(6, "AIO PUMP")]),
    ),
    b(
        "ASUS",
        &["ROG STRIX B850-I GAMING"],
        NCT6701,
        m(
            ASUS_800_VTT_VOLTS,
            &[t(0, "Motherboard"), t(5, "T_Sensor")],
            &[t(0, "CHA"), t(1, "CPU"), t(5, "Extra Flow"), t(6, "AIO PUMP")],
        ),
    ),
    b(
        "ASUS",
        &["TUF GAMING B850M-PLUS"],
        NCT6701,
        m(
            ASUS_800_VTT_VOLTS,
            &[t(0, "Motherboard"), t(5, "T_Sensor")],
            &[t(0, "CHA 1"), t(1, "CPU"), t(2, "CHA 2"), t(3, "CHA 3"), t(4, "CPU OPT"), t(5, "AIO PUMP")],
        ),
    ),
    b("ASUS", &["ROG STRIX B850-E GAMING"], NCT6701, m(ASUS_800_VTT_VOLTS, MB_TEMP, ASUS_FANS7_CHA5)),
    b(
        "ASUS",
        &["ROG STRIX B850-A GAMING", "TUF GAMING B850-BTF", "TUF GAMING X870-PRO"],
        NCT6701,
        m(
            ASUS_800_VOLTS,
            MB_TEMP,
            &[
                t(0, "CHA 1"),
                t(1, "CPU"),
                t(2, "CHA 2"),
                t(3, "CHA 3"),
                t(4, "CHA 4"),
                t(5, "W_PUMP+"),
                t(6, "AIO PUMP"),
            ],
        ),
    ),
    b(
        "ASUS",
        &["PROART X870E-CREATOR"],
        NCT6701,
        m(
            &[
                v(0, "CPU", 1.0),
                v(2, "AVCC", X2),
                v(3, "+3.3V", X2),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", 1.0),
            ],
            MB_TEMP,
            &[],
        ),
    ),
    b(
        "ASUS",
        &["ROG MAXIMUS XI FORMULA"],
        NCT679X,
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVSB", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(6, "CPU GT", 1.0),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", X2),
                v(10, "DRAM", 1.0),
                v(11, "CPU VCCIO", X2),
                v(12, "PCH", 1.0),
                v(13, "PLL", 1.0),
                v(14, "CPU SA", 1.0),
            ],
            ASUS_TEMPS,
            &[
                t(0, "CHA 1"),
                t(1, "CPU"),
                t(2, "CHA 2"),
                t(3, "CHA 3"),
                t(4, "HAMP"),
                t(5, "W_PUMP+"),
                t(6, "AIO PUMP"),
            ],
        ),
    ),
    b(
        "ASUS",
        &["ROG MAXIMUS XII FORMULA", "ROG MAXIMUS Z690 FORMULA", "ROG MAXIMUS Z690 HERO"],
        NCT679X,
        m(ASUS_INTEL_VOLTS, ASUS_TEMPS, ASUS_FANS7),
    ),
    b(
        "ASUS",
        &["PROART B760-CREATOR D4"],
        NCT679X,
        m(
            ASUS_INTEL_VOLTS,
            ASUS_TEMPS,
            &[
                t(0, "CHA 1"),
                t(1, "CPU"),
                t(2, "CHA 2"),
                t(3, "CHA 3"),
                t(4, "CHA 4"),
                t(5, "CPU OPT"),
                t(6, "AIO PUMP"),
            ],
        ),
    ),
    b(
        "ASUS",
        &["ROG MAXIMUS Z790 DARK HERO"],
        NCT679X,
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVSB", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", X2),
                v(12, "PCH", 1.0),
                v(13, "CPU SA", X2),
                v(14, "CPU AUX", X2),
            ],
            MB_TEMP,
            ASUS_FANS7,
        ),
    ),
    b("ASUS", &["ROG MAXIMUS Z790 HERO"], NCT679X, m(&[], ASUS_TEMPS, &[])),
    b("ASUS", &["ROG STRIX Z790-E GAMING WIFI II"], NCT679X, m(&[], MB_TEMP, ASUS_FANS7_CHA5)),
    b(
        "ASUS",
        &["ROG STRIX B760-I GAMING"],
        NCT679X,
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "+5V", X5),
                v(2, "AVSB", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", X2),
                v(10, "CPU IMC", X2),
            ],
            &[t(1, "CPU"), t(0, "Motherboard"), t(2, "VRM")],
            &[t(0, "CHA"), t(1, "CPU"), t(5, "AIO PUMP")],
        ),
    ),
    b(
        "ASUS",
        &["TUF GAMING B760M-PLUS WIFI D4"],
        NCT679X,
        m(
            &[
                v(0, "CPU", 1.110),
                v(1, "+5V", X5),
                v(2, "AVSB", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", X12),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(9, "VTT", X2),
                v(10, "DRAM", X2),
            ],
            MB_TEMP,
            &[t(0, "CHA 1"), t(1, "CPU"), t(2, "CPU OPT"), t(3, "CHA 2"), t(4, "CHA 3"), t(5, "AIO PUMP")],
        ),
    ),
    b(
        "ASUS",
        &["TUF X470-PLUS GAMING", "ROG STRIX B450-F GAMING"],
        &["IT8665"],
        m(ASUS_ITE_VOLTS, &[t(0, "CPU"), t(1, "Motherboard"), t(2, "Chipset")], &[t(0, "CPU")]),
    ),
    b(
        "ASUS",
        &["ROG STRIX X470-I GAMING"],
        &["IT8665"],
        m(
            ASUS_ITE_VOLTS,
            &[t(0, "CPU"), t(1, "Motherboard"), t(2, "T_Sensor"), t(3, "PCIe x16"), t(4, "VRM")],
            &[t(0, "CPU"), t(1, "CHA 1"), t(2, "CHA 2")],
        ),
    ),
    b(
        "ASUS",
        &["TUF GAMING B450-PLUS II"],
        &["IT8665"],
        m(
            &[
                v(0, "CPU", 1.0),
                v(1, "VCCP2", 1.0),
                v(2, "+12V", X6),
                v(3, "+5V", 2.5),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
            ],
            &[t(0, "CPU"), t(1, "Motherboard")],
            &[t(0, "CPU"), t(1, "CHA 1"), t(2, "CHA 2"), t(3, "CHA 3"), t(5, "AIO PUMP")],
        ),
    ),
    // series: the common ASUS rails per generation
    b("ASUS", &["X870", "B850", "B840"], NCT6701, m(ASUS_800_VOLTS, MB_TEMP, ASUS_FANS7)),
    b("ASUS", &["X670", "B650", "A620"], &["NCT6799", "NCT6798"], m(ASUS_AM5_VOLTS, MB_TEMP, ASUS_FANS7)),
    b(
        "ASUS",
        &["Z890", "B860", "Z790", "B760", "H770", "Z690", "B660", "Z590", "B560", "Z490", "B460", "Z390"],
        NCT679X,
        m(ASUS_INTEL_VOLTS, ASUS_TEMPS, ASUS_FANS7),
    ),
    // ---------------- ASRock ----------------
    b(
        "ASROCK",
        &["B450M PRO4 R2.0"],
        &["NCT6779"],
        m(ASR_AM4_VOLTS, MB_TEMP, &[t(0, "CHA 1"), t(1, "CPU"), t(2, "CHA 2"), t(3, "CPU PUMP"), t(4, "CHA 3")]),
    ),
    b(
        "ASROCK",
        &[
            "B450 STEEL LEGEND",
            "B450M STEEL LEGEND",
            "B450 PRO4",
            "B450M PRO4",
            "AB350 PRO4",
            "AB350M PRO4",
            "AB350M",
            "AB350 GAMING K4",
        ],
        &["NCT6779"],
        m(
            ASR_AM4_VOLTS,
            &[t(1, "CPU"), t(0, "Motherboard"), t(2, "Auxiliary"), t(3, "VRM")],
            &[t(1, "CPU"), t(2, "CPU PUMP"), t(0, "CHA 1"), t(3, "CHA 2"), t(4, "CHA 3")],
        ),
    ),
    b(
        "ASROCK",
        &["B550M PRO4", "X570 PHANTOM GAMING 4"],
        NCT679X,
        m(
            &[
                v(0, "CPU", X2),
                v(1, "+5V", 3.0),
                v(2, "AVCC", X2),
                v(3, "+3.3V", X2),
                v(4, "+12V", 6.6),
                v(5, "CPU SoC", 1.0),
                v(6, "DRAM", 1.0),
                v(7, "+3.3V Standby", X2),
                v(8, "VBAT", X2),
                v(11, "VPPM", 4.0),
                v(14, "+1.8V", X2),
            ],
            MB_TEMP,
            ASR_AM4_FANS,
        ),
    ),
    b("ASROCK", &["X570 TAICHI"], NCT679X, m(ASR_X2_VOLTS, MB_TEMP, ASR_AM4_FANS)),
    b(
        "ASROCK",
        &["Z690 STEEL LEGEND"],
        NCT679X,
        m(ASR_INTEL_VOLTS, &[t(1, "VRM"), t(0, "Motherboard")], ASR_INTEL_FANS),
    ),
    b("ASROCK", &["Z690 EXTREME", "Z790 PRO RS"], NCT679X, m(ASR_INTEL_VOLTS, MB_TEMP, ASR_INTEL_FANS)),
    b(
        "ASROCK",
        &["X870E TAICHI"],
        &["NCT6799"],
        m(
            ASR_AM5_VOLTS,
            &[t(0, "Motherboard"), t(4, "T_Sensor 1"), t(5, "T_Sensor 2")],
            &[t(1, "CPU 1"), t(2, "CPU 2"), t(3, "AIO PUMP"), t(0, "CHA 1"), t(4, "CHA 2")],
        ),
    ),
    b(
        "ASROCK",
        &["B850M STEEL LEGEND"],
        &["NCT6799"],
        m(
            ASR_X2_VOLTS,
            &[t(1, "CPU"), t(0, "Motherboard")],
            &[t(1, "CPU 1"), t(0, "CPU 2 / PUMP"), t(3, "CHA 1"), t(4, "CHA 2"), t(6, "CHA 3")],
        ),
    ),
    b(
        "ASROCK",
        &["B650M-C"],
        &["NCT6799"],
        m(ASR_AM5_VOLTS, MB_TEMP, &[t(1, "CPU 1"), t(0, "CPU 2"), t(3, "CHA 1"), t(4, "CHA 2"), t(6, "CHA 3")]),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Config registers of this machine's NCT6687 (reference dump, LDN 0x0B).
    const REGS: [u8; 128] = [
        0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x0B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
        0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x0A, 0xFF, 0xFF, 0xFF, 0xFF, 0x80, 0x02, 0x00, 0x00, 0xA2, 0x02, //
        0xD5, 0x92, 0xBC, 0xDF, 0x39, 0xFE, 0xC0, 0x12, 0x40, 0xFF, 0x8F, 0x28, 0x80, 0x00, 0x00, 0x82, //
        0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
        0x0A, 0x20, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
        0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, //
    ];

    #[derive(Default)]
    struct St {
        keys: u8,
        cfg: bool,
        idx: u8,
        page: u8,
        ecidx: u8,
        writes: Vec<(u16, u8)>,
    }

    /// Emulates an NCT6687 at 0x4E with EC base 0xA20 and the dump's sensor codes.
    struct Fake {
        st: Mutex<St>,
        ec: HashMap<u16, u8>,
    }

    impl Hw for Fake {
        fn name(&self) -> String {
            "fake".into()
        }
        fn io_in8(&self, port: u16) -> Option<u8> {
            let s = self.st.lock().unwrap();
            Some(match port {
                0x4F if s.cfg => REGS[s.idx as usize & 0x7F],
                0xA26 => *self.ec.get(&((s.page as u16) << 8 | s.ecidx as u16)).unwrap_or(&0),
                _ => 0xFF,
            })
        }
        fn io_out8(&self, port: u16, v: u8) -> bool {
            let mut s = self.st.lock().unwrap();
            s.writes.push((port, v));
            match port {
                0x4E if !s.cfg => {
                    s.keys = if v == 0x87 { s.keys + 1 } else { 0 };
                    s.cfg = s.keys == 2;
                }
                0x4E if v == 0xAA => (s.cfg, s.keys) = (false, 0),
                0x4E => s.idx = v,
                0xA24 if v != 0xFF => s.page = v,
                0xA25 => s.ecidx = v,
                _ => {}
            }
            true
        }
    }

    #[test]
    fn nct6687_like_the_dump() {
        let ec: HashMap<u16, u8> = [
            (0x100, 0x47),
            (0x102, 0x30),
            (0x104, 0x34),
            (0x106, 0x2E),
            // Inputs 0,1,2 then 3 = VDDIO (x2), 4 = Vcore, 5 = unshown rail, 8 = +3.3V,
            // positions as read from this board's EC.
            (0x120, 0x3E),
            (0x121, 0xC0),
            (0x122, 0x3E),
            (0x123, 0x20),
            (0x124, 0x51),
            (0x125, 0xE0),
            (0x126, 0x2A),
            (0x127, 0x60),
            (0x128, 0x55),
            (0x129, 0xE0),
            (0x12A, 0x16),
            (0x12B, 0xA0),
            (0x130, 0xCA),
            (0x131, 0x00),
            (0x15C, 0x05),
            (0x15D, 0x27),
            (0x15E, 0x03),
            (0x15F, 0x64),
        ]
        .into_iter()
        .collect();
        let hw = Fake { st: Mutex::default(), ec };
        let chips = detect(&hw);
        assert_eq!(chips.len(), 1);
        let c = &chips[0];
        assert_eq!((c.model, c.chip_id, c.rev, c.port, c.ldn, c.base), ("NCT6687", 0xD5, 0x92, 0x4E, 0x0B, 0x0A20));
        assert_eq!(c.regs, REGS);
        assert!(!hw.st.lock().unwrap().cfg, "config mode must be exited");
        // Only key/exit, index, and LDN-select writes reach the config ports.
        assert!(hw.st.lock().unwrap().writes.iter().all(|&(p, v)| p != 0x4F && p != 0x2F || v == 0x0B || v == 0x02));

        let m = read(&hw, c, "Micro-Star International Co., Ltd.", "PRO X870-P WIFI (MS-7E47)").unwrap();
        let txt = rows(&m);
        for want in [
            // Same lines as the reference dump's "Hardware monitor Nuvoton NCT6687".
            "\tVoltage 0\t\t12.05 Volts [0x3EC0] (+12V)\n",
            "\tVoltage 1\t\t4.97 Volts [0x3E20] (+5V)\n",
            "\tVoltage 2\t\t1.31 Volts [0x51E0] (CPU NB/SoC)\n",
            "\tVoltage 3\t\t1.37 Volts [0x55E0] (CPU)\n",
            "\tVoltage 6\t\t3.23 Volts [0xCA00] (+3.3V)\n",
            "\tVoltage 9\t\t1.36 Volts [0x2A60] (CPU VDDIO)\n",
            "\tTemperature 0\t\t71 degC (160 degF) [0x47] (CPU Core)\n",
            "\tTemperature 1\t\t48 degC (118 degF) [0x30] (System)\n",
            "\tTemperature 3\t\t46 degC (115 degF) [0x2E] (Chipset A)\n",
            "\tFan 15\t\t\t1319 RPM [0x527] (SYS 2)\n",
            "\tFan 16\t\t\t868 RPM [0x364] (SYS 1)\n",
        ] {
            assert!(txt.contains(want), "missing {want:?} in\n{txt}");
        }
        assert!(!txt.contains("0x16A0"), "unlabeled input shown:\n{txt}");

        // Unknown board: the chip's generic list, every valid input shown.
        let g = rows(&read(&hw, c, "", "").unwrap());
        assert!(g.contains("\tVoltage 0\t\t1.00 Volts [0x3EC0] (VIN0)\n"), "{g}");
        assert!(g.contains("[0x16A0] (VIN5)"), "{g}");
        assert!(g.contains("\tFan 16\t\t\t868 RPM [0x364] (FAN16)\n"), "{g}");
    }

    fn rows(m: &Monitor) -> String {
        crate::report::to_txt(&[crate::report::section(
            "H",
            vec![crate::report::gv("Hardware monitor", &m.name, super::super::reading_rows(&m.readings))],
        )])
    }

    #[test]
    fn board_lookup() {
        let gb = board_map("Gigabyte Technology Co., Ltd.", "B550 AORUS ELITE V2", "IT8688").unwrap();
        assert!(gb.temps.iter().any(|e| e.input == 5 && e.label == "Chipset"));
        assert!(
            board_map("Gigabyte Technology Co., Ltd.", "B550 AORUS ELITE V2", "IT8689").is_none(),
            "chip must match"
        );
        // Specific entries win over the series default they also match.
        let x870p = board_map("Micro-Star International Co., Ltd.", "PRO X870-P WIFI (MS-7E47)", "NCT6687").unwrap();
        assert_eq!(x870p.volts.len(), 6);
        let tomahawk = board_map("Micro-Star International Co., Ltd.", "MAG X870 TOMAHAWK WIFI (MS-7E51)", "NCT6687");
        assert!(tomahawk.unwrap().volts.len() > 6);
        let asus = board_map("ASUSTeK COMPUTER INC.", "ROG CROSSHAIR VIII DARK HERO", "NCT6798").unwrap();
        assert!(asus.fans.iter().any(|e| e.label == "HAMP"));
        assert!(board_map("ASRock", "B450M Pro4 R2.0", "NCT6779").unwrap().fans[3].label == "CPU PUMP");
        assert!(board_map("Unknown", "Board", "NCT6798").is_none());

        // Map application: listed inputs only, board multiplier, map order.
        let raw = Raw { volts: vec![(0, 1.0, 1), (4, 1.0, 2), (5, 0.5, 3)], temps: vec![(5, 40.0, 40)], fans: vec![] };
        let m = apply("x".into(), &raw, &ITE_GENERIC, Some(gb), false);
        let v: Vec<(&str, f64)> = m.readings.iter().map(|r| (r.label.as_str(), r.value)).collect();
        assert_eq!(v, [("CPU", 1.0), ("CPU SoC", 1.0), ("CPU VDDP", 0.5), ("Chipset", 40.0)]);
        let all = apply("x".into(), &raw, &ITE_GENERIC, Some(&MSI_X870P), true);
        assert!(all.readings.iter().any(|r| r.label == "VIN5 (unmapped)"), "{:?}", all.readings);
    }

    #[test]
    fn board_table_is_sane() {
        for b in BOARDS {
            assert!(!b.models.is_empty() && !b.chips.is_empty());
            for e in b.map.volts {
                assert!(e.k > 0.0 && e.k <= 13.0, "{} {}", b.models[0], e.label);
            }
        }
    }
}

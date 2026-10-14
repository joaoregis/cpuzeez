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

// ADC LSB: 16 mV on the original IT87 parts, 12 mV from IT8721 on, 10.9 mV on
// the 2017+ IT86xx generation. ponytail: per-generation, not per-stepping.
const OLD: Family = Family::Ite { lsb: 0.016, temps: 3 };
const MV12: Family = Family::Ite { lsb: 0.012, temps: 3 };
const MV12_6: Family = Family::Ite { lsb: 0.012, temps: 6 };
const MV10_6: Family = Family::Ite { lsb: 0.0109, temps: 6 };
const ITE: &[Id] = &[
    (0x8620, 0xFFFF, "ITE", "IT8620", MV12_6),
    (0x8625, 0xFFFF, "ITE", "IT8625", MV10_6),
    (0x8628, 0xFFFF, "ITE", "IT8628", MV12_6),
    (0x8655, 0xFFFF, "ITE", "IT8655", MV10_6),
    (0x8665, 0xFFFF, "ITE", "IT8665", MV10_6),
    (0x8686, 0xFFFF, "ITE", "IT8686", MV10_6),
    (0x8688, 0xFFFF, "ITE", "IT8688", MV10_6),
    (0x8689, 0xFFFF, "ITE", "IT8689", MV10_6),
    (0x8696, 0xFFFF, "ITE", "IT8696", MV10_6),
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
    (0x8733, 0xFFFF, "ITE", "IT8792", MV12),
    (0x8792, 0xFFFF, "ITE", "IT8792", MV12),
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

fn label(t: &[&str], i: usize) -> String {
    t.get(i).copied().unwrap_or("").to_string()
}

fn temp(m: &mut Monitor, ch: usize, label: String, t: f64, raw: u32) {
    if (1.0..=125.0).contains(&t) {
        m.push_ch(Kind::Temperature, ch, label, t, raw);
    }
}

fn fan(m: &mut Monitor, ch: usize, label: String, rpm: f64, raw: u32) {
    if (50.0..=20000.0).contains(&rpm) {
        m.push_ch(Kind::Fan, ch, label, rpm, raw);
    }
}

pub fn read(hw: &dyn Hw, c: &Chip) -> Option<Monitor> {
    if c.base == 0 || c.base == 0xFFF8 {
        return None;
    }
    let io = Io { hw, base: c.base };
    let mut m = Monitor::new(format!("{} {}", c.vendor, c.model));
    match c.family {
        Family::NctEc => nct_ec(&io, &mut m),
        Family::NctNew => {
            nct_new(&io, &mut m);
            io.bank(0); // leave bank 0 selected
        }
        Family::NctOld { rpm13 } => {
            nct_old(&io, rpm13, &mut m);
            io.bank(0);
        }
        Family::Ite { lsb, temps } => ite(&io, lsb, temps, &mut m),
        Family::Fintek => fintek(&io, c.model == "F71858", &mut m),
    }
    Some(m)
}

// How MSI wires the NCT6687's voltage inputs: (input, row number, label,
// divider). Board-level calibration table checked against this machine's
// readings; the other inputs aren't rails CPU-Z shows, so neither do we.
const EC_VOLT: [(u16, usize, &str, f64); 6] = [
    (0, 0, "+12V", 12.0),
    (1, 1, "+5V", 5.0),
    (2, 2, "CPU NB/SoC", 1.0),
    (4, 3, "CPU", 1.0),
    (8, 6, "+3.3V", 1.0),
    (3, 9, "CPU VDDIO", 2.0),
];
const EC_TEMP: [&str; 8] = ["CPU Core", "System", "MOS", "Chipset A", "", "", "", ""];
// 16 tachometer words at 0x140. Header names for 14/15 were checked on an MSI
// PRO X870-P against CPU-Z's labels; the rest are the usual MSI order.
const EC_FAN: [&str; 16] =
    ["CPU", "PUMP", "SYS 1", "SYS 2", "SYS 3", "SYS 4", "SYS 5", "SYS 6", "", "", "", "", "", "", "SYS 2", "SYS 1"];

fn nct_ec(io: &Io, m: &mut Monitor) {
    let w = |r: u16| (io.ec(r) as u32) << 8 | io.ec(r + 1) as u32;
    if std::env::var_os("CPUZEEZ_EC_DUMP").is_some() {
        m.registers.push(crate::report::hex("EC 0x100", (0x100..0x200).map(|r| io.ec(r)).collect()));
    }
    for (inp, ch, l, k) in EC_VOLT {
        let raw = w(0x120 + 2 * inp);
        if raw != 0 && raw != 0xFFFF {
            m.push_ch(Kind::Voltage, ch, l, (raw >> 4) as f64 / 1000.0 * k, raw);
        }
    }
    for i in 0..8u16 {
        let hi = io.ec(0x100 + 2 * i) as i8;
        let half = (io.ec(0x101 + 2 * i) >> 7) as f64 * 0.5;
        temp(m, i as usize, label(&EC_TEMP, i as usize), hi as f64 + half, hi as u8 as u32);
    }
    for i in 0..16u16 {
        let rpm = w(0x140 + 2 * i);
        if rpm != 0xFFFF {
            // Numbered from 1 like CPU-Z ("Fan 15", "Fan 16" for the last two).
            fan(m, i as usize + 1, label(&EC_FAN, i as usize), rpm as f64, rpm);
        }
    }
}

// Inputs with the chip's internal /2 divider (AVCC, 3VCC, 3VSB, VBAT).
const NCT_X2: [usize; 4] = [2, 3, 7, 8];
const NCT_VOLT: [&str; 10] = ["CPU", "", "AVCC", "+3.3V", "", "", "", "+3.3V Standby", "VBAT", "VTT"];
const NCT_FAN: [&str; 7] = ["SYSFAN", "CPUFAN", "AUXFAN0", "AUXFAN1", "AUXFAN2", "AUXFAN3", "AUXFAN4"];

fn nct_volt(m: &mut Monitor, i: usize, raw: u8) {
    if raw != 0 && raw != 0xFF {
        let k = if NCT_X2.contains(&i) { 2.0 } else { 1.0 };
        m.push_ch(Kind::Voltage, i, label(&NCT_VOLT, i), raw as f64 * 0.008 * k, raw as u32);
    }
}

fn nct_new(io: &Io, m: &mut Monitor) {
    for i in 0..15 {
        nct_volt(m, i, io.bank(0x480 + i as u16));
    }
    for (i, l) in ["SYSTIN", "CPUTIN", "AUXTIN0", "AUXTIN1", "AUXTIN2", "AUXTIN3"].iter().enumerate() {
        let t = io.bank(0x490 + i as u16);
        temp(m, i, l.to_string(), t as i8 as f64, t as u32);
    }
    for (i, r) in [0x4C0u16, 0x4C2, 0x4C4, 0x4C6, 0x4C8, 0x4CA, 0x4CE].into_iter().enumerate() {
        let rpm = (io.bank(r) as u32) << 8 | io.bank(r + 1) as u32;
        fan(m, i, label(&NCT_FAN, i), rpm as f64, rpm);
    }
}

fn nct_old(io: &Io, rpm13: bool, m: &mut Monitor) {
    for (i, r) in [0x20u16, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x550, 0x551].into_iter().enumerate() {
        nct_volt(m, i, io.bank(r));
    }
    let sys = io.bank(0x027);
    temp(m, 0, "SYSTIN".into(), sys as i8 as f64, sys as u32);
    for (j, (r, l)) in [(0x150u16, "CPUTIN"), (0x250, "AUXTIN")].into_iter().enumerate() {
        let hi = io.bank(r);
        temp(m, j + 1, l.into(), hi as i8 as f64 + (io.bank(r + 1) >> 7) as f64 * 0.5, hi as u32);
    }
    if rpm13 {
        for i in 0..5u16 {
            let count = (io.bank(0x630 + 2 * i) as u32) << 5 | (io.bank(0x631 + 2 * i) & 0x1F) as u32;
            if count != 0 && count != 0x1FFF {
                fan(m, i as usize, label(&NCT_FAN, i as usize), 1_350_000.0 / count as f64, count);
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
        for (i, r) in [0x28u16, 0x29, 0x2A].into_iter().enumerate() {
            let count = io.bank(r) as u32;
            if count != 0 && count != 0xFF {
                fan(m, i, label(&NCT_FAN, i), 1_350_000.0 / (count << div[i]) as f64, count);
            }
        }
    }
}

fn ite(io: &Io, lsb: f64, temps: u8, m: &mut Monitor) {
    for i in 0..9u8 {
        let raw = io.isa(0x20 + i);
        if raw == 0 || raw == 0xFF {
            continue;
        }
        // 12/10.9 mV parts divide 3VSB and VBAT by two internally.
        let k = if lsb < 0.016 && i >= 7 { 2.0 } else { 1.0 };
        let l = match i {
            7 => "+3.3V Standby",
            8 => "VBAT",
            _ => "",
        };
        m.push_ch(Kind::Voltage, i as usize, l, raw as f64 * lsb * k, raw as u32);
    }
    for i in 0..temps {
        let t = io.isa(0x29 + i);
        temp(m, i as usize, format!("TMPIN{}", i + 1), t as i8 as f64, t as u32);
    }
    for (i, (lo, hi)) in
        [(0x0D, 0x18), (0x0E, 0x19), (0x0F, 0x1A), (0x80, 0x81), (0x82, 0x83), (0x93, 0x94)].into_iter().enumerate()
    {
        let count = io.isa(lo) as u32 | (io.isa(hi) as u32) << 8;
        if count != 0 && count != 0xFFFF {
            fan(m, i, format!("FAN{}", i + 1), 1_350_000.0 / (2 * count) as f64, count);
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
            temp(m, i as usize, format!("Temperature {}", i + 1), t as i8 as f64, t as u32);
        }
    }
    for i in 0..4u8 {
        let count = (io.isa(0xA0 + 16 * i) as u32) << 8 | io.isa(0xA1 + 16 * i) as u32;
        if count != 0 && count != 0xFFFF {
            fan(m, i as usize, format!("Fan {}", i + 1), 1_500_000.0 / count as f64, count);
        }
    }
}

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

        let m = read(&hw, c).unwrap();
        let txt = crate::report::to_txt(&[crate::report::section(
            "H",
            vec![crate::report::gv("Hardware monitor", &m.name, super::super::reading_rows(&m.readings))],
        )]);
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
    }
}

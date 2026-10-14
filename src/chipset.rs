//! Chipset section (FEATURES.md 3.8) and the data behind the Mainboard and
//! Memory tabs (FEATURES.md 6, 7). Live DRAM timings come from the memory
//! controller when the privileged backend can reach it (AMD UMC over SMN);
//! otherwise the SMBIOS view or macOS memory information is shown.
#[cfg(any(target_os = "macos", test))]
mod macos;

use crate::Ctx;
use crate::pci::{self, Device, Link, decode::speed_text};
use crate::report::{Section, f, section};

#[derive(Debug, Clone, Default)]
pub struct Timings {
    pub cl: u32,
    pub trcd: u32,
    pub trp: u32,
    pub tras: u32,
    pub trc: u32,
    /// Command rate in clocks (1T/2T), when known.
    pub cr: Option<u32>,
    pub trfc: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct Memory {
    pub mem_type: Option<String>,
    pub size_mb: u64,
    /// "2 x 32-bit", "Dual", ...
    pub channels: Option<String>,
    /// DRAM clock (half the MT/s rate).
    pub freq_mhz: Option<f64>,
    /// FSB:DRAM, "1:30".
    pub ratio: Option<String>,
    pub max_freq_mhz: Option<f64>,
    pub timings: Option<Timings>,
    pub uncore_mhz: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct ChipsetSummary {
    pub northbridge: Option<String>,
    pub nb_rev: Option<u8>,
    pub southbridge: Option<String>,
    pub sb_rev: Option<u8>,
    /// "PCI-Express 5.0 (32.0 GT/s)": fastest root port.
    pub bus_spec: Option<String>,
    /// Upstream link of the main (discrete if any) graphics adapter.
    pub gfx_link: Option<Link>,
    pub memory: Memory,
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let s = summary(ctx);
    let mut v = Vec::new();
    if let Some(nb) = &s.northbridge {
        v.push(f("Northbridge", with_rev(nb, s.nb_rev)));
    }
    if let Some(sb) = &s.southbridge {
        v.push(f("Southbridge", with_rev(sb, s.sb_rev)));
    }
    if let Some(b) = &s.bus_spec {
        v.push(f("Bus Specification", b));
    }
    if let Some(l) = s.gfx_link {
        v.push(f("Graphic Interface", format!("PCI-Express {}.0", l.max_speed)));
        v.push(f("PCI-E Link Width", format!("x{} (max {}x)", l.width, l.max_width)));
        v.push(f("PCI-E Link Speed", format!("{} (max {})", speed_text(l.speed), speed_text(l.max_speed))));
    }
    let m = &s.memory;
    if let Some(t) = &m.mem_type {
        v.push(f("Memory Type", t));
    }
    if m.size_mb > 0 {
        let sz = if m.size_mb.is_multiple_of(1024) {
            format!("{} GBytes", m.size_mb / 1024)
        } else {
            format!("{} MBytes", m.size_mb)
        };
        v.push(f("Memory Size", sz));
    }
    if let Some(c) = &m.channels {
        v.push(f("Channels", c));
    }
    if let Some(fr) = m.freq_mhz {
        let r = m.ratio.as_ref().map(|r| format!(" ({r})")).unwrap_or_default();
        v.push(f("Memory Frequency", format!("{fr:.1} MHz{r}")));
    }
    if let Some(fr) = m.max_freq_mhz {
        v.push(f("Memory Max Frequency", format!("{fr:.1} MHz")));
    }
    if let Some(t) = &m.timings {
        v.push(f("CAS# latency (CL)", format!("{:.1}", t.cl as f64)));
        v.push(f("RAS# to CAS# delay (tRCD)", t.trcd));
        v.push(f("RAS# Precharge (tRP)", t.trp));
        v.push(f("Cycle Time (tRAS)", t.tras));
        v.push(f("Bank Cycle Time (tRC)", t.trc));
        if let Some(cr) = t.cr {
            v.push(f("Command Rate (CR)", format!("{cr}T")));
        }
        if let Some(rfc) = t.trfc {
            v.push(f("Row Refresh Cycle Time (tRFC)", rfc));
        }
    }
    if let Some(u) = m.uncore_mhz {
        v.push(f("Uncore Frequency", format!("{u:.1} MHz")));
    }
    if v.is_empty() {
        return Vec::new();
    }
    vec![section("Chipset", v)]
}

fn with_rev(name: &str, rev: Option<u8>) -> String {
    rev.map_or_else(|| name.to_string(), |r| format!("{name} rev. {r:02X}"))
}

pub fn summary(ctx: &Ctx) -> ChipsetSummary {
    let devs = pci::devices(ctx);
    let cpu = crate::cpu::get(ctx);
    let amd = cpu.vendor == "AuthenticAMD" || cpu.vendor == "HygonGenuine";
    let mut s = ChipsetSummary::default();

    if let Some(h) = devs.iter().find(|d| d.bdf.bus == 0 && d.bdf.dev == 0 && d.bdf.func == 0 && d.class >> 8 == 0x0600)
    {
        s.nb_rev = Some(h.revision);
        s.northbridge = Some(match h.vendor {
            0x1022 if cpu.sig.0 >= 0x17 => "AMD Ryzen SOC".to_string(),
            0x8086 => cpu.info.codename.as_ref().map_or_else(|| name_or_ids(h), |c| format!("Intel {c}")),
            _ => name_or_ids(h),
        });
    }
    if let Some(l) = devs.iter().find(|d| d.class >> 8 == 0x0601) {
        s.sb_rev = Some(l.revision);
        let ids: Vec<(u16, u16)> = devs.iter().map(|d| (d.vendor, d.device)).collect();
        s.southbridge = Some(match l.vendor {
            0x1022 => amd_chipset(&ids).map_or_else(|| "AMD FCH".to_string(), |c| format!("AMD {c}")),
            0x8086 => intel_pch(l.device)
                .map(str::to_string)
                .or_else(|| l.name.as_deref().and_then(chipset_token))
                .map_or_else(|| name_or_ids(l), |c| format!("Intel {c}")),
            _ => name_or_ids(l),
        });
    }

    s.bus_spec = devs
        .iter()
        .filter(|d| d.pcie_type() == Some(4))
        .filter_map(|d| d.pcie_link())
        .map(|l| l.max_speed)
        .max()
        .filter(|&g| (1..=6).contains(&g))
        .map(|g| format!("PCI-Express {g}.0 ({})", speed_text(g)));

    let gpus: Vec<&Device> = devs.iter().filter(|d| d.class >> 16 == 0x03).collect();
    let main_gpu = gpus.iter().find(|d| !integrated_gpu(devs, d)).or(gpus.first());
    s.gfx_link = main_gpu.and_then(|d| d.pcie_link()).filter(|l| l.max_width > 0);

    s.memory = memory(ctx, amd, cpu.sig.0, cpu.bus_mhz);
    s
}

fn name_or_ids(d: &Device) -> String {
    d.name.clone().unwrap_or_else(|| format!("{} 0x{:04X}", pci::vendor_short(d.vendor).unwrap_or("Vendor"), d.device))
}

/// ponytail: heuristic. iGPUs sit on bus 0 (Intel), behind a root port with
/// no slot, or share their bus with AMD SoC functions (APU PSP/USB/audio).
fn integrated_gpu(devs: &[Device], g: &Device) -> bool {
    if g.bdf.bus == 0 {
        return true;
    }
    if let Some(p) = g.parent.and_then(|p| devs.iter().find(|d| d.bdf == p))
        && p.has_slot() == Some(false)
    {
        return true;
    }
    g.vendor == 0x1002 && devs.iter().any(|d| d.bdf.bus == g.bdf.bus && d.bdf != g.bdf && d.vendor == 0x1022)
}

/// AMD chipset from the Promontory / chipset devices present.
/// ponytail: AM5 X870/X870E vs B650/X670 is inferred from the PROM21 count and
/// the mandatory USB4 host (ASM4242); B850 and B650 are indistinguishable.
pub fn amd_chipset(ids: &[(u16, u16)]) -> Option<&'static str> {
    let has = |v: u16, d: u16| ids.contains(&(v, d));
    let prom21 = ids.iter().filter(|&&x| x == (0x1022, 0x43F4)).count();
    let usb4 = has(0x1B21, 0x2425) || has(0x1B21, 0x2426);
    Some(match prom21 {
        n if n >= 2 => {
            if usb4 {
                "X870E"
            } else {
                "X670"
            }
        }
        1 => {
            if usb4 {
                "X870"
            } else {
                "B650"
            }
        }
        _ if has(0x1022, 0x57AD) => "X570",
        _ if has(0x1022, 0x43E9) || has(0x1022, 0x43EE) => "B550",
        _ if has(0x1022, 0x43C6) || has(0x1022, 0x43D5) => "X470",
        _ if has(0x1022, 0x43B9) || has(0x1022, 0x43B0) => "X370",
        _ if has(0x1022, 0x43BA) || has(0x1022, 0x43B1) => "X399",
        _ if has(0x1022, 0x43BB) || has(0x1022, 0x43B4) => "B350",
        _ => return None,
    })
}

/// Intel PCH LPC/eSPI device IDs (public datasheets / PCI ID lists).
fn intel_pch(dev: u16) -> Option<&'static str> {
    Some(match dev {
        0xA143 => "H110",
        0xA144 => "H170",
        0xA145 => "Z170",
        0xA148 => "B150",
        0xA2C4 => "H270",
        0xA2C5 => "Z270",
        0xA2C8 => "B250",
        0xA2C9 => "Z370",
        0xA2D2 => "X299",
        0xA303 => "H310",
        0xA304 => "H370",
        0xA305 => "Z390",
        0xA306 => "Q370",
        0xA308 => "B360",
        0xA3C8 => "B460",
        0xA3DA => "H410",
        0x0684 => "H470",
        0x0685 => "Z490",
        0x4385 => "Z590",
        0x4386 => "H570",
        0x4387 => "B560",
        0x4388 => "H510",
        0x7A84 => "Z690",
        0x7A85 => "H670",
        0x7A86 => "B660",
        0x7A87 => "H610",
        0x7A04 => "Z790",
        0x7A05 => "H770",
        0x7A06 => "B760",
        _ => return None,
    })
}

/// "Intel(R) Z890 Chipset LPC/eSPI Controller" -> "Z890".
fn chipset_token(name: &str) -> Option<String> {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .find(|w| {
            let b = w.as_bytes();
            (4..=5).contains(&b.len())
                && b[0].is_ascii_uppercase()
                && b[1..4].iter().all(u8::is_ascii_digit)
                && b[4..].iter().all(u8::is_ascii_uppercase)
        })
        .map(str::to_string)
}

// ---- memory ----

/// SMBIOS type 17 entries, populated slots only. Single hook so the source can change.
fn dimms(ctx: &Ctx) -> Vec<crate::smbios::MemDevice> {
    crate::smbios::memory_devices(ctx).into_iter().filter(|m| m.size_mb > 0).collect()
}

fn memory(ctx: &Ctx, amd: bool, family: u32, bus_mhz: Option<f64>) -> Memory {
    let ds = dimms(ctx);
    let mut m = Memory::default();
    m.mem_type =
        ds.iter().map(|d| d.mem_type).find(|t| !t.is_empty() && *t != "Unknown" && *t != "Other").map(str::to_string);
    m.size_mb = ds.iter().map(|d| d.size_mb).sum();
    let ddr5 = m.mem_type.as_deref().map_or(amd && family >= 0x19, |t| t.contains("DDR5"));
    m.max_freq_mhz = ds.iter().map(|d| d.speed).max().filter(|&s| s > 0).map(|s| s as f64 / 2.0);
    m.freq_mhz = ds.iter().map(|d| d.configured_speed).max().filter(|&s| s > 0).map(|s| s as f64 / 2.0);
    let mut nch = ds.len();

    if ctx.cfg.chipset
        && amd
        && matches!(family, 0x17 | 0x19 | 0x1A)
        && let Some(u) = amd_umc(ctx, ddr5)
    {
        nch = u.channels;
        let bclk = bus_mhz.filter(|b| (50.0..200.0).contains(b)).unwrap_or(100.0);
        m.freq_mhz = Some(u.ratio * bclk);
        m.ratio = Some(if (u.ratio - u.ratio.round()).abs() < 0.01 {
            format!("1:{}", u.ratio.round())
        } else {
            format!("3:{}", (u.ratio * 3.0).round())
        });
        m.timings = u.timings;
    }
    if nch > 0 {
        m.channels = Some(if ddr5 {
            format!("{nch} x 32-bit")
        } else {
            match nch {
                1 => "Single".into(),
                2 => "Dual".into(),
                3 => "Triple".into(),
                4 => "Quad".into(),
                n => format!("{n} x 64-bit"),
            }
        });
    }
    #[cfg(target_os = "macos")]
    macos::probe(&mut m);
    m
}

struct Umc {
    channels: usize,
    /// MEMCLK / reference clock.
    ratio: f64,
    timings: Option<Timings>,
}

/// AMD Zen UMC (memory controller) registers over SMN, per the AMD PPRs:
/// UMC n at SMN 0x50000 + n*0x100000; DramConfiguration +0x200, DramTiming1
/// +0x204 (Tcl 5:0, Tras 14:8, Trcdrd 21:16), DramTiming2 +0x208 (Trc 7:0,
/// Trp 21:16), DramTiming (refresh) +0x260 (DDR4 Trfc 10:0).
fn amd_umc(ctx: &Ctx, ddr5: bool) -> Option<Umc> {
    let rd = |ch: u32, off: u32| ctx.hw.amd_smn_read(0x50000 + ch * 0x10_0000 + off).filter(|&v| v != 0xFFFF_FFFF);
    let active: Vec<u32> = (0..12).filter(|&ch| rd(ch, 0x204).is_some_and(|v| v != 0)).collect();
    let ch = *active.first()?;
    let (cfg, t1, t2) = (rd(ch, 0x200)?, rd(ch, 0x204)?, rd(ch, 0x208)?);
    let trfc = if ddr5 { None } else { rd(ch, 0x260) };
    let (ratio, timings) = decode_umc(cfg, t1, t2, trfc, ddr5)?;
    Some(Umc { channels: active.len(), ratio, timings })
}

/// Pure decode of one UMC channel's DramConfiguration/DramTiming1/2 (+ DDR4 refresh).
fn decode_umc(cfg: u32, t1: u32, t2: u32, trfc: Option<u32>, ddr5: bool) -> Option<(f64, Option<Timings>)> {
    // DDR5 parts report MEMCLK in MHz (bits 15:0); DDR4 a ratio * 3 (bits 6:0).
    let ratio = if ddr5 { (cfg & 0xFFFF) as f64 / 100.0 } else { (cfg & 0x7F) as f64 / 3.0 };
    if !(3.0..=60.0).contains(&ratio) {
        return None;
    }
    let t = Timings {
        cl: t1 & 0x3F,
        tras: (t1 >> 8) & 0x7F,
        trcd: (t1 >> 16) & 0x3F,
        trc: t2 & 0xFF,
        trp: (t2 >> 16) & 0x3F,
        cr: (!ddr5).then_some(if cfg & (1 << 10) != 0 { 2 } else { 1 }),
        trfc: trfc.map(|r| r & 0x7FF).filter(|&r| r > 0),
    };
    let sane = (5..=80).contains(&t.cl)
        && (5..=120).contains(&t.trcd)
        && (5..=120).contains(&t.trp)
        && (10..=250).contains(&t.tras)
        && (15..=400).contains(&t.trc);
    Some((ratio, sane.then_some(t)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chipset_names() {
        let x870 = [(0x1022, 0x790E), (0x1022, 0x43F4), (0x1022, 0x43F5), (0x1B21, 0x2425)];
        assert_eq!(amd_chipset(&x870), Some("X870"));
        assert_eq!(amd_chipset(&[(0x1022, 0x43F4), (0x1022, 0x43F4)]), Some("X670"));
        assert_eq!(amd_chipset(&[(0x1022, 0x57AD)]), Some("X570"));
        assert_eq!(amd_chipset(&[(0x1022, 0x790E)]), None);
        assert_eq!(chipset_token("Intel(R) Z890 Chipset LPC/eSPI Controller - 7E04"), Some("Z890".into()));
        assert_eq!(chipset_token("PCI standard ISA bridge"), None);
        assert_eq!(with_rev("AMD X870", Some(0x51)), "AMD X870 rev. 51");
    }

    #[test]
    fn umc() {
        // Ryzen 9 7900X, DDR5-6000 CL30-36-36-76, tRC 112 (verified against CPU-Z).
        let t1 = 30 | 76 << 8 | 36 << 16 | 36 << 24;
        let (r, t) = decode_umc(3000, t1, 112 | 36 << 16, None, true).unwrap();
        let t = t.unwrap();
        assert_eq!(r, 30.0);
        assert_eq!((t.cl, t.trcd, t.trp, t.tras, t.trc, t.cr, t.trfc), (30, 36, 36, 76, 112, None, None));
        // DDR4-3200 (ratio 16 -> raw 48), 2T, tRFC 560.
        let (r, t) = decode_umc(48 | 1 << 10, 16 | 36 << 8 | 18 << 16, 54 | 18 << 16, Some(560), false).unwrap();
        assert_eq!(r, 16.0);
        let t = t.unwrap();
        assert_eq!((t.cl, t.cr, t.trfc), (16, Some(2), Some(560)));
        assert!(decode_umc(0, 0, 0, None, true).is_none());
    }
}

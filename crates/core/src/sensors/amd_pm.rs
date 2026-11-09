//! AMD SMU PM ("metrics") table decoding.
//!
//! AMD does not document these tables. The layouts below are f32 cell
//! indices gathered from public community work (facts only, no code taken):
//! ryzen_monitor / ryzen_monitor_ng (Zen 2/3 desktop, Renoir, Cezanne),
//! RyzenAdj `lib/api.c` (APU limits, Renoir/Cezanne/Strix cores),
//! ZenStates-Core `Hardware/PowerTable.cs` (fabric clocks and SVI3 rails,
//! Zen 2-5), LibreHardwareMonitor `RyzenSMU.cs` (Zen 4/5), plus the CPU-Z
//! report of a Ryzen 9 7900X (table 0x540004) matched value by value.
//! A version that is not listed decodes to nothing.

/// Index of a limit and, when known, of its current value.
#[derive(Clone, Copy)]
struct Lim(usize, Option<usize>);

/// Per-core blocks: `slots` consecutive cells each, split evenly into
/// `groups` L3 domains (CCX on Zen 2, CCD on Zen 3+).
#[derive(Clone, Copy)]
struct Cores {
    slots: usize,
    groups: usize,
    power: usize,
    volt: usize,
    temp: usize,
    eff: Option<usize>,
}

#[derive(Clone, Copy)]
struct Layout {
    /// Table size in cells; shorter tables are not this layout.
    size: usize,
    ppt: Option<Lim>,
    tdc: Option<Lim>,
    edc: Option<Lim>,
    thm: Option<Lim>,
    vid: Option<usize>,
    vdd: Option<usize>,
    soc: Option<usize>,
    misc: Option<usize>,
    socket_w: Option<usize>,
    vdd_w: Option<usize>,
    fclk: Option<usize>,
    uclk: Option<usize>,
    mclk: Option<usize>,
    soc_temp: Option<usize>,
    pbo: Option<usize>,
    /// Max core clock in GHz: one cell, or a per-slot block.
    cclk_max: Option<usize>,
    fmax_block: Option<usize>,
    cores: Option<Cores>,
    l3_temp: Option<usize>,
    l3_clk: Option<usize>,
    l3_n: usize,
}

const NONE: Layout = Layout {
    size: 0,
    ppt: None,
    tdc: None,
    edc: None,
    thm: None,
    vid: None,
    vdd: None,
    soc: None,
    misc: None,
    socket_w: None,
    vdd_w: None,
    fclk: None,
    uclk: None,
    mclk: None,
    soc_temp: None,
    pbo: None,
    cclk_max: None,
    fmax_block: None,
    cores: None,
    l3_temp: None,
    l3_clk: None,
    l3_n: 0,
};

const fn lim(l: usize, v: usize) -> Option<Lim> {
    Some(Lim(l, Some(v)))
}

/// Matisse / Vermeer desktop head (ryzen_monitor; clocks + SOC agree with ZenStates).
const ZEN23: Layout = Layout {
    ppt: lim(0, 1),
    tdc: lim(2, 3),
    thm: lim(4, 5),
    edc: lim(8, 9),
    vdd_w: Some(24),
    socket_w: Some(29),
    soc: Some(45),
    fclk: Some(48),
    uclk: Some(50),
    mclk: Some(51),
    ..NONE
};

/// Mobile/APU head, Renoir..Phoenix (RyzenAdj: slow PPT, VRM TDC/EDC, Tctl).
const APU: Layout = Layout { ppt: lim(4, 5), tdc: lim(8, 9), edc: lim(12, 13), thm: lim(16, 17), ..NONE };

/// Raphael / Dragon Range head. SVI3 rails at 46 (VID, V, A, W, degC each);
/// clocks per ZenStates/LHM; limits and powers matched against the reference report.
const ZEN4: Layout = Layout {
    ppt: lim(2, 3),
    tdc: lim(8, 9),
    thm: lim(10, 11),
    edc: lim(61, 62),
    vdd_w: Some(20),
    socket_w: Some(26),
    vdd: Some(47),
    soc: Some(52),
    misc: Some(57),
    fclk: Some(70),
    uclk: Some(74),
    mclk: Some(78),
    ..NONE
};

/// Granite Ridge head: rails at 48 (ZenStates), limits per LHM.
const ZEN5: Layout = Layout {
    ppt: lim(2, 3),
    tdc: lim(8, 9),
    thm: lim(10, 11),
    edc: Some(Lim(63, None)),
    vdd: Some(49),
    soc: Some(54),
    misc: Some(59),
    fclk: Some(71),
    uclk: Some(75),
    mclk: Some(79),
    ..NONE
};

const fn zen23(size: usize, vid: usize, soc_t: usize, cclk: usize, c: Cores, l3: Option<(usize, usize)>) -> Layout {
    let (l3_temp, l3_clk, l3_n) = match l3 {
        Some((t, k)) => (Some(t), Some(k), c.groups),
        None => (None, None, 0),
    };
    Layout {
        size,
        vid: Some(vid),
        vdd: Some(vid + 1),
        soc_temp: Some(soc_t),
        cclk_max: Some(cclk),
        cores: Some(c),
        l3_temp,
        l3_clk,
        l3_n,
        ..ZEN23
    }
}

const fn cores(slots: usize, groups: usize, power: usize, volt: usize, temp: usize, eff: usize) -> Cores {
    Cores { slots, groups, power, volt, temp, eff: Some(eff) }
}

fn layout(ver: u32) -> Option<Layout> {
    let renoir = |c: Cores, l3: usize, fclk: usize| Layout {
        size: 555,
        vid: Some(97),
        vdd: Some(98),
        soc: Some(102),
        vdd_w: Some(34),
        socket_w: Some(38),
        cores: Some(c),
        l3_temp: Some(l3),
        l3_clk: Some(l3 + 6),
        l3_n: 1,
        fclk: Some(fclk),
        uclk: Some(fclk + 1),
        mclk: Some(fclk + 2),
        ..APU
    };
    let zen4_size = |s: usize| Layout { size: s, ..ZEN4 };
    Some(match ver {
        // Zen 2 Matisse: 0x2409 = one CCD (2 CCX), 0x2408 = two CCDs (4 CCX).
        0x240903 => zen23(326, 39, 115, 130, cores(8, 2, 147, 155, 163, 195), Some((303, 309))),
        0x240803 => zen23(505, 40, 115, 130, cores(16, 4, 147, 163, 179, 243), Some((459, 471))),
        // Zen 3 Vermeer: 0x3808 = two CCDs, 0x3809 = one.
        0x380804 => zen23(553, 40, 127, 144, cores(16, 2, 169, 185, 201, 265), Some((525, 531))),
        0x380805 => zen23(572, 39, 127, 147, cores(16, 2, 172, 188, 204, 268), Some((544, 550))),
        0x380904 => zen23(361, 40, 127, 144, cores(8, 1, 169, 177, 185, 217), None),
        0x380905 => zen23(372, 39, 127, 147, cores(8, 1, 172, 180, 188, 220), None),
        // Zen 2 Renoir / Lucienne.
        0x370003 | 0x370004 => renoir(cores(8, 1, 192, 200, 208, 240), 340, 371),
        0x370005 => Layout { size: 562, ..renoir(cores(8, 1, 199, 207, 215, 247), 347, 378) },
        // Zen 3 Cezanne / Barcelo.
        0x400004 | 0x400005 => Layout {
            size: 593,
            vid: Some(98),
            vdd: Some(99),
            soc: Some(103),
            vdd_w: Some(34),
            socket_w: Some(38),
            cores: Some(cores(8, 1, 200, 208, 216, 248)),
            l3_temp: Some(386),
            l3_clk: Some(389),
            l3_n: 1,
            fclk: Some(409),
            uclk: Some(410),
            mclk: Some(411),
            ..APU
        },
        // Zen 3+ Rembrandt.
        0x450004 => Layout { size: 681, soc: Some(103), fclk: Some(409), uclk: Some(410), mclk: Some(411), ..APU },
        0x450005 => Layout { size: 684, soc: Some(114), fclk: Some(428), uclk: Some(429), mclk: Some(430), ..APU },
        // Zen 4 Phoenix / Hawk Point: SVI3 rails at 105 (4 cells each on 0x4C0008, 5 on 0x4C0009).
        0x4C0006 => Layout { size: 703, ..APU },
        0x4C0007 => Layout { size: 704, ..APU },
        0x4C0008 | 0x4C0009 => Layout {
            size: if ver == 0x4C0008 { 700 } else { 704 },
            vdd: Some(106),
            soc: Some(if ver == 0x4C0008 { 110 } else { 111 }),
            fclk: Some(89),
            uclk: Some(93),
            mclk: Some(97),
            ..APU
        },
        // Zen 4 Raphael: 0x5400xx two CCDs, 0x5401xx one. Only 0x540004 has a
        // verified per-core/L3 body (the reference report).
        0x540004 => Layout {
            size: 559,
            soc_temp: Some(211),
            pbo: Some(24),
            fmax_block: Some(437),
            cores: Some(Cores { slots: 16, groups: 2, power: 293, volt: 309, temp: 325, eff: Some(357) }),
            l3_temp: Some(539),
            l3_clk: Some(541),
            l3_n: 2,
            ..ZEN4
        },
        0x540000 => zen4_size(522),
        0x540001 => zen4_size(523),
        0x540002 => zen4_size(543),
        0x540003 => zen4_size(551),
        0x540005 => zen4_size(562),
        0x540100 => zen4_size(390),
        0x540101 => zen4_size(391),
        0x540102 => zen4_size(411),
        0x540103 => zen4_size(419),
        0x540104 => zen4_size(426),
        0x540105 => zen4_size(429),
        0x540108 => zen4_size(431),
        // Zen 5 Granite Ridge: 0x6202/0x6212 two CCDs, 0x6201/0x6211 one.
        0x620105 | 0x621102 => Layout { size: 457, ..ZEN5 },
        0x620205 => Layout { size: 613, ..ZEN5 },
        0x621202 => Layout { size: 613, l3_temp: Some(595), l3_n: 2, ..ZEN5 },
        // Zen 5 Strix Point (RyzenAdj): 12 core slots, disabled ones read 0 V.
        0x5D0008 | 0x5D0009 | 0x5D000B => Layout {
            size: 853,
            ppt: lim(4, 5),
            tdc: lim(12, 13),
            thm: lim(16, 17),
            socket_w: Some(52),
            fclk: Some(312),
            uclk: Some(313),
            cores: Some(Cores { slots: 12, groups: 1, power: 630, volt: 642, temp: 654, eff: None }),
            ..NONE
        },
        // Zen 5 Strix Halo (RyzenAdj).
        0x64020C => Layout {
            size: 916,
            ppt: lim(4, 5),
            thm: lim(22, 23),
            cores: Some(Cores { slots: 16, groups: 2, power: 740, volt: 756, temp: 772, eff: None }),
            ..NONE
        },
        // Zen 5 Krackan Point (RyzenAdj).
        0x650005 => Layout { size: 734, ppt: lim(4, 5), tdc: lim(12, 13), thm: lim(16, 17), ..NONE },
        _ => return None,
    })
}

/// A limit with its current value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limit {
    pub limit: f64,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Core {
    pub temp: f64,
    pub power: f64,
    pub volt: f64,
    /// Effective (C-state weighted) clock, MHz.
    pub eff_mhz: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct L3 {
    pub temp: Option<f64>,
    pub mhz: Option<f64>,
}

/// Data-fabric, memory-controller and memory clocks, MHz.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FabricClocks {
    pub fclk: Option<f64>,
    pub uclk: Option<f64>,
    pub mclk: Option<f64>,
}

/// Decoded table. Units: W, A, degC, V, MHz.
#[derive(Debug, Clone, Default)]
pub struct PmData {
    pub ppt: Option<Limit>,
    pub tdc: Option<Limit>,
    pub edc: Option<Limit>,
    pub thm: Option<Limit>,
    pub vid: Option<f64>,
    pub vddcr_vdd: Option<f64>,
    pub vddcr_soc: Option<f64>,
    pub vdd_misc: Option<f64>,
    /// Whole-socket power (what PPT tracks).
    pub socket_power: Option<f64>,
    /// VDDCR_VDD rail (cores) power.
    pub cores_power: Option<f64>,
    pub clocks: FabricClocks,
    pub soc_temp: Option<f64>,
    pub pbo_scalar: Option<f64>,
    pub max_mhz: Option<f64>,
    /// Enabled cores only, in core order.
    pub cores: Vec<Core>,
    pub l3: Vec<L3>,
}

/// Decode table `t` of version `ver`. `per_group` is the number of enabled
/// cores in each L3 domain (CCX/CCD) in order; empty when unknown, then
/// disabled slots are told apart by their 0 V core voltage.
pub fn decode(ver: u32, t: &[f32], per_group: &[usize]) -> Option<PmData> {
    let l = layout(ver)?;
    if t.len() < l.size {
        return None;
    }
    let get = |i: usize| t.get(i).map(|&v| v as f64).filter(|v| v.is_finite());
    let pos = |i: Option<usize>| i.and_then(get).filter(|&v| v > 0.0);
    let limit = |x: Option<Lim>| {
        let Lim(li, vi) = x?;
        Some(Limit { limit: pos(Some(li))?, value: vi.and_then(get) })
    };
    let mut d = PmData {
        ppt: limit(l.ppt),
        tdc: limit(l.tdc),
        edc: limit(l.edc),
        thm: limit(l.thm),
        vid: pos(l.vid),
        vddcr_vdd: pos(l.vdd),
        vddcr_soc: pos(l.soc),
        vdd_misc: pos(l.misc),
        socket_power: l.socket_w.and_then(get),
        cores_power: l.vdd_w.and_then(get),
        clocks: FabricClocks { fclk: pos(l.fclk), uclk: pos(l.uclk), mclk: pos(l.mclk) },
        soc_temp: pos(l.soc_temp),
        pbo_scalar: pos(l.pbo),
        max_mhz: pos(l.cclk_max).map(|g| g * 1000.0),
        ..Default::default()
    };
    let mut present = Vec::new();
    if let Some(c) = l.cores {
        let slots = enabled_slots(t, &c, per_group);
        for &s in &slots {
            d.cores.push(Core {
                temp: get(c.temp + s).unwrap_or(0.0),
                power: get(c.power + s).unwrap_or(0.0),
                volt: get(c.volt + s).unwrap_or(0.0),
                eff_mhz: c.eff.and_then(|e| get(e + s)).map(|g| g * 1000.0),
            });
        }
        let per = c.slots / c.groups;
        present = (0..c.groups).map(|g| slots.iter().any(|s| s / per == g)).collect();
        if let Some(b) = l.fmax_block {
            // Per-core Fmax cells wobble a little with the boost curve; the
            // value most cores report is the fused maximum (CPU-Z's figure).
            let mhz: Vec<i64> =
                slots.iter().filter_map(|s| pos(Some(b + s))).map(|g| (g * 1000.0).round() as i64).collect();
            d.max_mhz = mhz
                .iter()
                .max_by_key(|m| (mhz.iter().filter(|x| x == m).count(), std::cmp::Reverse(**m)))
                // Shown in multiplier steps (25 MHz), as CPU-Z does.
                .map(|&m| (m as f64 / 25.0).round() * 25.0);
        }
    }
    for i in 0..l.l3_n {
        // Skip the L3 of an absent CCD/CCX when the core blocks say which ones exist.
        if present.get(i) == Some(&false) {
            continue;
        }
        let x = L3 { temp: pos(l.l3_temp.map(|b| b + i)), mhz: pos(l.l3_clk.map(|b| b + i)).map(|g| g * 1000.0) };
        if x.temp.is_some() || x.mhz.is_some() {
            d.l3.push(x);
        }
    }
    Some(d)
}

/// Which core slots are real. Each L3 domain has its own block of slots and
/// fuses off its own cores, so pick the `n` most alive slots per domain.
// ponytail: liveness = nonzero core voltage, then power; a fused core never
// reports voltage, but an enabled core that also reads 0 V can swap places
// with a fused one. Read the core-disable fuses if that shows up.
fn enabled_slots(t: &[f32], c: &Cores, per_group: &[usize]) -> Vec<usize> {
    let per = c.slots / c.groups;
    let at = |b: usize, s: usize| t.get(b + s).copied().filter(|v| v.is_finite()).unwrap_or(0.0);
    let counts: Vec<usize> = match per_group {
        g if g.len() == c.groups && g.iter().all(|&n| n <= per) => g.to_vec(),
        g if c.groups == 1 && !g.is_empty() && g.iter().sum::<usize>() <= per => vec![g.iter().sum()],
        _ => (0..c.groups).map(|g| (0..per).filter(|&s| at(c.volt, g * per + s) > 0.0).count()).collect(),
    };
    let mut out = Vec::new();
    for (g, &n) in counts.iter().enumerate() {
        let mut s: Vec<usize> = (g * per..(g + 1) * per).collect();
        let alive = |s: usize| (at(c.volt, s) > 0.0, at(c.power, s) > 0.0);
        s.sort_by_key(|&s| std::cmp::Reverse(alive(s))); // stable: slot order within ties
        s.truncate(n);
        s.sort_unstable();
        out.extend(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cell indices of a layout, for the bounds check.
    fn indices(l: &Layout) -> Vec<usize> {
        let mut v: Vec<usize> = [l.ppt, l.tdc, l.edc, l.thm]
            .into_iter()
            .flatten()
            .flat_map(|Lim(a, b)| [Some(a), b])
            .chain([l.vid, l.vdd, l.soc, l.misc, l.socket_w, l.vdd_w, l.fclk, l.uclk, l.mclk, l.soc_temp, l.pbo])
            .chain([l.cclk_max])
            .flatten()
            .collect();
        if let Some(c) = l.cores {
            for b in [Some(c.power), Some(c.volt), Some(c.temp), c.eff, l.fmax_block].into_iter().flatten() {
                v.push(b + c.slots - 1);
            }
        }
        for b in [l.l3_temp, l.l3_clk].into_iter().flatten() {
            v.push(b + l.l3_n - 1);
        }
        v
    }

    #[test]
    fn layouts_fit_their_tables() {
        for ver in (0x240000..0x660000u32).filter(|&v| layout(v).is_some()) {
            let l = layout(ver).unwrap();
            assert!(l.size > 0, "{ver:#x}: no size");
            for i in indices(&l) {
                assert!(i < l.size, "{ver:#x}: cell {i} >= size {}", l.size);
            }
            if let Some(c) = l.cores {
                assert_eq!(c.slots % c.groups, 0, "{ver:#x}");
            }
        }
        assert!(layout(0x380005).is_none() && layout(0x540208).is_none(), "unlisted versions must decode to nothing");
    }

    /// The reference CPU-Z report's PM table (Ryzen 9 7900X, 6+6 cores).
    const RAPHAEL: &str = "
        0.00 0.00 230.00 83.59 0.00 0.00 0.00 0.00 160.00 35.00
        95.00 63.84 0.00 0.00 0.00 0.00 5371.21 2760.29 1.42 1.36
        48.36 20.42 8.19 4.64 1.00 1.00 83.60 5.70 5.70 5.70
        5.70 5.70 5.70 5.70 5.70 5.44 5.70 5.53 1.42 1.45
        1.52 1.47 1.52 1.52 1.47 1.47 1.36 1.36 35.01 48.36
        59.00 1.30 1.30 15.73 20.42 52.12 1.10 1.10 7.45 8.19
        52.00 225.00 78.50 40.36 208.00 0.00 100.00 0.00 0.00 0.00
        2000.00 0.00 0.00 0.00 3000.00 0.00 0.00 0.00 3000.00 0.00
        0.00 0.00 1.30 0.00 0.00 0.00 1.51 0.53 0.00 0.00
        0.00 0.00 0.00 0.00 0.02 1.51 0.52 0.72 58.29 0.00
        599.99 0.00 0.00 100.00 100.00 2000.00 3000.00 3000.00 0.00 0.00
        1200.00 1028.59 600.00 400.00 1000.00 600.00 600.00 2000.00 3000.00 3000.00
        0.78 0.78 1200.00 1028.59 10.40 3.20 1028.59 600.00 600.00 1200.00
        1200.00 1200.00 400.00 1028.57 1028.57 1028.57 300.00 1200.00 1200.00 1200.00
        400.00 1200.00 1200.00 1028.57 400.00 600.00 600.00 600.00 200.00 600.00
        600.00 600.00 400.00 1000.00 1000.00 1000.00 1000.00 600.00 600.00 600.00
        200.00 1028.57 1028.57 1028.57 600.00 0.95 0.95 0.86 0.80 0.00
        0.00 0.00 0.00 0.00 0.00 100.00 0.00 0.00 0.00 100.00
        0.00 0.00 0.00 100.00 0.00 0.00 0.00 0.00 100.00 0.00
        0.00 0.00 100.00 0.00 0.00 100.00 0.00 0.00 0.00 0.00
        100.00 0.50 0.63 12.80 0.00 4.00 0.01 0.00 4.64 0.63
        0.00 60.18 400.00 400.00 400.00 197.93 400.00 400.00 400.00 400.00
        0.60 0.00 48.00 48.00 200.00 200.00 600.00 600.00 600.00 600.00
        600.00 600.00 600.00 600.00 300.00 300.00 60.00 60.00 600.00 600.00
        3.12 3.12 0.00 0.00 0.00 0.00 25.00 25.00 25.00 0.00
        0.00 0.00 0.00 0.00 0.00 0.00 0.00 0.00 0.00 0.00
        0.00 0.00 32.00 16.00 8.00 4.00 0.01 170.00 1.05 0.85
        80.75 1.47 5.57 1.21 5.41 1.35 0.01 83.18 0.00 0.00
        14.44 0.00 0.00 0.00 43.22 45.57 0.00 0.00 0.00 0.00
        0.00 0.00 0.00 5.95 5.07 0.00 0.00 3.86 3.02 2.23
        1.45 0.21 0.35 0.00 0.00 0.11 0.53 0.09 0.24 1.25
        1.24 0.00 0.00 1.15 1.09 1.09 1.09 0.93 0.97 0.00
        0.00 0.91 1.04 0.92 0.94 59.66 57.15 50.49 49.73 55.56
        52.82 53.02 50.61 45.96 46.11 45.45 45.63 45.65 45.85 45.85
        45.74 5.06 5.06 0.00 0.00 4.58 4.39 4.33 4.29 3.86
        4.00 0.00 0.00 3.81 4.21 3.80 3.88 2.14 1.57 0.00
        0.00 0.81 0.49 0.30 0.16 0.02 0.04 0.00 0.00 0.00
        0.07 0.00 0.02 41.54 30.55 0.00 0.00 17.00 11.36 7.09
        4.56 1.39 1.93 0.00 0.00 0.83 2.57 0.69 1.12 58.46
        69.45 0.00 0.00 82.99 81.75 61.88 42.50 8.19 14.18 0.00
        0.00 3.10 22.48 2.25 10.01 0.00 0.00 100.00 100.00 0.01
        6.89 31.03 52.94 90.42 83.89 100.00 100.00 96.07 74.94 97.07
        88.88 0.00 0.00 0.00 0.00 0.00 0.00 0.00 0.00 0.00
        0.00 0.00 0.00 0.00 0.00 0.00 0.00 5.70 5.70 5.70
        5.70 5.70 5.70 5.70 5.70 5.70 5.70 5.70 5.70 5.70
        5.70 5.70 5.70 2.99 2.99 0.40 0.40 2.99 2.99 2.99
        2.99 2.99 2.99 0.40 0.40 2.99 2.99 2.99 2.99 3.20
        2.51 0.00 0.00 1.36 0.88 0.42 0.19 0.06 0.07 0.00
        0.00 0.02 0.11 0.02 0.03 2556.47 1491.15 0.00 0.00 668.93
        379.61 246.76 120.84 13.50 26.21 0.00 0.00 2.00 40.28 0.89
        8.93 0.30 0.22 0.00 0.00 0.14 0.08 0.06 0.03 0.00
        0.01 0.00 0.00 0.00 0.02 0.00 0.01 0.12 0.09 0.00
        0.00 0.03 0.03 0.02 0.01 0.01 0.01 0.00 0.00 0.01
        0.01 0.01 0.01 4.24 2.98 0.63 0.58 0.95 0.95 51.11
        46.89 5.23 4.28 5.23 4.28 292.08 48.65 6.22 0.49 0.00
        0.14 2.39 0.12 1.00 1.00 5.52 5.45 31.23 32.53 -10440.74";

    pub(crate) fn cells(s: &str) -> Vec<f32> {
        s.split_whitespace().map(|x| x.parse().unwrap()).collect()
    }

    fn close(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|a| (a - b).abs() < 0.006)
    }

    #[test]
    fn raphael_matches_cpuz_report() {
        let t = cells(RAPHAEL);
        assert_eq!(t.len(), 560);
        for groups in [&[6usize, 6][..], &[]] {
            let d = decode(0x540004, &t, groups).unwrap();
            // Processors Information: PPT/TDC/EDC 230/160/225, THM 95 (TDP/TJMAX 0x..005F).
            assert_eq!(d.ppt, Some(Limit { limit: 230.0, value: Some(83.59f32 as f64) }));
            assert_eq!((d.tdc.unwrap().limit, d.edc.unwrap().limit, d.thm.unwrap().limit), (160.0, 225.0, 95.0));
            // VDDCR_VDD 1.36, VDDCR_SOC 1.30, VDD_MISC 1.10.
            assert!(close(d.vddcr_vdd, 1.36) && close(d.vddcr_soc, 1.30) && close(d.vdd_misc, 1.10));
            // DDR5-6000 1:1, FCLK 2000 (PCI host bridge: Memory 2998.8 MHz, Uncore 2997.3 MHz).
            assert_eq!(d.clocks, FabricClocks { fclk: Some(2000.0), uclk: Some(3000.0), mclk: Some(3000.0) });
            assert!(close(d.max_mhz, 5700.0) && close(d.pbo_scalar, 1.0));
            // Temperature 4..15 (Core #0..#11) with raw codes 0x3B 0x39 0x37 0x34 0x35 0x32 0x2D 0x2E 0x2D...
            let temps: Vec<u32> = d.cores.iter().map(|c| c.temp as u32).collect();
            assert_eq!(temps, [0x3B, 0x39, 0x37, 0x34, 0x35, 0x32, 0x2D, 0x2E, 0x2D, 0x2D, 0x2D, 0x2D], "{groups:?}");
            let rounded: Vec<i32> = d.cores.iter().map(|c| c.temp.round() as i32).collect();
            assert_eq!(rounded, [60, 57, 56, 53, 53, 51, 46, 46, 46, 46, 46, 46]);
            assert!(d.cores.iter().all(|c| c.volt > 0.8 && c.power > 0.0));
            assert!(close(d.cores[0].eff_mhz.map(|m| m / 1000.0), 2.14));
            // L3 Cache #0/#1: 51/47 degC, 5229.61/4275.12 MHz; SoC 60 degC [0x3C].
            assert_eq!(d.l3.len(), 2);
            assert_eq!(d.l3.iter().map(|l| l.temp.unwrap().round() as i32).collect::<Vec<_>>(), [51, 47]);
            assert!(close(d.l3[0].mhz.map(|m| m / 1000.0), 5.23) && close(d.l3[1].mhz.map(|m| m / 1000.0), 4.28));
            assert_eq!(d.soc_temp.map(|t| t as u32), Some(0x3C));
        }
    }

    #[test]
    fn raphael_live_capture() {
        // First 60 cells captured live on the same machine (elevated, PawnIO).
        let mut t = cells(
            "0.00 0.00 230.00 96.04 0.00 0.00 0.00 0.00 160.00 44.77 95.00 76.80 0 0 0 0 3149.79 566.43 1.35 1.34 \
             59.63 21.48 8.22 4.70 1.00 1.00 96.03 5.70 5.70 5.70 5.70 5.70 5.70 5.70 5.70 5.23 5.70 5.51 1.35 1.37 \
             1.52 1.47 1.52 1.52 1.47 1.45 1.34 1.33 44.76 59.63 67.98 1.30 1.30 16.56 21.48 59.73 1.10 1.10 7.48 8.22",
        );
        assert!(decode(0x540004, &t, &[]).is_none(), "truncated table is not this layout");
        t.resize(559, 0.0);
        let d = decode(0x540004, &t, &[6, 6]).unwrap();
        assert_eq!((d.ppt.unwrap().limit, d.tdc.unwrap().limit, d.thm.unwrap().limit), (230.0, 160.0, 95.0));
        assert!(close(d.ppt.unwrap().value, 96.04) && close(d.tdc.unwrap().value, 44.77));
        assert!(close(d.vddcr_vdd, 1.33) && close(d.vddcr_soc, 1.30) && close(d.vdd_misc, 1.10));
        // Socket power tracks PPT; the VDD rail's power is its V x A.
        assert!(close(d.socket_power, 96.03) && close(d.cores_power, 59.63));
        assert!(d.edc.is_none() && d.clocks.fclk.is_none() && d.l3.is_empty(), "zeroed cells are not values");
    }

    #[test]
    fn unknown_and_disabled() {
        let t = cells(RAPHAEL);
        assert!(decode(0x540208, &t, &[]).is_none());
        assert!(decode(0x123456, &t, &[]).is_none());
        // 0x540104 shares only the head: no per-core guesses.
        let d = decode(0x540104, &t, &[]).unwrap();
        assert!(d.cores.is_empty() && d.l3.is_empty() && d.soc_temp.is_none() && d.ppt.is_some());
        // Voltage-based fallback on a Zen 3 shape: slots 2 and 5 fused off.
        let mut z3 = vec![0f32; 361];
        for s in [0usize, 1, 3, 4, 6, 7] {
            z3[177 + s] = 1.2;
            z3[185 + s] = 40.0 + s as f32;
        }
        let d = decode(0x380904, &z3, &[]).unwrap();
        assert_eq!(d.cores.iter().map(|c| c.temp as u32).collect::<Vec<_>>(), [40, 41, 43, 44, 46, 47]);
        assert_eq!(decode(0x380904, &z3, &[6]).unwrap().cores.len(), 6);
    }
}

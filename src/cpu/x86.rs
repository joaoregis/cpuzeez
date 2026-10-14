//! CPUID decoding (Intel SDM vol. 2A "CPUID", AMD APM vol. 3 appendix E).

#[cfg(target_arch = "x86")]
use core::arch::x86::__cpuid_count;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::__cpuid_count;

pub type Regs = [u32; 4]; // eax, ebx, ecx, edx

pub fn cpuid(leaf: u32, sub: u32) -> Regs {
    let r = __cpuid_count(leaf, sub);
    [r.eax, r.ebx, r.ecx, r.edx]
}

fn bit(v: u32, n: u32) -> bool {
    v >> n & 1 == 1
}
fn bits(v: u32, lo: u32, hi: u32) -> u32 {
    (v >> lo) & ((1u64 << (hi - lo + 1)) - 1) as u32
}

/// Every leaf/subleaf CPU-Z prints in "Thread dumps", in the same order.
pub fn dump_leaves() -> Vec<(u32, Regs)> {
    let mut out = Vec::new();
    for base in [0u32, 0x4000_0000, 0x8000_0000] {
        let max = cpuid(base, 0)[0];
        if base == 0x4000_0000 && !(0x4000_0000..0x4000_0100).contains(&max) {
            continue; // no hypervisor
        }
        if base != 0x4000_0000 && (max < base || max > base + 0xFF) {
            continue;
        }
        for leaf in base..=max {
            for r in subleaves(leaf) {
                out.push((leaf, r));
            }
        }
    }
    out
}

fn subleaves(leaf: u32) -> Vec<Regs> {
    let first = cpuid(leaf, 0);
    let mut v = vec![first];
    let mut more = |pred: &dyn Fn(u32, Regs) -> bool, cap: u32| {
        for s in 1..cap {
            let r = cpuid(leaf, s);
            if !pred(s, r) {
                break;
            }
            v.push(r);
        }
    };
    match leaf {
        4 | 0x8000_001D => more(&|_, r| r[0] & 0x1F != 0, 16),
        7 | 0x14 | 0x17 | 0x18 | 0x1D | 0x1E | 0x24 => {
            let max = first[0];
            more(&move |s, _| s <= max, 32)
        }
        0xB | 0x1F | 0x8000_0026 => more(&|_, r| bits(r[2], 8, 15) != 0, 8),
        0xD => more(&|s, _| s == 1, 2),
        0x10 | 0x12 | 0xF => more(&|s, _| s <= 3, 4),
        _ => {}
    }
    v
}

#[derive(Debug, Clone, Default)]
pub struct Signature {
    pub family: u32,
    pub model: u32,
    pub stepping: u32,
    pub base_family: u32,
    pub base_model: u32,
    pub ext_family: u32,
    pub ext_model: u32,
    pub raw: u32,
}

pub fn signature() -> Signature {
    let a = cpuid(1, 0)[0];
    let (bf, bm, st) = (bits(a, 8, 11), bits(a, 4, 7), bits(a, 0, 3));
    let (ef, em) = (bits(a, 20, 27), bits(a, 16, 19));
    let family = if bf == 0xF { bf + ef } else { bf };
    let model = if bf == 0x6 || bf == 0xF { (em << 4) | bm } else { bm };
    Signature { family, model, stepping: st, base_family: bf, base_model: bm, ext_family: ef, ext_model: em, raw: a }
}

pub fn vendor() -> String {
    let r = cpuid(0, 0);
    let b: Vec<u8> = [r[1], r[3], r[2]].iter().flat_map(|x| x.to_le_bytes()).collect();
    String::from_utf8_lossy(&b).into_owned()
}

pub fn brand() -> Option<String> {
    if cpuid(0x8000_0000, 0)[0] < 0x8000_0004 {
        return None;
    }
    let b: Vec<u8> = (0x8000_0002..=0x8000_0004u32)
        .flat_map(|l| cpuid(l, 0))
        .flat_map(|x| x.to_le_bytes())
        .take_while(|&c| c != 0)
        .collect();
    let s = String::from_utf8_lossy(&b).trim().to_string();
    (!s.is_empty()).then_some(s)
}

pub fn max_leaf() -> u32 {
    cpuid(0, 0)[0]
}
pub fn max_ext_leaf() -> u32 {
    cpuid(0x8000_0000, 0)[0]
}

pub fn hypervisor() -> Option<String> {
    if !bit(cpuid(1, 0)[2], 31) {
        return None;
    }
    let r = cpuid(0x4000_0000, 0);
    let b: Vec<u8> = [r[1], r[2], r[3]].iter().flat_map(|x| x.to_le_bytes()).filter(|&c| c != 0).collect();
    Some(String::from_utf8_lossy(&b).trim().to_string())
}

/// CPU-Z style instruction-set list.
pub fn instructions(vendor: &str) -> Vec<String> {
    let amd = vendor == "AuthenticAMD" || vendor == "HygonGenuine";
    let l1 = cpuid(1, 0);
    let (c1, d1) = (l1[2], l1[3]);
    let max = max_leaf();
    let l7 = if max >= 7 { cpuid(7, 0) } else { [0; 4] };
    let l7_1 = if max >= 7 && l7[0] >= 1 { cpuid(7, 1) } else { [0; 4] };
    let (b7, c7, d7, a71, d71) = (l7[1], l7[2], l7[3], l7_1[0], l7_1[3]);
    let e1 = if max_ext_leaf() >= 0x8000_0001 { cpuid(0x8000_0001, 0) } else { [0; 4] };
    let (ce, de) = (e1[2], e1[3]);

    let mut v: Vec<String> = Vec::new();
    let mut push = |cond: bool, s: &str| {
        if cond {
            v.push(s.to_string())
        }
    };
    push(bit(d1, 23), if amd && bit(de, 22) { "MMX (+)" } else { "MMX" });
    push(amd && bit(de, 31), if bit(de, 30) { "3DNow! (+)" } else { "3DNow!" });
    push(bit(d1, 25), "SSE");
    push(bit(d1, 26), "SSE2");
    push(bit(c1, 0), "SSE3");
    push(bit(c1, 9), "SSSE3");
    push(bit(c1, 19), "SSE4.1");
    push(bit(c1, 20), "SSE4.2");
    push(amd && bit(ce, 6), "SSE4A");
    push(bit(de, 29), if amd { "x86-64" } else { "EM64T" });
    push(bit(c1, 5), "VT-x");
    push(amd && bit(ce, 2), "AMD-V");
    push(bit(c1, 25), "AES");
    push(bit(c1, 28), "AVX");
    push(bit(b7, 5), "AVX2");
    push(bit(a71, 4), "AVX-VNNI");
    // AVX-512 with its subsets in parentheses, as CPU-Z prints them.
    if bit(b7, 16) {
        let subs = [
            (bit(b7, 17), "DQ"),
            (bit(b7, 30), "BW"),
            (bit(b7, 31), "VL"),
            (bit(b7, 28), "CD"),
            (bit(b7, 26), "PF"),
            (bit(b7, 27), "ER"),
            (bit(b7, 21), "IFMA"),
            (bit(c7, 1), "VBMI"),
            (bit(c7, 6), "VBMI2"),
            (bit(c7, 11), "VNNI"),
            (bit(c7, 12), "BITALG"),
            (bit(c7, 14), "VPOPCNTDQ"),
            (bit(a71, 5), "BF16"),
            (bit(d7, 8), "VP2INTERSECT"),
            (bit(d7, 23), "FP16"),
        ];
        let s: Vec<&str> = subs.iter().filter(|x| x.0).map(|x| x.1).collect();
        push(true, &if s.is_empty() { "AVX512F".to_string() } else { format!("AVX512 ({})", s.join(", ")) });
    }
    if bit(d71, 19) && max >= 0x24 {
        let ver = cpuid(0x24, 0)[1] & 0xFF;
        push(true, &format!("AVX10.{ver}"));
    }
    if bit(d7, 24) {
        let subs = [(bit(d7, 22), "BF16"), (bit(d7, 25), "INT8"), (bit(a71, 21), "FP16")];
        let s: Vec<&str> = subs.iter().filter(|x| x.0).map(|x| x.1).collect();
        push(true, &format!("AMX ({})", s.join(", ")));
    }
    push(bit(c1, 12), "FMA3");
    push(amd && bit(ce, 16), "FMA4");
    push(amd && bit(ce, 11), "XOP");
    push(bit(b7, 29), "SHA");
    push(bit(b7, 11) || bit(b7, 4), if bit(b7, 4) { "TSX (HLE, RTM)" } else { "TSX (RTM)" });
    v
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CacheDesc {
    pub level: u32,
    pub kind: char, // D, I, U
    pub size: u64,
    pub ways: u32, // 0 = fully associative
    pub line: u32,
    pub sharing: u32,
    pub inclusive: bool,
}

/// Deterministic cache parameters for the current thread.
pub fn caches(vendor: &str) -> Vec<CacheDesc> {
    let amd = vendor == "AuthenticAMD" || vendor == "HygonGenuine";
    let leaf = if amd { 0x8000_001D } else { 4 };
    let ok = if amd { max_ext_leaf() >= leaf && bit(cpuid(0x8000_0001, 0)[2], 22) } else { max_leaf() >= 4 };
    let mut v = Vec::new();
    if ok {
        for s in 0..16 {
            let [a, b, c, d] = cpuid(leaf, s);
            let kind = match a & 0x1F {
                0 => break,
                1 => 'D',
                2 => 'I',
                _ => 'U',
            };
            let ways = bits(b, 22, 31) + 1;
            let parts = bits(b, 12, 21) + 1;
            let line = bits(b, 0, 11) + 1;
            let sets = c as u64 + 1;
            v.push(CacheDesc {
                level: bits(a, 5, 7),
                kind,
                size: ways as u64 * parts as u64 * line as u64 * sets,
                ways: if bit(a, 9) { 0 } else { ways },
                line,
                sharing: bits(a, 14, 25) + 1,
                inclusive: bit(d, 1),
            });
        }
    }
    if v.is_empty() && amd && max_ext_leaf() >= 0x8000_0006 {
        v = amd_legacy_caches();
    }
    v
}

/// Pre-Zen AMD: leaves 0x80000005/6 (APM vol.3, E.4.4).
fn amd_legacy_caches() -> Vec<CacheDesc> {
    let l5 = cpuid(0x8000_0005, 0);
    let l6 = cpuid(0x8000_0006, 0);
    let assoc6 = |x: u32| match x {
        1 => 1,
        2 => 2,
        4 => 4,
        6 => 8,
        8 => 16,
        0xA => 32,
        0xB => 48,
        0xC => 64,
        0xD => 96,
        0xE => 128,
        _ => 0,
    };
    let mut v = vec![
        CacheDesc {
            level: 1,
            kind: 'D',
            size: bits(l5[2], 24, 31) as u64 * 1024,
            ways: bits(l5[2], 16, 23),
            line: bits(l5[2], 0, 7),
            sharing: 1,
            inclusive: false,
        },
        CacheDesc {
            level: 1,
            kind: 'I',
            size: bits(l5[3], 24, 31) as u64 * 1024,
            ways: bits(l5[3], 16, 23),
            line: bits(l5[3], 0, 7),
            sharing: 1,
            inclusive: false,
        },
        CacheDesc {
            level: 2,
            kind: 'U',
            size: bits(l6[2], 16, 31) as u64 * 1024,
            ways: assoc6(bits(l6[2], 12, 15)),
            line: bits(l6[2], 0, 7),
            sharing: 1,
            inclusive: false,
        },
    ];
    let l3 = bits(l6[3], 18, 31) as u64 * 512 * 1024;
    if l3 > 0 {
        v.push(CacheDesc {
            level: 3,
            kind: 'U',
            size: l3,
            ways: assoc6(bits(l6[3], 12, 15)),
            line: bits(l6[3], 0, 7),
            sharing: 0,
            inclusive: false,
        });
    }
    v.retain(|c| c.size > 0);
    v
}

/// Topology of the current thread, decoded from its APIC ID.
#[derive(Debug, Clone, Default)]
pub struct Topo {
    pub apic: u32,
    pub smt_bits: u32,
    pub core_bits: u32, // bits below the package level
    pub die_bits: Option<u32>,
    pub ccx_bits: Option<u32>,
    pub core_type: Option<u8>, // Intel 0x1A: 0x20 Atom/E, 0x40 Core/P
    pub node: Option<u32>,
}

impl Topo {
    pub fn package(&self) -> u32 {
        self.apic >> self.core_bits
    }
    pub fn core(&self) -> u32 {
        self.apic >> self.smt_bits
    }
    pub fn smt(&self) -> u32 {
        self.apic & ((1 << self.smt_bits) - 1)
    }
}

pub fn topology(vendor: &str) -> Topo {
    let amd = vendor == "AuthenticAMD" || vendor == "HygonGenuine";
    let l1 = cpuid(1, 0);
    let mut t = Topo { apic: bits(l1[1], 24, 31), ..Default::default() };
    let max = max_leaf();
    let maxe = max_ext_leaf();

    // Extended topology: AMD 0x80000026, Intel 0x1F, both 0x0B.
    let leaf = if amd && maxe >= 0x8000_0026 {
        Some(0x8000_0026)
    } else if max >= 0x1F && cpuid(0x1F, 0)[1] != 0 {
        Some(0x1F)
    } else if max >= 0xB && cpuid(0xB, 0)[1] != 0 {
        Some(0xB)
    } else {
        None
    };
    if let Some(leaf) = leaf {
        // AMD names the unit the shift identifies (1 core, 2 CCX, 3 CCD, 4 socket).
        // Intel names the members (1 SMT, 2 core, 3 module, 4 tile, 5 die), so a
        // unit's ID is the APIC ID shifted by the level *below* it.
        let mut prev = 0;
        for s in 0..8 {
            let [a, _, c, d] = cpuid(leaf, s);
            let ty = bits(c, 8, 15);
            if ty == 0 {
                break;
            }
            let shift = bits(a, 0, 4);
            t.apic = d;
            match (leaf == 0x8000_0026, ty) {
                (_, 1) => t.smt_bits = shift,
                (true, 2) => t.ccx_bits = Some(shift),
                (true, 3) => t.die_bits = Some(shift),
                (false, 5) => t.die_bits = Some(prev),
                _ => {}
            }
            t.core_bits = shift;
            prev = shift;
        }
    } else if amd && maxe >= 0x8000_0008 {
        // AMD legacy: ApicIdCoreIdSize in 0x80000008 ecx[15:12].
        let c = cpuid(0x8000_0008, 0)[2];
        let size = bits(c, 12, 15);
        let n = bits(c, 0, 7) + 1;
        t.core_bits = if size != 0 { size } else { 32 - (n.max(1) - 1).leading_zeros() };
        let tpc = if maxe >= 0x8000_001E { bits(cpuid(0x8000_001E, 0)[1], 8, 15) + 1 } else { 1 };
        t.smt_bits = 32 - (tpc.max(1) - 1).leading_zeros();
    } else {
        // Leaf 1 / leaf 4 (old Intel).
        let logical = bits(l1[1], 16, 23).max(1);
        let cores = if max >= 4 { bits(cpuid(4, 0)[0], 26, 31) + 1 } else { 1 };
        t.core_bits = 32 - (logical.max(1) - 1).leading_zeros();
        let tpc = (logical / cores).max(1);
        t.smt_bits = 32 - (tpc - 1).leading_zeros();
    }
    if t.core_bits < t.smt_bits {
        t.core_bits = t.smt_bits;
    }
    if amd && maxe >= 0x8000_001E {
        t.node = Some(bits(cpuid(0x8000_001E, 0)[2], 0, 7));
    }
    if !amd && max >= 0x1A {
        let ty = bits(cpuid(0x1A, 0)[0], 24, 31) as u8;
        t.core_type = (ty != 0).then_some(ty);
    }
    t
}

/// Intel leaf 0x16: base / max / bus MHz.
pub fn intel_freqs() -> Option<(u32, u32, u32)> {
    (max_leaf() >= 0x16)
        .then(|| cpuid(0x16, 0))
        .filter(|r| r[0] != 0)
        .map(|r| (r[0] & 0xFFFF, r[1] & 0xFFFF, r[2] & 0xFFFF))
}

/// TSC frequency from leaf 0x15 when the crystal is enumerated (Intel).
pub fn tsc_hz_from_cpuid() -> Option<u64> {
    if max_leaf() < 0x15 {
        return None;
    }
    let [den, num, crystal, _] = cpuid(0x15, 0);
    (den != 0 && num != 0 && crystal != 0).then(|| crystal as u64 * num as u64 / den as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_this_cpu() {
        let v = vendor();
        assert_eq!(v.len(), 12);
        let s = signature();
        assert!(s.family >= 6);
        let c = caches(&v);
        assert!(c.iter().any(|c| c.level == 1 && c.kind == 'D'), "{c:?}");
        assert!(instructions(&v).iter().any(|i| i == "SSE2"));
        assert!(!dump_leaves().is_empty());
    }
}

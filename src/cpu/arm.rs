//! ARM processors: Apple Silicon via sysctl, Linux via /proc/cpuinfo + sysfs,
//! Windows on ARM via the registry. There is no CPUID; MIDR_EL1 plays its role.

#[cfg(target_os = "macos")]
use super::Cache;
use super::{Cluster, Cpu, Logical};

/// MIDR implementer codes (ARM ARM, D23.2.122) and well-known part numbers.
fn implementer(code: u32) -> &'static str {
    match code {
        0x41 => "ARM",
        0x42 => "Broadcom",
        0x43 => "Cavium",
        0x46 => "Fujitsu",
        0x48 => "HiSilicon",
        0x4E => "NVIDIA",
        0x50 => "Applied Micro",
        0x51 => "Qualcomm",
        0x53 => "Samsung",
        0x56 => "Marvell",
        0x61 => "Apple",
        0x69 => "Intel",
        0x6D => "Microsoft",
        0xC0 => "Ampere",
        _ => "Unknown",
    }
}

fn part(imp: u32, p: u32) -> Option<&'static str> {
    Some(match (imp, p) {
        (0x41, 0xD03) => "Cortex-A53",
        (0x41, 0xD04) => "Cortex-A35",
        (0x41, 0xD05) => "Cortex-A55",
        (0x41, 0xD07) => "Cortex-A57",
        (0x41, 0xD08) => "Cortex-A72",
        (0x41, 0xD09) => "Cortex-A73",
        (0x41, 0xD0A) => "Cortex-A75",
        (0x41, 0xD0B) => "Cortex-A76",
        (0x41, 0xD0C) => "Neoverse-N1",
        (0x41, 0xD0D) => "Cortex-A77",
        (0x41, 0xD40) => "Neoverse-V1",
        (0x41, 0xD41) => "Cortex-A78",
        (0x41, 0xD44) => "Cortex-X1",
        (0x41, 0xD46) => "Cortex-A510",
        (0x41, 0xD47) => "Cortex-A710",
        (0x41, 0xD48) => "Cortex-X2",
        (0x41, 0xD49) => "Neoverse-N2",
        (0x41, 0xD4A) => "Neoverse-E1",
        (0x41, 0xD4D) => "Cortex-A715",
        (0x41, 0xD4E) => "Cortex-X3",
        (0x41, 0xD4F) => "Neoverse-V2",
        (0x41, 0xD80) => "Cortex-A520",
        (0x41, 0xD81) => "Cortex-A720",
        (0x41, 0xD82) => "Cortex-X4",
        (0x41, 0xD84) => "Neoverse-V3",
        (0x41, 0xD85) => "Cortex-X925",
        (0x41, 0xD87) => "Cortex-A725",
        (0x41, 0xD8E) => "Neoverse-N3",
        (0x51, 0x001) => "Oryon",
        (0x61, 0x022) | (0x61, 0x024) | (0x61, 0x028) => "Firestorm (M1)",
        (0x61, 0x023) | (0x61, 0x025) | (0x61, 0x029) => "Icestorm (M1)",
        (0x61, 0x032) | (0x61, 0x034) | (0x61, 0x038) => "Avalanche (M2)",
        (0x61, 0x033) | (0x61, 0x035) | (0x61, 0x039) => "Blizzard (M2)",
        (0xC0, 0xAC3) => "Ampere-1",
        (0xC0, 0xAC4) => "Ampere-1a",
        _ => return None,
    })
}

pub fn midr_text(midr: u64) -> String {
    let (imp, var, p, rev) =
        ((midr >> 24) as u32 & 0xFF, (midr >> 20) as u32 & 0xF, (midr >> 4) as u32 & 0xFFF, midr as u32 & 0xF);
    let name = part(imp, p).map_or_else(|| format!("part 0x{p:03X}"), str::to_string);
    format!("{} {name} r{var}p{rev}", implementer(imp))
}

#[cfg(target_os = "macos")]
pub fn sysctl_str(name: &str) -> Option<String> {
    let c = std::ffi::CString::new(name).ok()?;
    let mut len = 0usize;
    // SAFETY: standard two-call sysctlbyname pattern with a sized buffer.
    unsafe {
        if libc::sysctlbyname(c.as_ptr(), std::ptr::null_mut(), &mut len, std::ptr::null_mut(), 0) != 0 || len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len];
        if libc::sysctlbyname(c.as_ptr(), buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut(), 0) != 0 {
            return None;
        }
        buf.truncate(len);
        let s = String::from_utf8_lossy(&buf).trim_end_matches('\0').trim().to_string();
        (!s.is_empty()).then_some(s)
    }
}

#[cfg(target_os = "macos")]
pub fn sysctl_u64(name: &str) -> Option<u64> {
    let c = std::ffi::CString::new(name).ok()?;
    let mut v = 0u64;
    let mut len = 8usize;
    // SAFETY: integer sysctls are 4 or 8 bytes; the buffer is 8 and zeroed.
    unsafe {
        if libc::sysctlbyname(c.as_ptr(), (&mut v as *mut u64).cast(), &mut len, std::ptr::null_mut(), 0) != 0 {
            return None;
        }
    }
    Some(if len == 4 { v & 0xFFFF_FFFF } else { v })
}

/// Apple Silicon. `hw.perflevelN.*` describes each cluster type.
#[cfg(target_os = "macos")]
pub fn build(cpu: &mut Cpu) {
    cpu.brand = sysctl_str("machdep.cpu.brand_string").unwrap_or_else(|| "Apple Silicon".into());
    cpu.vendor = "Apple".into();
    cpu.name = cpu.brand.clone();
    cpu.sig = (sysctl_u64("hw.cpufamily").unwrap_or(0) as u32, sysctl_u64("hw.cpusubfamily").unwrap_or(0) as u32, 0);
    cpu.cpuid = Some(format!("cpufamily 0x{:08X}, subfamily {}", cpu.sig.0, cpu.sig.1));
    let levels = sysctl_u64("hw.nperflevels").unwrap_or(1);
    let line = sysctl_u64("hw.cachelinesize").unwrap_or(128) as u32;
    let mut first = 0usize;
    for l in 0..levels {
        let p = |k: &str| sysctl_u64(&format!("hw.perflevel{l}.{k}"));
        let kind = sysctl_str(&format!("hw.perflevel{l}.name")).unwrap_or_else(|| format!("Level {l}"));
        let cores = p("physicalcpu").unwrap_or(0) as usize;
        let threads = p("logicalcpu").unwrap_or(cores as u64) as usize;
        let label = match kind.as_str() {
            "Performance" => "P-Cores".to_string(),
            "Efficiency" => "E-Cores".to_string(),
            k => k.to_string(),
        };
        cpu.clusters.push(Cluster { kind: label.clone(), cores, threads });
        for (lvl, kind_c, key) in [(1, 'D', "l1dcachesize"), (1, 'I', "l1icachesize")] {
            if let Some(sz) = p(key) {
                cpu.caches.push(Cache {
                    level: lvl,
                    kind: kind_c,
                    size: sz,
                    ways: 0,
                    line,
                    count: cores,
                    sharing: 1,
                    note: Some(label.clone()),
                });
            }
        }
        if let Some(sz) = p("l2cachesize") {
            let per = p("cpusperl2").unwrap_or(cores as u64).max(1) as usize;
            cpu.caches.push(Cache {
                level: 2,
                kind: 'U',
                size: sz,
                ways: 0,
                line,
                count: cores.div_ceil(per),
                sharing: per as u32,
                note: Some(label.clone()),
            });
        }
        for i in 0..threads {
            cpu.logical.push(Logical {
                cpu: first + i,
                core: (first + i) as u32,
                kind: Some(label.clone()),
                ..Default::default()
            });
        }
        first += threads;
    }
    cpu.cores = sysctl_u64("hw.physicalcpu").unwrap_or(first as u64) as usize;
    cpu.threads = sysctl_u64("hw.logicalcpu").unwrap_or(first as u64) as usize;
    cpu.sockets = sysctl_u64("hw.packages").unwrap_or(1) as usize;
    // hw.optional.arm.FEAT_* are 1 when the feature is present.
    let feats = [
        ("hw.optional.AdvSIMD", "NEON"),
        ("hw.optional.arm.FEAT_AES", "AES"),
        ("hw.optional.arm.FEAT_SHA1", "SHA1"),
        ("hw.optional.arm.FEAT_SHA256", "SHA2"),
        ("hw.optional.arm.FEAT_SHA512", "SHA512"),
        ("hw.optional.arm.FEAT_SHA3", "SHA3"),
        ("hw.optional.armv8_crc32", "CRC32"),
        ("hw.optional.arm.FEAT_LSE", "LSE"),
        ("hw.optional.arm.FEAT_FP16", "FP16"),
        ("hw.optional.arm.FEAT_DotProd", "DotProd"),
        ("hw.optional.arm.FEAT_FHM", "FHM"),
        ("hw.optional.arm.FEAT_BF16", "BF16"),
        ("hw.optional.arm.FEAT_I8MM", "I8MM"),
        ("hw.optional.arm.FEAT_SME", "SME"),
        ("hw.optional.arm.FEAT_SME2", "SME2"),
        ("hw.optional.arm.FEAT_PAuth", "PAuth"),
        ("hw.optional.arm.FEAT_BTI", "BTI"),
    ];
    cpu.instructions = feats.iter().filter(|(k, _)| sysctl_u64(k) == Some(1)).map(|(_, n)| n.to_string()).collect();
    cpu.instructions.insert(0, "ARMv8 (AArch64)".into());
    cpu.max_mhz = sysctl_u64("hw.cpufrequency_max").map(|h| h as f64 / 1e6);
    if let Some(info) = super::db_apple(&cpu.brand) {
        cpu.info = info;
    }
}

/// Linux / other ARM: MIDR per CPU from sysfs or /proc/cpuinfo, caches from sysfs.
#[cfg(not(target_os = "macos"))]
pub fn build(cpu: &mut Cpu) {
    let n = crate::os::logical_cpus();
    let mut kinds: Vec<(u64, usize)> = Vec::new();
    let info = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let midr_of = |i: usize| -> Option<u64> {
        crate::os::read_u64(format!("/sys/devices/system/cpu/cpu{i}/regs/identification/midr_el1"))
    };
    for i in 0..n {
        let midr = midr_of(i).unwrap_or_else(|| cpuinfo_midr(&info, i));
        let core = crate::os::read_u64(format!("/sys/devices/system/cpu/cpu{i}/topology/core_id")).unwrap_or(i as u64);
        let pkg =
            crate::os::read_u64(format!("/sys/devices/system/cpu/cpu{i}/topology/physical_package_id")).unwrap_or(0);
        cpu.logical.push(Logical {
            cpu: i,
            package: pkg as u32,
            core: core as u32,
            apic: midr as u32,
            kind: Some(midr_text(midr)),
            ..Default::default()
        });
        match kinds.iter_mut().find(|k| k.0 == midr) {
            Some(k) => k.1 += 1,
            None => kinds.push((midr, 1)),
        }
    }
    for (midr, cnt) in &kinds {
        cpu.clusters.push(Cluster { kind: midr_text(*midr), cores: *cnt, threads: *cnt });
    }
    let (imp, p) = kinds.first().map_or((0, 0), |k| ((k.0 >> 24) as u32 & 0xFF, (k.0 >> 4) as u32 & 0xFFF));
    cpu.vendor = implementer(imp).into();
    cpu.sig = (imp, p, kinds.first().map_or(0, |k| k.0 as u32 & 0xF));
    cpu.brand = cpuinfo_field(&info, "model name")
        .or_else(|| crate::os::read_trim("/sys/firmware/devicetree/base/model"))
        .unwrap_or_else(|| kinds.iter().map(|k| midr_text(k.0)).collect::<Vec<_>>().join(" + "));
    cpu.name = cpu.brand.clone();
    cpu.cpuid = kinds.first().map(|k| format!("MIDR 0x{:08X}", k.0));
    cpu.threads = n;
    cpu.cores = n;
    cpu.sockets = 1 + cpu.logical.iter().map(|l| l.package as usize).max().unwrap_or(0);
    cpu.instructions = cpuinfo_field(&info, "Features")
        .map(|f| f.split_whitespace().map(str::to_uppercase).collect())
        .unwrap_or_default();
    cpu.instructions.insert(0, "ARMv8 (AArch64)".into());
    cpu.caches = super::sysfs_caches(n);
}

#[cfg(not(target_os = "macos"))]
fn cpuinfo_field(info: &str, key: &str) -> Option<String> {
    info.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == key).then(|| v.trim().to_string()).filter(|v| !v.is_empty())
    })
}

/// Rebuild MIDR from "CPU implementer/variant/part/revision" of processor `i`.
#[cfg(not(target_os = "macos"))]
fn cpuinfo_midr(info: &str, i: usize) -> u64 {
    let block = info.split("\n\n").find(|b| {
        b.lines().any(|l| l.split_once(':').is_some_and(|(k, v)| k.trim() == "processor" && v.trim() == i.to_string()))
    });
    let Some(b) = block else { return 0 };
    let num = |k: &str| {
        cpuinfo_field(b, k)
            .and_then(|v| {
                let v = v.trim_start_matches("0x");
                u64::from_str_radix(v, 16).ok().or_else(|| v.parse().ok())
            })
            .unwrap_or(0)
    };
    (num("CPU implementer") << 24)
        | (num("CPU variant") << 20)
        | (0xF << 16)
        | (num("CPU part") << 4)
        | num("CPU revision")
}

#[cfg(test)]
mod tests {
    #[test]
    fn midr() {
        assert_eq!(super::midr_text(0x410F_D0B1), "ARM Cortex-A76 r0p1");
    }
}

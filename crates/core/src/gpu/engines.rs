//! GPU engine utilization at adapter level (S1-D04), from Windows' `GPU
//! Engine` performance counters. Their instances are per process and per
//! engine ("pid_1234_luid_0x00000000_0x0000D1A3_phys_0_eng_3_engtype_3D"). Like
//! the Task Manager, an engine's load is the sum over processes, and an engine
//! type's load is its busiest engine.

use std::collections::BTreeMap;

/// One adapter's engine types and their load (%), busiest first.
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterEngines {
    /// "0x00000000_0x0000D1A3" (high_low), as DXGI's LUID prints in the counters.
    pub luid: String,
    pub engines: Vec<(String, f64)>,
}

/// (luid, physical adapter, engine index, engine type) from an instance name.
fn parse(name: &str) -> Option<(&str, &str, &str, &str)> {
    let rest = name.split_once("luid_")?.1;
    let (luid, rest) = rest.split_once("_phys_")?;
    let (phys, rest) = rest.split_once("_eng_")?;
    let (eng, ty) = rest.split_once("_engtype_")?;
    Some((luid, phys, eng, ty))
}

/// "VideoDecode" → "Video Decode", "Compute_0" → "Compute 0", "3D" stays.
pub fn engine_label(ty: &str) -> String {
    let mut s = String::new();
    for (i, c) in ty.replace('_', " ").chars().enumerate() {
        if i > 0
            && c.is_ascii_uppercase()
            && !s.ends_with(' ')
            && s.chars().last().is_some_and(|p| p.is_ascii_lowercase())
        {
            s.push(' ');
        }
        s.push(c);
    }
    s
}

/// Adapter-level load per engine type from (instance name, %) pairs.
pub fn aggregate<'a>(items: impl IntoIterator<Item = (&'a str, f64)>) -> Vec<AdapterEngines> {
    // luid → (phys, eng, type) → summed %
    let mut per_engine: BTreeMap<&str, BTreeMap<(&str, &str, &str), f64>> = BTreeMap::new();
    for (name, v) in items {
        let Some((luid, phys, eng, ty)) = parse(name) else { continue };
        if v.is_finite() {
            *per_engine.entry(luid).or_default().entry((phys, eng, ty)).or_default() += v.max(0.0);
        }
    }
    per_engine
        .into_iter()
        .map(|(luid, engines)| {
            let mut by_type: BTreeMap<String, f64> = BTreeMap::new();
            for ((_, _, ty), v) in engines {
                let e = by_type.entry(engine_label(ty)).or_default();
                *e = e.max(v.min(100.0));
            }
            let mut engines: Vec<(String, f64)> = by_type.into_iter().collect();
            engines.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            AdapterEngines { luid: luid.to_string(), engines }
        })
        .collect()
}

/// A process's GPU use, as the Task Manager shows it: its busiest engine.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessGpu {
    pub pid: u32,
    pub percent: f64,
    /// That engine's type ("3D", "Video Decode"…).
    pub engine: String,
}

/// Per-process GPU from the same (instance name, %) pairs.
pub fn per_process<'a>(items: impl IntoIterator<Item = (&'a str, f64)>) -> Vec<ProcessGpu> {
    type EngineMap<'a> = BTreeMap<(&'a str, &'a str, &'a str), (f64, &'a str)>;
    // pid → (luid, phys, eng) → (%, type)
    let mut m: BTreeMap<u32, EngineMap<'a>> = BTreeMap::new();
    for (name, v) in items {
        let Some(pid) = name.strip_prefix("pid_").and_then(|r| r.split('_').next()?.parse::<u32>().ok()) else {
            continue;
        };
        let Some((luid, phys, eng, ty)) = parse(name) else { continue };
        if v.is_finite() {
            let e = m.entry(pid).or_default().entry((luid, phys, eng)).or_insert((0.0, ty));
            e.0 += v.max(0.0);
        }
    }
    m.into_iter()
        .filter_map(|(pid, engines)| {
            let (pct, ty) = engines.into_values().max_by(|a, b| a.0.total_cmp(&b.0))?;
            Some(ProcessGpu { pid, percent: pct.min(100.0), engine: engine_label(ty) })
        })
        .collect()
}

/// Per-process memory from `GPU Process Memory(*)` instances ("pid_1234_luid_…_phys_0"): bytes per pid.
pub fn per_process_memory<'a>(items: impl IntoIterator<Item = (&'a str, f64)>) -> BTreeMap<u32, u64> {
    let mut m = BTreeMap::new();
    for (name, v) in items {
        if let Some(pid) = name.strip_prefix("pid_").and_then(|r| r.split('_').next()?.parse::<u32>().ok()) {
            *m.entry(pid).or_default() += v.max(0.0) as u64;
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_processes_per_engine_then_takes_the_busiest_engine_per_type() {
        let l = "luid_0x00000000_0x0000D1A3";
        let items = [
            (format!("pid_10_{l}_phys_0_eng_0_engtype_3D"), 30.0),
            (format!("pid_20_{l}_phys_0_eng_0_engtype_3D"), 25.0), // same engine: 55
            (format!("pid_10_{l}_phys_0_eng_1_engtype_3D"), 40.0), // other 3D engine: 40
            (format!("pid_10_{l}_phys_0_eng_5_engtype_VideoDecode"), 12.5),
            (format!("pid_10_{l}_phys_0_eng_7_engtype_Compute_0"), 0.0),
            ("pid_30_luid_0x00000000_0x0000BEEF_phys_0_eng_0_engtype_Copy".into(), 150.0), // clamped
            ("garbage".into(), 99.0),
        ];
        let v = aggregate(items.iter().map(|(n, x)| (n.as_str(), *x)));
        assert_eq!(v.len(), 2);
        let a = v.iter().find(|a| a.luid == "0x00000000_0x0000D1A3").unwrap();
        assert_eq!(a.engines, [("3D".into(), 55.0), ("Video Decode".into(), 12.5), ("Compute 0".into(), 0.0)]);
        let b = v.iter().find(|a| a.luid.ends_with("BEEF")).unwrap();
        assert_eq!(b.engines, [("Copy".into(), 100.0)]);
    }

    #[test]
    fn labels() {
        assert_eq!(engine_label("VideoProcessing"), "Video Processing");
        assert_eq!(engine_label("3D"), "3D");
        assert_eq!(engine_label("LegacyOverlay"), "Legacy Overlay");
    }

    #[test]
    fn per_process_is_the_busiest_engine() {
        let items = [
            ("pid_10_luid_0x00000000_0x0000D1A3_phys_0_eng_0_engtype_3D", 30.0),
            ("pid_10_luid_0x00000000_0x0000D1A3_phys_0_eng_3_engtype_VideoDecode", 45.0),
            ("pid_10_luid_0x00000000_0x0000D1A3_phys_0_eng_0_engtype_3D", 5.0),
            ("pid_20_luid_0x00000000_0x0000D1A3_phys_0_eng_0_engtype_3D", 2.0),
        ];
        let p = per_process(items);
        assert_eq!(p[0], ProcessGpu { pid: 10, percent: 45.0, engine: "Video Decode".into() });
        assert_eq!(p[1].pid, 20);
        let mem = per_process_memory([("pid_10_luid_x_phys_0", 1000.0), ("pid_10_luid_y_phys_0", 24.0)]);
        assert_eq!(mem[&10], 1024);
    }
}

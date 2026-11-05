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
}

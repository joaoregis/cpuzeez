//! Facts for the build-health checks (S1-G02) from the report and the OS;
//! the rules are `cpuzeez_core::health`.

use crate::Ctx;
use crate::report::{self, Node, Section};
use cpuzeez_core::health::{Display, Facts, Finding, GpuLink, Profile};

/// "3000 MHz" / "2999.9 MHz (1:30)" / "DDR5-6000 (3000 MHz)" → MHz.
fn mhz_in(s: &str) -> Option<f64> {
    let i = s.find(" MHz")?;
    let head = s[..i].rsplit(|c: char| !(c.is_ascii_digit() || c == '.')).next()?;
    head.parse().ok()
}

/// An integrated GPU shares system memory and the CPU's link.
pub fn integrated(a: &crate::gpu::Adapter) -> bool {
    let n = a.name.to_ascii_lowercase();
    (a.vendor_id == 0x8086 && !n.contains("arc"))
        || a.mem_size_mb.is_some_and(|m| m <= 512)
        || (a.vendor_id == 0x1002 && n.ends_with("radeon(tm) graphics"))
}

fn fields_named<'a>(nodes: &'a [Node], key: &str, out: &mut Vec<&'a str>) {
    for n in nodes {
        match n {
            Node::Field { key: k, value } if k == key => out.push(value),
            Node::Group { children, .. } => fields_named(children, key, out),
            _ => {}
        }
    }
}

pub fn facts(ctx: &Ctx, r: &[Section]) -> Facts {
    let spd = report::find_section(r, "Memory SPD");
    // SPD needs SMBus access; SMBIOS lists the populated slots too (a size set).
    let modules = spd.map(|s| report::groups(&s.nodes, "DIMM #").len()).filter(|&n| n > 0).or_else(|| {
        let dmi = report::find_section(r, "DMI")?;
        let n = report::groups(&dmi.nodes, "DMI Memory Device")
            .iter()
            .filter(|(_, c)| {
                report::find(c, "size").is_some_and(|s| s.chars().next().is_some_and(|c| c.is_ascii_digit()))
            })
            .count();
        (n > 0).then_some(n)
    });
    // SPD's "Max bandwidth" includes the XMP/EXPO profiles; above the JEDEC max, it's a profile.
    let best = spd.and_then(|s| {
        let mut v = Vec::new();
        fields_named(&s.nodes, "Max bandwidth", &mut v);
        let max = v.iter().filter_map(|x| mhz_in(x)).reduce(f64::max)?;
        let mut j = Vec::new();
        fields_named(&s.nodes, "Max JEDEC", &mut j);
        let jedec = j.iter().filter_map(|x| mhz_in(x)).reduce(f64::max).unwrap_or(0.0);
        let (xmp, expo) =
            (!report::groups(&s.nodes, "XMP profile").is_empty(), !report::groups(&s.nodes, "EXPO profile").is_empty());
        let name = match (expo, xmp) {
            (true, true) => "EXPO/XMP",
            (true, false) => "EXPO",
            (false, true) => "XMP",
            _ => return None,
        };
        (max > jedec * 1.01).then(|| Profile { name: name.into(), mhz: max })
    });
    let adapters = crate::gpu::adapters(ctx);
    let gpus = adapters
        .iter()
        .map(|a| GpuLink {
            name: a.name.clone(),
            width: a.link.width.unwrap_or(0).min(64) as u8,
            max_width: a.link_max.width.unwrap_or(0).min(64) as u8,
            discrete: !integrated(a),
        })
        .collect();
    let displays = cpuzeez_platform::display::modes()
        .into_iter()
        .map(|m| Display {
            on_integrated: adapters.iter().any(|a| a.name == m.adapter && integrated(a)),
            name: m.monitor,
            hz: m.hz,
            max_hz: m.max_hz,
        })
        .collect();
    Facts {
        modules,
        mem_mhz: report::find_section(r, "Chipset").and_then(|s| s.get("Memory Frequency")).and_then(mhz_in),
        best_profile: best,
        gpus,
        power_saver: cpuzeez_platform::display::power_saver(),
        displays,
    }
}

pub fn check(ctx: &Ctx, r: &[Section]) -> Vec<Finding> {
    cpuzeez_core::health::check(&facts(ctx, r))
}

#[cfg(test)]
mod tests {
    #[test]
    fn mhz_parsing() {
        assert_eq!(super::mhz_in("2999.9 MHz (1:30)"), Some(2999.9));
        assert_eq!(super::mhz_in("DDR5-6000 (3000 MHz)"), Some(3000.0));
        assert_eq!(super::mhz_in("n/a"), None);
    }
}

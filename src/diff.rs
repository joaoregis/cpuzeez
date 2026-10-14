//! Compare two reports (our JSON, or a fresh one against a saved one) and list
//! what changed: added / removed / modified fields, by section and group path.
//! Noisy live values (clocks, temperatures, voltages, powers, fans, free space)
//! are skipped unless `include_volatile`.
//!
//! Matching: sections by title; fields by key (repeated keys compared as a
//! multiset of values); groups by "family" (title without digits and without a
//! parenthesised suffix, so "DIMM # 1", "Display adapter 0 (primary)" and
//! "CPU Thread (3, 0x6)" each form one family). Inside a family groups are
//! paired by identity: most equal fields first (serials, part numbers, IDs…),
//! then equal value, then equal title, so a module moved to another slot or an
//! adapter that became primary shows as changed fields, not as delete + add.
//!
//! Volatile (skipped unless asked), see `VOLATILE_*` below:
//! - sections: Hardware Monitors (sensor readings), Timers (measured rates);
//! - groups: Hardware registers (SMU PM table and similar raw dumps);
//! - sensor rows: Voltage/Temperature/Fan/Power/Current/Clock Speed/
//!   Utilization/Level N;
//! - keys: Core Speed, Multiplier x Bus Speed, Temperature, Power On Hours,
//!   Percentage Used, Volume (free space), current PCIe link width/speed
//!   (power management drops them at idle), battery charge/rate, Wi-Fi signal;
//! - any value marked "(measured)";
//! - Thread dumps: CPUID rows of hypervisor leaves 0x4000_xxxx.

use crate::report::{Node, Section};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone)]
pub struct Change {
    pub kind: ChangeKind,
    pub section: String,
    /// Group path inside the section, e.g. "DIMM # 1 › XMP profile".
    pub path: String,
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

pub const SEP: &str = " › ";
const VOLATILE_SECTIONS: &[&str] = &["Hardware Monitors", "Timers"];
const VOLATILE_GROUPS: &[&str] = &["Hardware registers"];
/// Sensor row names, followed by a channel number.
const VOLATILE_SENSORS: &[&str] =
    &["voltage", "temperature", "fan", "power", "current", "clock speed", "utilization", "level"];
/// Whole keys (case-insensitive).
const VOLATILE_KEYS: &[&str] = &[
    "core speed",
    "multiplier x bus speed",
    "power on hours",
    "percentage used",
    "volume",
    "current link width",
    "current link speed",
    "pci-e link width",
    "pci-e link speed",
    "remaining capacity",
    "charge level",
    "charge rate",
    "rate",
    "signal",
    "uptime",
];

fn volatile_key(section: &str, key: &str, value: &str) -> bool {
    let k = key.trim().to_ascii_lowercase();
    let base = k.trim_end_matches(|c: char| c.is_ascii_digit() || c == ' ' || c == '#');
    VOLATILE_SENSORS.contains(&base)
        || VOLATILE_KEYS.contains(&k.as_str())
        || value.contains("(measured)")
        || (section == "Thread dumps" && k.starts_with("0x4000"))
}

fn volatile_group(title: &str) -> bool {
    VOLATILE_GROUPS.iter().any(|g| title.starts_with(g))
}

pub fn diff(a: &[Section], b: &[Section], include_volatile: bool) -> Vec<Change> {
    let mut out = Vec::new();
    let skip = |s: &Section| !include_volatile && VOLATILE_SECTIONS.contains(&s.title.as_str());
    let summary = |s: &Section| Some(format!("{} entries", s.nodes.len()));
    for sa in a.iter().filter(|s| !skip(s)) {
        match b.iter().find(|s| s.title == sa.title) {
            Some(sb) => {
                Walk { section: &sa.title, vol: include_volatile, out: &mut out }.nodes("", &sa.nodes, &sb.nodes)
            }
            None => out.push(change(ChangeKind::Removed, &sa.title, "", "", summary(sa), None)),
        }
    }
    for sb in b.iter().filter(|s| !skip(s) && !a.iter().any(|x| x.title == s.title)) {
        out.push(change(ChangeKind::Added, &sb.title, "", "", None, summary(sb)));
    }
    out
}

fn change(
    kind: ChangeKind,
    section: &str,
    path: &str,
    key: &str,
    before: Option<String>,
    after: Option<String>,
) -> Change {
    Change { kind, section: section.into(), path: path.into(), key: key.into(), before, after }
}

struct Walk<'a> {
    section: &'a str,
    vol: bool,
    out: &'a mut Vec<Change>,
}

type Group<'n> = (&'n str, &'n str, &'n [Node]);

/// "DIMM # 3" -> "DIMM #", "Display adapter 0 (primary)" -> "Display adapter".
fn family(title: &str) -> String {
    let t = title.split(" (").next().unwrap_or(title);
    let t: String = t.chars().filter(|c| !c.is_ascii_digit()).collect();
    t.trim().to_string()
}

/// Path segment of a group: its title, or its first values for untitled blocks (PCI).
fn label(g: &Group) -> String {
    if !g.0.is_empty() {
        return g.0.to_string();
    }
    let v: Vec<&str> = crate::report::fields(g.2).map(|(_, v)| v).take(2).collect();
    if v.is_empty() { "(block)".into() } else { v.join(", ") }
}

fn summary(g: &Group) -> String {
    if g.1.is_empty() { format!("({} entries)", g.2.len()) } else { g.1.to_string() }
}

impl Walk<'_> {
    fn keep(&self, key: &str, value: &str) -> bool {
        self.vol || !volatile_key(self.section, key, value)
    }

    fn push(&mut self, kind: ChangeKind, path: &str, key: &str, before: Option<String>, after: Option<String>) {
        self.out.push(change(kind, self.section, path, key, before, after));
    }

    fn nodes(&mut self, path: &str, a: &[Node], b: &[Node]) {
        self.fields(path, a, b);
        self.hex(path, a, b);
        self.groups(path, a, b);
    }

    fn fields(&mut self, path: &str, a: &[Node], b: &[Node]) {
        let mut order: Vec<&str> = Vec::new();
        let mut map: HashMap<&str, (Vec<&str>, Vec<&str>)> = HashMap::new();
        for (side, nodes) in [(0, a), (1, b)] {
            for (k, v) in crate::report::fields(nodes).filter(|(k, v)| self.keep(k, v)) {
                let e = map.entry(k).or_insert_with(|| {
                    order.push(k);
                    Default::default()
                });
                if side == 0 { e.0.push(v) } else { e.1.push(v) }
            }
        }
        for k in order {
            let (mut av, mut bv) = map.remove(k).unwrap();
            av.retain(|v| match bv.iter().position(|x| x == v) {
                Some(i) => {
                    bv.remove(i);
                    false
                }
                None => true,
            });
            let n = av.len().max(bv.len());
            for i in 0..n {
                let (x, y) = (av.get(i).map(|s| s.to_string()), bv.get(i).map(|s| s.to_string()));
                let kind = match (&x, &y) {
                    (Some(_), Some(_)) => ChangeKind::Modified,
                    (Some(_), None) => ChangeKind::Removed,
                    _ => ChangeKind::Added,
                };
                self.push(kind, path, k, x, y);
            }
        }
    }

    fn hex(&mut self, path: &str, a: &[Node], b: &[Node]) {
        let hexes = |n: &[Node]| -> Vec<(String, Vec<u8>)> {
            n.iter()
                .filter_map(|n| match n {
                    Node::Hex { title, bytes } => Some((title.clone(), bytes.clone())),
                    _ => None,
                })
                .collect()
        };
        let (ha, hb) = (hexes(a), hexes(b));
        for (t, x) in &ha {
            match hb.iter().find(|(u, _)| u == t) {
                Some((_, y)) if x != y => {
                    let n = x.iter().zip(y).filter(|(p, q)| p != q).count() + x.len().abs_diff(y.len());
                    let after = format!("{} bytes, {n} differ", y.len());
                    self.push(ChangeKind::Modified, path, t, Some(format!("{} bytes", x.len())), Some(after));
                }
                Some(_) => {}
                None => self.push(ChangeKind::Removed, path, t, Some(format!("{} bytes", x.len())), None),
            }
        }
        for (t, y) in hb.iter().filter(|(t, _)| !ha.iter().any(|(u, _)| u == t)) {
            self.push(ChangeKind::Added, path, t, None, Some(format!("{} bytes", y.len())));
        }
    }

    fn groups(&mut self, path: &str, a: &[Node], b: &[Node]) {
        let (ga, gb) = (groups_of(a, self.vol), groups_of(b, self.vol));
        let mut fams: Vec<&str> = Vec::new();
        for (f, _) in ga.iter().chain(&gb) {
            if !fams.contains(&f.as_str()) {
                fams.push(f);
            }
        }
        let join = |seg: &str| if path.is_empty() { seg.to_string() } else { format!("{path}{SEP}{seg}") };
        for fam in fams {
            let xa: Vec<&Group> = ga.iter().filter(|(f, _)| f == fam).map(|(_, g)| g).collect();
            let xb: Vec<&Group> = gb.iter().filter(|(f, _)| f == fam).map(|(_, g)| g).collect();
            let mut pairs = Vec::new();
            if xa.len() == 1 && xb.len() == 1 {
                pairs.push((0, 0));
            } else {
                // Field (key, value) -> occurrences on each side: values unique on both
                // sides (serials, names, bus locations) identify a group best.
                let mut occ: HashMap<(&str, &str), (usize, usize)> = HashMap::new();
                for (side, gs) in [(0, &xa), (1, &xb)] {
                    for g in gs.iter() {
                        for kv in crate::report::fields(g.2).filter(|(k, v)| self.keep(k, v)) {
                            let e = occ.entry(kv).or_default();
                            if side == 0 { e.0 += 1 } else { e.1 += 1 }
                        }
                    }
                }
                let score = |x: &Group, y: &Group| {
                    let yf: Vec<(&str, &str)> = crate::report::fields(y.2).collect();
                    let same: usize = crate::report::fields(x.2)
                        .filter(|kv| occ.contains_key(kv) && yf.contains(kv))
                        .map(|kv| if occ[&kv] == (1, 1) { 8 } else { 2 })
                        .sum();
                    same + 2 * usize::from(!x.1.is_empty() && x.1 == y.1) + usize::from(x.0 == y.0)
                };
                let mut cand: Vec<(usize, usize, usize)> = Vec::new();
                for (i, x) in xa.iter().enumerate() {
                    for (j, y) in xb.iter().enumerate() {
                        let s = score(x, y);
                        if s > 0 {
                            cand.push((s, i, j));
                        }
                    }
                }
                // ponytail: greedy best-first, O(n²·fields); fine for the ~100 PCI blocks of a report.
                cand.sort_by_key(|&(s, i, j)| (std::cmp::Reverse(s), i.abs_diff(j), i));
                let (mut ua, mut ub) = (vec![false; xa.len()], vec![false; xb.len()]);
                for (_, i, j) in cand {
                    if !ua[i] && !ub[j] {
                        ua[i] = true;
                        ub[j] = true;
                        pairs.push((i, j));
                    }
                }
            }
            pairs.sort_by_key(|&(_, j)| j);
            for &(i, j) in &pairs {
                let (x, y) = (xa[i], xb[j]);
                if x.0 != y.0 || x.1 != y.1 {
                    let show = |g: &Group| if g.1.is_empty() { g.0.to_string() } else { format!("{} {}", g.0, g.1) };
                    self.push(ChangeKind::Modified, path, &label(y), Some(show(x)), Some(show(y)));
                }
                self.nodes(&join(&label(y)), x.2, y.2);
            }
            for (_, x) in xa.iter().enumerate().filter(|(i, _)| !pairs.iter().any(|p| p.0 == *i)) {
                self.push(ChangeKind::Removed, path, &label(x), Some(summary(x)), None);
            }
            for (_, y) in xb.iter().enumerate().filter(|(j, _)| !pairs.iter().any(|p| p.1 == *j)) {
                self.push(ChangeKind::Added, path, &label(y), None, Some(summary(y)));
            }
        }
    }
}

/// Direct child groups with their family, minus volatile dumps.
fn groups_of(n: &[Node], vol: bool) -> Vec<(String, Group<'_>)> {
    n.iter()
        .filter_map(|n| match n {
            Node::Group { title, value, children } => {
                Some((family(title), (title.as_str(), value.as_str(), children.as_slice())))
            }
            _ => None,
        })
        .filter(|(_, g)| vol || !volatile_group(g.0))
        .collect()
}

/// Load a cpuzeez JSON report (`-json=`, F7 `.cvf`).
pub fn load(path: &std::path::Path) -> Result<Vec<Section>, String> {
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&b).map_err(|e| e.to_string())
}

/// Readable text, grouped by section then path.
pub fn to_text(changes: &[Change]) -> String {
    use std::fmt::Write;
    let mut o = String::new();
    let (mut sec, mut path) = (None::<&str>, None::<&str>);
    for c in changes {
        if sec != Some(c.section.as_str()) {
            let _ = writeln!(o, "\n{}", c.section);
            (sec, path) = (Some(c.section.as_str()), None);
        }
        if !c.path.is_empty() && path != Some(c.path.as_str()) {
            let _ = writeln!(o, "  {}", c.path);
            path = Some(c.path.as_str());
        }
        let ind = if c.path.is_empty() { "  " } else { "    " };
        let key = if c.key.is_empty() { "(whole section)" } else { &c.key };
        let (b, a) = (c.before.as_deref().unwrap_or(""), c.after.as_deref().unwrap_or(""));
        let _ = match c.kind {
            ChangeKind::Added => writeln!(o, "{ind}+ {key}: {a}"),
            ChangeKind::Removed => writeln!(o, "{ind}- {key}: {b}"),
            ChangeKind::Modified => writeln!(o, "{ind}~ {key}: {b}  ->  {a}"),
        };
    }
    o
}

/// `-diff=OLD.json,NEW.json` (NEW may be "now" for a fresh report; a third
/// item "all" keeps volatile values).
pub fn cli(ctx: &crate::Ctx, spec: &str) {
    let parts: Vec<&str> = spec.split(',').map(str::trim).collect();
    let (old, new) = match parts.as_slice() {
        [o, n, ..] if !o.is_empty() && !n.is_empty() => (*o, *n),
        _ => return eprintln!("usage: -diff=OLD.json,NEW.json  or  -diff=OLD.json,now  [,all]"),
    };
    let vol = parts.get(2).is_some_and(|s| s.eq_ignore_ascii_case("all"));
    let a = match load(old.as_ref()) {
        Ok(r) => r,
        Err(e) => return eprintln!("{old}: {e}"),
    };
    let b = if new.eq_ignore_ascii_case("now") {
        crate::collect(ctx)
    } else {
        match load(new.as_ref()) {
            Ok(r) => r,
            Err(e) => return eprintln!("{new}: {e}"),
        }
    };
    let ch = diff(&a, &b, vol);
    let count = |k: ChangeKind| ch.iter().filter(|c| c.kind == k).count();
    println!(
        "{} report diff: {old} -> {new}\n{} modified, {} added, {} removed{}",
        crate::DISPLAY_NAME,
        count(ChangeKind::Modified),
        count(ChangeKind::Added),
        count(ChangeKind::Removed),
        if vol {
            ""
        } else {
            " (live values such as clocks, temperatures and voltages skipped; add \",all\" to include)"
        }
    );
    print!("{}", to_text(&ch));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{f, g, gv, hex, section};

    fn dimm(slot: u32, serial: &str, size: &str) -> Node {
        g(format!("DIMM # {slot}"), vec![f("Serial Number", serial), f("Part Number", "KF560"), f("Size", size)])
    }

    fn find<'a>(c: &'a [Change], key: &str) -> Vec<&'a Change> {
        c.iter().filter(|c| c.key == key).collect()
    }

    #[test]
    fn fields_groups_and_volatile() {
        let a = vec![
            section(
                "Processors Information",
                vec![f("Name", "X"), f("Core Speed", "5000 MHz"), f("Clock Speed 0", "1")],
            ),
            section("Memory SPD", vec![dimm(1, "AAA", "16 GB"), dimm(2, "BBB", "16 GB")]),
            section("Timers", vec![f("Perf timer", "10 MHz")]),
            section("Gone", vec![f("x", "1")]),
            section("Thread dumps", vec![g("CPU Thread (0, 0x0)", vec![f("0x40000001", "a"), f("0x00000001", "b")])]),
        ];
        let b = vec![
            section(
                "Processors Information",
                vec![f("Name", "Y"), f("Core Speed", "4000 MHz"), f("Clock Speed 0", "2")],
            ),
            // Module BBB moved to slot 1 and AAA to slot 3, AAA's size changed; CCC is new.
            section("Memory SPD", vec![dimm(1, "BBB", "16 GB"), dimm(3, "AAA", "32 GB"), dimm(4, "CCC", "8 GB")]),
            section("Timers", vec![f("Perf timer", "11 MHz")]),
            section("Thread dumps", vec![g("CPU Thread (0, 0x0)", vec![f("0x40000001", "z"), f("0x00000001", "b")])]),
            section("New", vec![]),
        ];
        let c = diff(&a, &b, false);
        let name = find(&c, "Name");
        assert_eq!(name.len(), 1);
        assert_eq!((name[0].before.as_deref(), name[0].after.as_deref()), (Some("X"), Some("Y")));
        assert!(find(&c, "Core Speed").is_empty() && find(&c, "Clock Speed 0").is_empty());
        assert!(c.iter().all(|c| c.section != "Timers" && c.section != "Thread dumps"));
        // Matched by serial, not by slot: only AAA's size and slot changed, CCC added.
        let size = find(&c, "Size");
        assert_eq!(size.len(), 1);
        assert_eq!(size[0].path, "DIMM # 3");
        assert_eq!(size[0].after.as_deref(), Some("32 GB"));
        assert!(find(&c, "Serial Number").is_empty());
        let added = find(&c, "DIMM # 4");
        assert!(added.len() == 1 && added[0].kind == ChangeKind::Added);
        assert!(c.iter().any(|c| c.section == "Gone" && c.kind == ChangeKind::Removed));
        assert!(c.iter().any(|c| c.section == "New" && c.kind == ChangeKind::Added));
        assert!(!to_text(&c).is_empty());

        let all = diff(&a, &b, true);
        assert_eq!(find(&all, "Core Speed").len(), 1);
        assert!(all.iter().any(|c| c.section == "Timers"));
        assert!(all.iter().any(|c| c.section == "Thread dumps" && c.key == "0x40000001"));
    }

    #[test]
    fn repeated_keys_hex_and_nesting() {
        let a = vec![section(
            "S",
            vec![
                f("USB Device", "Mouse"),
                f("USB Device", "Keyboard"),
                gv("Display adapter 0", "", vec![f("Name", "GPU A"), g("Hardware registers", vec![f("r", "1")])]),
                gv("Display adapter 1 (primary)", "", vec![f("Name", "GPU B"), hex("Regs", vec![0, 1, 2])]),
            ],
        )];
        let b = vec![section(
            "S",
            vec![
                f("USB Device", "Keyboard"),
                f("USB Device", "Pad"),
                gv("Display adapter 0 (primary)", "", vec![f("Name", "GPU B"), hex("Regs", vec![0, 9, 2])]),
                gv("Display adapter 1", "", vec![f("Name", "GPU A"), g("Hardware registers", vec![f("r", "2")])]),
            ],
        )];
        let c = diff(&a, &b, false);
        let usb = find(&c, "USB Device");
        assert_eq!(usb.len(), 1);
        assert_eq!((usb[0].before.as_deref(), usb[0].after.as_deref()), (Some("Mouse"), Some("Pad")));
        let regs = find(&c, "Regs");
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].path, "Display adapter 0 (primary)");
        assert_eq!(regs[0].after.as_deref(), Some("3 bytes, 1 differ"));
        assert!(find(&c, "Name").is_empty(), "adapters matched by identity: {c:?}");
        assert!(find(&c, "r").is_empty());
        assert_eq!(find(&diff(&a, &b, true), "r").len(), 1);
    }

    #[test]
    fn json_roundtrip() {
        let r = vec![section("S", vec![f("k", "v"), gv("G", "1", vec![hex("H", vec![1, 2])])])];
        let p = std::env::temp_dir().join(format!("cpuzeez-diff-{}.json", std::process::id()));
        std::fs::write(&p, crate::report::to_json(&r)).unwrap();
        let back = load(&p).unwrap();
        let _ = std::fs::remove_file(&p);
        assert_eq!(crate::report::to_json(&back), crate::report::to_json(&r));
        assert!(diff(&r, &back, true).is_empty());
    }
}

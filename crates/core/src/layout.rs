//! The Widget Engine's model (docs/v1/README.md §4.2): one layout format for
//! the ZeeZ Live overlay, the mini monitor, the tray and the Overview tiles.
//!
//! A layout is a list of widgets, each showing one metric (an exact id such
//! as `cpu/temp/package`, or an alias such as `@cpu_temp` that the app
//! resolves to the best sensor on each machine), with value thresholds that
//! colour it. Layouts are plain JSON with a schema version, so users can keep,
//! share and edit them; [`Layout::from_json`] migrates older versions and
//! never silently accepts a newer one.

use serde::{Deserialize, Serialize};

/// Current schema version of [`Layout`].
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    #[default]
    Vertical,
    Horizontal,
    /// `Layout::columns` per row.
    Grid,
}

/// Colours of a surface: the app's theme, or always dark / light.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    Follow,
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// Just the value ("62 °C").
    Value,
    /// Label and value ("CPU 62 °C").
    LabelValue,
    /// Label, value and a bar from `min` to `max`.
    Bar,
    /// Label, value and a small history graph.
    Sparkline,
    /// A group title (no metric).
    Header,
    Separator,
    /// Local time (no metric).
    Clock,
    /// One pill per throttle that is on now ("CPU thermal", "GPU power"), or a
    /// quiet "no throttling". `metric` may be `@throttle_cpu` or `@throttle_gpu`
    /// to watch one device; empty or `@throttle` watches both.
    Throttle,
}

/// The throttles to show: `active` holds `cpu-thermal`, `cpu-power`,
/// `gpu-thermal` and `gpu-power`; `metric` is the widget's (see [`Kind::Throttle`]).
/// Returned in a fixed order, so pills do not jump around.
pub fn throttle_pills<'a>(active: &[&str], metric: Option<&str>) -> Vec<&'a str> {
    let device = match metric {
        Some("@throttle_cpu") => "cpu-",
        Some("@throttle_gpu") => "gpu-",
        _ => "",
    };
    ["cpu-thermal", "cpu-power", "gpu-thermal", "gpu-power"]
        .into_iter()
        .filter(|k| k.starts_with(device) && active.contains(k))
        .collect()
}

/// How a value reads against its thresholds (the GUI maps it to colours, and
/// always pairs colour with an icon or text for colour-blind users).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Tone {
    #[default]
    Normal,
    Good,
    Warning,
    Danger,
}

/// From `at` upwards the value reads as `tone` (the highest matching wins).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Threshold {
    pub at: f64,
    pub tone: Tone,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Widget {
    pub kind: Kind,
    /// Metric id or `@alias`; None for headers, separators and the clock.
    pub metric: Option<String>,
    /// Shown label; None = the metric's own label.
    pub label: Option<String>,
    /// Decimal places; None = automatic for the unit.
    pub decimals: Option<u8>,
    pub thresholds: Vec<Threshold>,
    /// Bar range.
    pub min: f64,
    pub max: f64,
}

impl Default for Widget {
    fn default() -> Self {
        Widget {
            kind: Kind::LabelValue,
            metric: None,
            label: None,
            decimals: None,
            thresholds: Vec::new(),
            min: 0.0,
            max: 100.0,
        }
    }
}

impl Widget {
    pub fn metric(kind: Kind, metric: &str, label: &str) -> Widget {
        Widget { kind, metric: Some(metric.into()), label: Some(label.into()), ..Default::default() }
    }

    pub fn header(label: &str) -> Widget {
        Widget { kind: Kind::Header, label: Some(label.into()), ..Default::default() }
    }

    pub fn with_thresholds(mut self, t: &[(f64, Tone)]) -> Widget {
        self.thresholds = t.iter().map(|&(at, tone)| Threshold { at, tone }).collect();
        self
    }

    pub fn range(mut self, min: f64, max: f64) -> Widget {
        (self.min, self.max) = (min, max);
        self
    }

    /// The tone of `value` under this widget's thresholds.
    pub fn tone(&self, value: f64) -> Tone {
        self.thresholds.iter().filter(|t| value >= t.at).map(|t| t.tone).max().unwrap_or_default()
    }

    /// Bar fill in 0..=1.
    pub fn fill(&self, value: f64) -> f32 {
        let span = self.max - self.min;
        if span <= 0.0 { 0.0 } else { ((value - self.min) / span).clamp(0.0, 1.0) as f32 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
    /// Text scale (1.0 = the UI's body size).
    pub scale: f32,
    /// Background opacity, 0..=1.
    pub opacity: f32,
    /// Outline text so it reads over any background (overlay).
    pub outline: bool,
    /// Corner radius in points.
    pub corner: u8,
    pub padding: u8,
    pub theme: Theme,
}

impl Default for Style {
    fn default() -> Self {
        Style { scale: 1.0, opacity: 0.85, outline: false, corner: 8, padding: 8, theme: Theme::Follow }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub version: u32,
    pub name: String,
    pub direction: Direction,
    /// Widgets per row in a grid.
    pub columns: u8,
    pub style: Style,
    pub widgets: Vec<Widget>,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            version: VERSION,
            name: String::new(),
            direction: Direction::Vertical,
            columns: 2,
            style: Style::default(),
            widgets: Vec::new(),
        }
    }
}

impl Layout {
    /// Parse a saved layout, migrating older schema versions. A layout from a
    /// newer app is refused rather than half-read.
    pub fn from_json(s: &str) -> Result<Layout, String> {
        let v: serde_json::Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
        let ver = v.get("version").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
        if ver > VERSION {
            return Err(format!("layout version {ver} is newer than this app understands ({VERSION})"));
        }
        // Migrations go here as `if ver < N { v = migrate_to_n(v) }`.
        let mut l: Layout = serde_json::from_value(v).map_err(|e| e.to_string())?;
        l.version = VERSION;
        Ok(l)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Every metric the layout shows (for the Metric Engine lease).
    pub fn metrics(&self) -> Vec<&str> {
        self.widgets.iter().filter_map(|w| w.metric.as_deref()).collect()
    }
}

/// Temperature thresholds relative to a limit (Tjmax or the warning setting).
pub fn temp_thresholds(limit: f64) -> Vec<(f64, Tone)> {
    vec![(limit - 10.0, Tone::Warning), (limit, Tone::Danger)]
}

/// Built-in layouts. They use aliases, so they work on every machine.
pub mod presets {
    use super::*;

    /// The mini monitor's default: temperatures, clock and power.
    pub fn mini(tjmax: f64) -> Layout {
        Layout {
            name: "Mini".into(),
            widgets: vec![
                Widget::metric(Kind::LabelValue, "@cpu_temp", "CPU").with_thresholds(&temp_thresholds(tjmax)),
                Widget::metric(Kind::LabelValue, "@gpu_temp", "GPU").with_thresholds(&temp_thresholds(90.0)),
                Widget::metric(Kind::LabelValue, "@chipset_temp", "Chipset"),
                Widget::metric(Kind::LabelValue, "@cpu_clock", "Clock"),
                Widget::metric(Kind::LabelValue, "@cpu_power", "Power"),
            ],
            ..Default::default()
        }
    }

    /// One horizontal line (the ZeeZ Live "Compact" style once FPS exists).
    pub fn compact(tjmax: f64) -> Layout {
        Layout {
            name: "Compact".into(),
            direction: Direction::Horizontal,
            widgets: vec![
                Widget::metric(Kind::LabelValue, "@cpu_temp", "CPU").with_thresholds(&temp_thresholds(tjmax)),
                Widget::metric(Kind::LabelValue, "@cpu_load", "CPU %").with_thresholds(&[(90.0, Tone::Warning)]),
                Widget::metric(Kind::LabelValue, "@gpu_temp", "GPU").with_thresholds(&temp_thresholds(90.0)),
                Widget::metric(Kind::LabelValue, "@gpu_load", "GPU %"),
                Widget::metric(Kind::LabelValue, "@mem_load", "RAM").with_thresholds(&[(90.0, Tone::Warning)]),
            ],
            ..Default::default()
        }
    }

    /// Grouped, with bars and graphs.
    pub fn detailed(tjmax: f64) -> Layout {
        Layout {
            name: "Detailed".into(),
            widgets: vec![
                Widget::header("CPU"),
                Widget::metric(Kind::Sparkline, "@cpu_temp", "Temperature").with_thresholds(&temp_thresholds(tjmax)),
                Widget::metric(Kind::Bar, "@cpu_load", "Load"),
                Widget::metric(Kind::Sparkline, "@cpu_clock", "Clock"),
                Widget::metric(Kind::LabelValue, "@cpu_power", "Power"),
                Widget::header("GPU"),
                Widget::metric(Kind::Sparkline, "@gpu_temp", "Temperature").with_thresholds(&temp_thresholds(90.0)),
                Widget::metric(Kind::Bar, "@gpu_load", "Load"),
                Widget::header("Memory"),
                Widget::metric(Kind::Bar, "@mem_load", "Load").with_thresholds(&[(90.0, Tone::Warning)]),
            ],
            ..Default::default()
        }
    }

    /// Four readings in two columns.
    pub fn grid(tjmax: f64) -> Layout {
        Layout {
            name: "Grid".into(),
            direction: Direction::Grid,
            columns: 2,
            widgets: vec![
                Widget::metric(Kind::LabelValue, "@cpu_temp", "CPU").with_thresholds(&temp_thresholds(tjmax)),
                Widget::metric(Kind::LabelValue, "@gpu_temp", "GPU").with_thresholds(&temp_thresholds(90.0)),
                Widget::metric(Kind::LabelValue, "@cpu_load", "CPU %").with_thresholds(&[(90.0, Tone::Warning)]),
                Widget::metric(Kind::LabelValue, "@gpu_load", "GPU %"),
            ],
            ..Default::default()
        }
    }

    /// One big number (the CPU temperature).
    pub fn big(tjmax: f64) -> Layout {
        Layout {
            name: "Big number".into(),
            style: Style { scale: 3.0, ..Style::default() },
            widgets: vec![Widget::metric(Kind::Value, "@cpu_temp", "CPU").with_thresholds(&temp_thresholds(tjmax))],
            ..Default::default()
        }
    }

    pub fn all(tjmax: f64) -> Vec<Layout> {
        vec![mini(tjmax), compact(tjmax), detailed(tjmax), grid(tjmax), big(tjmax)]
    }

    /// Frame metrics (ZeeZ Live's frame source; empty until a game presents).
    fn fps(label: &str) -> Widget {
        Widget::metric(Kind::LabelValue, "frames/fps", label)
    }

    fn low1() -> Widget {
        Widget::metric(Kind::LabelValue, "frames/low1", "1% low")
    }

    /// ZeeZ Live overlay styles (zeez-live §5.2), all editable.
    pub mod live {
        use super::super::*;
        use super::{fps, low1};

        /// One number in a corner: FPS.
        pub fn minimal() -> Layout {
            Layout {
                name: "Minimal".into(),
                style: Style { scale: 1.6, ..Style::default() },
                widgets: vec![Widget::metric(Kind::Value, "frames/fps", "FPS").with_thresholds(&[])],
                ..Default::default()
            }
        }

        /// One line: FPS · 1% low · GPU °C/% · CPU °C/% · RAM.
        pub fn compact(tjmax: f64) -> Layout {
            Layout {
                name: "Compact".into(),
                direction: Direction::Horizontal,
                widgets: vec![
                    fps("FPS"),
                    low1(),
                    Widget::metric(Kind::LabelValue, "@gpu_temp", "GPU").with_thresholds(&temp_thresholds(90.0)),
                    Widget::metric(Kind::LabelValue, "@gpu_load", "GPU %"),
                    Widget::metric(Kind::LabelValue, "@cpu_temp", "CPU").with_thresholds(&temp_thresholds(tjmax)),
                    Widget::metric(Kind::LabelValue, "@cpu_load", "CPU %"),
                    Widget::metric(Kind::LabelValue, "@mem_load", "RAM").with_thresholds(&[(90.0, Tone::Warning)]),
                ],
                ..Default::default()
            }
        }

        /// Frames / GPU / CPU / Memory groups with values and bars.
        pub fn detailed(tjmax: f64) -> Layout {
            Layout {
                name: "Detailed".into(),
                widgets: vec![
                    Widget::header("Frames"),
                    fps("FPS"),
                    low1(),
                    Widget::metric(Kind::LabelValue, "frames/ms", "Frame time"),
                    Widget::header("GPU"),
                    Widget::metric(Kind::Bar, "@gpu_load", "Load"),
                    Widget::metric(Kind::LabelValue, "@gpu_temp", "Temperature")
                        .with_thresholds(&temp_thresholds(90.0)),
                    Widget::metric(Kind::LabelValue, "@gpu_power", "Power"),
                    Widget::header("CPU"),
                    Widget::metric(Kind::Bar, "@cpu_load", "Load"),
                    Widget::metric(Kind::Bar, "@cpu_max_core", "Busiest core"),
                    Widget::metric(Kind::LabelValue, "@cpu_temp", "Temperature")
                        .with_thresholds(&temp_thresholds(tjmax)),
                    Widget::header("Memory"),
                    Widget::metric(Kind::Bar, "@mem_load", "RAM").with_thresholds(&[(90.0, Tone::Warning)]),
                ],
                ..Default::default()
            }
        }

        /// FPS, 1%/0.1% lows, the frame-time graph, GPU Busy, the busiest core and a clock.
        pub fn benchmark() -> Layout {
            Layout {
                name: "Benchmark".into(),
                widgets: vec![
                    fps("FPS"),
                    low1(),
                    Widget::metric(Kind::LabelValue, "frames/low01", "0.1% low"),
                    Widget::metric(Kind::Sparkline, "frames/ms", "Frame time"),
                    Widget::metric(Kind::LabelValue, "frames/gpu_busy", "GPU Busy"),
                    Widget::metric(Kind::LabelValue, "@cpu_max_core", "Busiest core"),
                    Widget { kind: Kind::Throttle, ..Default::default() },
                    Widget { kind: Kind::Clock, ..Widget::header("") },
                ],
                ..Default::default()
            }
        }

        /// Clocks, voltage, power and temperatures in a grid.
        pub fn overclocker(tjmax: f64) -> Layout {
            Layout {
                name: "Overclocker".into(),
                direction: Direction::Grid,
                columns: 2,
                widgets: vec![
                    Widget::metric(Kind::LabelValue, "@cpu_clock", "CPU clock"),
                    Widget::metric(Kind::LabelValue, "@gpu_clock", "GPU clock"),
                    Widget::metric(Kind::LabelValue, "@cpu_vid", "VID"),
                    Widget::metric(Kind::LabelValue, "@cpu_power", "CPU power"),
                    Widget::metric(Kind::LabelValue, "@cpu_temp", "CPU").with_thresholds(&temp_thresholds(tjmax)),
                    Widget::metric(Kind::LabelValue, "@gpu_temp", "GPU").with_thresholds(&temp_thresholds(90.0)),
                    Widget::metric(Kind::LabelValue, "@gpu_power", "GPU power"),
                    fps("FPS"),
                    Widget { kind: Kind::Throttle, ..Default::default() },
                ],
                ..Default::default()
            }
        }

        pub fn all(tjmax: f64) -> Vec<Layout> {
            vec![minimal(), compact(tjmax), detailed(tjmax), benchmark(), overclocker(tjmax)]
        }
    }

    /// The Overview's tile sets (S1-G01). "Default" is the pre-1.0 Overview.
    pub const OVERVIEW: [&str; 5] = ["Default", "Gamer", "Overclocker", "Laptop", "Workstation"];

    pub fn overview(name: &str, tjmax: f64) -> Vec<Widget> {
        let t = |m: &str, l: &str| Widget::metric(Kind::Sparkline, m, l);
        let temp = |m: &str, l: &str, lim: f64| t(m, l).with_thresholds(&temp_thresholds(lim));
        let load = |m: &str, l: &str| t(m, l).range(0.0, 100.0).with_thresholds(&[(90.0, Tone::Warning)]);
        match name {
            "Gamer" => vec![
                temp("@cpu_temp", "CPU temperature", tjmax),
                load("@cpu_load", "CPU load"),
                temp("@gpu_temp", "GPU temperature", 90.0),
                load("@gpu_load", "GPU load"),
            ],
            "Overclocker" => vec![
                temp("@cpu_temp", "CPU temperature", tjmax),
                t("@cpu_clock", "CPU clock"),
                t("@cpu_power", "CPU power"),
                t("@cpu_vid", "CPU voltage"),
            ],
            "Laptop" => vec![
                temp("@cpu_temp", "CPU temperature", tjmax),
                t("@cpu_power", "CPU power"),
                t("@battery_charge", "Battery"),
                t("@battery_rate", "Charge rate"),
            ],
            "Workstation" => vec![
                load("@cpu_load", "CPU load"),
                load("@mem_load", "Memory load"),
                load("@disk_active", "Disk active time"),
                t("@net_rx", "Download"),
            ],
            _ => vec![
                temp("@cpu_temp", "CPU temperature", tjmax),
                t("@cpu_clock", "CPU clock"),
                temp("@gpu_temp", "GPU temperature", 90.0),
                temp("@chipset_temp", "Chipset temperature", 90.0),
            ],
        }
    }
}

/// Mini monitors (S1-M02): each one is a layout shown in its own small
/// always-on-top window, with where it sits and how it behaves.
pub mod mini {
    use super::Layout;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct Mini {
        pub layout: Layout,
        pub open: bool,
        /// Top-left corner on the desktop (points); None = the OS picks.
        pub pos: Option<[f32; 2]>,
        /// Click-through: it becomes part of the desktop.
        pub locked: bool,
        pub hide_fullscreen: bool,
        /// Show more detail while the pointer is over it.
        pub expand_hover: bool,
    }

    impl Default for Mini {
        fn default() -> Self {
            Mini {
                layout: super::presets::mini(95.0),
                open: true,
                pos: None,
                locked: false,
                hide_fullscreen: true,
                expand_hover: false,
            }
        }
    }

    /// Snaps a window at `pos` (size `size`) to the edges of a `screen`-sized
    /// area when it's within `dist` of one.
    pub fn snap(pos: [f32; 2], size: [f32; 2], screen: [f32; 2], dist: f32) -> [f32; 2] {
        let axis = |p: f32, s: f32, max: f32| {
            if p.abs() <= dist {
                0.0
            } else if (max - (p + s)).abs() <= dist {
                max - s
            } else {
                p
            }
        };
        [axis(pos[0], size[0], screen[0]), axis(pos[1], size[1], screen[1])]
    }

    /// The hover-expanded version: every reading gets its graph, one per line.
    pub fn expanded(l: &Layout) -> Layout {
        use super::{Direction, Kind};
        let mut e = l.clone();
        e.direction = Direction::Vertical;
        for w in &mut e.widgets {
            if matches!(w.kind, Kind::Value | Kind::LabelValue) {
                w.kind = Kind::Sparkline;
            }
        }
        e.style.scale = e.style.scale.min(1.5);
        e
    }
}

/// Tray icons (S1-M01): each one is a Widget Engine widget (metric,
/// thresholds, range) drawn in one of four styles.
pub mod tray {
    use super::{Kind, Widget};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
    #[serde(rename_all = "kebab-case")]
    pub enum Style {
        #[default]
        Number,
        Bar,
        Graph,
        Ring,
    }

    impl Style {
        pub const ALL: [Style; 4] = [Style::Number, Style::Bar, Style::Graph, Style::Ring];
        pub fn label(self) -> &'static str {
            match self {
                Style::Number => "Number",
                Style::Bar => "Bar",
                Style::Graph => "Mini graph",
                Style::Ring => "Ring",
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct Item {
        pub widget: Widget,
        pub style: Style,
    }

    impl Default for Item {
        fn default() -> Self {
            Item { widget: Widget::metric(Kind::Value, "@cpu_temp", "CPU"), style: Style::Number }
        }
    }

    /// What a left click on an icon does.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
    #[serde(rename_all = "kebab-case")]
    pub enum Click {
        #[default]
        App,
        Mini,
    }

    #[cfg(test)]
    #[test]
    fn mini_snaps_and_expands() {
        use crate::layout::{Direction, Kind, mini::*, presets};
        let scr = [1920.0, 1080.0];
        assert_eq!(snap([10.0, 500.0], [200.0, 100.0], scr, 20.0), [0.0, 500.0]);
        assert_eq!(snap([1715.0, 975.0], [200.0, 100.0], scr, 20.0), [1720.0, 980.0], "right/bottom edges");
        assert_eq!(snap([600.0, 500.0], [200.0, 100.0], scr, 20.0), [600.0, 500.0], "far from edges: unchanged");
        let e = expanded(&presets::compact(95.0));
        assert_eq!(e.direction, Direction::Vertical);
        assert!(e.widgets.iter().all(|w| w.kind == Kind::Sparkline));
        let m = Mini::default();
        let back: Mini = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_pills_follow_the_filter_in_a_fixed_order() {
        let on = ["gpu-power", "cpu-thermal", "gpu-thermal"];
        assert_eq!(throttle_pills(&on, None), ["cpu-thermal", "gpu-thermal", "gpu-power"]);
        assert_eq!(throttle_pills(&on, Some("@throttle")), throttle_pills(&on, None));
        assert_eq!(throttle_pills(&on, Some("@throttle_cpu")), ["cpu-thermal"]);
        assert_eq!(throttle_pills(&on, Some("@throttle_gpu")), ["gpu-thermal", "gpu-power"]);
        assert!(throttle_pills(&[], None).is_empty());
        assert!(throttle_pills(&["bogus"], None).is_empty(), "unknown keys never show");
    }

    #[test]
    fn a_throttle_widget_survives_json_and_the_live_presets_have_one() {
        let w = Widget { kind: Kind::Throttle, metric: Some("@throttle_gpu".into()), ..Default::default() };
        let back: Widget = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!((back.kind, back.metric.as_deref()), (Kind::Throttle, Some("@throttle_gpu")));
        for name in ["Benchmark", "Overclocker"] {
            let l = presets::live::all(95.0).into_iter().find(|l| l.name == name).unwrap();
            assert!(l.widgets.iter().any(|w| w.kind == Kind::Throttle), "{name} shows throttling");
        }
    }

    #[test]
    fn overview_presets_have_four_tiles_each() {
        for name in presets::OVERVIEW {
            let tiles = presets::overview(name, 95.0);
            assert_eq!(tiles.len(), 4, "{name}");
            assert!(tiles.iter().all(|w| w.metric.as_deref().is_some_and(|m| m.starts_with('@'))), "{name}");
        }
        assert_eq!(presets::overview("nope", 95.0), presets::overview("Default", 95.0));
    }

    #[test]
    fn thresholds_and_bars() {
        let w =
            Widget::metric(Kind::Bar, "@cpu_temp", "CPU").with_thresholds(&temp_thresholds(95.0)).range(20.0, 100.0);
        assert_eq!(w.tone(60.0), Tone::Normal);
        assert_eq!(w.tone(85.0), Tone::Warning);
        assert_eq!(w.tone(96.0), Tone::Danger, "the highest matching threshold wins");
        assert_eq!(w.fill(60.0), 0.5);
        assert_eq!((w.fill(0.0), w.fill(200.0)), (0.0, 1.0));
    }

    #[test]
    fn json_round_trip_and_versions() {
        for l in presets::all(95.0) {
            assert_eq!(Layout::from_json(&l.to_json()), Ok(l.clone()), "{}", l.name);
            assert!(!l.metrics().is_empty());
        }
        // Missing keys take defaults; an unversioned file is version 1.
        let l = Layout::from_json(r#"{"name":"x","widgets":[{"kind":"value","metric":"cpu/temp/package"}]}"#).unwrap();
        assert_eq!((l.version, l.widgets[0].max, l.style.scale), (VERSION, 100.0, 1.0));
        assert!(Layout::from_json(r#"{"version":99}"#).is_err(), "a newer layout is refused, not half-read");
        assert!(Layout::from_json("not json").is_err());
    }
}

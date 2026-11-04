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
}

impl Default for Style {
    fn default() -> Self {
        Style { scale: 1.0, opacity: 0.85, outline: false, corner: 8, padding: 8 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub version: u32,
    pub name: String,
    pub direction: Direction,
    pub style: Style,
    pub widgets: Vec<Widget>,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            version: VERSION,
            name: String::new(),
            direction: Direction::Vertical,
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

    pub fn all(tjmax: f64) -> Vec<Layout> {
        vec![mini(tjmax), compact(tjmax), detailed(tjmax)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

//! Local alerts (S1-G01): opt-in rules like "CPU above 90 °C for 30 s",
//! checked against the metric history on this machine. They show on the
//! Overview and in the tray, and are never sent anywhere (P1).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rule {
    /// A metric id or alias (`@cpu_temp`).
    pub metric: String,
    /// Shown in the alert ("CPU temperature").
    pub label: String,
    /// Fires above the threshold (false: below).
    pub above: bool,
    /// In the metric's base unit (°C, %, MHz…).
    pub threshold: f64,
    /// For at least this long.
    pub secs: u32,
    pub enabled: bool,
}

impl Default for Rule {
    fn default() -> Self {
        Rule { metric: String::new(), label: String::new(), above: true, threshold: 0.0, secs: 30, enabled: true }
    }
}

/// The starter rules (each can be switched off or tuned).
pub fn defaults() -> Vec<Rule> {
    let r = |metric: &str, label: &str, threshold: f64, secs: u32| Rule {
        metric: metric.into(),
        label: label.into(),
        threshold,
        secs,
        ..Default::default()
    };
    vec![
        r("@cpu_temp", "CPU temperature", 90.0, 30),
        r("@gpu_temp", "GPU temperature", 85.0, 30),
        r("@mem_load", "Memory load", 95.0, 60),
        r("@disk_active", "Disk active time", 98.0, 120),
    ]
}

/// Whether `rule` holds for its whole window, ending at the latest point.
/// `points` are (seconds, value), oldest first. A window the history doesn't
/// cover yet (just started, a sensor that dropped out) doesn't fire.
pub fn firing(rule: &Rule, points: &[(f64, f64)]) -> bool {
    let Some(&(now, _)) = points.last() else { return false };
    let from = now - rule.secs as f64;
    let window: Vec<f64> = points.iter().filter(|p| p.0 >= from).map(|p| p.1).collect();
    let covered = points.first().is_some_and(|p| p.0 <= from + 1.5);
    let holds = |v: f64| if rule.above { v > rule.threshold } else { v < rule.threshold };
    rule.enabled && covered && !window.is_empty() && window.iter().all(|&v| holds(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(vals: &[f64]) -> Vec<(f64, f64)> {
        vals.iter().enumerate().map(|(i, v)| (i as f64, *v)).collect()
    }

    #[test]
    fn fires_only_when_the_whole_window_is_over() {
        let r = Rule { metric: "@cpu_temp".into(), threshold: 90.0, secs: 5, ..Default::default() };
        assert!(firing(&r, &pts(&[80.0, 91.0, 92.0, 93.0, 94.0, 95.0, 96.0])));
        assert!(!firing(&r, &pts(&[91.0, 92.0, 89.0, 93.0, 94.0, 95.0, 96.0])), "a dip inside the window");
        assert!(!firing(&r, &pts(&[95.0, 96.0])), "the history doesn't cover 5 s yet");
        assert!(!firing(&Rule { enabled: false, ..r.clone() }, &pts(&[95.0; 8])));
        let below = Rule { above: false, threshold: 10.0, ..r };
        assert!(firing(&below, &pts(&[5.0; 8])));
        assert!(!firing(&below, &[]));
    }

    #[test]
    fn rules_load_from_older_settings() {
        let r: Rule = serde_json::from_str(r#"{"metric":"@cpu_temp","threshold":80}"#).unwrap();
        assert_eq!((r.secs, r.above, r.enabled), (30, true, true));
        assert_eq!(defaults().len(), 4);
    }
}

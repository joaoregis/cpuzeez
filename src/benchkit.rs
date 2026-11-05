//! Benchmark principles (S1-K01), shared by every benchmark:
//! - **Safe:** tests only write a test file in a folder the user picks
//!   (storage) and never change settings.
//! - **Reproducible:** each result records the conditions it ran under
//!   (version, power plan, starting CPU temperature, number of runs), and a
//!   "run N times" mode reports the median and the spread.
//! - **Our own documented score:** 1000 points = the reference machine
//!   (Ryzen 9 7900X, `reference/`), per sub-test; the total is the geometric
//!   mean of the sub-test scores. Not comparable with other tools.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// One measured figure of a benchmark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sub {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub higher_better: bool,
    /// Points against the reference machine (1000 = equal), when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<f64>,
    /// (max - min) / median across runs, in %, when it ran more than once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<f64>,
}

impl Sub {
    pub fn new(name: &str, value: f64, unit: &str, higher_better: bool) -> Sub {
        Sub { name: name.into(), value, unit: unit.into(), higher_better, points: None, spread: None }
    }

    /// Scores this figure against the reference machine's `reference` value.
    pub fn scored(mut self, reference: f64) -> Sub {
        self.points = points(self.value, reference, self.higher_better);
        self
    }
}

/// 1000 × value / reference (or its inverse when lower is better).
pub fn points(value: f64, reference: f64, higher_better: bool) -> Option<f64> {
    if value <= 0.0 || reference <= 0.0 {
        return None;
    }
    Some(1000.0 * if higher_better { value / reference } else { reference / value })
}

/// Geometric mean of the scored sub-tests: one slow sub-test can't be hidden
/// by a fast one, and every sub-test weighs the same.
pub fn total(subs: &[Sub]) -> Option<f64> {
    let p: Vec<f64> = subs.iter().filter_map(|s| s.points).collect();
    (!p.is_empty()).then(|| (p.iter().map(|x| x.ln()).sum::<f64>() / p.len() as f64).exp())
}

/// (median, spread in %) of `v`.
pub fn median_spread(v: &[f64]) -> Option<(f64, f64)> {
    let mut v: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    let median = if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 };
    let spread = if median != 0.0 { (v[n - 1] - v[0]) / median.abs() * 100.0 } else { 0.0 };
    Some((median, spread))
}

/// Merges N runs of the same benchmark: each sub-test becomes its median,
/// with the spread when there was more than one run.
pub fn merge(runs: &[Vec<Sub>]) -> Vec<Sub> {
    let Some(first) = runs.first() else { return Vec::new() };
    first
        .iter()
        .map(|s| {
            let vals: Vec<f64> =
                runs.iter().filter_map(|r| r.iter().find(|x| x.name == s.name)).map(|x| x.value).collect();
            let pts: Vec<f64> =
                runs.iter().filter_map(|r| r.iter().find(|x| x.name == s.name)).filter_map(|x| x.points).collect();
            let (value, spread) = median_spread(&vals).unwrap_or((s.value, 0.0));
            Sub {
                value,
                points: median_spread(&pts).map(|m| m.0),
                spread: (runs.len() > 1).then_some(spread),
                ..s.clone()
            }
        })
        .collect()
}

/// What a result ran under, saved with it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Conditions {
    pub app_version: String,
    #[serde(default)]
    pub power_plan: Option<String>,
    #[serde(default)]
    pub start_temp_c: Option<f64>,
    pub runs: usize,
}

pub fn conditions(start_temp_c: Option<f64>, runs: usize) -> Conditions {
    let p = cpuzeez_platform::cpu_os::power();
    let plan = p.plan.or(match (p.governor, p.epp) {
        (Some(g), Some(e)) => Some(format!("{g} / {e}")),
        (g, _) => g,
    });
    Conditions { app_version: crate::VERSION.into(), power_plan: plan, start_temp_c, runs: runs.max(1) }
}

/// Progress, cancel flag and result of a benchmark running on its own thread.
pub struct Job<T> {
    ctl: Arc<Ctl>,
    result: Arc<Mutex<Option<Result<T, String>>>>,
}

/// The running side of a [`Job`].
#[derive(Default)]
pub struct Ctl {
    stage: Mutex<(String, f32)>,
    cancel: AtomicBool,
}

impl Ctl {
    /// Stage text and progress (0..=1).
    pub fn set(&self, stage: &str, f: f32) {
        *self.stage.lock().unwrap_or_else(|e| e.into_inner()) = (stage.to_string(), f.clamp(0.0, 1.0));
    }

    /// The flag itself, for loops that poll an `&AtomicBool`.
    pub fn cancelled_flag(&self) -> &AtomicBool {
        &self.cancel
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// `Err("cancelled")` once cancelled, for `?`.
    pub fn check(&self) -> Result<(), String> {
        if self.cancelled() { Err("cancelled".into()) } else { Ok(()) }
    }
}

impl<T: Send + 'static> Job<T> {
    pub fn spawn(f: impl FnOnce(&Ctl) -> Result<T, String> + Send + 'static) -> Job<T> {
        let job = Job { ctl: Arc::new(Ctl::default()), result: Arc::new(Mutex::new(None)) };
        let (ctl, result) = (job.ctl.clone(), job.result.clone());
        std::thread::spawn(move || {
            let r = f(&ctl);
            *result.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
        });
        job
    }

    /// The result, once (None while running).
    pub fn take(&self) -> Option<Result<T, String>> {
        self.result.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    pub fn stage(&self) -> (String, f32) {
        self.ctl.stage.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn cancel(&self) {
        self.ctl.cancel.store(true, Ordering::Relaxed);
    }

    /// Blocks until done (tests).
    #[cfg(test)]
    pub fn wait(self) -> Result<T, String> {
        loop {
            if let Some(r) = self.take() {
                return r;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

/// Runs `f` `runs` times and merges the results (median + spread).
pub fn repeat(
    ctl: &Ctl,
    runs: usize,
    mut f: impl FnMut(&Ctl, usize) -> Result<Vec<Sub>, String>,
) -> Result<Vec<Sub>, String> {
    let mut all = Vec::new();
    for i in 0..runs.max(1) {
        ctl.check()?;
        all.push(f(ctl, i)?);
    }
    Ok(merge(&all))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_and_medians() {
        assert_eq!(points(200.0, 100.0, true), Some(2000.0));
        assert_eq!(points(50.0, 100.0, false), Some(2000.0), "lower is better: half the time = double");
        assert_eq!(points(0.0, 100.0, true), None);
        assert_eq!(median_spread(&[3.0, 1.0, 2.0]), Some((2.0, 100.0)));
        assert_eq!(median_spread(&[1.0, 3.0]), Some((2.0, 100.0)));
        assert_eq!(median_spread(&[]), None);
        let s = |v: f64| vec![Sub::new("a", v, "x", true).scored(10.0)];
        let m = merge(&[s(10.0), s(12.0), s(11.0)]);
        assert_eq!(m[0].value, 11.0);
        assert_eq!(m[0].points, Some(1100.0));
        assert!((m[0].spread.unwrap() - 2.0 / 11.0 * 100.0).abs() < 1e-9);
        assert_eq!(merge(&[s(10.0)])[0].spread, None, "one run has no spread");
        let t = total(&[Sub::new("a", 1.0, "", true).scored(0.5), Sub::new("b", 1.0, "", true).scored(2.0)]);
        assert!((t.unwrap() - 1000.0).abs() < 1e-9, "geomean of 2000 and 500");
    }

    #[test]
    fn job_runs_and_cancels() {
        let j = Job::spawn(|ctl| {
            ctl.set("x", 0.5);
            Ok(7)
        });
        assert_eq!(j.wait(), Ok(7));
        let j: Job<()> = Job::spawn(|ctl| {
            while !ctl.cancelled() {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            ctl.check()
        });
        j.cancel();
        assert_eq!(j.wait(), Err("cancelled".into()));
        let c = conditions(Some(40.0), 0);
        assert_eq!((c.runs, c.start_temp_c), (1, Some(40.0)));
    }
}

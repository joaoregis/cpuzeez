//! ZeeZ Live capture sessions (S2b-09, S2b-10; docs/v1/zeez-live.md §6):
//! what `session.json` holds, folder names, and the checks a comparison
//! needs (same game, resolution and driver; repeat variance).

use crate::frames::{LowConvention, Summary};
use serde::{Deserialize, Serialize};

/// The machine a session ran on, as far as CpuZeeZ knows it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct System {
    pub cpu: String,
    pub gpu: String,
    pub gpu_driver: String,
    pub ram_gb: f64,
    pub os: String,
    /// "2560x1440" when known.
    pub resolution: String,
    /// The display's refresh rate, when every display runs the same one.
    pub refresh_hz: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub t_s: f64,
    pub label: String,
}

/// A span with a CPU or GPU throttling flag set.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Band {
    pub from_s: f64,
    pub to_s: f64,
    pub label: String,
    /// "cpu-thermal", "cpu-power", "gpu-thermal" or "gpu-power" (empty in old sessions).
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    #[default]
    Manual,
    Timed,
    Automatic,
    /// Frames imported from another tool's CSV.
    Imported,
}

/// `session.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    /// File format version.
    pub version: u32,
    pub game: String,
    pub pid: u32,
    /// Start, ISO 8601 local time.
    pub started: String,
    pub duration_s: f64,
    pub mode: Mode,
    /// Where the frames came from: "PresentMon 2.6.0", "MangoHud", "CSV import".
    pub source: String,
    pub system: System,
    pub markers: Vec<Marker>,
    pub throttling: Vec<Band>,
    pub summary: Summary,
    pub convention: LowConvention,
    /// The user's note.
    pub note: String,
}

pub const VERSION: u32 = 1;

/// `<yyyy-mm-dd_hhmm>_<game>` with the game's name made safe for a folder.
pub fn folder_name(started_ymd_hm: &str, game: &str) -> String {
    let stem = game.rsplit(['/', '\\']).next().unwrap_or(game);
    let stem = stem.strip_suffix(".exe").or_else(|| stem.strip_suffix(".EXE")).unwrap_or(stem);
    let safe: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(48)
        .collect();
    let safe = if safe.trim_matches('_').is_empty() { "session".to_string() } else { safe };
    format!("{started_ymd_hm}_{safe}")
}

/// Why two sessions shouldn't be compared as like for like (empty: fine).
pub fn comparability(a: &Session, b: &Session) -> Vec<&'static str> {
    let mut v = Vec::new();
    if !a.game.eq_ignore_ascii_case(&b.game) {
        v.push("Different games");
    }
    if !a.system.resolution.is_empty() && !b.system.resolution.is_empty() && a.system.resolution != b.system.resolution
    {
        v.push("Different resolutions");
    }
    if !a.system.gpu_driver.is_empty() && !b.system.gpu_driver.is_empty() && a.system.gpu_driver != b.system.gpu_driver
    {
        v.push("Different GPU drivers");
    }
    if a.convention != b.convention {
        v.push("Different 1% low conventions");
    }
    v
}

/// Repeats of one test vary too much when the average FPS spreads more than
/// 5 % around the median: "run it 3x and use the median".
pub fn high_variance(avg_fps: &[f64]) -> bool {
    if avg_fps.len() < 2 {
        return false;
    }
    let mut s = avg_fps.to_vec();
    s.sort_by(f64::total_cmp);
    let median = crate::frames::percentile(&s, 50.0);
    median > 0.0 && (s[s.len() - 1] - s[0]) / median > 0.05
}

/// The median of each summary figure across repeats (avg, 1 %, 0.1 %).
pub fn median_of(runs: &[Summary]) -> (f64, f64, f64) {
    let med = |f: fn(&Summary) -> f64| {
        let mut v: Vec<f64> = runs.iter().map(f).collect();
        v.sort_by(f64::total_cmp);
        crate::frames::percentile(&v, 50.0)
    };
    (med(|s| s.avg_fps), med(|s| s.low1_fps), med(|s| s.low01_fps))
}

/// A capture's `hw.csv` (the telemetry CSV format: `time,<ids>`, a
/// `# units` line, then ISO 8601 rows), with times in seconds from the first row.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HwLog {
    pub ids: Vec<String>,
    pub units: Vec<String>,
    /// (seconds, one value per id; None where the row has none).
    pub rows: Vec<(f64, Vec<Option<f64>>)>,
}

impl HwLog {
    /// (seconds, value) of one metric.
    pub fn series(&self, id: &str) -> Vec<(f64, f64)> {
        let Some(k) = self.ids.iter().position(|i| i == id) else { return Vec::new() };
        self.rows.iter().filter_map(|(t, v)| v.get(k).copied().flatten().map(|x| (*t, x))).collect()
    }
}

/// Seconds of the day in "YYYY-MM-DDTHH:MM:SS.mmmZ".
fn day_secs(iso: &str) -> Option<f64> {
    let t = iso.split('T').nth(1)?.trim_end_matches('Z');
    let mut it = t.split(':');
    let (h, m, s) = (it.next()?.parse::<f64>().ok()?, it.next()?.parse::<f64>().ok()?, it.next()?.parse::<f64>().ok()?);
    Some(h * 3600.0 + m * 60.0 + s)
}

pub fn parse_hw_csv(text: &str) -> HwLog {
    let mut log = HwLog::default();
    let (mut t0, mut last, mut days) = (None, 0.0, 0.0);
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(rest) = line.strip_prefix("time,") {
            // The writer repeats the header when a new metric appears: keep the latest.
            log.ids = rest.split(',').map(str::to_string).collect();
            continue;
        }
        if let Some(rest) = line.strip_prefix("# units,") {
            log.units = rest.split(',').map(str::to_string).collect();
            continue;
        }
        let mut f = line.split(',');
        let Some(t) = f.next().and_then(day_secs) else { continue };
        if t + days < last - 1.0 {
            days += 86_400.0; // past midnight
        }
        let t = t + days;
        last = t;
        let t0 = *t0.get_or_insert(t);
        let vals: Vec<Option<f64>> = f.map(|v| v.trim().parse().ok()).collect();
        log.rows.push((t - t0, vals));
    }
    log
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A polyline chart as inline SVG: `lines` of (label, colour, points).
/// One line of a chart: (label, CSS colour, (x, y) points).
pub type ChartLine<'a> = (&'a str, &'a str, &'a [(f64, f64)]);

pub fn svg_chart(lines: &[ChartLine], w: f64, h: f64, y_unit: &str, bands: &[Band]) -> String {
    let all = || lines.iter().flat_map(|l| l.2.iter());
    let (x0, x1) = all().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let y1 = all().fold(0.0f64, |a, p| a.max(p.1)) * 1.05;
    if x1 <= x0 || y1 <= 0.0 {
        return String::new();
    }
    let (ml, mb) = (48.0, 18.0);
    let px = |x: f64| ml + (x - x0) / (x1 - x0) * (w - ml - 4.0);
    let py = |y: f64| 4.0 + (1.0 - y / y1) * (h - mb - 4.0);
    let mut s = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="100%" role="img">"#);
    for b in bands {
        s += &format!(
            r##"<rect x="{:.1}" y="4" width="{:.1}" height="{:.1}" fill="#e5484d" opacity="0.15"><title>{}</title></rect>"##,
            px(b.from_s.max(x0)),
            (px(b.to_s.min(x1)) - px(b.from_s.max(x0))).max(1.0),
            h - mb - 4.0,
            esc(&b.label)
        );
    }
    for k in 0..=4 {
        let y = y1 * k as f64 / 4.0;
        s += &format!(
            r##"<line x1="{ml}" x2="{}" y1="{:.1}" y2="{:.1}" stroke="#8884" /><text x="2" y="{:.1}" font-size="10" fill="#888">{y:.0} {}</text>"##,
            w - 4.0,
            py(y),
            py(y),
            py(y) + 3.0,
            esc(y_unit)
        );
    }
    s += &format!(
        r##"<text x="{ml}" y="{}" font-size="10" fill="#888">{x0:.0}</text><text x="{}" y="{}" font-size="10" fill="#888" text-anchor="end">{x1:.0}</text>"##,
        h - 4.0,
        w - 4.0,
        h - 4.0
    );
    for (label, color, pts) in lines {
        // At most ~2000 points per line keeps the file small.
        let step = (pts.len() / 2000).max(1);
        let d: Vec<String> = pts.iter().step_by(step).map(|p| format!("{:.1},{:.1}", px(p.0), py(p.1))).collect();
        s += &format!(
            r#"<polyline fill="none" stroke="{color}" stroke-width="1.2" points="{}"><title>{}</title></polyline>"#,
            d.join(" "),
            esc(label)
        );
    }
    s + "</svg>"
}

/// Vertical bars as inline SVG: (label, value, colour).
pub fn svg_bars(bars: &[(String, f64, &str)], w: f64, h: f64) -> String {
    let max = bars.iter().map(|b| b.1).fold(0.0, f64::max);
    if bars.is_empty() || max <= 0.0 {
        return String::new();
    }
    let bw = (w - 8.0) / bars.len() as f64;
    let mut s = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="100%" role="img">"#);
    for (i, (label, v, color)) in bars.iter().enumerate() {
        let bh = v / max * (h - 30.0);
        let x = 4.0 + i as f64 * bw;
        s += &format!(
            r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{bh:.1}" fill="{color}"><title>{} {v:.1}</title></rect><text x="{:.1}" y="{}" font-size="9" fill="#888" text-anchor="middle">{}</text>"##,
            x + 1.0,
            h - 16.0 - bh,
            (bw - 2.0).max(1.0),
            esc(label),
            x + bw / 2.0,
            h - 4.0,
            esc(label)
        );
    }
    s + "</svg>"
}

/// The whole report as one offline HTML file (no scripts, no network).
pub fn html_report(s: &Session, frames: &[crate::frames::Frame], hw: &HwLog) -> String {
    use crate::frames;
    let ms: Vec<f64> = frames.iter().map(|f| f.ms).collect();
    let ft: Vec<(f64, f64)> = frames.iter().map(|f| (f.t_s, f.ms)).collect();
    let curve = frames::percentile_curve(&ms);
    let hist = frames::histogram(&ms, 10.0);
    let sm = &s.summary;
    let row = |k: &str, v: String| format!("<tr><th>{}</th><td>{}</td></tr>", esc(k), esc(&v));
    let mut out = String::from(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>CpuZeeZ · ZeeZ Live report</title><style>
body{font:14px system-ui,sans-serif;max-width:1000px;margin:24px auto;padding:0 16px;color:#222;background:#fff}
@media(prefers-color-scheme:dark){body{color:#ddd;background:#151318}}
h1{font-size:22px}h2{font-size:16px;margin-top:28px}table{border-collapse:collapse}th{text-align:left;padding:2px 16px 2px 0;font-weight:500;color:#888}
.big{display:flex;gap:24px;flex-wrap:wrap}.big div{font-size:28px;font-weight:600}.big span{display:block;font-size:12px;color:#888;font-weight:400}
</style></head><body>"#,
    );
    out += &format!("<h1>{}</h1><p>{} · {:.0} s · {}</p>", esc(&s.game), esc(&s.started), s.duration_s, esc(&s.source));
    out += &format!(
        r#"<div class="big"><div>{:.1}<span>Average FPS</span></div><div>{:.1}<span>1% low</span></div><div>{:.1}<span>0.1% low</span></div><div>{}<span>Stutters</span></div></div><p><small>{}</small></p>"#,
        sm.avg_fps,
        sm.low1_fps,
        sm.low01_fps,
        sm.stutters,
        esc(sm.convention.label())
    );
    out += "<table>";
    out += &row("Frames", sm.frames.to_string());
    out += &row("Min / max FPS", format!("{:.1} / {:.1}", sm.min_fps, sm.max_fps));
    out += &row("Median frame time", format!("{:.2} ms", sm.median_ms));
    out += &row("Generated frames (FG)", sm.generated.to_string());
    out += &row("Dropped frames", sm.dropped.to_string());
    out += &row("CPU", s.system.cpu.clone());
    out += &row("GPU", format!("{} {}", s.system.gpu, s.system.gpu_driver));
    out += "</table>";
    out += "<h2>Frame time (ms)</h2>";
    out += &svg_chart(&[("Frame time", "#8b5cf6", &ft)], 960.0, 220.0, "ms", &s.throttling);
    for (title, ids) in [
        ("Load (%)", ["cpu/load/total", "gpu0/load/core"]),
        ("Temperature (°C)", ["cpu/temp/package", "gpu0/temp/core"]),
    ] {
        let a = hw.series(ids[0]);
        let b = hw.series(ids[1]);
        if !a.is_empty() || !b.is_empty() {
            out += &format!("<h2>{}</h2>", esc(title));
            out += &svg_chart(&[("CPU", "#3b82f6", &a), ("GPU", "#22c55e", &b)], 960.0, 160.0, "", &s.throttling);
        }
    }
    if !s.markers.is_empty() {
        out += "<h2>Markers</h2><ul>";
        for m in &s.markers {
            out += &format!("<li>{:.1} s: {}</li>", m.t_s, esc(&m.label));
        }
        out += "</ul>";
    }
    if !s.throttling.is_empty() {
        out += "<h2>Throttling</h2><ul>";
        for b in &s.throttling {
            out += &format!("<li>{:.1}–{:.1} s: {}</li>", b.from_s, b.to_s, esc(&b.label));
        }
        out += "</ul>";
    }
    out += "<h2>Frame-time percentiles (ms)</h2>";
    out += &svg_chart(&[("Percentile", "#f59e0b", &curve)], 960.0, 180.0, "ms", &[]);
    let windows = crate::bottleneck::windows_from_session(frames, hw, s.system.refresh_hz, &s.throttling);
    out += &crate::bottleneck::html_section(&crate::bottleneck::analyze(&windows, &Default::default()));
    out += "<h2>FPS histogram</h2>";
    let bars: Vec<(String, f64, &str)> = hist.iter().map(|(b, v)| (format!("{b:.0}"), v * 100.0, "#8b5cf6")).collect();
    out += &svg_bars(&bars, 960.0, 160.0);
    out += "<p><small>Made by CpuZeeZ. Frames from the graphics kernel's events; nothing was injected into the game.</small></p></body></html>";
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names_are_safe() {
        assert_eq!(
            folder_name("2026-11-05_2140", "VALORANT-Win64-Shipping.exe"),
            "2026-11-05_2140_VALORANT-Win64-Shipping"
        );
        assert_eq!(folder_name("2026-11-05_2140", r"C:\Games\My Game!.exe"), "2026-11-05_2140_My_Game_");
        assert_eq!(folder_name("2026-11-05_2140", "../.."), "2026-11-05_2140_session");
        assert_eq!(folder_name("x", ""), "x_session");
    }

    #[test]
    fn hw_log_and_html() {
        let csv = "time,cpu/load/total,gpu0/load/core\n# units,%,%\n2026-11-05T23:59:59.000Z,10,50\ntime,cpu/load/total,gpu0/load/core,cpu/temp/package\n# units,%,%,°C\n2026-11-06T00:00:01.500Z,20,,70\n";
        let h = parse_hw_csv(csv);
        assert_eq!(h.ids.len(), 3);
        assert_eq!(h.rows[1].0, 2.5, "across midnight");
        assert_eq!(h.series("cpu/load/total"), [(0.0, 10.0), (2.5, 20.0)]);
        assert_eq!(h.series("gpu0/load/core"), [(0.0, 50.0)], "an empty cell is no value");
        let frames: Vec<crate::frames::Frame> =
            (0..50).map(|i| crate::frames::Frame { t_s: i as f64 * 0.02, ms: 20.0, ..Default::default() }).collect();
        let s = Session {
            game: "<game>".into(),
            summary: crate::frames::summarize(&frames, Default::default()),
            throttling: vec![Band { from_s: 0.2, to_s: 0.4, label: "CPU".into(), kind: String::new() }],
            ..Default::default()
        };
        let html = html_report(&s, &frames, &h);
        assert!(html.contains("&lt;game&gt;") && !html.contains("<game>"), "escaped");
        assert!(html.contains("<svg") && html.contains("<polyline") && html.contains("<rect"));
        assert!(!html.contains("<script"), "no scripts");
        assert_eq!(
            html.matches("http").count(),
            html.matches("http://www.w3.org/2000/svg").count(),
            "offline: no URL but the SVG namespace"
        );
        assert!(html.contains("50.0<span>Average FPS"));
    }

    #[test]
    fn comparisons_and_repeats() {
        let a = Session { game: "cs2.exe".into(), ..Default::default() };
        let mut b = a.clone();
        assert!(comparability(&a, &b).is_empty());
        b.game = "game.exe".into();
        b.system.resolution = "1920x1080".into();
        assert_eq!(comparability(&a, &b), ["Different games"], "an unknown resolution isn't a difference");
        assert!(!high_variance(&[100.0, 101.0, 102.0]));
        assert!(high_variance(&[100.0, 110.0, 101.0]));
        let s = |avg| Summary { avg_fps: avg, ..Default::default() };
        assert_eq!(median_of(&[s(90.0), s(100.0), s(140.0)]).0, 100.0);
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Session>(&json).unwrap(), a);
    }
}

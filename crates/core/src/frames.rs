//! ZeeZ Live frame data (S2b, docs/v1/zeez-live.md §3.1, §6): one record per
//! presented frame from PresentMon (Windows) or MangoHud (Linux), and the
//! statistics every surface shares (overlay, Live tab, reports):
//! average FPS, 1 % / 0.1 % lows (both conventions, always labelled),
//! percentiles, stutters and the FPS histogram. Pure, so it's tested
//! everywhere with fixtures.

use serde::{Deserialize, Serialize};

/// One presented frame.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// Seconds since the capture started (the frame's CPU start).
    pub t_s: f64,
    /// Time since the previous present, ms (what "frame time" means).
    pub ms: f64,
    /// Time since the previous change on screen, ms (None: not displayed / unknown).
    pub displayed_ms: Option<f64>,
    /// GPU busy / CPU busy time, ms (PresentMon 2.x; the bottleneck detector's input).
    pub gpu_busy_ms: Option<f64>,
    pub cpu_busy_ms: Option<f64>,
    /// Click/input to photon latency, ms.
    pub latency_ms: Option<f64>,
    /// Made by Frame Generation (DLSS FG, FSR FG, AFMF…), not rendered by the game.
    pub generated: bool,
    /// Never reached the screen.
    pub dropped: bool,
    /// PresentMon's present mode ("Hardware: Independent Flip", "Composed: Flip"…).
    pub present_mode: String,
    pub app: String,
    pub pid: u32,
}

/// Which "1 % low" a number is (zeez-live §3.1): the industry uses both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LowConvention {
    /// The 99th (99.9th) percentile frame time, as FPS. The default.
    #[default]
    Percentile,
    /// The average of the slowest 1 % (0.1 %) of frames, as FPS.
    Average,
}

impl LowConvention {
    pub fn label(self) -> &'static str {
        match self {
            LowConvention::Percentile => "1% low = 99th-percentile frame time",
            LowConvention::Average => "1% low = average of the slowest 1% of frames",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub frames: usize,
    pub duration_s: f64,
    pub avg_fps: f64,
    pub low1_fps: f64,
    pub low01_fps: f64,
    pub min_fps: f64,
    pub max_fps: f64,
    pub median_ms: f64,
    pub p99_ms: f64,
    pub stutters: usize,
    pub generated: usize,
    pub dropped: usize,
    pub convention: LowConvention,
}

/// The `p` (0–100) percentile of sorted values, linear between ranks.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    match sorted.len() {
        0 => 0.0,
        1 => sorted[0],
        n => {
            let r = (p / 100.0).clamp(0.0, 1.0) * (n - 1) as f64;
            let (lo, hi) = (r.floor() as usize, r.ceil() as usize);
            sorted[lo] + (sorted[hi] - sorted[lo]) * (r - lo as f64)
        }
    }
}

fn fps(ms: f64) -> f64 {
    if ms > 0.0 { 1000.0 / ms } else { 0.0 }
}

/// A stutter: a frame more than twice the median frame time, and at least
/// 4 ms over it (so a 1 ms → 2.5 ms wobble at 600 FPS doesn't count).
pub fn is_stutter(ms: f64, median: f64) -> bool {
    ms > 2.0 * median && ms - median >= 4.0
}

/// Statistics of frame times (ms), in capture order.
pub fn summary(ms: &[f64], conv: LowConvention) -> Summary {
    let ms: Vec<f64> = ms.iter().copied().filter(|m| m.is_finite() && *m > 0.0).collect();
    if ms.is_empty() {
        return Summary { convention: conv, ..Default::default() };
    }
    let total: f64 = ms.iter().sum();
    let mut sorted = ms.clone();
    sorted.sort_by(f64::total_cmp);
    let median = percentile(&sorted, 50.0);
    let slowest_avg = |frac: f64| {
        let n = ((sorted.len() as f64 * frac).ceil() as usize).clamp(1, sorted.len());
        sorted[sorted.len() - n..].iter().sum::<f64>() / n as f64
    };
    let (low1, low01) = match conv {
        LowConvention::Percentile => (percentile(&sorted, 99.0), percentile(&sorted, 99.9)),
        LowConvention::Average => (slowest_avg(0.01), slowest_avg(0.001)),
    };
    Summary {
        frames: ms.len(),
        duration_s: total / 1000.0,
        avg_fps: ms.len() as f64 / (total / 1000.0),
        low1_fps: fps(low1),
        low01_fps: fps(low01),
        min_fps: fps(sorted[sorted.len() - 1]),
        max_fps: fps(sorted[0]),
        median_ms: median,
        p99_ms: percentile(&sorted, 99.0),
        stutters: ms.iter().filter(|&&m| is_stutter(m, median)).count(),
        generated: 0,
        dropped: 0,
        convention: conv,
    }
}

/// [`summary`] of frames, counting generated and dropped ones.
pub fn summarize(frames: &[Frame], conv: LowConvention) -> Summary {
    let ms: Vec<f64> = frames.iter().map(|f| f.ms).collect();
    Summary {
        generated: frames.iter().filter(|f| f.generated).count(),
        dropped: frames.iter().filter(|f| f.dropped).count(),
        ..summary(&ms, conv)
    }
}

/// Frame time at each percentile 0..=100 (the report's percentile curve).
pub fn percentile_curve(ms: &[f64]) -> Vec<(f64, f64)> {
    let mut s: Vec<f64> = ms.iter().copied().filter(|m| *m > 0.0).collect();
    s.sort_by(f64::total_cmp);
    if s.is_empty() {
        return Vec::new();
    }
    (0..=100).map(|p| (p as f64, percentile(&s, p as f64))).collect()
}

/// FPS histogram: (bin start FPS, share of frames 0–1), `width` FPS per bin.
pub fn histogram(ms: &[f64], width: f64) -> Vec<(f64, f64)> {
    let f: Vec<f64> = ms.iter().filter(|m| **m > 0.0).map(|m| 1000.0 / m).collect();
    if f.is_empty() || width <= 0.0 {
        return Vec::new();
    }
    let max = f.iter().copied().fold(0.0, f64::max);
    let bins = ((max / width).floor() as usize + 1).min(1000);
    let mut counts = vec![0usize; bins];
    for v in &f {
        counts[((v / width).floor() as usize).min(bins - 1)] += 1;
    }
    counts.into_iter().enumerate().map(|(i, c)| (i as f64 * width, c as f64 / f.len() as f64)).collect()
}

/// Frames of the last `secs` seconds (the overlay's rolling window).
pub fn last_secs(frames: &[Frame], secs: f64) -> &[Frame] {
    let Some(end) = frames.last().map(|f| f.t_s) else { return frames };
    let start = frames.partition_point(|f| f.t_s < end - secs);
    &frames[start..]
}

// ---------------------------------------------------------------- PresentMon

/// Column positions of a PresentMon CSV header (1.x and 2.x names).
#[derive(Debug, Clone, Default)]
pub struct PresentMonColumns {
    app: Option<usize>,
    pid: Option<usize>,
    time_s: Option<usize>,
    time_ms: Option<usize>,
    present_ms: Option<usize>,
    between: Option<usize>,
    display: Option<usize>,
    until_displayed: Option<usize>,
    gpu_busy: Option<usize>,
    cpu_busy: Option<usize>,
    latency: Option<usize>,
    frame_type: Option<usize>,
    dropped: Option<usize>,
    mode: Option<usize>,
    n: usize,
}

impl PresentMonColumns {
    /// None when the line isn't a PresentMon header.
    pub fn parse(header: &str) -> Option<PresentMonColumns> {
        let names: Vec<&str> = header.trim().split(',').map(str::trim).collect();
        let at = |n: &str| names.iter().position(|x| x.eq_ignore_ascii_case(n));
        let c = PresentMonColumns {
            app: at("Application"),
            pid: at("ProcessID"),
            time_s: at("TimeInSeconds"),
            time_ms: at("CPUStartTime").or(at("CPUStartTimeInMs")),
            present_ms: at("TimeInMs"),
            between: at("MsBetweenPresents"),
            display: at("MsBetweenDisplayChange"),
            until_displayed: at("MsUntilDisplayed"),
            gpu_busy: at("MsGPUBusy").or(at("MsGPUActive")),
            cpu_busy: at("MsCPUBusy"),
            latency: at("MsAllInputToPhotonLatency").or(at("MsClickToPhotonLatency")),
            frame_type: at("FrameType"),
            dropped: at("Dropped"),
            mode: at("PresentMode"),
            n: names.len(),
        };
        (c.between.is_some() && c.app.is_some()).then_some(c)
    }

    /// One data line; None for lines that aren't a frame.
    pub fn frame(&self, line: &str) -> Option<Frame> {
        let f: Vec<&str> = line.trim().split(',').collect();
        if f.len() < self.n.min(4) {
            return None;
        }
        let get = |i: Option<usize>| i.and_then(|i| f.get(i)).map(|s| s.trim());
        let num = |i: Option<usize>| get(i).and_then(|s| s.parse::<f64>().ok()).filter(|v| v.is_finite());
        let ms = num(self.between)?;
        // 1.x TimeInSeconds; 2.6 TimeInMs (the present) or CPUStartTimeInMs; 2.0 CPUStartTime.
        let t_s = num(self.time_s)
            .or_else(|| num(self.present_ms).map(|m| m / 1000.0))
            .or_else(|| num(self.time_ms).map(|m| m / 1000.0))
            .unwrap_or(0.0);
        let frame_type = get(self.frame_type).unwrap_or("");
        let generated = !frame_type.is_empty()
            && !["application", "notset", "na", "repeated"].contains(&frame_type.to_ascii_lowercase().as_str());
        // 1.x: a Dropped column. 2.x: MsUntilDisplayed / MsBetweenDisplayChange are "NA".
        let dropped = match (get(self.dropped), get(self.until_displayed)) {
            (Some(d), _) => d == "1",
            (None, Some(u)) => u.eq_ignore_ascii_case("NA"),
            _ => false,
        };
        Some(Frame {
            t_s,
            ms,
            displayed_ms: num(self.display),
            gpu_busy_ms: num(self.gpu_busy),
            cpu_busy_ms: num(self.cpu_busy),
            latency_ms: num(self.latency),
            generated,
            dropped,
            present_mode: get(self.mode).unwrap_or("").to_string(),
            app: get(self.app).unwrap_or("").to_string(),
            pid: get(self.pid).and_then(|s| s.parse().ok()).unwrap_or(0),
        })
    }
}

/// A whole PresentMon CSV (a capture's `frames.csv`, or one imported from
/// CapFrameX / PresentMon).
pub fn parse_presentmon(text: &str) -> Result<Vec<Frame>, String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let cols = lines
        .next()
        .and_then(PresentMonColumns::parse)
        .ok_or("not a PresentMon CSV: no Application / MsBetweenPresents columns")?;
    Ok(lines.filter_map(|l| cols.frame(l)).collect())
}

/// The CSV header CpuZeeZ writes for `frames.csv` (PresentMon 2.x names, so
/// CapFrameX and PresentMon's own tools read it).
pub const FRAMES_CSV_HEADER: &str = "Application,ProcessID,PresentMode,FrameType,CPUStartTime,MsBetweenPresents,MsBetweenDisplayChange,MsUntilDisplayed,MsCPUBusy,MsGPUBusy,MsAllInputToPhotonLatency";

/// One `frames.csv` line for a frame (the inverse of [`parse_presentmon`]).
pub fn frame_csv_line(f: &Frame) -> String {
    let o = |v: Option<f64>| v.map_or("NA".to_string(), |x| format!("{x:.4}"));
    let clean = |s: &str| s.replace([',', '\n', '\r'], " ");
    format!(
        "{},{},{},{},{:.4},{:.4},{},{},{},{},{}",
        clean(&f.app),
        f.pid,
        clean(&f.present_mode),
        if f.generated { "Generated" } else { "Application" },
        f.t_s * 1000.0,
        f.ms,
        if f.dropped { "NA".into() } else { o(f.displayed_ms) },
        if f.dropped { "NA" } else { "0" },
        o(f.cpu_busy_ms),
        o(f.gpu_busy_ms),
        o(f.latency_ms),
    )
}

/// Present modes in which an external window can't show on top (zeez-live §3.2).
pub fn exclusive_fullscreen(present_mode: &str) -> bool {
    let m = present_mode.to_ascii_lowercase();
    m.contains("legacy") || m.contains("hardware: legacy")
}

/// Lines of PresentMon's stdout, fed in chunks of any size. It writes
/// UTF-16LE with a BOM when redirected to a file and UTF-8 through a pipe,
/// so the first bytes decide.
#[derive(Default)]
pub struct OutputLines {
    /// None until the first two bytes arrive; Some(true) = UTF-16LE.
    utf16: Option<bool>,
    bytes: Vec<u8>,
}

impl OutputLines {
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<String> {
        self.bytes.extend_from_slice(chunk);
        if self.utf16.is_none() {
            if self.bytes.len() < 2 {
                return Vec::new();
            }
            let bom16 = self.bytes[..2] == [0xFF, 0xFE];
            self.utf16 = Some(bom16);
            if bom16 {
                self.bytes.drain(..2);
            } else if self.bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                self.bytes.drain(..3);
            }
        }
        let mut out = Vec::new();
        if self.utf16 == Some(true) {
            // Whole code units only: a newline is 0x0A 0x00 at an even offset.
            let (mut start, mut k) = (0, 0);
            while k + 1 < self.bytes.len() {
                if self.bytes[k] == b'\n' && self.bytes[k + 1] == 0 {
                    let units: Vec<u16> =
                        self.bytes[start..k].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
                    out.push(String::from_utf16_lossy(&units).trim_end_matches('\r').to_string());
                    start = k + 2;
                }
                k += 2;
            }
            self.bytes.drain(..start);
        } else {
            while let Some(i) = self.bytes.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = self.bytes.drain(..=i).collect();
                out.push(String::from_utf8_lossy(&line).trim_end_matches(['\r', '\n']).to_string());
            }
        }
        out
    }

    /// The last line, if it had no newline.
    pub fn finish(&mut self) -> Option<String> {
        if self.bytes.is_empty() {
            return None;
        }
        let b = std::mem::take(&mut self.bytes);
        Some(if self.utf16 == Some(true) {
            String::from_utf16_lossy(&b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect::<Vec<_>>())
        } else {
            String::from_utf8_lossy(&b).into_owned()
        })
    }
}

// ---------------------------------------------------------------- MangoHud

/// A MangoHud CSV log (`output_folder`): a system line, its values, then
/// `fps,frametime,…,elapsed` rows (elapsed in ns).
pub fn parse_mangohud(text: &str, app: &str) -> Result<Vec<Frame>, String> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let (mut ft, mut el) = (None, None);
    for l in lines.by_ref() {
        let names: Vec<&str> = l.split(',').map(str::trim).collect();
        if let Some(i) = names.iter().position(|n| n.eq_ignore_ascii_case("frametime")) {
            ft = Some(i);
            el = names.iter().position(|n| n.eq_ignore_ascii_case("elapsed"));
            break;
        }
    }
    let ft = ft.ok_or("not a MangoHud log: no frametime column")?;
    Ok(lines
        .filter_map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            let ms: f64 = f.get(ft)?.trim().parse().ok()?;
            let t_s = el.and_then(|i| f.get(i)?.trim().parse::<f64>().ok()).map_or(0.0, |ns| ns / 1e9);
            Some(Frame { t_s, ms, app: app.to_string(), ..Default::default() })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_of_known_frame_times() {
        // 99 frames at 10 ms and one at 50 ms: 1 s... plus 40 ms.
        let mut ms = vec![10.0; 99];
        ms.push(50.0);
        let s = summary(&ms, LowConvention::Percentile);
        assert_eq!(s.frames, 100);
        assert!((s.duration_s - 1.04).abs() < 1e-9);
        assert!((s.avg_fps - 100.0 / 1.04).abs() < 1e-6);
        assert_eq!(s.median_ms, 10.0);
        assert_eq!(s.stutters, 1, "the 50 ms frame");
        assert_eq!(s.max_fps, 100.0);
        assert_eq!(s.min_fps, 20.0);
        // p99 of 100 values: between rank 98 (10) and 99 (50) at 0.01 → 10.4 ms.
        assert!((s.p99_ms - 10.4).abs() < 1e-9);
        assert!((s.low1_fps - 1000.0 / 10.4).abs() < 1e-6);
        let a = summary(&ms, LowConvention::Average);
        assert_eq!(a.low1_fps, 20.0, "the slowest 1 % is the one 50 ms frame");
        assert_eq!(a.convention, LowConvention::Average);
        assert_eq!(summary(&[], LowConvention::Percentile).frames, 0);
    }

    #[test]
    fn small_wobbles_at_high_fps_are_not_stutters() {
        assert!(!is_stutter(2.5, 1.0), "2.5x but only 1.5 ms over");
        assert!(is_stutter(40.0, 16.7));
        assert!(!is_stutter(30.0, 16.7), "under twice the median");
    }

    #[test]
    fn curve_histogram_and_window() {
        let ms = [10.0, 20.0, 10.0, 5.0];
        let c = percentile_curve(&ms);
        assert_eq!((c[0], c[100]), ((0.0, 5.0), (100.0, 20.0)));
        let h = histogram(&ms, 50.0); // 200, 100, 50, 100 FPS
        assert_eq!(h.len(), 5);
        assert_eq!(h[2], (100.0, 0.5));
        let total: f64 = h.iter().map(|b| b.1).sum();
        assert!((total - 1.0).abs() < 1e-9);
        let frames: Vec<Frame> = (0..10).map(|i| Frame { t_s: i as f64, ms: 1.0, ..Default::default() }).collect();
        assert_eq!(last_secs(&frames, 3.0).len(), 4, "t = 6, 7, 8, 9");
    }

    #[test]
    fn presentmon_2x_csv() {
        let csv = "Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,FrameType,CPUStartTime,MsBetweenSimulationStart,MsBetweenPresents,MsBetweenDisplayChange,MsInPresentAPI,MsRenderPresentLatency,MsUntilDisplayed,MsPCLatency,MsCPUBusy,MsCPUWait,MsGPULatency,MsGPUTime,MsGPUBusy,MsGPUWait,MsVideoBusy,MsAnimationError,AnimationTime,MsFlipDelay,MsAllInputToPhotonLatency,MsClickToPhotonLatency
VALORANT-Win64-Shipping.exe,1234,0x1,DXGI,0,0,1,Hardware Composed: Independent Flip,Application,1000.5,NA,6.94,6.95,0.1,1.2,3.4,NA,4.1,2.8,0.2,2.5,2.5,0.3,0,0,0,0,18.2,NA
VALORANT-Win64-Shipping.exe,1234,0x1,DXGI,0,0,1,Hardware Composed: Independent Flip,DLSS-G,1007.4,NA,3.47,NA,0.1,1.2,NA,NA,0.5,0,0.2,0,0,0,0,0,0,0,NA,NA
";
        let f = parse_presentmon(csv).unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].app.as_str(), f[0].pid), ("VALORANT-Win64-Shipping.exe", 1234));
        assert!((f[0].t_s - 1.0005).abs() < 1e-9 && f[0].ms == 6.94);
        assert_eq!((f[0].gpu_busy_ms, f[0].cpu_busy_ms, f[0].latency_ms), (Some(2.5), Some(4.1), Some(18.2)));
        assert!(!f[0].generated && !f[0].dropped);
        assert!(f[1].generated, "DLSS-G frames are generated");
        assert!(f[1].dropped, "MsUntilDisplayed = NA");
        assert!(!exclusive_fullscreen(&f[0].present_mode));
        assert!(exclusive_fullscreen("Hardware: Legacy Flip"));
    }

    /// Two lines of PresentMon 2.6.0's real output (Deadlock, RTX 4060), as
    /// it writes them to a file (UTF-16LE with a BOM) and to a pipe (UTF-8).
    #[test]
    fn presentmon_2_6_real_output() {
        let csv = "Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,TimeInMs,MsBetweenSimulationStart,MsBetweenPresents,MsBetweenDisplayChange,MsInPresentAPI,MsRenderPresentLatency,MsUntilDisplayed,CPUStartTimeInMs,MsBetweenAppStart,MsCPUBusy,MsCPUWait,MsGPULatency,MsGPUTime,MsGPUBusy,MsGPUWait,MsAnimationError,AnimationTime,MsFlipDelay
deadlock.exe,51892,0x22A8A266F70,DXGI,0,512,1,Hardware Composed: Independent Flip,12.0427,NA,7.69620000000000,8.13470000000000,1.80370000000000,9.38120000000000,9.4676,5.1852,8.6612,6.8575,1.8037,8.0649,8.1738,8.1686,0.0052,NA,5.1852,NA
deadlock.exe,51892,0x22A8A266F70,DXGI,0,512,1,Hardware Composed: Independent Flip,23.8011,NA,11.75840000000000,10.38210000000000,0.06660000000000,6.92320000000000,8.0913,13.8464,10.0213,9.9547,0.0666,7.5775,9.3004,7.9833,1.3171,-1.7209,13.8464,NA";
        let mut utf16: Vec<u8> = vec![0xFF, 0xFE];
        utf16.extend(csv.encode_utf16().flat_map(|u| u.to_le_bytes()));
        let decode = |bytes: &[u8], cut: usize| {
            let mut dec = OutputLines::default();
            let mut lines = dec.feed(&bytes[..1]); // the BOM split too
            lines.extend(dec.feed(&bytes[1..cut])); // and a character
            lines.extend(dec.feed(&bytes[cut..]));
            lines.extend(dec.finish());
            lines
        };
        let from_file = decode(&utf16, 77);
        let from_pipe = decode(csv.as_bytes(), 40);
        assert_eq!(from_file, from_pipe, "UTF-16 (file) and UTF-8 (pipe) read the same");
        let f = parse_presentmon(&from_file.join("\n")).unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].app.as_str(), f[0].pid, f[0].ms), ("deadlock.exe", 51892, 7.6962));
        assert!((f[1].t_s - 0.0238011).abs() < 1e-9, "TimeInMs is the present's time");
        assert_eq!(f[1].gpu_busy_ms, Some(7.9833));
        assert_eq!(f[0].present_mode, "Hardware Composed: Independent Flip");
        assert!(!f[0].dropped && !f[0].generated);
    }

    #[test]
    fn presentmon_1x_csv_and_round_trip() {
        let v1 = "Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,Dropped,TimeInSeconds,msInPresentAPI,MsBetweenPresents,msBetweenDisplayChange
game.exe,42,0x1,DXGI,1,0,0,Composed: Flip,1,2.5,0.1,16.6,0.0
";
        let f = parse_presentmon(v1).unwrap();
        assert_eq!((f[0].t_s, f[0].ms, f[0].dropped), (2.5, 16.6, true));
        assert!(parse_presentmon("a,b\n1,2").is_err());
        // What we write, we read back (CapFrameX reads the same columns).
        let mine = format!("{FRAMES_CSV_HEADER}\n{}\n", frame_csv_line(&f[0]));
        let back = parse_presentmon(&mine).unwrap();
        assert_eq!((back[0].ms, back[0].pid, back[0].dropped), (16.6, 42, true));
        assert!((back[0].t_s - 2.5).abs() < 1e-9);
    }

    #[test]
    fn mangohud_log() {
        let log = "os,cpu,gpu,ram,kernel,driver,cpuscheduler
Arch Linux,AMD Ryzen 9 7900X,RTX 4060,32 GB,6.11,560,performance
--------------------FRAME METRICS--------------------
fps,frametime,cpu_load,gpu_load,cpu_temp,gpu_temp,gpu_core_clock,gpu_mem_clock,gpu_vram_used,gpu_power,ram_used,swap_used,process_rss,elapsed
144,6.94,30,90,60,65,2500,8000,4.1,110,9.5,0,2.1,1000000000
143,7.01,31,91,60,65,2500,8000,4.1,110,9.5,0,2.1,1007010000
";
        let f = parse_mangohud(log, "cs2").unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].ms, f[0].t_s, f[0].app.as_str()), (6.94, 1.0, "cs2"));
        assert!(parse_mangohud("fps,x\n1,2", "g").is_err());
    }
}

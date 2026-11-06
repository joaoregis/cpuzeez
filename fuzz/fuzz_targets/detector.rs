#![no_main]
// The detector on arbitrary windows: no panic, and every text has as many arguments as placeholders.
use cpuzeez_core::bottleneck::{Thresholds, Window, analyze_verdicts, classify};

struct Bytes<'a>(&'a [u8]);

impl Bytes<'_> {
    fn byte(&mut self) -> u8 {
        let (b, rest) = self.0.split_first().map_or((0, self.0), |(b, r)| (*b, r));
        self.0 = rest;
        b
    }
    fn float(&mut self) -> f64 {
        let b = self.byte();
        match b % 9 {
            0 => f64::NAN,
            1 => f64::INFINITY,
            2 => -(f64::from(self.byte())),
            3 => 1e300,
            _ => f64::from(self.byte()) * f64::from(b) / 17.0,
        }
    }
    fn opt(&mut self) -> Option<f64> {
        (self.byte() & 1 == 0).then(|| self.float())
    }
    fn window(&mut self) -> Window {
        Window {
            frames: usize::from(self.byte()) * 2,
            fps: self.float(),
            median_ms: self.float(),
            ms_cv: self.float(),
            stutters: usize::from(self.byte() % 30),
            gpu_busy_frac: self.opt(),
            cpu_busy_frac: self.opt(),
            present_mode: if self.byte() & 1 == 0 { "Composed: Flip".into() } else { String::new() },
            refresh_hz: self.opt(),
            focused: [None, Some(true), Some(false)][usize::from(self.byte() % 3)],
            gpu_util: self.opt(),
            cpu_total: self.opt(),
            busiest_core: self.opt(),
            busiest_core_id: (self.byte() & 1 == 0).then(|| usize::from(self.byte())),
            game_thread: self.opt(),
            vram_used_pct: self.opt(),
            ram_used_pct: self.opt(),
            pages_in: self.opt(),
            game_read_mbps: self.opt(),
            cpu_thermal: self.byte() & 1 == 0,
            cpu_power: self.byte() & 1 == 0,
            gpu_thermal: self.byte() & 1 == 0,
            gpu_power: self.byte() & 1 == 0,
            cpu_temp: self.opt(),
            gpu_temp: self.opt(),
            background: (self.byte() & 1 == 0).then(|| ("x.exe".to_string(), self.float())),
        }
    }
}

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let t = Thresholds::default();
    let mut b = Bytes(data);
    let windows: Vec<Window> = (0..1 + data.len() / 120).map(|_| b.window()).collect();
    let verdicts: Vec<_> = windows.iter().map(|w| classify(w, &t)).collect();
    for v in &verdicts {
        for e in std::iter::once(&v.primary).chain(&v.also).flat_map(|f| &f.evidence) {
            assert_eq!(e.key.matches("{}").count(), e.args.len());
        }
    }
    let a = analyze_verdicts(&windows, &verdicts, &t);
    for r in &a.recommendations {
        assert_eq!(r.key.matches("{}").count(), r.args.len());
    }
});

//! Robustness tests (S0-T07): every parser that reads untrusted input (bytes
//! from firmware or devices, files from other versions, the elevated-op
//! argument) gets tens of thousands of random and mutated inputs and must
//! never panic. Deterministic (fixed seed) and dependency-free, so they run on
//! stable Rust on every OS in CI. Deep coverage-guided fuzzing of the same
//! entry points lives in `fuzz/` (cargo-fuzz, nightly, scheduled in CI).

use crate::{actions, gpu::edid, layout::Layout, pci, query, sensors::amd_pm, smbios, spd::decode};

use crate::testkit::Rng;

const N: usize = 20_000;

#[test]
fn elevated_op_parser() {
    let mut r = Rng(0xC0FFEE);
    let good = actions::encode(&actions::Action::LoadKernelModule { module: actions::KernelModule::Msr });
    for _ in 0..N {
        let s: String = String::from_utf8_lossy(&r.mutate(good.as_bytes())).into_owned();
        let _ = actions::decode(&s);
        let hex: String = r.bytes(64).iter().map(|b| format!("{b:02x}")).collect();
        let _ = actions::decode(&hex);
    }
}

#[test]
fn layout_files() {
    let mut r = Rng(1);
    let samples: Vec<String> = crate::layout::presets::all(95.0).iter().map(Layout::to_json).collect();
    for i in 0..N {
        let base = samples[i % samples.len()].as_bytes();
        let _ = Layout::from_json(&String::from_utf8_lossy(&r.mutate(base)));
    }
}

#[test]
fn filter_syntax() {
    let mut r = Rng(2);
    let alphabet = b"name:cpu>=<10GB%ms\"- chrome.exe1234";
    for _ in 0..N {
        let q: String = (0..r.below(40)).map(|_| alphabet[r.below(alphabet.len())] as char).collect();
        let terms = query::parse(&q);
        let vals = [query::Value::Text("chrome.exe"), query::Value::Num(12.5)];
        let _ = query::matches(&terms, |f| (f == "cpu").then_some(query::Value::Num(12.5)), &vals);
    }
}

#[test]
fn smbios_tables() {
    let mut r = Rng(3);
    // A minimal valid table: type 0 (BIOS) with two strings, then end-of-table.
    let mut valid = vec![0u8, 0x18, 0, 0, 1, 2, 0, 0, 3, 0x80];
    valid.resize(0x18, 0);
    valid.extend(b"Vendor\0v1.0\0\0");
    valid.extend([127, 4, 1, 0, 0, 0]);
    for _ in 0..N {
        let t = smbios::Table::parse(3, 7, &r.mutate(&valid));
        let _ = smbios::bios_info(&t);
        let _ = smbios::memory_devices_of(&t);
        let _ = crate::report::to_txt(&[smbios::dmi_section(&t)]);
        let _ = smbios::Table::parse(2, 8, &r.bytes(300));
    }
}

#[test]
fn spd_images() {
    let mut r = Rng(4);
    for _ in 0..N / 4 {
        // DDR4 (0x0C) and DDR5 (0x12) headers on otherwise random images.
        let mut b = r.bytes(1024);
        if b.len() > 2 {
            b[2] = [0x0C, 0x12, 0x0B, 0x08][r.below(4)];
        }
        let hub = r.bytes(128);
        let _ = decode::decode(&b, &hub);
        let _ = decode::decode(&r.mutate(&b), &[]);
    }
}

#[test]
fn edid_blocks() {
    let mut r = Rng(5);
    let mut header = vec![0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
    header.resize(128, 0);
    for _ in 0..N {
        let _ = edid::parse(&r.mutate(&header));
        let _ = edid::parse(&r.bytes(256));
    }
}

#[test]
fn pci_config_space() {
    let mut r = Rng(6);
    for _ in 0..N / 4 {
        let mut d = pci::Device::new(pci::Bdf { seg: 0, bus: 1, dev: 0, func: 0 });
        d.config = r.bytes(4096);
        if d.config.len() > 0x34 {
            d.config[0x06] |= 0x10; // capability list present
        }
        let _ = pci::decode::device_nodes(&d);
        let _ = d.pcie_link();
    }
}

#[test]
fn smu_pm_tables() {
    let mut r = Rng(7);
    let versions = [0x240903, 0x380804, 0x380905, 0x370003, 0x400005, 0x5D0008, 0x620205, 0x650005];
    for _ in 0..N / 4 {
        let ver = if r.below(4) == 0 { r.next() as u32 } else { versions[r.below(versions.len())] };
        let t: Vec<f32> = (0..r.below(1100)).map(|_| f32::from_bits(r.next() as u32)).collect();
        let groups: Vec<usize> = (0..r.below(4)).map(|_| r.below(16)).collect();
        let _ = amd_pm::decode(ver, &t, &groups);
    }
}

// ------------------------------------------------------------ ZeeZ Live and the detector (S3-05)

use crate::bottleneck::{self, Thresholds, Window};
use crate::frames::{self, Frame};
use crate::session::{self, Band, Session};

const FIXTURE: &str = include_str!("../tests/fixtures/presentmon-2.6-deadlock.csv");

#[test]
fn presentmon_csv_files() {
    let mut r = Rng(0xF2A3E5);
    let head: String = FIXTURE.lines().take(40).collect::<Vec<_>>().join("\n");
    for _ in 0..N {
        let t = r.mutate_text(&head);
        let _ = frames::parse_presentmon(&t);
        let _ = frames::PresentMonColumns::parse(&r.mutate_text(FIXTURE.lines().next().unwrap_or("")));
    }
    // Whatever parses never yields a non-finite frame time.
    for f in frames::parse_presentmon(FIXTURE).unwrap() {
        assert!(f.ms.is_finite() && f.t_s.is_finite());
    }
}

/// Feeding a stream in any chunking gives the same lines, in both encodings.
#[test]
fn output_lines_ignore_chunking() {
    let mut r = Rng(0x0DDBA11);
    let text: String = FIXTURE.lines().take(30).collect::<Vec<_>>().join("\n") + "\n";
    let utf16: Vec<u8> = [0xFF, 0xFE].into_iter().chain(text.encode_utf16().flat_map(|u| u.to_le_bytes())).collect();
    for stream in [text.as_bytes().to_vec(), utf16] {
        let whole = {
            let mut d = frames::OutputLines::default();
            let mut v = d.feed(&stream);
            v.extend(d.finish());
            v
        };
        assert!(whole.len() >= 30);
        for _ in 0..2_000 {
            let mut d = frames::OutputLines::default();
            let mut got = Vec::new();
            let mut at = 0;
            while at < stream.len() {
                let n = 1 + r.below(97);
                got.extend(d.feed(&stream[at..(at + n).min(stream.len())]));
                at += n;
            }
            got.extend(d.finish());
            assert_eq!(got, whole);
        }
    }
    // And random bytes never panic.
    for _ in 0..N {
        let mut d = frames::OutputLines::default();
        let _ = d.feed(&r.bytes(300));
        let _ = d.feed(&r.bytes(300));
        let _ = d.finish();
    }
}

#[test]
fn hardware_and_mangohud_logs() {
    let mut r = Rng(0xAB1E);
    let hw = "time,cpu/load/total,gpu0/load/core\n# units,%,%\n2026-11-05T23:59:59.000Z,10,50\n2026-11-06T00:00:01.500Z,20,\n";
    let mh = "os,cpu\nLinux,x\nfps,frametime,elapsed\n144,6.9,1000000000\n143,7.0,1007000000\n";
    for _ in 0..N {
        let h = session::parse_hw_csv(&r.mutate_text(hw));
        for id in &h.ids {
            let _ = h.series(id);
        }
        let _ = frames::parse_mangohud(&r.mutate_text(mh), "game");
    }
}

fn random_frame(r: &mut Rng) -> Frame {
    let opt = |r: &mut Rng| (r.below(2) == 0).then(|| r.float());
    Frame {
        t_s: r.float(),
        ms: r.float(),
        displayed_ms: opt(r),
        gpu_busy_ms: opt(r),
        cpu_busy_ms: opt(r),
        latency_ms: opt(r),
        generated: r.below(2) == 0,
        dropped: r.below(2) == 0,
        present_mode: ["", "Composed: Flip", "Hardware: Legacy Flip", "<b>x</b>"][r.below(4)].into(),
        app: ["game.exe", "<script>alert(1)</script>", "", "a,b\nc"][r.below(4)].into(),
        pid: r.next() as u32,
    }
}

fn random_window(r: &mut Rng) -> Window {
    let opt = |r: &mut Rng| (r.below(2) == 0).then(|| r.float());
    Window {
        frames: r.below(400),
        fps: r.float(),
        median_ms: r.float(),
        ms_cv: r.float(),
        stutters: r.below(50),
        gpu_busy_frac: opt(r),
        cpu_busy_frac: opt(r),
        present_mode: ["", "Composed: Flip", "Hardware: Independent Flip"][r.below(3)].into(),
        refresh_hz: opt(r),
        focused: [None, Some(true), Some(false)][r.below(3)],
        gpu_util: opt(r),
        cpu_total: opt(r),
        busiest_core: opt(r),
        busiest_core_id: (r.below(2) == 0).then(|| r.below(64)),
        game_thread: opt(r),
        vram_used_pct: opt(r),
        ram_used_pct: opt(r),
        pages_in: opt(r),
        game_read_mbps: opt(r),
        cpu_thermal: r.below(4) == 0,
        cpu_power: r.below(4) == 0,
        gpu_thermal: r.below(4) == 0,
        gpu_power: r.below(4) == 0,
        cpu_temp: opt(r),
        gpu_temp: opt(r),
        background: (r.below(3) == 0).then(|| ("x.exe".to_string(), r.float())),
    }
}

/// The detector never panics, and every text it makes has as many arguments
/// as placeholders (a mismatch would show `{}` on screen or drop a number).
#[test]
fn detector_rules_on_random_windows() {
    let mut r = Rng(0x00B0_771E);
    let t = Thresholds::default();
    let holes = |key: &str| key.matches("{}").count();
    for _ in 0..4_000 {
        let w: Vec<Window> = (0..1 + r.below(40)).map(|_| random_window(&mut r)).collect();
        let verdicts: Vec<_> = w.iter().map(|x| bottleneck::classify(x, &t)).collect();
        for v in &verdicts {
            for f in std::iter::once(&v.primary).chain(&v.also) {
                assert!(!f.evidence.is_empty(), "a finding without evidence: {f:?}");
                for e in &f.evidence {
                    assert_eq!(holes(e.key), e.args.len(), "{e:?}");
                    assert!(!e.render().contains("{}"));
                }
            }
        }
        let a = bottleneck::analyze_verdicts(&w, &verdicts, &t);
        let total: f64 = a.share.iter().map(|s| s.1).sum();
        assert!((total - 1.0).abs() < 1e-9 && a.timeline.len() == w.len(), "{total}");
        for rec in &a.recommendations {
            assert_eq!(holes(rec.key), rec.args.len(), "{rec:?}");
            assert_eq!(holes(rec.because.key), rec.because.args.len(), "{rec:?}");
            assert!(!rec.render().contains("{}") && !rec.because.render().contains("{}"));
        }
        for e in &a.evidence {
            assert_eq!(holes(e.key), e.args.len(), "{e:?}");
        }
        let _ = bottleneck::html_section(&a);
        let _ = bottleneck::smooth(&verdicts);
    }
}

/// Statistics keep their order whatever the frame times are.
#[test]
fn frame_statistics_invariants() {
    let mut r = Rng(0x57A7);
    for _ in 0..5_000 {
        let ms: Vec<f64> = (0..r.below(300)).map(|_| r.float()).collect();
        for conv in [frames::LowConvention::Percentile, frames::LowConvention::Average] {
            let s = frames::summary(&ms, conv);
            if s.frames == 0 {
                continue;
            }
            // Relative tolerance: frame times down to 1e-300 ms give FPS near 1e303.
            let le = |a: f64, b: f64| a <= b * (1.0 + 1e-9) + 1e-9;
            assert!(le(s.min_fps, s.low01_fps) && le(s.low01_fps, s.low1_fps), "{s:?}");
            assert!(le(s.min_fps, s.avg_fps) && le(s.avg_fps, s.max_fps), "{s:?}");
            assert!(s.stutters <= s.frames);
        }
        let _ = frames::percentile_curve(&ms);
        let _ = frames::histogram(&ms, r.float());
    }
}

/// A frames.csv or session.json from anywhere: absurd times, NaN, huge
/// counts. Building windows and the report must stay bounded in memory and time.
#[test]
fn corrupt_sessions_stay_bounded() {
    let mut r = Rng(0xDEAD_5E55);
    let started = std::time::Instant::now();
    for _ in 0..120 {
        let frames: Vec<Frame> = (0..r.below(200)).map(|_| random_frame(&mut r)).collect();
        let hw = session::parse_hw_csv(&r.mutate_text("time,cpu/load/total\n# units,%\n2026-11-05T20:00:00.000Z,10\n"));
        let bands = vec![Band { from_s: r.float(), to_s: r.float(), label: "x".into(), kind: "cpu-thermal".into() }];
        let w = bottleneck::windows_from_session(&frames, &hw, Some(r.float()), &bands);
        assert!(w.len() <= 100_000, "{} windows from {} frames", w.len(), frames.len());
        let s = Session { game: "<img src=x onerror=alert(1)>".into(), ..Default::default() };
        let html = session::html_report(&s, &frames, &hw);
        assert!(!html.contains("<img") && !html.contains("<script"), "report HTML isn't escaped");
    }
    // A slow CI machine does this in ~10 s; a quadratic loop needs minutes.
    assert!(started.elapsed().as_secs() < 90, "took {:?}", started.elapsed());
}

/// A real long session (an 8-hour soak is 28 800 windows) joins the
/// hardware log in time proportional to its size, not its square.
#[test]
fn long_sessions_are_linear() {
    let n = 7_200usize; // two hours of one-second windows
    let frames: Vec<Frame> = (0..n * 4)
        .map(|i| Frame { t_s: i as f64 / 4.0, ms: 250.0, gpu_busy_ms: Some(200.0), ..Default::default() })
        .collect();
    let mut csv = String::from(
        "time,cpu/load/total,gpu0/load/core,cpu/load/max,mem/load/used,cpu/temp/package,gpu0/temp/core\n# units,%,%,%,%,C,C\n",
    );
    for i in 0..n {
        csv += &format!("2026-11-05T{:02}:{:02}:{:02}.000Z,10,50,40,60,70,65\n", i / 3600, i / 60 % 60, i % 60);
    }
    let hw = session::parse_hw_csv(&csv);
    let t = std::time::Instant::now();
    let w = bottleneck::windows_from_session(&frames, &hw, Some(60.0), &[]);
    let took = t.elapsed();
    assert_eq!(w.len(), n);
    assert_eq!(w[100].gpu_util, Some(50.0));
    assert!(took.as_millis() < 1_500, "joining {n} windows took {took:?}");
    let a = bottleneck::analyze(&w, &Thresholds::default());
    assert_eq!(a.seconds, n);
}

#[test]
fn session_json_files() {
    let mut r = Rng(0x5E5510);
    let good = serde_json::to_string(&Session {
        game: "cs2.exe".into(),
        markers: vec![session::Marker { t_s: 1.0, label: "m".into() }],
        throttling: vec![Band { from_s: 0.0, to_s: 1.0, label: "x".into(), kind: "cpu-thermal".into() }],
        ..Default::default()
    })
    .unwrap();
    for _ in 0..N {
        let _ = serde_json::from_str::<Session>(&r.mutate_text(&good));
    }
}

#[test]
fn small_text_parsers() {
    let mut r = Rng(0x7E57);
    for _ in 0..N {
        let _ = crate::services::parse_systemd_show(&r.mutate_text(
            "TriggeredBy=a.socket\nRestart=on-failure\nRestartUSec=1min 30s\nFailureAction=none\nCapabilityBoundingSet=cap_chown\n",
        ));
        let _ = crate::services::parse_systemd_secs(&r.mutate_text("1min 30.5s 100ms"));
        let _ = crate::memory::parse_meminfo(&r.mutate_text("MemTotal: 100 kB\nMemFree: 10 kB\nMemAvailable: 50 kB\n"));
        let _ = crate::netio::parse_proc_net_dev(
            &r.mutate_text("Inter-|   Receive\n eth0: 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16\n"),
        );
        let _ = crate::mangohud::config(
            &crate::layout::presets::live::compact(95.0),
            crate::live::Anchor::Top,
            &r.mutate_text("/x"),
        );
        let _ = crate::live::normalize_keys(&r.mutate_text("Ctrl+Shift+O"));
        let _ = crate::session::folder_name(&r.mutate_text("2026-11-05_2140"), &r.mutate_text("game.exe"));
        let _ = crate::procs::anti_cheat(&r.mutate_text("vgc.exe"));
    }
}

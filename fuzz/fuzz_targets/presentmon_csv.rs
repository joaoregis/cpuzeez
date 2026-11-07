#![no_main]
// PresentMon / CapFrameX CSVs from anywhere (a capture file, an import): never panic,
// and whatever parses has finite times.
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    if let Ok(frames) = cpuzeez_core::frames::parse_presentmon(&text) {
        for f in &frames {
            assert!(f.ms.is_finite() && f.t_s.is_finite());
        }
        let hw = cpuzeez_core::session::HwLog::default();
        let w = cpuzeez_core::bottleneck::windows_from_session(&frames, &hw, Some(60.0), &[]);
        assert!(w.len() <= cpuzeez_core::bottleneck::MAX_SESSION_SECONDS);
    }
});

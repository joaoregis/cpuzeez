#![no_main]
// session.json from another version or a damaged disk, and the report built from it.
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Ok(s) = serde_json::from_slice::<cpuzeez_core::session::Session>(data) {
        let html = cpuzeez_core::session::html_report(&s, &[], &Default::default());
        assert!(!html.contains("<script"));
    }
});

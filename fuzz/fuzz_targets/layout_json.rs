#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = cpuzeez_core::layout::Layout::from_json(s);
        let _ = cpuzeez_core::query::parse(s);
    }
});

#![no_main]
// The elevated entry point's only input: must reject anything that isn't an Action.
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = cpuzeez_core::actions::decode(s);
    }
});

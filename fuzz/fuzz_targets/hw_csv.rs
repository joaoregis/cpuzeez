#![no_main]
// A capture's hw.csv and MangoHud logs.
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let h = cpuzeez_core::session::parse_hw_csv(&text);
    for id in &h.ids {
        let _ = h.series(id);
    }
    let _ = cpuzeez_core::frames::parse_mangohud(&text, "game");
});

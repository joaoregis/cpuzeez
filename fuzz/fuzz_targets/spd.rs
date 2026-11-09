#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let (b, hub) = data.split_at(data.len().min(1024));
    let _ = cpuzeez_core::spd::decode::decode(b, hub);
});

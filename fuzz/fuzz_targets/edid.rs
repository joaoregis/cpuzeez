#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = cpuzeez_core::gpu::edid::parse(data);
});

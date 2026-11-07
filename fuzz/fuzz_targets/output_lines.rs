#![no_main]
// PresentMon's stdout, UTF-8 or UTF-16, in any chunking: never panic, and chunking never changes the lines.
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let whole = {
        let mut d = cpuzeez_core::frames::OutputLines::default();
        let mut v = d.feed(data);
        v.extend(d.finish());
        v
    };
    let Some((&cut, rest)) = data.split_first() else { return };
    let mut d = cpuzeez_core::frames::OutputLines::default();
    let at = (cut as usize).min(rest.len());
    let mut v = d.feed(&data[..at.min(data.len())]);
    v.extend(d.feed(&data[at.min(data.len())..]));
    v.extend(d.finish());
    assert_eq!(v, whole);
});

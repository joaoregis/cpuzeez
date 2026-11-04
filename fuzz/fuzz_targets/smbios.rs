#![no_main]
use cpuzeez_core::smbios;
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let t = smbios::Table::parse(3, 7, data);
    let _ = smbios::bios_info(&t);
    let _ = smbios::memory_devices_of(&t);
    let _ = cpuzeez_core::report::to_txt(&[smbios::dmi_section(&t)]);
});

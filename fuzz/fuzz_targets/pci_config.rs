#![no_main]
use cpuzeez_core::pci;
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let mut d = pci::Device::new(pci::Bdf { seg: 0, bus: 1, dev: 0, func: 0 });
    d.config = data.iter().copied().take(4096).collect();
    let _ = pci::decode::device_nodes(&d);
    let _ = d.pcie_link();
});

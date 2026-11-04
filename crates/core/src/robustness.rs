//! Robustness tests (S0-T07): every parser that reads untrusted input (bytes
//! from firmware or devices, files from other versions, the elevated-op
//! argument) gets tens of thousands of random and mutated inputs and must
//! never panic. Deterministic (fixed seed) and dependency-free, so they run on
//! stable Rust on every OS in CI. Deep coverage-guided fuzzing of the same
//! entry points lives in `fuzz/` (cargo-fuzz, nightly, scheduled in CI).

use crate::{actions, gpu::edid, layout::Layout, pci, query, sensors::amd_pm, smbios, spd::decode};

/// xorshift64*: small, fast, reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn bytes(&mut self, max: usize) -> Vec<u8> {
        let n = self.below(max + 1);
        (0..n).map(|_| self.next() as u8).collect()
    }
    /// `base` with a few bytes flipped, inserted or cut.
    fn mutate(&mut self, base: &[u8]) -> Vec<u8> {
        let mut v = base.to_vec();
        for _ in 0..1 + self.below(8) {
            match self.below(4) {
                0 if !v.is_empty() => {
                    let i = self.below(v.len());
                    v[i] = self.next() as u8;
                }
                1 => {
                    let i = self.below(v.len() + 1);
                    v.insert(i, self.next() as u8);
                }
                2 if !v.is_empty() => {
                    let cut = self.below(v.len());
                    v.truncate(cut);
                }
                _ => {
                    if let Some(i) = v.len().checked_sub(1).map(|l| self.below(l + 1)) {
                        v[i] = [0x00, 0xFF, 0x7F, 0x80][self.below(4)];
                    }
                }
            }
        }
        v
    }
}

const N: usize = 20_000;

#[test]
fn elevated_op_parser() {
    let mut r = Rng(0xC0FFEE);
    let good = actions::encode(&actions::Action::LoadKernelModule { module: actions::KernelModule::Msr });
    for _ in 0..N {
        let s: String = String::from_utf8_lossy(&r.mutate(good.as_bytes())).into_owned();
        let _ = actions::decode(&s);
        let hex: String = r.bytes(64).iter().map(|b| format!("{b:02x}")).collect();
        let _ = actions::decode(&hex);
    }
}

#[test]
fn layout_files() {
    let mut r = Rng(1);
    let samples: Vec<String> = crate::layout::presets::all(95.0).iter().map(Layout::to_json).collect();
    for i in 0..N {
        let base = samples[i % samples.len()].as_bytes();
        let _ = Layout::from_json(&String::from_utf8_lossy(&r.mutate(base)));
    }
}

#[test]
fn filter_syntax() {
    let mut r = Rng(2);
    let alphabet = b"name:cpu>=<10GB%ms\"- chrome.exe1234";
    for _ in 0..N {
        let q: String = (0..r.below(40)).map(|_| alphabet[r.below(alphabet.len())] as char).collect();
        let terms = query::parse(&q);
        let vals = [query::Value::Text("chrome.exe"), query::Value::Num(12.5)];
        let _ = query::matches(&terms, |f| (f == "cpu").then_some(query::Value::Num(12.5)), &vals);
    }
}

#[test]
fn smbios_tables() {
    let mut r = Rng(3);
    // A minimal valid table: type 0 (BIOS) with two strings, then end-of-table.
    let mut valid = vec![0u8, 0x18, 0, 0, 1, 2, 0, 0, 3, 0x80];
    valid.resize(0x18, 0);
    valid.extend(b"Vendor\0v1.0\0\0");
    valid.extend([127, 4, 1, 0, 0, 0]);
    for _ in 0..N {
        let t = smbios::Table::parse(3, 7, &r.mutate(&valid));
        let _ = smbios::bios_info(&t);
        let _ = smbios::memory_devices_of(&t);
        let _ = crate::report::to_txt(&[smbios::dmi_section(&t)]);
        let _ = smbios::Table::parse(2, 8, &r.bytes(300));
    }
}

#[test]
fn spd_images() {
    let mut r = Rng(4);
    for _ in 0..N / 4 {
        // DDR4 (0x0C) and DDR5 (0x12) headers on otherwise random images.
        let mut b = r.bytes(1024);
        if b.len() > 2 {
            b[2] = [0x0C, 0x12, 0x0B, 0x08][r.below(4)];
        }
        let hub = r.bytes(128);
        let _ = decode::decode(&b, &hub);
        let _ = decode::decode(&r.mutate(&b), &[]);
    }
}

#[test]
fn edid_blocks() {
    let mut r = Rng(5);
    let mut header = vec![0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
    header.resize(128, 0);
    for _ in 0..N {
        let _ = edid::parse(&r.mutate(&header));
        let _ = edid::parse(&r.bytes(256));
    }
}

#[test]
fn pci_config_space() {
    let mut r = Rng(6);
    for _ in 0..N / 4 {
        let mut d = pci::Device::new(pci::Bdf { seg: 0, bus: 1, dev: 0, func: 0 });
        d.config = r.bytes(4096);
        if d.config.len() > 0x34 {
            d.config[0x06] |= 0x10; // capability list present
        }
        let _ = pci::decode::device_nodes(&d);
        let _ = d.pcie_link();
    }
}

#[test]
fn smu_pm_tables() {
    let mut r = Rng(7);
    let versions = [0x240903, 0x380804, 0x380905, 0x370003, 0x400005, 0x5D0008, 0x620205, 0x650005];
    for _ in 0..N / 4 {
        let ver = if r.below(4) == 0 { r.next() as u32 } else { versions[r.below(versions.len())] };
        let t: Vec<f32> = (0..r.below(1100)).map(|_| f32::from_bits(r.next() as u32)).collect();
        let groups: Vec<usize> = (0..r.below(4)).map(|_| r.below(16)).collect();
        let _ = amd_pm::decode(ver, &t, &groups);
    }
}

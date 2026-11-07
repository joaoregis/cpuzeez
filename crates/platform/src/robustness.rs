//! Robustness tests for the platform parsers that read what the OS or a
//! tool prints (S3-05): the same deterministic mutation approach as
//! `cpuzeez_core::robustness`. They must never panic.

use cpuzeez_core::testkit::Rng;

const N: usize = 20_000;

#[test]
fn network_and_process_text() {
    let mut r = Rng(0x9E7_4E7);
    let nettop = ",bytes_in,bytes_out,\nlaunchd.1,0,0,\nGoogle Chrome He.812,1500,300,\nbad line\n";
    let args = {
        let mut a = 2u32.to_ne_bytes().to_vec();
        a.extend(b"/bin/sleep\0\0\0sleep\x0010\0FOO=bar\0");
        a
    };
    let mut diag = vec![0u8; 72];
    diag[68..72].copy_from_slice(&4242u32.to_ne_bytes());
    diag.extend(8u16.to_ne_bytes());
    diag.extend(1u16.to_ne_bytes());
    diag.extend([0u8; 4]);
    diag.extend(140u16.to_ne_bytes());
    diag.extend(2u16.to_ne_bytes());
    diag.extend(vec![1u8; 136]);
    for _ in 0..N {
        let _ = crate::proc_net::parse_nettop(&r.mutate_text(nettop));
        let _ = crate::proc_net::parse_inet_diag(&r.mutate(&diag));
        let _ = crate::procs::parse_procargs2(&r.mutate(&args));
        let _ = crate::procs::elf_arch(&r.bytes(40));
        let _ = crate::procs::integrity_name(r.next() as u32);
        let _ = crate::procs::machine_name(r.next() as u16);
        let _ = crate::memory::parse_major_faults(&r.mutate_text("pgfault 1\npgmajfault 42\n"));
    }
    // Random bytes of any size never panic the netlink decoder.
    for _ in 0..N {
        let _ = crate::proc_net::parse_inet_diag(&r.bytes(400));
    }
}

/// Any bytes from the kernel's mode query decode, or not, without a panic, and
/// the rates they give are finite.
#[test]
fn display_modes_from_any_bytes() {
    use crate::display_drm::{best_hz, parse_modeinfo};
    let mut r = Rng(0xD15_91A7);
    for _ in 0..N {
        let n = r.below(200);
        let Some(m) = parse_modeinfo(&r.bytes(n)) else { continue };
        assert!(m.hz().is_finite() && m.hz() >= 0.0);
        let others: Vec<_> = (0..4).filter_map(|_| parse_modeinfo(&r.bytes(80))).collect();
        let _ = best_hz(&m, &others);
    }
}

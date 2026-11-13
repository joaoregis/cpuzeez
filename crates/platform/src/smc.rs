//! The Apple SMC over IOKit (user client selector 2 with the widely
//! documented 80-byte SMCParamStruct). No root needed on Intel or Apple
//! Silicon Macs.

use std::ffi::{c_char, c_void};
use std::sync::OnceLock;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOServiceGetMatchingService(main: u32, matching: *mut c_void) -> u32;
    fn IOServiceOpen(svc: u32, task: u32, ty: u32, conn: *mut u32) -> i32;
    fn IOObjectRelease(o: u32) -> i32;
    fn IOConnectCallStructMethod(
        conn: u32,
        sel: u32,
        inp: *const c_void,
        in_size: usize,
        out: *mut c_void,
        out_size: *mut usize,
    ) -> i32;
}
unsafe extern "C" {
    static mach_task_self_: u32;
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Vers {
    major: u8,
    minor: u8,
    build: u8,
    reserved: u8,
    release: u16,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct PLimit {
    version: u16,
    length: u16,
    cpu: u32,
    gpu: u32,
    mem: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct KeyInfo {
    size: u32,
    ty: u32,
    attr: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Param {
    key: u32,
    vers: Vers,
    plimit: PLimit,
    info: KeyInfo,
    result: u8,
    status: u8,
    data8: u8,
    data32: u32,
    bytes: [u8; 32],
}
const _: () = assert!(std::mem::size_of::<Param>() == 80);

const READ_KEY: u8 = 5;
const KEY_AT_INDEX: u8 = 8;
const KEY_INFO: u8 = 9;

fn conn() -> Option<u32> {
    static C: OnceLock<Option<u32>> = OnceLock::new();
    *C.get_or_init(|| unsafe {
        // SAFETY: documented IOKit calls; the matching dict is consumed by
        // IOServiceGetMatchingService. The connection lives for the process.
        let svc = IOServiceGetMatchingService(0, IOServiceMatching(c"AppleSMC".as_ptr()));
        if svc == 0 {
            return None;
        }
        let mut c = 0u32;
        let kr = IOServiceOpen(svc, mach_task_self_, 0, &mut c);
        IOObjectRelease(svc);
        (kr == 0).then_some(c)
    })
}

fn call(c: u32, p: &Param) -> Option<Param> {
    let mut out = Param::default();
    let mut sz = std::mem::size_of::<Param>();
    // SAFETY: in/out are 80-byte SMCParamStruct values.
    let kr = unsafe {
        IOConnectCallStructMethod(c, 2, (p as *const Param).cast(), sz, (&mut out as *mut Param).cast(), &mut sz)
    };
    (kr == 0 && out.result == 0).then_some(out)
}

/// (type code, bytes) of an SMC key.
pub fn read(k: u32) -> Option<(u32, Vec<u8>)> {
    let c = conn()?;
    let info = call(c, &Param { key: k, data8: KEY_INFO, ..Default::default() })?.info;
    let o = call(c, &Param { key: k, data8: READ_KEY, info, ..Default::default() })?;
    Some((info.ty, o.bytes[..(info.size as usize).min(32)].to_vec()))
}

/// The SMC can be opened.
pub fn available() -> bool {
    conn().is_some()
}

/// The key at `index` in the SMC's key table.
pub fn key_at(index: u32) -> Option<u32> {
    Some(call(conn()?, &Param { data8: KEY_AT_INDEX, data32: index, ..Default::default() })?.key)
}

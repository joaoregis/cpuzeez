//! Minimal dynamic library loading (LoadLibraryW / dlopen) for vendor APIs
//! that may or may not be installed (NVML, NVAPI, ADL, DXCore).

use std::ffi::c_void;

pub struct Lib(*mut c_void);
// SAFETY: a module handle is process-global; the libraries we load are thread safe.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(m: *mut c_void, name: *const u8) -> *mut c_void;
}

impl Lib {
    /// First of `names` that loads. Never unloaded (the APIs keep global state).
    pub fn open(names: &[&str]) -> Option<Lib> {
        names.iter().find_map(|n| {
            #[cfg(windows)]
            let h = {
                let w: Vec<u16> = n.encode_utf16().chain(Some(0)).collect();
                unsafe { LoadLibraryW(w.as_ptr()) }
            };
            #[cfg(unix)]
            let h = {
                let c = std::ffi::CString::new(*n).ok()?;
                unsafe { libc::dlopen(c.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) }
            };
            (!h.is_null()).then_some(Lib(h))
        })
    }

    pub fn sym(&self, name: &str) -> Option<*mut c_void> {
        let c = std::ffi::CString::new(name).ok()?;
        #[cfg(windows)]
        let p = unsafe { GetProcAddress(self.0, c.as_ptr().cast()) };
        #[cfg(unix)]
        let p = unsafe { libc::dlsym(self.0, c.as_ptr()) };
        (!p.is_null()).then_some(p)
    }
}

/// `let f: fn-type = sym!(lib, "name")?;`
macro_rules! sym {
    ($lib:expr, $name:expr) => {
        $lib.sym($name).map(|p| unsafe { std::mem::transmute(p) })
    };
}
pub(crate) use sym;

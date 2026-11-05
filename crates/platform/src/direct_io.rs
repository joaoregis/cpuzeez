//! Uncached file I/O for the storage benchmark (S1-K06): reads and writes go
//! to the drive, not the OS cache. Windows `FILE_FLAG_NO_BUFFERING |
//! FILE_FLAG_WRITE_THROUGH`, Linux `O_DIRECT`, macOS `F_NOCACHE`. Buffers,
//! offsets and lengths must be multiples of [`ALIGN`].

use std::fs::File;
use std::path::Path;

/// Sector alignment that satisfies every current drive (4Kn included).
pub const ALIGN: usize = 4096;

/// Opens (creating if needed) `path` for uncached reads and writes.
pub fn open(path: &Path) -> std::io::Result<File> {
    imp::open(path)
}

/// A zeroed buffer of `len` bytes (rounded up to [`ALIGN`]) at an aligned
/// address, as uncached I/O requires.
pub struct Buffer {
    ptr: *mut u8,
    len: usize,
}

// SAFETY: the buffer owns its allocation exclusively.
unsafe impl Send for Buffer {}

impl Buffer {
    pub fn new(len: usize) -> Buffer {
        let len = len.div_ceil(ALIGN).max(1) * ALIGN;
        let layout = std::alloc::Layout::from_size_align(len, ALIGN).expect("aligned layout");
        // SAFETY: non-zero size; freed with the same layout in Drop.
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null(), "out of memory for the benchmark buffer");
        Buffer { ptr, len }
    }

    pub fn bytes(&mut self) -> &mut [u8] {
        // SAFETY: ptr..ptr+len is our allocation.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: allocated in `new` with this layout.
        unsafe { std::alloc::dealloc(self.ptr, std::alloc::Layout::from_size_align_unchecked(self.len, ALIGN)) }
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_NO_BUFFERING: u32 = 0x2000_0000;
    const FILE_FLAG_WRITE_THROUGH: u32 = 0x8000_0000;

    pub fn open(path: &std::path::Path) -> std::io::Result<std::fs::File> {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .custom_flags(FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH)
            .open(path)
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::os::unix::fs::OpenOptionsExt;

    pub fn open(path: &std::path::Path) -> std::io::Result<std::fs::File> {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .custom_flags(libc::O_DIRECT)
            .open(path)
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::os::fd::AsRawFd;

    pub fn open(path: &std::path::Path) -> std::io::Result<std::fs::File> {
        let f = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)?;
        // SAFETY: fcntl on our own descriptor.
        if unsafe { libc::fcntl(f.as_raw_fd(), libc::F_NOCACHE, 1) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(f)
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn open(path: &std::path::Path) -> std::io::Result<std::fs::File> {
        std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(path)
    }
}

/// Writes all of `buf` at `offset` (positional, so threads can share a file).
pub fn write_at(f: &File, buf: &[u8], offset: u64) -> std::io::Result<()> {
    let mut done = 0;
    while done < buf.len() {
        #[cfg(windows)]
        let n = std::os::windows::fs::FileExt::seek_write(f, &buf[done..], offset + done as u64)?;
        #[cfg(unix)]
        let n = std::os::unix::fs::FileExt::write_at(f, &buf[done..], offset + done as u64)?;
        if n == 0 {
            return Err(std::io::ErrorKind::WriteZero.into());
        }
        done += n;
    }
    Ok(())
}

/// Fills `buf` from `offset`.
pub fn read_at(f: &File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    let mut done = 0;
    while done < buf.len() {
        #[cfg(windows)]
        let n = std::os::windows::fs::FileExt::seek_read(f, &mut buf[done..], offset + done as u64)?;
        #[cfg(unix)]
        let n = std::os::unix::fs::FileExt::read_at(f, &mut buf[done..], offset + done as u64)?;
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        done += n;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uncached_round_trip() {
        let dir = std::env::temp_dir().join(format!("cz-direct-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.bin");
        let f = open(&path).unwrap();
        let mut a = Buffer::new(ALIGN * 2);
        a.bytes().iter_mut().enumerate().for_each(|(i, b)| *b = i as u8);
        write_at(&f, a.bytes(), ALIGN as u64).unwrap();
        let mut b = Buffer::new(ALIGN * 2);
        read_at(&f, b.bytes(), ALIGN as u64).unwrap();
        assert_eq!(a.bytes(), b.bytes());
        drop(f);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

//! Partitions of a disk and the OS's TRIM setting (S1-G06). No privileges:
//! Windows `IOCTL_DISK_GET_DRIVE_LAYOUT_EX` on the disk's path, Linux
//! `/sys/block/<disk>/<part>/start`.

#[derive(Debug, Clone, PartialEq)]
pub struct Partition {
    pub number: u32,
    /// Bytes from the start of the disk.
    pub offset: u64,
    pub length: u64,
}

impl Partition {
    /// Starts on a 1 MiB boundary (what every current OS does; 4 KiB is the
    /// minimum for SSDs and Advanced Format drives).
    pub fn aligned(&self) -> bool {
        self.offset.is_multiple_of(4096)
    }
}

/// Partitions of the disk at `path` (Windows device path or `/dev/<name>`).
pub fn of(path: &str) -> Vec<Partition> {
    imp::of(path)
}

/// TRIM (delete notifications) enabled by the OS. Windows: the NTFS
/// setting; Linux: the device accepts discards. None: unknown.
pub fn trim(path: &str) -> Option<bool> {
    imp::trim(path)
}

#[cfg(windows)]
mod imp {
    use super::Partition;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{
        DRIVE_LAYOUT_INFORMATION_EX, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, PARTITION_INFORMATION_EX,
    };

    pub fn of(path: &str) -> Vec<Partition> {
        let w: Vec<u16> = path.encode_utf16().chain([0]).collect();
        // SAFETY: query-only handle; the buffer is large enough for 128
        // entries and DeviceIoControl writes at most its size.
        unsafe {
            let h = CreateFileW(
                w.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if h == INVALID_HANDLE_VALUE {
                return Vec::new();
            }
            let size = size_of::<DRIVE_LAYOUT_INFORMATION_EX>() + 128 * size_of::<PARTITION_INFORMATION_EX>();
            let mut buf = vec![0u64; size.div_ceil(8)];
            let mut got = 0u32;
            let ok = DeviceIoControl(
                h,
                IOCTL_DISK_GET_DRIVE_LAYOUT_EX,
                std::ptr::null(),
                0,
                buf.as_mut_ptr().cast(),
                (buf.len() * 8) as u32,
                &mut got,
                std::ptr::null_mut(),
            );
            CloseHandle(h);
            if ok == 0 {
                return Vec::new();
            }
            let l = &*buf.as_ptr().cast::<DRIVE_LAYOUT_INFORMATION_EX>();
            let n = (l.PartitionCount as usize).min(128);
            std::slice::from_raw_parts(l.PartitionEntry.as_ptr(), n)
                .iter()
                .filter(|p| p.PartitionLength > 0 && p.PartitionNumber > 0)
                .map(|p| Partition {
                    number: p.PartitionNumber,
                    offset: p.StartingOffset as u64,
                    length: p.PartitionLength as u64,
                })
                .collect()
        }
    }

    pub fn trim(_path: &str) -> Option<bool> {
        // DisableDeleteNotification: 0 = TRIM on (fsutil behavior query DisableDeleteNotify).
        crate::os::win::reg_u64(r"SYSTEM\CurrentControlSet\Control\FileSystem", "DisableDeleteNotification")
            .map(|v| v & 0xFFFF_FFFF == 0)
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::Partition;

    fn name(path: &str) -> &str {
        path.trim_start_matches("/dev/")
    }

    pub fn of(path: &str) -> Vec<Partition> {
        let disk = name(path);
        let Ok(dir) = std::fs::read_dir(format!("/sys/block/{disk}")) else { return Vec::new() };
        let mut v: Vec<Partition> = dir
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                let start: u64 = std::fs::read_to_string(p.join("start")).ok()?.trim().parse().ok()?;
                let size: u64 = std::fs::read_to_string(p.join("size")).ok()?.trim().parse().ok()?;
                let number = std::fs::read_to_string(p.join("partition")).ok()?.trim().parse().ok()?;
                // sysfs counts 512-byte sectors whatever the device's sector size.
                Some(Partition { number, offset: start * 512, length: size * 512 })
            })
            .collect();
        v.sort_by_key(|p| p.number);
        v
    }

    pub fn trim(path: &str) -> Option<bool> {
        let v: u64 = std::fs::read_to_string(format!("/sys/block/{}/queue/discard_max_bytes", name(path)))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        Some(v > 0)
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn of(_path: &str) -> Vec<super::Partition> {
        Vec::new()
    }
    pub fn trim(_path: &str) -> Option<bool> {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn alignment() {
        let p = |offset| super::Partition { number: 1, offset, length: 1 };
        assert!(p(1 << 20).aligned());
        assert!(!p(63 * 512).aligned(), "the old DOS 63-sector start");
    }
}

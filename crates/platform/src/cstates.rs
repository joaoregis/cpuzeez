//! Per-CPU C-state residency (S1-G03): the share of time each logical CPU
//! spent in each idle state since the previous call, at most once a second.
//! Windows: `\Processor Information(*)\% C1/C2/C3 Time` (PDH, no admin; C2/C3
//! are the OS's ACPI classes, which modern CPUs map their deep states to).
//! Linux: cpuidle (`/sys/devices/system/cpu/cpuN/cpuidle/stateM/{name,time}`).
//! macOS: none.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// (state, %) per logical CPU, by CPU index.
pub type Residency = Vec<Vec<(String, f64)>>;

static LAST: Mutex<Option<(Instant, Residency)>> = Mutex::new(None);

pub fn read() -> Residency {
    let mut g = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((t, r)) = g.as_ref()
        && t.elapsed() < Duration::from_secs(1)
    {
        return r.clone();
    }
    let r = imp::read();
    *g = Some((Instant::now(), r.clone()));
    r
}

/// Linux cpuidle: % of `secs` spent in each state from two cumulative µs readings.
pub fn shares(before: &[(String, u64)], after: &[(String, u64)], secs: f64) -> Vec<(String, f64)> {
    after
        .iter()
        .filter(|(n, _)| n != "POLL")
        .filter_map(|(n, t)| {
            let b = before.iter().find(|x| &x.0 == n)?.1;
            Some((n.clone(), (t.saturating_sub(b) as f64 / 1e6 / secs * 100.0).clamp(0.0, 100.0)))
        })
        .collect()
}

#[cfg(windows)]
mod imp {
    use std::sync::Mutex;
    use windows_sys::Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW,
        PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    };

    struct Query(PDH_HQUERY, [PDH_HCOUNTER; 3]);
    // SAFETY: PDH handles may be used from any thread; access is serialized by the Mutex.
    unsafe impl Send for Query {}
    static QUERY: Mutex<Option<Query>> = Mutex::new(None);

    /// (instance, value) of one counter array.
    unsafe fn array(c: PDH_HCOUNTER) -> Vec<(String, f64)> {
        // SAFETY: standard PDH use; PDH sizes the buffer, and the names point into it.
        unsafe {
            let (mut size, mut count) = (0u32, 0u32);
            if PdhGetFormattedCounterArrayW(c, PDH_FMT_DOUBLE, &mut size, &mut count, std::ptr::null_mut())
                != PDH_MORE_DATA
            {
                return Vec::new();
            }
            let mut buf = vec![0u64; (size as usize).div_ceil(8)];
            let items = buf.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            if PdhGetFormattedCounterArrayW(c, PDH_FMT_DOUBLE, &mut size, &mut count, items) != 0 {
                return Vec::new();
            }
            std::slice::from_raw_parts(items, count as usize)
                .iter()
                .filter(|i| i.FmtValue.CStatus == 0 && !i.szName.is_null())
                .map(|i| {
                    let n = (0..).take_while(|&k| *i.szName.add(k) != 0).count();
                    (
                        String::from_utf16_lossy(std::slice::from_raw_parts(i.szName, n)),
                        i.FmtValue.Anonymous.doubleValue,
                    )
                })
                .collect()
        }
    }

    pub fn read() -> super::Residency {
        let mut g = QUERY.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: standard PDH use on our own query.
        unsafe {
            if g.is_none() {
                let mut q = std::ptr::null_mut();
                if PdhOpenQueryW(std::ptr::null(), 0, &mut q) != 0 {
                    return Vec::new();
                }
                let mut cs = [std::ptr::null_mut(); 3];
                for (k, c) in cs.iter_mut().enumerate() {
                    let path: Vec<u16> =
                        format!(r"\Processor Information(*)\% C{} Time", k + 1).encode_utf16().chain([0]).collect();
                    if PdhAddEnglishCounterW(q, path.as_ptr(), 0, c) != 0 {
                        return Vec::new();
                    }
                }
                PdhCollectQueryData(q); // rates: the first value needs a second sample
                *g = Some(Query(q, cs));
                return Vec::new();
            }
            let Some(Query(q, cs)) = g.as_ref() else { return Vec::new() };
            if PdhCollectQueryData(*q) != 0 {
                return Vec::new();
            }
            let mut out: super::Residency = Vec::new();
            for (k, c) in cs.iter().enumerate() {
                for (name, v) in array(*c) {
                    // "group,cpu"; skip the "_Total" instances.
                    let Some((grp, cpu)) = name.split_once(',') else { continue };
                    let (Ok(grp), Ok(cpu)) = (grp.parse::<usize>(), cpu.parse::<usize>()) else { continue };
                    let i = grp * 64 + cpu;
                    if out.len() <= i {
                        out.resize(i + 1, Vec::new());
                    }
                    out[i].push((format!("C{}", k + 1), v));
                }
            }
            out
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::sync::Mutex;
    use std::time::Instant;

    type Raw = Vec<Vec<(String, u64)>>;
    static PREV: Mutex<Option<(Instant, Raw)>> = Mutex::new(None);

    fn raw() -> Raw {
        let mut out = Vec::new();
        for cpu in 0.. {
            let base = format!("/sys/devices/system/cpu/cpu{cpu}");
            if !std::path::Path::new(&base).exists() {
                break;
            }
            let mut states = Vec::new();
            for s in 0.. {
                let d = format!("{base}/cpuidle/state{s}");
                let (Some(name), Some(time)) =
                    (crate::os::read_trim(format!("{d}/name")), crate::os::read_u64(format!("{d}/time")))
                else {
                    break;
                };
                states.push((name, time));
            }
            out.push(states);
        }
        out
    }

    pub fn read() -> super::Residency {
        let now = (Instant::now(), raw());
        let mut g = PREV.lock().unwrap_or_else(|e| e.into_inner());
        let r = match g.as_ref() {
            Some((t, before)) => {
                let secs = now.0.duration_since(*t).as_secs_f64().max(1e-3);
                now.1.iter().zip(before).map(|(a, b)| super::shares(b, a, secs)).collect()
            }
            None => Vec::new(),
        };
        *g = Some(now);
        r
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    pub fn read() -> super::Residency {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shares_from_cumulative_microseconds() {
        let b = vec![("POLL".to_string(), 0), ("C1".into(), 1_000_000), ("C6".into(), 0)];
        let a = vec![("POLL".to_string(), 10), ("C1".into(), 1_250_000), ("C6".into(), 500_000)];
        assert_eq!(super::shares(&b, &a, 1.0), vec![("C1".into(), 25.0), ("C6".into(), 50.0)]);
        // The second call after a warm-up returns one entry per CPU (or none on an unsupported OS).
        let _ = super::read();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let r = super::read();
        assert!(r.iter().flatten().all(|(_, v)| (0.0..=100.0).contains(v)), "{r:?}");
    }
}

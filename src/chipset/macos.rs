//! Apple Silicon has no SMBIOS type 17 records. Ask macOS for capacity and
//! memory type without inventing DIMM slots, channel counts or live timings.

use super::Memory;

#[cfg(target_os = "macos")]
pub(super) fn probe(m: &mut Memory) {
    let bytes = if m.size_mb == 0 {
        crate::os::cmd("/usr/sbin/sysctl", &["-n", "hw.memsize"]).and_then(|s| s.trim().parse().ok())
    } else {
        None
    };
    let profiler = if m.mem_type.is_none() {
        crate::os::cmd("/usr/sbin/system_profiler", &["SPMemoryDataType", "-json"])
    } else {
        None
    };
    fill(m, bytes, profiler.as_deref());
}

fn fill(m: &mut Memory, bytes: Option<u64>, profiler: Option<&str>) {
    if m.size_mb == 0 {
        m.size_mb = bytes.unwrap_or(0) / (1024 * 1024);
    }
    if m.mem_type.is_none() {
        m.mem_type = profiler.and_then(memory_type);
    }
}

fn memory_type(json: &str) -> Option<String> {
    fn types<'a>(items: &'a [serde_json::Value], out: &mut Vec<&'a str>) {
        for item in items {
            if let Some(t) = item.get("dimm_type").and_then(|v| v.as_str()).map(str::trim)
                && !t.is_empty()
                && !["unknown", "other", "empty", "none"].iter().any(|s| t.eq_ignore_ascii_case(s))
            {
                out.push(t);
            }
            if let Some(children) = item.get("_items").and_then(|v| v.as_array()) {
                types(children, out);
            }
        }
    }
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let mut found = Vec::new();
    types(v.get("SPMemoryDataType")?.as_array()?, &mut found);
    let first = *found.first()?;
    found.iter().all(|t| t.eq_ignore_ascii_case(first)).then(|| first.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_without_smbios() {
        // Synthetic profiler input; capacity comes from hw.memsize in bytes,
        // not the localized display string.
        let json = r#"{"SPMemoryDataType":[{"SPMemoryDataType":"8 Go","dimm_type":"LPDDR5"}]}"#;
        let mut m = Memory::default();
        fill(&mut m, Some(8 * 1024 * 1024 * 1024), Some(json));
        assert_eq!(m.size_mb, 8192);
        assert_eq!(m.mem_type.as_deref(), Some("LPDDR5"));
        assert!(m.channels.is_none());
        assert!(m.freq_mhz.is_none());
        assert!(m.timings.is_none());
    }

    #[test]
    fn preserves_smbios_and_handles_nested_banks() {
        let json =
            r#"{"SPMemoryDataType":[{"_items":[{"dimm_type":"DDR4"},{"dimm_type":"DDR4"},{"dimm_type":"Empty"}]}]}"#;
        assert_eq!(memory_type(json).as_deref(), Some("DDR4"));
        let mut m = Memory { size_mb: 4096, mem_type: Some("DDR3".into()), ..Default::default() };
        fill(&mut m, Some(8 * 1024 * 1024 * 1024), Some(json));
        assert_eq!(m.size_mb, 4096);
        assert_eq!(m.mem_type.as_deref(), Some("DDR3"));
    }

    #[test]
    fn partial_and_unavailable_data() {
        let mut m = Memory::default();
        fill(&mut m, Some(8 * 1024 * 1024 * 1024), Some("not JSON"));
        assert_eq!(m.size_mb, 8192);
        assert!(m.mem_type.is_none());
        let json = r#"{"SPMemoryDataType":[{"dimm_type":"LPDDR4X"}]}"#;
        fill(&mut m, None, Some(json));
        assert_eq!(m.mem_type.as_deref(), Some("LPDDR4X"));
        for json in [
            "{}",
            r#"{"SPMemoryDataType":[]}"#,
            r#"{"SPMemoryDataType":[{"dimm_type":"Unknown"},{"dimm_type":" "}]}"#,
            r#"{"SPMemoryDataType":[{"dimm_type":"DDR4"},{"dimm_type":"DDR5"}]}"#,
        ] {
            assert!(memory_type(json).is_none());
        }
        let mut m = Memory::default();
        fill(&mut m, None, None);
        assert_eq!(m.size_mb, 0);
        assert!(m.mem_type.is_none());
    }
}

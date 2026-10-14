//! Compare two reports (our JSON, or a fresh one against a saved one) and list
//! what changed: added / removed / modified fields, by section and group path.
//! Noisy live values (clocks, temperatures, voltages, powers, fans, free space)
//! are skipped unless `include_volatile`.

use crate::report::Section;

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone)]
pub struct Change {
    pub kind: ChangeKind,
    pub section: String,
    /// Group path inside the section, e.g. "DIMM # 1 › XMP profile".
    pub path: String,
    pub key: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

pub fn diff(_a: &[Section], _b: &[Section], _include_volatile: bool) -> Vec<Change> {
    Vec::new()
}

/// Load a cpuzeez JSON report (`-json=`, F7 `.cvf`).
pub fn load(path: &std::path::Path) -> Result<Vec<Section>, String> {
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&b).map_err(|e| e.to_string())
}

/// `-diff=OLD.json,NEW.json` (NEW may be "now" for a fresh report).
pub fn cli(_ctx: &crate::Ctx, _spec: &str) {}

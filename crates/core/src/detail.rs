//! Detail levels (S1-U01): the same page at three depths. *Advanced* is what
//! the pages showed before levels existed, so it's the default and nothing a
//! user saw disappears. *Basic* keeps what a gamer looks for; *Expert* adds
//! raw values (CPUID leaves, register dumps) on top of Advanced.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub enum Level {
    Basic,
    #[default]
    Advanced,
    Expert,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Basic, Level::Advanced, Level::Expert];

    pub fn label(self) -> &'static str {
        match self {
            Level::Basic => "Basic",
            Level::Advanced => "Advanced",
            Level::Expert => "Expert",
        }
    }
}

/// Rows a Basic view keeps, by their English label.
const BASIC: &[&str] = &[
    // Processor
    "Name",
    "Code Name",
    "Max TDP",
    "Package",
    "Technology",
    "Core Speed",
    "Base",
    "Level 3",
    // Mainboard
    "Manufacturer",
    "Model",
    "Chipset",
    "Brand",
    "Version",
    "Date",
    // Memory
    "Type",
    "Size",
    "Channel #",
    "DRAM Frequency",
    "Frequency",
    "CAS# Latency (CL)",
    "Module Size",
    "Max Bandwidth",
    // Graphics
    "Memory Size",
    "Memory Type",
    "Driver version",
    "Board Manufacturer",
];

/// The level a row belongs to. Rows that only make sense raw are Expert;
/// everything not in the Basic list is Advanced.
pub fn level_of(label: &str) -> Level {
    let l = label.to_ascii_lowercase();
    if l.contains("(raw)") || l.contains("register") || l.starts_with("cpuid ") {
        Level::Expert
    } else if BASIC.contains(&label) {
        Level::Basic
    } else {
        Level::Advanced
    }
}

/// Whether a row at `row` shows when the page is at `page`.
pub fn shows(page: Level, row: Level) -> bool {
    row <= page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert_eq!(level_of("Core Speed"), Level::Basic);
        assert_eq!(level_of("Microcode"), Level::Advanced, "shown before levels existed");
        assert_eq!(level_of("CPUID 0x1 EAX"), Level::Expert);
        assert_eq!(level_of("Hardware registers"), Level::Expert);
        assert!(shows(Level::Advanced, Level::Basic) && !shows(Level::Basic, Level::Advanced));
        assert_eq!(Level::default(), Level::Advanced);
    }
}

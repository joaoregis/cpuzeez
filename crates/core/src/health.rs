//! Build health (S1-G02, docs/v1/bottleneck-detector.md §2.5): configuration
//! mistakes that cost performance every day and are easy to fix. Pure rules
//! over [`Facts`] gathered from the report and the OS; a fact nobody could
//! read skips its rule instead of guessing.
//!
//! Messages are English templates with `{}` arguments, translated by the GUI.

/// One memory profile the modules support, e.g. ("EXPO", 3000 MHz).
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    /// Memory clock (MHz, half the MT/s).
    pub mhz: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuLink {
    pub name: String,
    pub width: u8,
    pub max_width: u8,
    /// A dedicated card (integrated GPUs share the CPU's link).
    pub discrete: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    pub name: String,
    pub hz: u32,
    /// Best refresh rate at the current resolution.
    pub max_hz: u32,
    /// Plugged into an integrated GPU.
    pub on_integrated: bool,
}

/// What the rules look at. `None`/empty: couldn't be read here.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {
    /// Installed memory modules (from SPD or SMBIOS).
    pub modules: Option<usize>,
    /// Current memory clock (MHz).
    pub mem_mhz: Option<f64>,
    /// The fastest XMP/EXPO profile the modules offer.
    pub best_profile: Option<Profile>,
    pub gpus: Vec<GpuLink>,
    /// Windows "Power saver" plan, or Linux's acpi-cpufreq "powersave" governor.
    pub power_saver: Option<bool>,
    pub displays: Vec<Display>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    /// Short title template and its arguments.
    pub title: &'static str,
    pub detail: &'static str,
    pub args: Vec<String>,
}

fn finding(
    id: &'static str,
    severity: Severity,
    title: &'static str,
    detail: &'static str,
    args: Vec<String>,
) -> Finding {
    Finding { id, severity, title, detail, args }
}

/// Memory below the profile by more than this is "profile off" (not just a
/// slightly lower manual setting).
const PROFILE_GAP: f64 = 0.95;

pub fn check(f: &Facts) -> Vec<Finding> {
    let mut v = Vec::new();
    if f.modules == Some(1) {
        v.push(finding(
            "single-channel",
            Severity::Warning,
            "One memory module: single channel",
            "The CPU gets half the memory bandwidth it could. Two matching modules (in the slots the manual names for dual channel) fix it.",
            vec![],
        ));
    }
    if let (Some(cur), Some(p)) = (f.mem_mhz, &f.best_profile)
        && cur < p.mhz * PROFILE_GAP
    {
        v.push(finding(
            "memory-profile-off",
            Severity::Warning,
            "Memory runs at {} MT/s; the modules support {} MT/s ({})",
            "The memory profile isn't enabled. Turn on {} in the BIOS (often called XMP, EXPO or DOCP).",
            vec![format!("{:.0}", cur * 2.0), format!("{:.0}", p.mhz * 2.0), p.name.clone()],
        ));
    }
    for g in f.gpus.iter().filter(|g| g.discrete && g.width > 0 && g.width < g.max_width) {
        v.push(finding(
            "gpu-link-narrow",
            Severity::Warning,
            "{} runs on x{} lanes (it supports x{})",
            "Usually the wrong slot, a riser, or a lane-sharing BIOS setting (M.2 drives can take lanes from the GPU slot).",
            vec![g.name.clone(), g.width.to_string(), g.max_width.to_string()],
        ));
    }
    if f.power_saver == Some(true) {
        v.push(finding(
            "power-saver",
            Severity::Warning,
            "Power saving plan active",
            "The CPU is held back to save energy. Pick Balanced or High performance in the power settings.",
            vec![],
        ));
    }
    for d in f.displays.iter().filter(|d| d.hz > 0 && d.max_hz > d.hz + 1) {
        v.push(finding(
            "refresh-below-max",
            Severity::Info,
            "{} runs at {} Hz; it supports {} Hz",
            "Choose the higher refresh rate in the display settings (and check the cable supports it).",
            vec![d.name.clone(), d.hz.to_string(), d.max_hz.to_string()],
        ));
    }
    let dgpu = f.gpus.iter().any(|g| g.discrete);
    if dgpu && !f.displays.is_empty() && f.displays.iter().all(|d| d.on_integrated) {
        v.push(finding(
            "display-on-igpu",
            Severity::Warning,
            "Monitor plugged into the motherboard",
            "A graphics card is installed, but the monitor is connected to the motherboard's video output, so games render on the integrated GPU. Plug it into the graphics card.",
            vec![],
        ));
    }
    v.sort_by_key(|f| std::cmp::Reverse(f.severity));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(f: &Facts) -> Vec<&'static str> {
        check(f).iter().map(|x| x.id).collect()
    }

    fn good() -> Facts {
        Facts {
            modules: Some(2),
            mem_mhz: Some(2999.9),
            best_profile: Some(Profile { name: "EXPO".into(), mhz: 3000.0 }),
            gpus: vec![GpuLink { name: "RTX 4060".into(), width: 8, max_width: 8, discrete: true }],
            power_saver: Some(false),
            displays: vec![Display { name: "DELL".into(), hz: 165, max_hz: 165, on_integrated: false }],
        }
    }

    #[test]
    fn a_healthy_build_has_nothing_to_say() {
        assert!(check(&good()).is_empty());
        assert!(check(&Facts::default()).is_empty(), "unknown facts skip their rules");
    }

    #[test]
    fn each_mistake() {
        assert_eq!(ids(&Facts { modules: Some(1), ..good() }), ["single-channel"]);
        let off = check(&Facts { mem_mhz: Some(2400.0), ..good() });
        assert_eq!(off[0].id, "memory-profile-off");
        assert_eq!(off[0].args, ["4800", "6000", "EXPO"]);
        assert!(ids(&Facts { mem_mhz: Some(2900.0), ..good() }).is_empty(), "a mild manual tune isn't 'off'");
        let narrow = GpuLink { width: 4, max_width: 16, ..good().gpus[0].clone() };
        assert_eq!(ids(&Facts { gpus: vec![narrow], ..good() }), ["gpu-link-narrow"]);
        let igpu = GpuLink { name: "Radeon".into(), width: 0, max_width: 0, discrete: false };
        assert!(ids(&Facts { gpus: vec![igpu.clone()], ..good() }).is_empty());
        assert_eq!(ids(&Facts { power_saver: Some(true), ..good() }), ["power-saver"]);
        let slow = Display { hz: 60, ..good().displays[0].clone() };
        assert_eq!(ids(&Facts { displays: vec![slow], ..good() }), ["refresh-below-max"]);
        let on_mb = Display { on_integrated: true, ..good().displays[0].clone() };
        assert_eq!(ids(&Facts { displays: vec![on_mb], ..good() }), ["display-on-igpu"]);
    }

    #[test]
    fn warnings_come_first() {
        let slow = Display { hz: 60, ..good().displays[0].clone() };
        let f = Facts { displays: vec![slow], modules: Some(1), ..good() };
        assert_eq!(ids(&f), ["single-channel", "refresh-below-max"]);
    }
}

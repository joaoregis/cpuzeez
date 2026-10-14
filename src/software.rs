//! Software section (FEATURES.md 3.21): OS name/version/build, install date,
//! graphics API version (DirectX on Windows, Vulkan/OpenGL on Linux, Metal on macOS).
// Parsers stay OS-neutral so their unit tests run on every host.
#![allow(dead_code)]

use crate::Ctx;
use crate::report::{Node, Section, f, section};

/// Unix seconds to "M/D/YYYY" (UTC), CPU-Z's date style.
pub fn mdy(secs: i64) -> String {
    // Days-to-civil, proleptic Gregorian (H. Hinnant's public-domain algorithm).
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{m}/{d}/{y}")
}

/// `KEY=value` lines of /etc/os-release (quotes stripped).
pub fn os_release(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches(['"', '\'']).to_string())
    })
}

/// Value after `label` on the first line containing it ("Vulkan Instance Version: 1.3.275").
pub fn after(text: &str, label: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let rest = &l[l.find(label)? + label.len()..];
        let v = rest.trim_start_matches([':', '=', ' ', '\t']).trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// Windows product line from registry values. Win11 still reports
/// "Windows 10" in ProductName, so the major is taken from the build.
pub fn windows_line(edition: &str, display: &str, build: u32, ubr: Option<u64>, arch: &str) -> String {
    let major = if build >= 22000 { "11" } else { "10" };
    let ed = if edition.starts_with("Core") { "Home" } else { edition };
    let ubr = ubr.map(|u| format!(".{u}")).unwrap_or_default();
    // Double space matches CPU-Z's output.
    let mut s = format!("Microsoft Windows {major}  {ed} ({arch})");
    if !display.is_empty() {
        s += &format!(", Version {display}");
    }
    s + &format!(", Build {build}{ubr}")
}

fn created_secs(p: &str) -> Option<i64> {
    let m = std::fs::metadata(p).ok()?;
    let t = m.created().or_else(|_| m.modified()).ok()?;
    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs() as i64)
}

pub fn os_line(ctx: &Ctx) -> String {
    imp::fields(ctx).into_iter().next().map(|(_, v)| v).unwrap_or_else(|| std::env::consts::OS.into())
}

pub fn collect(ctx: &Ctx) -> Vec<Section> {
    let nodes: Vec<Node> = imp::fields(ctx).into_iter().map(|(k, v)| f(k, v)).collect();
    vec![section("Software", nodes)]
}

#[cfg(windows)]
mod imp {
    use crate::os::win::{reg_str, reg_u64};
    const CV: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

    pub fn fields(_: &crate::Ctx) -> Vec<(&'static str, String)> {
        let build: u32 = reg_str(CV, "CurrentBuild").and_then(|b| b.parse().ok()).unwrap_or(0);
        let arch = std::env::var("PROCESSOR_ARCHITEW6432")
            .or_else(|_| std::env::var("PROCESSOR_ARCHITECTURE"))
            .unwrap_or_default();
        let arch = match arch.as_str() {
            "AMD64" => "x64",
            "ARM64" => "ARM64",
            "x86" => "x86",
            a => a,
        };
        let display = reg_str(CV, "DisplayVersion").or_else(|| reg_str(CV, "ReleaseId")).unwrap_or_default();
        let edition = reg_str(CV, "EditionID").unwrap_or_default();
        let mut v = vec![(
            "Windows Version",
            super::windows_line(&edition, &display, build, reg_u64(CV, "UBR").map(|u| u & 0xFFFF_FFFF), arch),
        )];
        if let Some(t) = reg_u64(CV, "InstallDate").filter(|&t| t > 0) {
            v.push(("Windows Installation Date", super::mdy((t & 0xFFFF_FFFF) as i64)));
        }
        let dx = match build {
            10240.. => "12.0",
            9600.. => "11.2",
            9200.. => "11.1",
            _ => "11.0",
        };
        v.push(("DirectX Version", dx.into()));
        v
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{after, created_secs, mdy, os_release};
    use crate::os::cmd;

    pub fn fields(_: &crate::Ctx) -> Vec<(&'static str, String)> {
        let rel = std::fs::read_to_string("/etc/os-release")
            .or_else(|_| std::fs::read_to_string("/usr/lib/os-release"))
            .unwrap_or_default();
        let distro = os_release(&rel, "PRETTY_NAME").or_else(|| os_release(&rel, "NAME")).unwrap_or("Linux".into());
        let kernel = crate::os::read_trim("/proc/sys/kernel/osrelease").unwrap_or_default();
        let mut v = vec![("Linux Version", format!("{distro}, kernel {kernel} ({})", std::env::consts::ARCH))];
        // ponytail: root fs birth time ~ install time; misses reinstalls onto an old fs.
        if let Some(t) = ["/", "/var/log/installer", "/etc/machine-id"].iter().find_map(|p| created_secs(p)) {
            v.push(("Installation Date", mdy(t)));
        }
        if let Some(s) = cmd("vulkaninfo", &["--summary"]).and_then(|o| after(&o, "Vulkan Instance Version")) {
            v.push(("Vulkan Version", s));
        }
        if let Some(s) = cmd("glxinfo", &["-B"]).and_then(|o| after(&o, "OpenGL version string")) {
            v.push(("OpenGL Version", s));
        }
        v
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{after, created_secs, mdy};
    use crate::os::cmd;

    pub fn fields(_: &crate::Ctx) -> Vec<(&'static str, String)> {
        let sw = cmd("sw_vers", &[]).unwrap_or_default();
        let get = |k: &str| after(&sw, k).unwrap_or_default();
        let mut v = vec![(
            "macOS Version",
            format!(
                "{} {} ({}), Build {}",
                get("ProductName"),
                get("ProductVersion"),
                std::env::consts::ARCH,
                get("BuildVersion")
            ),
        )];
        if let Some(t) = created_secs("/var/db/.AppleSetupDone") {
            v.push(("Installation Date", mdy(t)));
        }
        // "spdisplays_mtlgpufamilysupport" : "spdisplays_metal3"
        let disp = cmd("system_profiler", &["SPDisplaysDataType", "-json"]).unwrap_or_default();
        let metal = disp.lines().find(|l| l.contains("mtlgpufamilysupport") || l.contains("\"spdisplays_metal\""));
        if let Some(m) = metal.and_then(|l| l.rsplit("metal").next()) {
            let n = m.trim_matches(|c: char| !c.is_ascii_digit());
            v.push(("Metal Version", if n.is_empty() { "Supported".into() } else { format!("Metal {n}") }));
        }
        v
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod imp {
    pub fn fields(_: &crate::Ctx) -> Vec<(&'static str, String)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(mdy(1_711_954_171), "4/1/2024");
        assert_eq!(mdy(0), "1/1/1970");
        assert_eq!(mdy(951_782_400), "2/29/2000");
    }

    #[test]
    fn parsers() {
        let r = "NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nID=ubuntu\n";
        assert_eq!(os_release(r, "PRETTY_NAME").as_deref(), Some("Ubuntu 24.04.1 LTS"));
        assert_eq!(os_release(r, "ID").as_deref(), Some("ubuntu"));
        assert_eq!(os_release(r, "X"), None);
        assert_eq!(after("ProductName:\t\tmacOS\nBuildVersion:\t24B83", "BuildVersion").as_deref(), Some("24B83"));
        assert_eq!(after("Vulkan Instance Version: 1.3.275", "Vulkan Instance Version").as_deref(), Some("1.3.275"));
    }

    #[test]
    fn windows() {
        assert_eq!(
            windows_line("Professional", "25H2", 26200, Some(9457), "x64"),
            "Microsoft Windows 11  Professional (x64), Version 25H2, Build 26200.9457"
        );
        assert!(windows_line("Core", "", 19045, None, "x64").starts_with("Microsoft Windows 10  Home"));
    }
}

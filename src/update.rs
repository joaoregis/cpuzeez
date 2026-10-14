//! Update notice, like CPU-Z's version checker: one anonymous GET to the
//! GitHub releases API (`CheckUpdates=0` in cpuzeez.ini turns it off).
//! Uses the system `curl` (Windows 10+, macOS and desktop Linux ship it), so
//! the binary carries no HTTP/TLS stack.

use serde_json::Value;

const API: &str = "https://api.github.com/repos/joaoregis/cpuzeez/releases/latest";

#[derive(Debug, Clone)]
pub struct Release {
    pub version: String,
    /// Release page (notes).
    pub page: String,
    /// The installer for this OS/arch, when the release has one.
    pub download: Option<String>,
}

/// The latest release if it is newer than this build.
pub fn check() -> Option<Release> {
    latest().filter(|r| newer(crate::VERSION, &r.version))
}

/// The latest published release, whatever its version.
pub fn latest() -> Option<Release> {
    let ua = format!("User-Agent: {}/{}", crate::NAME, crate::VERSION);
    let body = crate::os::cmd(
        "curl",
        &["-fsSL", "--max-time", "8", "-H", "Accept: application/vnd.github+json", "-H", &ua, API],
    )?;
    parse(&body)
}

fn parse(body: &str) -> Option<Release> {
    let v: Value = serde_json::from_str(body).ok()?;
    let tag = v["tag_name"].as_str()?;
    let names: Vec<(String, String)> = v["assets"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    Some((x["name"].as_str()?.to_string(), x["browser_download_url"].as_str()?.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Release {
        version: tag.trim_start_matches('v').to_string(),
        page: v["html_url"].as_str().unwrap_or(crate::REPO_URL).to_string(),
        download: pick(&names, &platform_suffixes()),
    })
}

/// Asset-name endings to try, best first, for this OS / distro / CPU.
fn platform_suffixes() -> Vec<String> {
    let arch = std::env::consts::ARCH; // x86_64, aarch64
    let deb = if arch == "aarch64" { "arm64" } else { "amd64" };
    match std::env::consts::OS {
        "windows" => vec![".msi".into(), "-portable.zip".into()],
        "macos" => vec!["-macos-universal.dmg".into(), "-macos-universal.pkg".into()],
        _ if std::path::Path::new("/etc/arch-release").exists() => {
            vec![format!("-{arch}.pkg.tar.zst"), format!("-linux-{arch}.tar.gz")]
        }
        _ if std::path::Path::new("/etc/debian_version").exists() => {
            vec![format!("_{deb}.deb"), format!("-linux-{arch}.tar.gz")]
        }
        _ => vec![format!("-linux-{arch}.tar.gz")],
    }
}

fn pick(assets: &[(String, String)], suffixes: &[String]) -> Option<String> {
    suffixes.iter().find_map(|s| assets.iter().find(|(n, _)| n.ends_with(s.as_str())).map(|(_, u)| u.clone()))
}

/// Semver-ish compare of "major.minor.patch" (pre-release tags sort lower).
pub fn newer(current: &str, latest: &str) -> bool {
    fn key(v: &str) -> (Vec<u64>, bool) {
        let (core, pre) = v.trim_start_matches('v').split_once('-').map_or((v, false), |(c, _)| (c, true));
        (core.trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0)).collect(), !pre)
    }
    key(latest) > key(current)
}

/// `-checkupdate`
pub fn cli() {
    match latest() {
        None => eprintln!("Could not reach GitHub (is curl installed and the network up?)."),
        Some(r) if !newer(crate::VERSION, &r.version) => {
            println!("{} {} is up to date (latest release: {}).", crate::DISPLAY_NAME, crate::VERSION, r.version)
        }
        Some(r) => {
            println!("{} {} is available (you have {}).", crate::DISPLAY_NAME, r.version, crate::VERSION);
            println!("Release notes: {}", r.page);
            if let Some(d) = r.download {
                println!("Download:      {d}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(newer("0.1.0", "0.1.1"));
        assert!(newer("0.1.9", "0.2.0"));
        assert!(newer("0.1.0", "v1.0.0"));
        assert!(!newer("0.2.0", "0.1.9"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(newer("0.2.0-rc1", "0.2.0"));
        assert!(!newer("0.2.0", "0.2.0-rc1"));
    }

    #[test]
    fn parses_release_and_picks_installer() {
        let body = r#"{"tag_name":"v9.8.7","html_url":"https://example/rel","assets":[
            {"name":"cpuzeez-9.8.7-windows-x64.msi","browser_download_url":"https://example/a.msi"},
            {"name":"cpuzeez_9.8.7_amd64.deb","browser_download_url":"https://example/a.deb"},
            {"name":"cpuzeez-9.8.7-macos-universal.dmg","browser_download_url":"https://example/a.dmg"}]}"#;
        let r = parse(body).unwrap();
        assert_eq!(r.version, "9.8.7");
        assert_eq!(r.page, "https://example/rel");
        let assets = vec![("x.msi".to_string(), "u-msi".to_string()), ("x_arm64.deb".to_string(), "u-deb".to_string())];
        assert_eq!(pick(&assets, &[".msi".into()]), Some("u-msi".into()));
        assert_eq!(pick(&assets, &["_amd64.deb".into(), "_arm64.deb".into()]), Some("u-deb".into()));
        assert_eq!(pick(&assets, &[".dmg".into()]), None);
    }
}

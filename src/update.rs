//! Update notice, like CPU-Z's version checker: one anonymous GET to the
//! GitHub releases API (`CheckUpdates=0` in cpuzeez.ini turns it off), and
//! one-click install: download the installer for this OS with the system
//! `curl` (Windows 10+, macOS and desktop Linux ship it, so the binary carries
//! no HTTP/TLS stack), check it against the release's SHA256SUMS.txt, then
//! hand it to the OS installer (msiexec / open / pkexec apt-get|pacman).

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const API: &str = "https://api.github.com/repos/joaoregis/cpuzeez/releases/latest";

#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    /// Bytes, from the API (0 = unknown).
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct Release {
    pub version: String,
    /// Release page (notes).
    pub page: String,
    /// The installer for this OS/arch, when the release has one.
    pub asset: Option<Asset>,
    /// SHA256SUMS.txt of the release.
    pub sums: Option<String>,
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
    let assets: Vec<Asset> = v["assets"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    Some(Asset {
                        name: x["name"].as_str()?.to_string(),
                        url: x["browser_download_url"].as_str()?.to_string(),
                        size: x["size"].as_u64().unwrap_or(0),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Release {
        version: tag.trim_start_matches('v').to_string(),
        page: v["html_url"].as_str().unwrap_or(crate::REPO_URL).to_string(),
        asset: pick(&assets, &platform_suffixes()),
        sums: assets.iter().find(|a| a.name == "SHA256SUMS.txt").map(|a| a.url.clone()),
    })
}

/// Asset-name endings to try, best first, for this OS / distro / CPU.
fn platform_suffixes() -> Vec<String> {
    let arch = std::env::consts::ARCH; // x86_64, aarch64
    let deb = if arch == "aarch64" { "arm64" } else { "amd64" };
    match std::env::consts::OS {
        "windows" => vec![".msi".into(), "-portable.zip".into()],
        "macos" => vec!["-macos-universal.dmg".into(), "-macos-universal.pkg".into()],
        _ if Path::new("/etc/arch-release").exists() => {
            vec![format!("-{arch}.pkg.tar.zst"), format!("-linux-{arch}.tar.gz")]
        }
        _ if Path::new("/etc/debian_version").exists() => {
            vec![format!("_{deb}.deb"), format!("-linux-{arch}.tar.gz")]
        }
        _ => vec![format!("-linux-{arch}.tar.gz")],
    }
}

fn pick(assets: &[Asset], suffixes: &[String]) -> Option<Asset> {
    suffixes.iter().find_map(|s| assets.iter().find(|a| a.name.ends_with(s.as_str())).cloned())
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
            if let Some(a) = r.asset {
                println!("Download:      {}", a.url);
            }
        }
    }
}

// ---------------------------------------------------------------- install

#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    Downloading {
        done: u64,
        total: u64,
    },
    Verifying,
    Installing,
    /// The package manager finished: restart to use the new version.
    Installed,
    /// The OS installer UI was opened (macOS .dmg/.pkg).
    OpenedInstaller,
    /// No automatic install for this package: its folder was opened.
    OpenedFolder(PathBuf),
    /// msiexec started: exit now so it can replace our files.
    Quit,
    Failed(String),
}

/// A download + verify + install on a background thread.
pub struct Installer {
    stage: Arc<Mutex<Stage>>,
}

impl Installer {
    pub fn start(rel: &Release) -> Installer {
        let stage = Arc::new(Mutex::new(Stage::Downloading { done: 0, total: 0 }));
        let (s, rel) = (stage.clone(), rel.clone());
        std::thread::spawn(move || {
            let end = download_and_verify(&rel, &s)
                .and_then(|p| {
                    *s.lock().unwrap() = Stage::Installing;
                    launch(&p)
                })
                .unwrap_or_else(Stage::Failed);
            *s.lock().unwrap() = end;
        });
        Installer { stage }
    }

    pub fn stage(&self) -> Stage {
        self.stage.lock().unwrap().clone()
    }
}

fn command(prog: &str) -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new(prog);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

/// Download the release's installer to the temp dir and check its SHA-256.
fn download_and_verify(rel: &Release, stage: &Mutex<Stage>) -> Result<PathBuf, String> {
    let a = rel.asset.as_ref().ok_or("this release has no package for this system")?;
    let sums_url = rel.sums.as_ref().ok_or("this release has no SHA256SUMS.txt; not installing an unverified file")?;
    // The name comes from the network: never let it pick a path.
    if Path::new(&a.name).file_name().and_then(|n| n.to_str()) != Some(a.name.as_str()) {
        return Err(format!("unexpected asset name {:?}", a.name));
    }
    let dir = std::env::temp_dir().join(format!("{}-update-{}", crate::NAME, rel.version));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(&a.name);
    let _ = std::fs::remove_file(&path);

    let mut child = command("curl")
        .args(["-fL", "--retry", "2", "--max-time", "900", "-o"])
        .arg(&path)
        .arg(&a.url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("curl: {e}"))?;
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break st;
        }
        let done = std::fs::metadata(&path).map_or(0, |m| m.len());
        *stage.lock().unwrap() = Stage::Downloading { done, total: a.size };
        std::thread::sleep(std::time::Duration::from_millis(150));
    };
    if !status.success() {
        return Err(format!("download failed (curl exit code {})", status.code().unwrap_or(-1)));
    }
    let len = std::fs::metadata(&path).map_or(0, |m| m.len());
    *stage.lock().unwrap() = Stage::Downloading { done: len, total: a.size };
    if a.size > 0 && len != a.size {
        return Err(format!("download incomplete ({len} of {} bytes)", a.size));
    }

    *stage.lock().unwrap() = Stage::Verifying;
    let sums =
        crate::os::cmd("curl", &["-fsSL", "--max-time", "30", sums_url]).ok_or("could not download SHA256SUMS.txt")?;
    let want = sum_for(&sums, &a.name).ok_or_else(|| format!("SHA256SUMS.txt has no entry for {}", a.name))?;
    let got = sha256_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !got.eq_ignore_ascii_case(&want) {
        let _ = std::fs::remove_file(&path);
        return Err(format!("checksum mismatch for {} (expected {want}, got {got}); file deleted", a.name));
    }
    Ok(path)
}

/// Hand the verified package to the OS.
fn launch(path: &Path) -> Result<Stage, String> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let folder = || {
        let dir = path.parent().unwrap_or(path).to_path_buf();
        open_path(&dir);
        Ok(Stage::OpenedFolder(dir))
    };
    if name.ends_with(".msi") {
        // msiexec shows its own UI and the UAC prompt; we exit so files aren't locked.
        command("msiexec").arg("/i").arg(path).spawn().map_err(|e| format!("msiexec: {e}"))?;
        return Ok(Stage::Quit);
    }
    if cfg!(target_os = "macos") && (name.ends_with(".dmg") || name.ends_with(".pkg")) {
        let st = command("open").arg(path).status().map_err(|e| format!("open: {e}"))?;
        return if st.success() { Ok(Stage::OpenedInstaller) } else { Err("open failed".into()) };
    }
    let manager: Option<&[&str]> = if name.ends_with(".deb") {
        Some(&["apt-get", "install", "-y"])
    } else if name.ends_with(".pkg.tar.zst") {
        Some(&["pacman", "-U", "--noconfirm"])
    } else {
        None
    };
    match manager {
        Some(args) if in_path("pkexec") => {
            let st = command("pkexec").args(args).arg(path).status().map_err(|e| format!("pkexec: {e}"))?;
            match st.code() {
                Some(0) => Ok(Stage::Installed),
                Some(126) | Some(127) => Err("authorization was cancelled or refused".into()),
                c => Err(format!("{} failed (exit code {})", args[0], c.unwrap_or(-1))),
            }
        }
        _ => folder(), // tarball / zip, or no pkexec
    }
}

fn in_path(prog: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file()))
}

/// Open a folder or file with the OS's default handler (file manager for folders).
pub fn open_path(p: &Path) -> bool {
    let prog = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    command(prog).arg(p).spawn().is_ok()
}

/// The hash for `name` in a `sha256sum` listing ("HASH  name" or "HASH *name").
fn sum_for(listing: &str, name: &str) -> Option<String> {
    listing.lines().find_map(|l| {
        let (hash, file) = l.trim().split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name && hash.len() == 64).then(|| hash.to_ascii_lowercase())
    })
}

// ---------------------------------------------------------------- SHA-256 (FIPS 180-4)

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98,
    0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8,
    0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
    0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
    0xc67178f2,
];

pub struct Sha256 {
    h: [u32; 8],
    buf: Vec<u8>,
    len: u64,
}

impl Sha256 {
    pub fn new() -> Sha256 {
        Sha256 {
            h: [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19],
            buf: Vec::with_capacity(128),
            len: 0,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.len += data.len() as u64;
        self.buf.extend_from_slice(data);
        let n = self.buf.len() / 64 * 64;
        for block in self.buf[..n].as_chunks::<64>().0 {
            compress(&mut self.h, block);
        }
        self.buf.drain(..n);
    }

    pub fn finish(mut self) -> [u8; 32] {
        let bits = self.len.wrapping_mul(8);
        self.buf.push(0x80);
        while self.buf.len() % 64 != 56 {
            self.buf.push(0);
        }
        self.buf.extend_from_slice(&bits.to_be_bytes());
        for block in self.buf.as_chunks::<64>().0 {
            compress(&mut self.h, block);
        }
        let mut out = [0; 32];
        for (o, h) in out.as_chunks_mut::<4>().0.iter_mut().zip(self.h) {
            *o = h.to_be_bytes();
        }
        out
    }
}

fn compress(h: &mut [u32; 8], block: &[u8; 64]) {
    let mut w = [0u32; 64];
    for (i, c) in block.as_chunks::<4>().0.iter().enumerate() {
        w[i] = u32::from_be_bytes(*c);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = *h;
    for i in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ (!e & g);
        let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(maj);
        (hh, g, f, e, d, c, b, a) = (g, f, e, d.wrapping_add(t1), c, b, a, t1.wrapping_add(t2));
    }
    for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
        *x = x.wrapping_add(y);
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let (mut h, mut buf) = (Sha256::new(), vec![0; 1 << 16]);
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            return Ok(hex(&h.finish()));
        }
        h.update(&buf[..n]);
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
            {"name":"cpuzeez-9.8.7-windows-x64.msi","browser_download_url":"https://example/a.msi","size":123},
            {"name":"cpuzeez_9.8.7_amd64.deb","browser_download_url":"https://example/a.deb","size":5},
            {"name":"SHA256SUMS.txt","browser_download_url":"https://example/sums","size":9},
            {"name":"cpuzeez-9.8.7-macos-universal.dmg","browser_download_url":"https://example/a.dmg"}]}"#;
        let r = parse(body).unwrap();
        assert_eq!(r.version, "9.8.7");
        assert_eq!(r.page, "https://example/rel");
        assert_eq!(r.sums.as_deref(), Some("https://example/sums"));
        let a = |n: &str, u: &str| Asset { name: n.into(), url: u.into(), size: 1 };
        let assets = vec![a("x.msi", "u-msi"), a("x_arm64.deb", "u-deb")];
        assert_eq!(pick(&assets, &[".msi".into()]).unwrap().url, "u-msi");
        assert_eq!(pick(&assets, &["_amd64.deb".into(), "_arm64.deb".into()]).unwrap().url, "u-deb");
        assert_eq!(pick(&assets, &[".dmg".into()]), None);
    }

    #[test]
    fn sha256_fips_vectors() {
        let h = |d: &[u8]| {
            let mut s = Sha256::new();
            s.update(d);
            hex(&s.finish())
        };
        assert_eq!(h(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(h(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(
            h(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // One million 'a', fed in uneven pieces to exercise buffering.
        let mut s = Sha256::new();
        let a = vec![b'a'; 1000];
        for i in 0..1000 {
            let cut = i % 7;
            s.update(&a[..cut]);
            s.update(&a[cut..]);
        }
        assert_eq!(hex(&s.finish()), "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
    }

    #[test]
    fn checksum_listing() {
        let l = "AA11  other.zip\n0123456789abcdef0123456789abcdef0123456789abcdef0123456789ABCDEF *cpuzeez.msi\n";
        assert_eq!(
            sum_for(l, "cpuzeez.msi").unwrap(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
        assert_eq!(sum_for(l, "other.zip"), None); // not 64 hex chars
        assert_eq!(sum_for(l, "missing"), None);
    }

    /// Real download + verification of the latest release's package for this
    /// OS (no install). `cargo test -- --ignored update::`
    #[test]
    #[ignore]
    fn downloads_and_verifies_latest_release() {
        let rel = latest().expect("GitHub reachable");
        let p = download_and_verify(&rel, &Mutex::new(Stage::Verifying)).unwrap();
        assert!(p.is_file());
        eprintln!("verified {}", p.display());
    }
}

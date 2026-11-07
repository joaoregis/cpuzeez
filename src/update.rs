//! Update notice, like CPU-Z's version checker: one anonymous GET to the
//! GitHub releases API (`CheckUpdates=0` in cpuzeez.ini turns it off), and
//! one-click install: download the installer for this OS with the system
//! `curl` (Windows 10+, macOS and desktop Linux ship it, so the binary carries
//! no HTTP/TLS stack), check it against the release's SHA256SUMS.txt, then
//! hand it to the OS installer (msiexec / open / pkexec apt-get|pacman).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The list, newest first. `/releases/latest` is no use here: GitHub leaves
/// pre-releases out of it, and the betas are the point of the channels.
const API: &str = "https://api.github.com/repos/joaoregis/cpuzeez/releases?per_page=30";

/// Which releases the update check may offer. Each channel also offers
/// everything steadier than itself: Beta sees stable releases too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum Channel {
    /// Releases only (`1.0.0`).
    #[default]
    Stable,
    /// Plus betas and release candidates (`1.0.0-beta.8`, `1.0.0-rc.1`).
    Beta,
    /// Plus alphas (`1.1.0-alpha.1`): everything that is published.
    Alpha,
}

impl Channel {
    pub const ALL: [Channel; 3] = [Channel::Stable, Channel::Beta, Channel::Alpha];

    /// The channel a version belongs to, from its pre-release label.
    pub fn of(version: &str) -> Channel {
        let v = version.trim().trim_start_matches('v');
        let v = v.split_once('+').map_or(v, |(c, _)| c);
        match v.split_once('-').map(|(_, pre)| pre) {
            None => Channel::Stable,
            Some(pre) if pre.to_ascii_lowercase().starts_with("alpha") => Channel::Alpha,
            Some(_) => Channel::Beta,
        }
    }

    /// Whether this channel offers `version`.
    pub fn sees(self, version: &str) -> bool {
        Channel::of(version) <= self
    }

    pub fn name(self) -> &'static str {
        match self {
            Channel::Stable => "Stable",
            Channel::Beta => "Beta",
            Channel::Alpha => "Alpha",
        }
    }

    /// `stable`, `beta` or `alpha` (the `-checkupdate=` argument).
    pub fn from_arg(s: &str) -> Option<Channel> {
        Channel::ALL.into_iter().find(|c| c.name().eq_ignore_ascii_case(s.trim()))
    }
}

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

/// The newest release on `channel` if it is newer than this build.
pub fn check(channel: Channel) -> Option<Release> {
    latest(channel).filter(|r| newer(crate::VERSION, &r.version))
}

/// The newest published release on `channel`, whatever this build is.
pub fn latest(channel: Channel) -> Option<Release> {
    let ua = format!("User-Agent: {}/{}", crate::NAME, crate::VERSION);
    let body = crate::os::cmd(
        "curl",
        &["-fsSL", "--max-time", "8", "-H", "Accept: application/vnd.github+json", "-H", &ua, API],
    )?;
    newest(&body, channel)
}

/// The newest release of a `/releases` listing that `channel` offers. Drafts
/// are skipped; the tag, not GitHub's pre-release flag, decides the channel.
pub(crate) fn newest(body: &str, channel: Channel) -> Option<Release> {
    let list: Vec<Value> = serde_json::from_str(body).ok()?;
    list.iter()
        .filter(|v| v["draft"].as_bool() != Some(true))
        .filter_map(release_from)
        .filter(|r| channel.sees(&r.version))
        .reduce(|best, r| if newer(&best.version, &r.version) { r } else { best })
}

#[cfg(test)]
pub(crate) fn parse(body: &str) -> Option<Release> {
    release_from(&serde_json::from_str(body).ok()?)
}

pub(crate) fn release_from(v: &Value) -> Option<Release> {
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

/// The startup check runs only when cpuzeez.ini allows it and the user opted
/// in on the first-run screen or in Settings (P1). Not asked yet = no check.
pub fn startup_check_allowed(ini_allows: bool, opted_in: Option<bool>) -> bool {
    ini_allows && opted_in == Some(true)
}

/// SemVer precedence: "major.minor.patch", then pre-releases below the release
/// they precede, ordered identifier by identifier (`beta.2` > `beta.1`, `rc.1` > `beta.9`).
pub fn newer(current: &str, latest: &str) -> bool {
    use std::cmp::Ordering;
    // Numeric identifiers sort before alphanumeric ones and compare as numbers (SemVer §11).
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum Id<'a> {
        Num(u64),
        Text(&'a str),
    }
    fn parse(v: &str) -> (Vec<u64>, Option<Vec<Id<'_>>>) {
        let v = v.trim().trim_start_matches('v');
        let v = v.split_once('+').map_or(v, |(c, _)| c); // build metadata never counts
        let (core, pre) = v.split_once('-').map_or((v, None), |(c, p)| (c, Some(p)));
        let core = core.split('.').map(|p| p.parse().unwrap_or(0)).collect();
        let pre = pre.map(|p| p.split('.').map(|i| i.parse().map_or(Id::Text(i), Id::Num)).collect());
        (core, pre)
    }
    let ((lc, lp), (cc, cp)) = (parse(latest), parse(current));
    match lc.cmp(&cc) {
        Ordering::Equal => match (lp, cp) {
            (None, Some(_)) => true,     // the release beats its own pre-releases
            (Some(l), Some(c)) => l > c, // Vec compares element-wise, then the longer wins
            _ => false,
        },
        o => o == Ordering::Greater,
    }
}

/// `-checkupdate[=stable|beta|alpha]`; without a channel, the one in Settings.
pub fn cli(arg: &str) {
    let channel = Channel::from_arg(arg).unwrap_or_else(|| crate::settings::Settings::load().channel());
    match latest(channel) {
        None => eprintln!("Could not find a {} release (is curl installed and the network up?).", channel.name()),
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
            let end = match download_and_verify(&rel, &s) {
                Err(e) => Stage::Failed(e),
                Ok(p) => {
                    *s.lock().unwrap() = Stage::Installing;
                    let end = launch(&p).unwrap_or_else(Stage::Failed);
                    // The package has been consumed by the install (or the
                    // install failed for good): reclaim the space. msiexec,
                    // macOS `open` and the OpenedFolder fallback still need
                    // the file after we return, so those stages keep it.
                    if matches!(end, Stage::Installed | Stage::Failed(_)) {
                        if let Some(d) = p.parent() {
                            let _ = std::fs::remove_dir_all(d);
                        }
                    }
                    end
                }
            };
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
    // Never inherit stdin: from a GUI whose console was dropped it is not a valid handle.
    c.stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

/// Download the release's installer to a fresh private directory and check its
/// SHA-256. On any failure the directory (with any partial download) is
/// removed before the error is returned.
fn download_and_verify(rel: &Release, stage: &Mutex<Stage>) -> Result<PathBuf, String> {
    let a = rel.asset.as_ref().ok_or("this release has no package for this system")?;
    let sums_url = rel.sums.as_ref().ok_or("this release has no SHA256SUMS.txt; not installing an unverified file")?;
    // The name comes from the network: never let it pick a path.
    if Path::new(&a.name).file_name().and_then(|n| n.to_str()) != Some(a.name.as_str()) {
        return Err(format!("unexpected asset name {:?}", a.name));
    }
    let dir = private_download_dir(&rel.version)?;
    let path = dir.join(&a.name);
    fetch_and_verify(a, sums_url, &path, stage).inspect_err(|_| {
        let _ = std::fs::remove_dir_all(&dir);
    })?;
    Ok(path)
}

/// The download + checksum half of [`download_and_verify`], into `path`.
fn fetch_and_verify(a: &Asset, sums_url: &str, path: &Path, stage: &Mutex<Stage>) -> Result<(), String> {
    let mut child = command("curl")
        .args(["-fL", "--retry", "2", "--max-time", "900", "-o"])
        .arg(path)
        .arg(&a.url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("curl: {e}"))?;
    let status = loop {
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            break st;
        }
        let done = std::fs::metadata(path).map_or(0, |m| m.len());
        *stage.lock().unwrap() = Stage::Downloading { done, total: a.size };
        std::thread::sleep(std::time::Duration::from_millis(150));
    };
    if !status.success() {
        return Err(format!("download failed (curl exit code {})", status.code().unwrap_or(-1)));
    }
    let len = std::fs::metadata(path).map_or(0, |m| m.len());
    *stage.lock().unwrap() = Stage::Downloading { done: len, total: a.size };
    if a.size > 0 && len != a.size {
        return Err(format!("download incomplete ({len} of {} bytes)", a.size));
    }

    *stage.lock().unwrap() = Stage::Verifying;
    let sums =
        crate::os::cmd("curl", &["-fsSL", "--max-time", "30", sums_url]).ok_or("could not download SHA256SUMS.txt")?;
    let want = sum_for(&sums, &a.name).ok_or_else(|| format!("SHA256SUMS.txt has no entry for {}", a.name))?;
    let got = sha256_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !got.eq_ignore_ascii_case(&want) {
        return Err(format!("checksum mismatch for {} (expected {want}, got {got}); file deleted", a.name));
    }
    Ok(())
}

/// A fresh directory for one download: an unpredictable name, an exclusive
/// `create` (fails if the name is already taken) and, on Unix, mode 0700
/// inside a validated base directory (see [`base_is_safe`]) and verified to be
/// owned by the current user.
///
/// The verified installer is later opened *by path* by a privileged installer
/// (pkexec apt-get/pacman, msiexec, open), so its path must live where no
/// other local user can create files, write, or swap: the old predictable
/// shared `/tmp/cpuzeez-update-<version>` let anyone pre-create it world-
/// writable and rename the verified file during the polkit/UAC prompt,
/// installing a swapped package as root (CWE-377/59/426). On Windows the
/// default per-user %TEMP% ACL already provides this; on Unix the base is
/// validated so a non-sticky or user-owned `TMPDIR` (e.g. preserved through
/// sudo) cannot re-enable the directory-swap attack.
fn private_download_dir(version: &str) -> Result<PathBuf, String> {
    // The version is a network-provided tag: keep it to one path component
    // (no `/`, `\`, `..` tricks — those become `_`).
    let v: String = version
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect();
    let mut bases = vec![std::env::temp_dir()];
    #[cfg(unix)]
    bases.push(PathBuf::from("/tmp")); // fallback when TMPDIR is unusable
    #[cfg(unix)]
    bases.retain(|b| base_is_safe(b));
    let mut last = "no usable temporary directory".to_string();
    for base in &bases {
        for _ in 0..8 {
            let dir = base.join(format!("{}-update-{v}-{}", crate::NAME, random_token()));
            match create_exclusive_private(&dir) {
                Ok(()) => return Ok(dir),
                Err((e, retry)) if retry => last = e,
                Err((e, _)) => return Err(e),
            }
        }
    }
    Err(format!("could not create a private download directory: {last}"))
}

/// Create `dir` with mode 0700, failing if it (or a symlink at that name)
/// already exists. `Ok` means it exists, is ours, and is private; on failure
/// returns the message and whether another name might still succeed.
fn create_exclusive_private(dir: &Path) -> Result<(), (String, bool)> {
    let mut b = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o700);
    }
    b.create(dir).map_err(|e| (format!("{}: {e}", dir.display()), e.kind() == std::io::ErrorKind::AlreadyExists))?;
    #[cfg(unix)]
    {
        if let Err(e) = private_and_ours(dir) {
            // Deterministic for a given filesystem (NFS root-squash, FAT,
            // FUSE): retrying a different name cannot help.
            return Err((e, false));
        }
    }
    Ok(())
}

/// `dir` must be owned by the effective user and give nobody else access;
/// otherwise remove it and say why. Complements the exclusive create: only
/// proceed with a directory nobody else made, owns, or can reach into.
#[cfg(unix)]
fn private_and_ours(dir: &Path) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let md = std::fs::symlink_metadata(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // SAFETY: geteuid takes no arguments, has no preconditions and cannot fail.
    let euid = unsafe { libc::geteuid() };
    let mode = md.permissions().mode();
    if md.uid() != euid || mode & 0o077 != 0 {
        let _ = std::fs::remove_dir_all(dir);
        return Err(format!(
            "{} is owned by uid {} with mode {:o}; need uid {euid} (us) and mode 0700",
            dir.display(),
            md.uid(),
            mode & 0o777
        ));
    }
    Ok(())
}

/// A base directory our private subdirectory cannot be renamed out of or
/// replaced in: a real directory (not a symlink) whose owner is root or us,
/// and which is not writable by others unless it is sticky like /tmp — in a
/// sticky directory only the entry's owner, the directory's owner or root can
/// rename or unlink entries, so nobody can swap our subdirectory.
#[cfg(unix)]
fn base_is_safe(p: &Path) -> bool {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let Ok(md) = std::fs::symlink_metadata(p) else { return false };
    if !md.is_dir() {
        return false;
    }
    // SAFETY: geteuid takes no arguments, has no preconditions and cannot fail.
    let euid = unsafe { libc::geteuid() };
    let mode = md.permissions().mode();
    (md.uid() == 0 || md.uid() == euid) && (mode & 0o022 == 0 || mode & 0o1000 != 0)
}

/// An unpredictable hex token for the download directory name: OS entropy
/// (`/dev/urandom` where present) mixed with the time, the pid, an address
/// (ASLR) and `RandomState`'s OS-seeded keys.
fn random_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let mut buf = [0u8; 16];
        if f.read_exact(&mut buf).is_ok() {
            h.write(&buf);
        }
    }
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    h.write(&t.to_ne_bytes());
    h.write(&std::process::id().to_ne_bytes());
    let probe = 0u8;
    h.write(&(std::ptr::addr_of!(probe) as usize).to_ne_bytes());
    format!("{:016x}", h.finish())
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
        // apt's sandbox user `_apt` cannot enter our 0700 download dir, so apt
        // reads the verified file as root itself and may print a harmless
        // "Download is performed unsandboxed" notice. Keep the mode strict.
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
    sha256_of(&mut std::fs::File::open(path)?)
}

/// SHA-256 of everything `f` yields, as lowercase hex.
pub fn sha256_of(f: &mut impl std::io::Read) -> std::io::Result<String> {
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
    fn startup_check_is_opt_in() {
        assert!(!startup_check_allowed(true, None), "not asked yet: no request");
        assert!(!startup_check_allowed(true, Some(false)));
        assert!(!startup_check_allowed(false, Some(true)), "CheckUpdates=0 wins");
        assert!(startup_check_allowed(true, Some(true)));
    }

    /// P1: this module is the app's only network path. A socket, an HTTP
    /// client or a `curl` call anywhere else fails this test.
    #[test]
    fn update_is_the_only_network_path() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        walk(&root.join("src"), &mut files);
        walk(&root.join("crates"), &mut files);
        assert!(files.len() > 50, "found the sources");
        let net =
            ["TcpStream", "UdpSocket", "TcpListener", "\"curl\"", "reqwest", "ureq::", "hyper::", "ToSocketAddrs"];
        for f in files.iter().filter(|f| !f.ends_with("update.rs")) {
            let src = std::fs::read_to_string(f).unwrap_or_default();
            for n in net {
                assert!(!src.contains(n), "{} uses {n}: only update.rs may reach the network", f.display());
            }
        }
    }

    #[test]
    fn prerelease_order() {
        // The 1.0 release chain: each step must be offered over the previous one.
        let chain =
            ["0.3.0", "1.0.0-beta.1", "1.0.0-beta.2", "1.0.0-beta.10", "1.0.0-rc.1", "1.0.0-rc.2", "1.0.0", "1.0.1"];
        for w in chain.windows(2) {
            assert!(newer(w[0], w[1]), "{} -> {}", w[0], w[1]);
            assert!(!newer(w[1], w[0]), "{} -> {} must not be an update", w[1], w[0]);
        }
        assert!(!newer("1.0.0-beta.2", "1.0.0-beta.2"));
        // SemVer §11 details: numeric < alphanumeric, a longer identifier set wins, build metadata is ignored.
        assert!(newer("1.0.0-1", "1.0.0-alpha"));
        assert!(newer("1.0.0-beta", "1.0.0-beta.1"));
        assert!(!newer("1.0.0+abc", "1.0.0+def"));
    }

    fn listing() -> &'static str {
        r#"[
        {"tag_name":"v1.1.0-alpha.1","draft":false,"prerelease":true,"html_url":"u-a1","assets":[]},
        {"tag_name":"v1.0.0-beta.8","draft":false,"prerelease":true,"html_url":"u-b8","assets":[]},
        {"tag_name":"v1.0.0-rc.1","draft":false,"prerelease":true,"html_url":"u-rc1","assets":[]},
        {"tag_name":"v0.9.0","draft":false,"prerelease":false,"html_url":"u-090","assets":[]},
        {"tag_name":"v9.9.9","draft":true,"prerelease":false,"html_url":"u-draft","assets":[]}
        ]"#
    }

    #[test]
    fn channels_by_label() {
        assert_eq!(Channel::of("1.0.0"), Channel::Stable);
        assert_eq!(Channel::of("v1.0.0+build"), Channel::Stable);
        assert_eq!(Channel::of("1.0.0-beta.7"), Channel::Beta);
        assert_eq!(Channel::of("1.0.0-rc.1"), Channel::Beta);
        assert_eq!(Channel::of("1.1.0-alpha.2"), Channel::Alpha);
        assert!(
            Channel::Beta.sees("1.0.0") && Channel::Beta.sees("1.0.0-rc.1") && !Channel::Beta.sees("1.1.0-alpha.1")
        );
        assert!(Channel::Alpha.sees("1.1.0-alpha.1") && Channel::Alpha.sees("0.9.0"));
        assert!(Channel::Stable.sees("0.9.0") && !Channel::Stable.sees("1.0.0-rc.1"));
        assert_eq!(Channel::from_arg("BETA"), Some(Channel::Beta));
        assert_eq!(Channel::from_arg("nightly"), None);
    }

    /// Each channel offers the newest release it may see; drafts never.
    #[test]
    fn each_channel_gets_its_newest() {
        let v = |c| newest(listing(), c).map(|r| r.version);
        assert_eq!(v(Channel::Stable).as_deref(), Some("0.9.0"));
        assert_eq!(v(Channel::Beta).as_deref(), Some("1.0.0-rc.1"));
        assert_eq!(v(Channel::Alpha).as_deref(), Some("1.1.0-alpha.1"));
        assert!(newest("[]", Channel::Alpha).is_none());
        assert!(newest("not json", Channel::Alpha).is_none());
        // The order of the listing does not matter.
        let mut l: Vec<serde_json::Value> = serde_json::from_str(listing()).unwrap();
        l.reverse();
        let rev = serde_json::to_string(&l).unwrap();
        assert_eq!(newest(&rev, Channel::Beta).unwrap().version, "1.0.0-rc.1");
    }

    /// A beta build follows betas, a release follows releases, unless chosen.
    #[test]
    fn the_channel_follows_the_build_unless_chosen() {
        let mut s = crate::settings::Settings::default();
        assert_eq!(s.channel(), Channel::of(crate::VERSION));
        s.update_channel = Some(Channel::Alpha);
        assert_eq!(s.channel(), Channel::Alpha);
        let back: crate::settings::Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.update_channel, Some(Channel::Alpha));
        let old: crate::settings::Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(old.update_channel, None);
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

    /// The download dir must be freshly created, private to us, and never
    /// reuse a predictable name another local user could pre-create or swap
    /// files in (the verified installer is handed to a privileged installer).
    #[test]
    fn download_dirs_are_fresh_private_and_unique() {
        let (a, b) = (private_download_dir("9.9.9").unwrap(), private_download_dir("9.9.9").unwrap());
        #[cfg(unix)]
        let private = {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            std::fs::symlink_metadata(&a).is_ok_and(|md| {
                // SAFETY: geteuid takes no arguments, has no preconditions and cannot fail.
                md.is_dir() && md.uid() == unsafe { libc::geteuid() } && md.permissions().mode() & 0o777 == 0o700
            })
        };
        let (existed, distinct) = (a.is_dir(), a != b);
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
        assert!(existed);
        assert!(distinct, "each download must get its own unpredictable directory");
        #[cfg(unix)]
        assert!(private, "the download directory must be 0700 and owned by us");
    }

    /// A network-provided version (a git tag) must never pick a path: slashes
    /// and separators collapse into one plain directory component.
    #[test]
    fn hostile_versions_stay_in_one_component() {
        let d = private_download_dir("1.0/../evil").unwrap();
        let name = d.file_name().unwrap().to_str().unwrap().to_string();
        let existed = d.is_dir();
        let _ = std::fs::remove_dir_all(&d);
        assert!(existed);
        assert!(
            name.strip_prefix("cpuzeez-update-1.0_.._evil-").is_some(),
            "version must be sanitized into one path component: {name:?}"
        );
        assert!(!name.contains(['/', '\\']));
    }

    /// Exclusive creation: a squatted name is refused, never written into.
    #[test]
    fn refuses_a_pre_existing_directory() {
        let squat = std::env::temp_dir().join(format!("cpuzeez-test-squat-{}", random_token()));
        std::fs::create_dir(&squat).unwrap();
        let r = create_exclusive_private(&squat);
        let _ = std::fs::remove_dir(&squat);
        assert!(r.is_err(), "a pre-existing name must be rejected, never reused");
    }

    /// The post-create check rejects anything not 0700-and-ours, and cleans up.
    #[cfg(unix)]
    #[test]
    fn private_dir_check_rejects_shared_permissions() {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let d = std::env::temp_dir().join(format!("cpuzeez-test-shared-{}", random_token()));
        std::fs::DirBuilder::new().mode(0o777).create(&d).unwrap();
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o777)).unwrap();
        let r = private_and_ours(&d);
        let gone = !d.exists();
        assert!(r.is_err(), "a 0777 directory must never be accepted");
        assert!(gone, "a rejected directory must be removed");
    }

    /// Base validation: writable by others only with the sticky bit, and the
    /// default temp dir must pass on every platform we test.
    #[cfg(unix)]
    #[test]
    fn base_validation_requires_sticky_when_shared() {
        use std::os::unix::fs::PermissionsExt;
        let mk = |mode: u32| {
            let d = std::env::temp_dir().join(format!("cpuzeez-test-base-{mode:o}-{}", random_token()));
            std::fs::create_dir(&d).unwrap();
            std::fs::set_permissions(&d, std::fs::Permissions::from_mode(mode)).unwrap();
            d
        };
        let shared = mk(0o777); // world-writable, not sticky: others can rename our dir
        let sticky = mk(0o1777); // world-writable but sticky (like /tmp)
        let closed = mk(0o700); // not writable by others at all
        let results =
            (base_is_safe(&shared), base_is_safe(&sticky), base_is_safe(&closed), base_is_safe(&std::env::temp_dir()));
        for d in [shared, sticky, closed] {
            let _ = std::fs::remove_dir(&d);
        }
        let (bad, sticky_ok, closed_ok, default_ok) = results;
        assert!(!bad, "0777 without sticky lets others replace our directory");
        assert!(sticky_ok, "a sticky base like /tmp must be accepted");
        assert!(closed_ok, "a private base must be accepted");
        assert!(default_ok, "the default temp dir must validate on this platform");
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
        let rel = latest(Channel::Alpha).expect("GitHub reachable");
        let p = download_and_verify(&rel, &Mutex::new(Stage::Verifying)).unwrap();
        assert!(p.is_file());
        eprintln!("verified {}", p.display());
    }
}

//! Private temporary directories (contributed in joaoregis/cpuzeez#4 and #5).
//! Shared by the updater's download and the "Restart as root" handoff marker:
//! both hand a path to a more privileged process, so the path must be one that
//! no other local user can create, write into, or swap.

use std::path::{Path, PathBuf};

/// A fresh directory `<base>/<prefix>-<random>`: an unpredictable name, an
/// exclusive `create` (fails if the name is already taken) and, on Unix, mode
/// 0700 inside a validated base directory (see [`base_is_safe`]) and verified
/// to be owned by the current user.
///
/// What goes in it is later used *by path* with more privilege than ours (the
/// updater's package by pkexec apt-get/pacman or msiexec; the handoff marker by
/// the root copy), so it must live where no other local user can create files,
/// write, or swap: a predictable shared `/tmp/<name>` let anyone pre-create it
/// world-writable or plant a symlink there (CWE-377/59/426). On Windows the
/// default per-user %TEMP% ACL already provides this; on Unix the base is
/// validated so a non-sticky or user-owned `TMPDIR` (e.g. preserved through
/// sudo) cannot re-enable the directory-swap attack.
pub fn create(prefix: &str) -> Result<PathBuf, String> {
    // The prefix may carry a network-provided version tag: keep it to one path
    // component (no `/`, `\`, `..` tricks — those become `_`).
    let v: String = prefix
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect();
    // The /tmp fallback, push and retain below are Unix-only.
    #[allow(unused_mut)]
    let mut bases = vec![std::env::temp_dir()];
    #[cfg(unix)]
    bases.push(PathBuf::from("/tmp")); // fallback when TMPDIR is unusable
    #[cfg(unix)]
    bases.retain(|b| base_is_safe(b));
    let mut last = "no usable temporary directory".to_string();
    'base: for base in &bases {
        for _ in 0..8 {
            let dir = base.join(format!("{v}-{}", token()));
            match create_exclusive_private(&dir) {
                Ok(()) => return Ok(dir),
                // Name taken: try another name in the same base.
                Err((e, true)) => last = e,
                // Anything else (EACCES on a validated-but-unwritable base
                // such as TMPDIR=/root, read-only FS, deterministic
                // ownership/mode failure): another *name* cannot help, but
                // the next *base* might — e.g. the /tmp fallback.
                Err((e, false)) => {
                    last = e;
                    continue 'base;
                }
            }
        }
    }
    Err(format!("could not create a private temporary directory: {last}"))
}

/// Create `dir` with mode 0700, failing if it (or a symlink at that name)
/// already exists. `Ok` means it exists, is ours, and is private; on failure
/// returns the message and whether another name might still succeed.
fn create_exclusive_private(dir: &Path) -> Result<(), (String, bool)> {
    // mode() is Unix-only; create() takes &self.
    #[allow(unused_mut)]
    let mut b = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o700);
    }
    b.create(dir).map_err(|e| (format!("{}: {e}", dir.display()), e.kind() == std::io::ErrorKind::AlreadyExists))?;
    #[cfg(unix)]
    {
        // The umask masks DirBuilder::mode (0o277 would leave 0500: private, but
        // nothing could be written into it). Pin exactly 0700, then verify.
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
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
/// replaced in: every directory on the way to it — the base, each ancestor,
/// and each directory a symlink passes through — must be owned by root or us
/// and not writable by others unless sticky like /tmp: in a sticky directory
/// only the entry's owner, the directory's owner or root can rename or unlink
/// entries, so nobody can swap our subdirectory. Symlink components (macOS
/// `/tmp` → `/private/tmp`, `/var` → `/private/var`) are followed one
/// component at a time: a link must itself be root-or-ours and sit in a safe
/// directory, so nobody can swap the link to redirect the path. A base inside
/// a directory someone else can rename in (e.g. `TMPDIR=/srv/team/tmp` with a
/// team-writable `/srv/team`) is rejected: it can be swapped wholesale during
/// the polkit prompt — and a listable 0755 base makes the random directory
/// name observable, so the replacement can even carry the same name.
#[cfg(unix)]
pub fn base_is_safe(p: &Path) -> bool {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    // SAFETY: geteuid takes no arguments, has no preconditions and cannot fail.
    let euid = unsafe { libc::geteuid() };
    let ours = |md: &std::fs::Metadata| md.uid() == 0 || md.uid() == euid;
    let dir_ok = |md: &std::fs::Metadata| {
        let mode = md.permissions().mode();
        md.is_dir() && ours(md) && (mode & 0o022 == 0 || mode & 0o1000 != 0)
    };
    if !p.is_absolute() || !std::fs::symlink_metadata("/").is_ok_and(|md| dir_ok(&md)) {
        return false; // a relative TMPDIR is not a path we can validate; `/` gates `/tmp` itself
    }
    // Walk `p` one component at a time from the root; hitting a symlink
    // continues the walk at its target (with the rest of the path appended),
    // so a link chained through an unsafe directory still meets every check.
    let mut queue = vec![p.to_path_buf()];
    let mut links = 64u32; // symlink budget: a cycle must not hang the check
    while let Some(path) = queue.pop() {
        let comps: Vec<_> = path.components().collect();
        let mut cur = PathBuf::from("/");
        for i in 0..comps.len() {
            let c = match &comps[i] {
                std::path::Component::RootDir => continue,
                // `.`/`..`/prefix components have no place in a temp base.
                std::path::Component::Normal(c) => c,
                _ => return false,
            };
            let next = cur.join(c); // the entry lives in `cur`, already validated
            let Ok(md) = std::fs::symlink_metadata(&next) else { return false };
            if md.is_symlink() {
                if links == 0 || !ours(&md) {
                    return false;
                }
                links -= 1;
                let Ok(t) = std::fs::read_link(&next) else { return false };
                let t = if t.is_absolute() { t } else { cur.join(t) };
                let rest: PathBuf = comps[i + 1..].iter().map(|c| c.as_os_str()).collect();
                queue.push(t.join(rest));
                break;
            }
            if !dir_ok(&md) {
                return false;
            }
            cur = next;
        }
    }
    true
}

/// An unpredictable hex token for the download directory name: OS entropy
/// (`/dev/urandom` where present) mixed with the time, the pid, an address
/// (ASLR) and `RandomState`'s OS-seeded keys.
pub fn token() -> String {
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

/// Remove our own `<prefix>*` directories older than `older_than` from the
/// temporary bases: the installer folders msiexec and `open` still needed when
/// we returned, and handoff folders a crash left behind. Only directories we
/// own are touched (never a symlink, never someone else's entry).
pub fn sweep(prefix: &str, older_than: std::time::Duration) {
    #[allow(unused_mut)]
    let mut bases = vec![std::env::temp_dir()];
    #[cfg(unix)]
    bases.push(PathBuf::from("/tmp"));
    bases.dedup();
    let now = std::time::SystemTime::now();
    for base in bases {
        let Ok(rd) = std::fs::read_dir(&base) else { continue };
        for e in rd.flatten() {
            if !e.file_name().to_string_lossy().starts_with(prefix) {
                continue;
            }
            let p = e.path();
            let Ok(md) = std::fs::symlink_metadata(&p) else { continue };
            if !md.is_dir() {
                continue;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                // SAFETY: geteuid takes no arguments, has no preconditions and cannot fail.
                if md.uid() != unsafe { libc::geteuid() } {
                    continue;
                }
            }
            let old = md.modified().ok().and_then(|m| now.duration_since(m).ok()).is_some_and(|age| age >= older_than);
            if old {
                let _ = std::fs::remove_dir_all(&p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exclusive creation: a squatted name is refused, never written into.
    #[test]
    fn refuses_a_pre_existing_directory() {
        let squat = std::env::temp_dir().join(format!("cz-test-squat-{}", token()));
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
        let d = std::env::temp_dir().join(format!("cpuzeez-test-shared-{}", token()));
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
            let d = std::env::temp_dir().join(format!("cpuzeez-test-base-{mode:o}-{}", token()));
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

    /// A safe base inside a swappable directory must be rejected: renaming a
    /// non-sticky writable ancestor during the polkit prompt swaps the whole
    /// path, and a listable 0755 base even exposes the random name to replay.
    #[cfg(unix)]
    #[test]
    fn base_validation_rejects_swappable_ancestors() {
        use std::os::unix::fs::PermissionsExt;
        let parent = std::env::temp_dir().join(format!("cpuzeez-test-swap-{}", token()));
        std::fs::create_dir(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o777)).unwrap(); // others may rename entries in it
        let inside = parent.join("tmp");
        std::fs::create_dir(&inside).unwrap();
        std::fs::set_permissions(&inside, std::fs::Permissions::from_mode(0o700)).unwrap();
        let (inside_ok, default_ok) = (base_is_safe(&inside), base_is_safe(&std::env::temp_dir()));
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
        let _ = std::fs::remove_dir_all(&parent);
        assert!(!inside_ok, "a base inside a directory others can rename in must be rejected");
        assert!(default_ok, "the default temp dir must still validate");
    }

    /// macOS reaches the default temp dir through symlinks (`/var` →
    /// `/private/var`, and `/tmp` → `/private/tmp` for the fallback base): a
    /// safe base reached through such links must be accepted — and the links
    /// themselves validated (they can only be swapped through a safe parent).
    #[cfg(unix)]
    #[test]
    fn base_validation_follows_symlinks_in_the_path() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("cpuzeez-test-link-{}", token()));
        std::fs::create_dir(&root).unwrap();
        let target = root.join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink("target", root.join("link")).unwrap();
        let ok = base_is_safe(&root.join("link"));
        let _ = std::fs::remove_dir_all(&root);
        assert!(ok, "a symlink to a safe base, in a safe directory, must be accepted");
    }

    /// A symlink in a directory others can rename in must be rejected even
    /// though it points at a safe target: swapping the link during the polkit
    /// prompt redirects the whole path.
    #[cfg(unix)]
    #[test]
    fn base_validation_rejects_swappable_symlinks() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("cpuzeez-test-swaplink-{}", token()));
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(root.join("target")).unwrap();
        std::os::unix::fs::symlink("target", root.join("link")).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o777)).unwrap(); // others may rename `link`
        let ok = base_is_safe(&root.join("link"));
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let _ = std::fs::remove_dir_all(&root);
        assert!(!ok, "a symlink others can replace must not be trusted");
    }

    /// Old directories of ours with the prefix go; other names, files and
    /// symlinks stay.
    #[test]
    fn sweep_removes_only_our_old_prefixed_directories() {
        let tag = token();
        let base = std::env::temp_dir();
        let ours = base.join(format!("cz-sweep-{tag}-a"));
        let other = base.join(format!("cz-keep-{tag}"));
        std::fs::create_dir(&ours).unwrap();
        std::fs::create_dir(&other).unwrap();
        let file = base.join(format!("cz-sweep-{tag}-file"));
        std::fs::write(&file, b"x").unwrap();
        sweep(&format!("cz-sweep-{tag}"), std::time::Duration::ZERO);
        let (ours_gone, other_kept, file_kept) = (!ours.exists(), other.exists(), file.exists());
        let _ = std::fs::remove_dir_all(&other);
        let _ = std::fs::remove_file(&file);
        assert!(ours_gone, "an old directory of ours with the prefix is removed");
        assert!(other_kept, "other names are left alone");
        assert!(file_kept, "files are left alone");
        // Nothing is younger than a day: a long threshold keeps everything.
        let fresh = create(&format!("cz-sweep-{tag}-fresh")).unwrap();
        sweep(&format!("cz-sweep-{tag}"), std::time::Duration::from_secs(86_400));
        let kept = fresh.exists();
        let _ = std::fs::remove_dir_all(&fresh);
        assert!(kept, "a recent directory is kept");
    }

    /// A restrictive umask must not leave an unwritable (0500) directory.
    #[cfg(unix)]
    #[test]
    fn the_mode_is_pinned_to_0700() {
        use std::os::unix::fs::PermissionsExt;
        let d = create("cz-test-mode").unwrap();
        let mode = std::fs::symlink_metadata(&d).unwrap().permissions().mode() & 0o777;
        let writable = std::fs::write(d.join("probe"), b"x").is_ok();
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(mode, 0o700);
        assert!(writable);
    }
}

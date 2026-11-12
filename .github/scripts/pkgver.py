"""Map a release tag's SemVer to the version string each package format accepts.

Not every format takes `1.0.0-beta.2`:
  - MSI (and macOS bundle/pkg versions) need numeric `major.minor.build`, and
    MSI ignores a 4th field for upgrades. Pre-releases of X.0.0 map to
    (X-1).99.N (beta.N -> N, rc.N -> 50+N) so 0.3.0 -> beta -> rc -> 1.0.0
    upgrades in order. Pre-releases of X.Y.0 (Y >= 1) map to
    X.(Y-1).(60000+N): above every patch of X.(Y-1), below X.Y.0, and within
    MSI's 65535 for the third field.
  - Debian sorts `~` before everything: 1.0.0~beta.2 < 1.0.0.
  - pacman sorts letters before the release: 1.0.0beta2 < 1.0.0rc1 < 1.0.0.

Usage:
  python pkgver.py 1.0.0-beta.2   # prints KEY=VALUE lines for $GITHUB_ENV
  python pkgver.py --test
"""
import re
import sys

# beta.N -> N, rc.N -> 50+N, snapshot.N -> 100+N (alpha < beta < rc < snapshot < release)
PRE_BASE = {"beta": 0, "rc": 50, "snapshot": 100}


def versions(ver):
    m = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:-(beta|rc|snapshot)\.(\d+))?", ver)
    if not m:
        raise ValueError(f"unsupported version {ver!r}: use X.Y.Z, X.Y.Z-beta.N, X.Y.Z-rc.N or X.Y.Z-snapshot.N")
    major, minor, patch, kind, n = m.groups()
    if not kind:
        return {"VER": ver, "MSI_VER": ver, "MAC_VER": ver, "DEB_VER": ver, "ARCH_VER": ver, "PRERELEASE": "false"}
    n = int(n)
    if patch != "0" or (major == "0" and minor == "0"):
        raise ValueError(f"pre-releases are only supported for X.Y.0 (not X.Y.Z, not 0.0.0), got {ver!r}")
    if not 1 <= n <= 49:
        raise ValueError(f"pre-release number must be 1..49, got {n}")
    if minor == "0":
        if major == "0":
            raise ValueError(f"pre-releases are only supported for X.Y.0 with X >= 1, got {ver!r}")
        numeric = f"{int(major) - 1}.99.{PRE_BASE[kind] + n}"
    else:
        numeric = f"{major}.{int(minor) - 1}.{60000 + PRE_BASE[kind] + n}"
    return {
        "VER": ver,
        "MSI_VER": numeric,
        "MAC_VER": numeric,
        "DEB_VER": f"{major}.{minor}.{patch}~{kind}.{n}",
        "ARCH_VER": f"{major}.{minor}.{patch}{kind}{n}",
        "PRERELEASE": "true",
    }


def _test():
    def key(v):  # numeric version tuple, as MSI/macOS compare them
        return tuple(int(p) for p in v.split("."))

    order = ["0.3.0", "1.0.0-beta.1", "1.0.0-beta.2", "1.0.0-beta.9", "1.0.0-beta.10", "1.0.0-rc.1", "1.0.0-rc.2",
             "1.0.0-snapshot.1", "1.0.0-snapshot.2", "1.0.0", "1.0.1", "1.0.2", "1.0.99", "1.1.0-beta.1",
             "1.1.0-beta.2", "1.1.0-rc.1", "1.1.0", "1.1.1", "1.2.0-beta.1", "1.2.0"]
    msi = [key(versions(v)["MSI_VER"]) for v in order]
    assert msi == sorted(msi) and len(set(msi)) == len(msi), msi
    assert versions("1.0.0-beta.2")["DEB_VER"] == "1.0.0~beta.2"
    assert versions("1.0.0-rc.1")["ARCH_VER"] == "1.0.0rc1"
    assert versions("1.0.0-snapshot.1")["DEB_VER"] == "1.0.0~snapshot.1"
    assert versions("1.0.0-snapshot.1")["ARCH_VER"] == "1.0.0snapshot1"
    assert versions("1.0.0-snapshot.1")["PRERELEASE"] == "true"
    assert versions("1.0.0")["PRERELEASE"] == "false" and versions("1.0.0-rc.3")["PRERELEASE"] == "true"
    assert versions("1.1.0-beta.1")["MSI_VER"] == "1.0.60001"
    assert versions("1.1.0-beta.1")["DEB_VER"] == "1.1.0~beta.1"
    assert versions("1.1.0-beta.1")["ARCH_VER"] == "1.1.0beta1"
    for bad in ["1.0", "1.0.0-alpha.1", "1.1.1-beta.1", "0.0.0-beta.1", "1.0.0-beta.50", "v1.0.0"]:
        try:
            versions(bad)
        except ValueError:
            continue
        raise AssertionError(f"accepted {bad}")
    print("pkgver: ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--test"]:
        _test()
    else:
        for k, v in versions(sys.argv[1]).items():
            print(f"{k}={v}")

"""Map a release tag's SemVer to the version string each package format accepts.

Not every format takes `1.0.0-beta.2`:
  - MSI (and macOS bundle/pkg versions) need numeric `major.minor.build`, and
    MSI ignores a 4th field for upgrades. Pre-releases of X.0.0 map to
    (X-1).99.N (beta.N -> N, rc.N -> 50+N) so 0.3.0 -> beta -> rc -> 1.0.0
    upgrades in order.
  - Debian sorts `~` before everything: 1.0.0~beta.2 < 1.0.0.
  - pacman sorts letters before the release: 1.0.0beta2 < 1.0.0rc1 < 1.0.0.

Usage:
  python pkgver.py 1.0.0-beta.2   # prints KEY=VALUE lines for $GITHUB_ENV
  python pkgver.py --test
"""
import re
import sys

PRE_BASE = {"beta": 0, "rc": 50}


def versions(ver):
    m = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:-(beta|rc)\.(\d+))?", ver)
    if not m:
        raise ValueError(f"unsupported version {ver!r}: use X.Y.Z, X.Y.Z-beta.N or X.Y.Z-rc.N")
    major, minor, patch, kind, n = m.groups()
    if not kind:
        return {"VER": ver, "MSI_VER": ver, "MAC_VER": ver, "DEB_VER": ver, "ARCH_VER": ver, "PRERELEASE": "false"}
    n = int(n)
    if (minor, patch) != ("0", "0") or major == "0":
        raise ValueError(f"pre-releases are only supported for major releases (X.0.0, X >= 1), got {ver!r}")
    if not 1 <= n <= 49:
        raise ValueError(f"pre-release number must be 1..49, got {n}")
    numeric = f"{int(major) - 1}.99.{PRE_BASE[kind] + n}"
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

    order = ["0.3.0", "1.0.0-beta.1", "1.0.0-beta.2", "1.0.0-beta.9", "1.0.0-rc.1", "1.0.0-rc.2", "1.0.0", "1.0.1"]
    msi = [key(versions(v)["MSI_VER"]) for v in order]
    assert msi == sorted(msi) and len(set(msi)) == len(msi), msi
    assert versions("1.0.0-beta.2")["DEB_VER"] == "1.0.0~beta.2"
    assert versions("1.0.0-rc.1")["ARCH_VER"] == "1.0.0rc1"
    assert versions("1.0.0")["PRERELEASE"] == "false" and versions("1.0.0-rc.3")["PRERELEASE"] == "true"
    for bad in ["1.0", "1.0.0-alpha.1", "1.1.0-beta.1", "0.4.0-beta.1", "1.0.0-beta.50", "v1.0.0"]:
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

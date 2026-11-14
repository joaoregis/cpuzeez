"""The README screenshot set for one OS (M5-02), from a `-screenshots` run.

Copies the safe pages only (never a process list, users, startup items,
services or app history: they show what runs on the machine) and fails when
a page the READMEs show for that OS is missing.

    python readme_shots.py out/screenshots out/readme/linux linux
    python readme_shots.py --selftest
"""
import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
UNSAFE = {"system-processes.png", "system-users.png", "system-startup.png", "system-services.png", "system-history.png"}


def wanted(folder, readmes):
    """Every docs/screenshots/<folder>/<page>.png the READMEs link."""
    pat = re.compile(r"docs/screenshots/" + re.escape(folder) + r"/([a-z0-9-]+\.png)")
    return sorted({m for text in readmes for m in pat.findall(text)})


def pick(found, folder, readmes):
    """(pages to copy, pages missing)."""
    safe = sorted(f for f in found if f not in UNSAFE)
    return safe, [p for p in wanted(folder, readmes) if p not in safe]


def selftest():
    readme = "![](docs/screenshots/linux/overview.png) ![](docs/screenshots/windows/bench.png) docs/screenshots/linux/sensors.png"
    assert wanted("linux", [readme]) == ["overview.png", "sensors.png"]
    copy, missing = pick(["overview.png", "system-processes.png", "sensors.png"], "linux", [readme])
    assert copy == ["overview.png", "sensors.png"] and missing == [], (copy, missing)
    assert pick(["overview.png"], "linux", [readme])[1] == ["sensors.png"], "a missing page is reported"
    assert UNSAFE.isdisjoint(pick(sorted(UNSAFE), "linux", [readme])[0]), "the unsafe pages are never copied"
    print("readme_shots selftest: ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--selftest"]:
        selftest()
        sys.exit(0)
    src, dst, folder = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
    readmes = [(ROOT / n).read_text(encoding="utf-8") for n in ("README.md", "README.pt-BR.md")]
    found = [p.name for p in src.glob("*.png")] if src.is_dir() else []
    copy, missing = pick(found, folder, readmes)
    dst.mkdir(parents=True, exist_ok=True)
    for name in copy:
        shutil.copy2(src / name, dst / name)
    print(f"{folder}: {len(copy)} pages kept, {len(found) - len(copy)} left out")
    if missing:
        sys.exit(f"missing for the README: {', '.join(missing)}")

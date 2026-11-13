"""The `unsafe` ratchet (D3): outside `crates/platform`, `unsafe` may only go down.

Counts the `unsafe` keyword per tracked .rs file (comments and string
literals ignored) and compares with .github/unsafe-allowlist.txt. Fails when
a file's count rises or a file not in the list appears. A lower count passes
but asks for `--update`, so the list only ever shrinks.

    python unsafe_ratchet.py            # check (CI and the gate)
    python unsafe_ratchet.py --update   # rewrite the list from the tree
    python unsafe_ratchet.py --selftest
"""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LIST = ROOT / ".github" / "unsafe-allowlist.txt"
HEADER = "# unsafe outside crates/platform, per file. Only goes down: python .github/scripts/unsafe_ratchet.py --update\n"
# ponytail: a regex strip, not a parser; a raw string holding `//` or `unsafe` would miscount. Parse with syn if it ever does.
NOISE = re.compile(r'/\*.*?\*/|"(?:\\.|[^"\\])*"|//[^\n]*', re.S)


def count(src):
    return len(re.findall(r"\bunsafe\b", NOISE.sub("", src)))


def tree():
    files = subprocess.run(["git", "ls-files", "*.rs"], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split()
    found = {f: count((ROOT / f).read_text(encoding="utf-8", errors="replace")) for f in files if not f.startswith("crates/platform/")}
    return {f: n for f, n in found.items() if n}


def allowed():
    rows = [l.split(None, 1) for l in LIST.read_text(encoding="utf-8").splitlines() if l.strip() and not l.startswith("#")]
    return {path.strip(): int(n) for n, path in rows}


def verdict(now, ok):
    """(problems, lowered): problems fail the check."""
    problems = [
        f"{f}: {n} unsafe, the list allows {ok.get(f, 0)}" + ("" if f in ok else " (a new file)")
        for f, n in sorted(now.items())
        if n > ok.get(f, 0)
    ]
    lowered = [f for f in ok if now.get(f, 0) < ok[f]]
    return problems, lowered


def selftest():
    assert count("unsafe { x() }\nunsafe impl Send for A {}") == 2
    assert count('// unsafe\n/* unsafe */ let s = "unsafe";\n#![forbid(unsafe_code)]') == 0
    assert count('let s = "a \\" unsafe"; unsafe { }') == 1, "an escaped quote stays inside the string"
    assert verdict({"a.rs": 2}, {"a.rs": 2}) == ([], [])
    assert verdict({"a.rs": 3}, {"a.rs": 2})[0], "a higher count must fail"
    assert "a new file" in verdict({"b.rs": 1}, {"a.rs": 2})[0][0], "a new file must fail"
    assert verdict({}, {"a.rs": 2}) == ([], ["a.rs"]), "a lower count passes and is reported"
    print("unsafe_ratchet selftest: ok")


if __name__ == "__main__":
    if "--selftest" in sys.argv:
        selftest()
    elif "--update" in sys.argv:
        now = tree()
        LIST.write_text(HEADER + "".join(f"{n} {f}\n" for f, n in sorted(now.items())), encoding="utf-8", newline="\n")
        print(f"{len(now)} files, {sum(now.values())} unsafe")
    else:
        now = tree()
        problems, lowered = verdict(now, allowed())
        for p in problems:
            print("FAIL", p)
        if lowered and not problems:
            print("lower than the list for:", ", ".join(lowered), "- run --update to keep the gain")
        sys.exit(1 if problems else 0)

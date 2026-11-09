"""Build every icon format from assets/icon.svg.

Renders with `resvg` when it is on PATH (`cargo install resvg`), and with the
built-in `raster.py` (standard library only, same geometry) when it is not.
Writes PNGs, a Windows .ico and a macOS .icns into assets/. Both containers
simply embed PNGs.

    python packaging/icons/make_icons.py
"""

import pathlib
import shutil
import struct
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
ASSETS = ROOT / "assets"
SVG = ASSETS / "icon.svg"
SIZES = [16, 24, 32, 48, 64, 128, 256, 512, 1024]


def render(size: int) -> bytes:
    out = ASSETS / f"icon-{size}.png"
    if not shutil.which("resvg"):
        sys.path.insert(0, str(pathlib.Path(__file__).parent))
        import raster

        out.write_bytes(raster.render_png(size))
        return out.read_bytes()
    subprocess.run(["resvg", "-w", str(size), "-h", str(size), str(SVG), str(out)], check=True)
    return out.read_bytes()


def ico(pngs: dict[int, bytes]) -> bytes:
    # ICONDIR + one ICONDIRENTRY per image; PNG payloads are valid since Vista.
    sizes = [s for s in (16, 24, 32, 48, 64, 128, 256) if s in pngs]
    head = struct.pack("<HHH", 0, 1, len(sizes))
    offset = 6 + 16 * len(sizes)
    entries, data = b"", b""
    for s in sizes:
        png = pngs[s]
        dim = 0 if s >= 256 else s  # 0 means 256
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(png), offset + len(data))
        data += png
    return head + entries + data


def icns(pngs: dict[int, bytes]) -> bytes:
    # OSType -> pixel size of the embedded PNG.
    types = [("icp4", 16), ("icp5", 32), ("icp6", 64), ("ic07", 128), ("ic08", 256), ("ic09", 512),
             ("ic10", 1024), ("ic11", 32), ("ic12", 64), ("ic13", 256), ("ic14", 512)]
    body = b"".join(t.encode() + struct.pack(">I", 8 + len(pngs[s])) + pngs[s] for t, s in types)
    return b"icns" + struct.pack(">I", 8 + len(body)) + body


def main() -> None:
    pngs = {s: render(s) for s in SIZES}
    (ASSETS / "cpuzeez.ico").write_bytes(ico(pngs))
    (ASSETS / "cpuzeez.icns").write_bytes(icns(pngs))
    # Keep only the PNG sizes the app and Linux packages use.
    for s in SIZES:
        if s not in (32, 48, 64, 128, 256, 512):
            (ASSETS / f"icon-{s}.png").unlink()
    print("wrote", sorted(p.name for p in ASSETS.iterdir()))


if __name__ == "__main__":
    main()

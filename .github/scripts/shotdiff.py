"""Compare two `cpuzeez -screenshots=DIR` sets pixel by pixel (stdlib only).

Screens show live values (clocks, temperatures, timers), so a run never matches
another exactly. What a code change must not do is move or resize things: that
shows up as a different image size or as differences spread over a large part
of the screen. Each pair is reported with the share of differing pixels and the
bounding box of the differences; a pair fails above --max-diff (default 3%).

usage: python shotdiff.py BEFORE_DIR AFTER_DIR [--max-diff 0.03]
       python shotdiff.py --test
"""
import os
import struct
import sys
import zlib


def read_png(path):
    """8-bit RGB/RGBA, non-interlaced PNG -> (width, height, channels, rows)."""
    data = open(path, "rb").read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    pos, idat, ihdr = 8, b"", None
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos : pos + 8])
        body = data[pos + 8 : pos + 8 + n]
        if kind == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + n
    w, h, depth, color, _, _, interlace = ihdr
    if depth != 8 or interlace or color not in (2, 6):
        raise ValueError(f"{path}: only 8-bit non-interlaced RGB/RGBA is supported")
    ch = 4 if color == 6 else 3
    raw, stride, rows, prev = zlib.decompress(idat), w * ch, [], bytearray(w * ch)
    for y in range(h):
        f, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)])
        for x in range(stride):
            a = line[x - ch] if x >= ch else 0
            b = prev[x]
            c = prev[x - ch] if x >= ch else 0
            if f == 1:
                line[x] = (line[x] + a) & 0xFF
            elif f == 2:
                line[x] = (line[x] + b) & 0xFF
            elif f == 3:
                line[x] = (line[x] + ((a + b) >> 1)) & 0xFF
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 0xFF
        rows.append(bytes(line))
        prev = line
    return w, h, ch, rows


def diff(a, b, tol=24):
    """(share of differing pixels, bbox) for two decoded images of equal size."""
    w, h, ch, ra = a
    _, _, _, rb = b
    n, box = 0, None
    for y in range(h):
        if ra[y] == rb[y]:
            continue
        for x in range(w):
            i = x * ch
            if max(abs(ra[y][i + k] - rb[y][i + k]) for k in range(3)) > tol:
                n += 1
                box = (x, y, x, y) if box is None else (min(box[0], x), min(box[1], y), max(box[2], x), max(box[3], y))
    return n / (w * h), box


def compare(before, after, max_diff):
    names = sorted(f for f in os.listdir(before) if f.endswith(".png"))
    missing = sorted(set(names) ^ {f for f in os.listdir(after) if f.endswith(".png")})
    failed = bool(missing)
    for m in missing:
        print(f"FAIL {m}: present in only one set")
    for name in names:
        if name in missing:
            continue
        a, b = read_png(os.path.join(before, name)), read_png(os.path.join(after, name))
        if a[:2] != b[:2]:
            print(f"FAIL {name}: size {a[0]}x{a[1]} -> {b[0]}x{b[1]}")
            failed = True
            continue
        share, box = diff(a, b)
        bad = share > max_diff
        failed |= bad
        print(f"{'FAIL' if bad else 'ok  '} {name}: {share:.2%} of pixels differ" + (f", within {box}" if box else ""))
    return failed


def _test():
    import tempfile

    def png(path, pixels, w, h):
        raw = b"".join(b"\x00" + bytes(pixels[y * w * 4 : (y + 1) * w * 4]) for y in range(h))
        chunk = lambda k, d: struct.pack(">I", len(d)) + k + d + struct.pack(">I", zlib.crc32(k + d) & 0xFFFFFFFF)
        open(path, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))

    d = tempfile.mkdtemp()
    px = [10, 20, 30, 255] * 16
    png(os.path.join(d, "a.png"), px, 4, 4)
    changed = list(px)
    changed[4 * 5 : 4 * 5 + 3] = [200, 200, 200]  # one pixel of 16
    png(os.path.join(d, "b.png"), changed, 4, 4)
    a, b = read_png(os.path.join(d, "a.png")), read_png(os.path.join(d, "b.png"))
    share, box = diff(a, b)
    assert abs(share - 1 / 16) < 1e-9 and box == (1, 1, 1, 1), (share, box)
    assert diff(a, a) == (0.0, None)
    print("shotdiff: ok")


if __name__ == "__main__":
    if sys.argv[1:] == ["--test"]:
        _test()
    else:
        limit = float(sys.argv[sys.argv.index("--max-diff") + 1]) if "--max-diff" in sys.argv else 0.03
        sys.exit(1 if compare(sys.argv[1], sys.argv[2], limit) else 0)

"""The app icon, drawn with the standard library only (no resvg, no Pillow).

`make_icons.py` uses `resvg` when it is installed and this when it is not. The
geometry is the one in `assets/icon.svg` (a 180-unit design scaled to the
canvas): a dark tile, an open gauge arc, an area chart as a shadow, and a Z
whose bars are white and whose diagonal is violet, cut flush to the bars.
`python packaging/icons/raster.py 256 out.png` renders one size.
"""

import math
import struct
import sys
import zlib

# --- the design, in 180 x 180 units -----------------------------------------

BG = (0x14, 0x10, 0x2A)
ARC = (0x8B, 0x5C, 0xF6)
AREA = (0x7C, 0x3A, 0xED)
LINE = (0xA7, 0x8B, 0xFA)
DIAG = (0xA7, 0x8B, 0xFA)
BAR = (0xF5, 0xF3, 0xFF)


def bezier(p0, p1, p2, p3, steps=32):
    out = []
    for i in range(1, steps + 1):
        t = i / steps
        u = 1 - t
        out.append((u**3 * p0[0] + 3 * u * u * t * p1[0] + 3 * u * t * t * p2[0] + t**3 * p3[0],
                    u**3 * p0[1] + 3 * u * u * t * p1[1] + 3 * u * t * t * p2[1] + t**3 * p3[1]))
    return out


# The chart's top edge: M54 121 C62 115 70 117 78 119 S92 107 100 111 S116 105 126 109
EDGE = [(54, 121)]
EDGE += bezier((54, 121), (62, 115), (70, 117), (78, 119))
EDGE += bezier((78, 119), (86, 121), (92, 107), (100, 111))
EDGE += bezier((100, 111), (108, 115), (116, 105), (126, 109))
AREA_POLY = [(54, 132)] + EDGE + [(126, 132)]
DIAG_POLY = [(103.09, 59), (117.09, 59), (76.91, 85), (62.91, 85)]
CLIP = (66, 50, 114, 94)  # the Z's box: the diagonal is cut to it
ARC_C, ARC_R = (90.0, 90.0), 66.0
ARC_START = math.radians(135)  # clockwise (screen) from here, 270 degrees


def in_rrect(x, y, x0, y0, x1, y1, r):
    if not (x0 <= x <= x1 and y0 <= y <= y1):
        return False
    cx = min(max(x, x0 + r), x1 - r)
    cy = min(max(y, y0 + r), y1 - r)
    return (x - cx) ** 2 + (y - cy) ** 2 <= r * r


def in_poly(x, y, pts):
    inside = False
    j = len(pts) - 1
    for i in range(len(pts)):
        xi, yi = pts[i]
        xj, yj = pts[j]
        if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi:
            inside = not inside
        j = i
    return inside


def dist_seg(x, y, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = ((x - a[0]) * dx + (y - a[1]) * dy) / (dx * dx + dy * dy)
    t = max(0.0, min(1.0, t))
    return math.hypot(x - (a[0] + t * dx), y - (a[1] + t * dy))


def on_arc(x, y, half_width):
    d = math.hypot(x - ARC_C[0], y - ARC_C[1])
    ang = (math.atan2(y - ARC_C[1], x - ARC_C[0]) - ARC_START) % (2 * math.pi)
    if ang <= math.radians(270) and abs(d - ARC_R) <= half_width:
        return True
    # Round caps at both ends.
    for a in (ARC_START, ARC_START + math.radians(270)):
        ex = ARC_C[0] + ARC_R * math.cos(a)
        ey = ARC_C[1] + ARC_R * math.sin(a)
        if math.hypot(x - ex, y - ey) <= half_width:
            return True
    return False


def sample(x, y):
    """The colour at one point of the 180-unit canvas: (r, g, b, a) with a in 0..1."""
    if not in_rrect(x, y, 0, 0, 180, 180, 40):
        return (0, 0, 0, 0.0)
    c = list(BG)

    def over(col, alpha):
        for i in range(3):
            c[i] = c[i] * (1 - alpha) + col[i] * alpha

    if 17 <= x <= 163 and 17 <= y <= 143 and on_arc(x, y, 4.5):
        over(ARC, 1.0)
    near_chart = 52 <= x <= 128 and 103 <= y <= 133
    if near_chart and in_poly(x, y, AREA_POLY):
        over(AREA, 0.5)
    if near_chart and any(dist_seg(x, y, EDGE[i], EDGE[i + 1]) <= 1.5 for i in range(len(EDGE) - 1)):
        over(LINE, 1.0)
    if CLIP[0] <= x <= CLIP[2] and CLIP[1] <= y <= CLIP[3] and in_poly(x, y, DIAG_POLY):
        over(DIAG, 1.0)
    if in_rrect(x, y, 66, 50, 114, 61, 2) or in_rrect(x, y, 66, 83, 114, 94, 2):
        over(BAR, 1.0)
    return (c[0], c[1], c[2], 1.0)


def render(size, ss=None):
    """RGBA bytes of the icon at `size` pixels, with `ss` x `ss` samples per pixel."""
    ss = ss or (8 if size <= 32 else 4 if size <= 128 else 3 if size <= 512 else 2)
    k = 180.0 / size
    rows = []
    for py in range(size):
        row = bytearray()
        for px in range(size):
            r = g = b = a = 0.0
            for sy in range(ss):
                for sx in range(ss):
                    cr, cg, cb, ca = sample((px + (sx + 0.5) / ss) * k, (py + (sy + 0.5) / ss) * k)
                    r += cr * ca
                    g += cg * ca
                    b += cb * ca
                    a += ca
            n = ss * ss
            if a == 0:
                row += b"\0\0\0\0"
            else:
                row += bytes((round(r / a), round(g / a), round(b / a), round(255 * a / n)))
        rows.append(bytes(row))
    return size, rows


def png(size, rows):
    raw = b"".join(b"\0" + r for r in rows)

    def chunk(tag, data):
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def render_png(size):
    return png(*render(size))


if __name__ == "__main__":
    s = int(sys.argv[1]) if len(sys.argv) > 1 else 256
    out = sys.argv[2] if len(sys.argv) > 2 else f"icon-{s}.png"
    with open(out, "wb") as f:
        f.write(render_png(s))
    print("wrote", out)

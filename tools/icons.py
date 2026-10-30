#!/usr/bin/env python3
"""Draws the WOL•E Link icon at every size and writes:

  assets/icon/wole-link-<size>.png   16, 32, 48, 64, 128, 256 (for looking at them)
  assets/icon/wole-link.ico          the Windows icon, embedded in the .exe
  assets/icon/wm-icon.bin            the X11 _NET_WM_ICON property (32-bit words, little-endian)

The mark is the WOL•E app's launcher icon (res/drawable/ic_launcher_foreground.xml): the red
dot and the first bar unchanged, the second bar ending in a QR finder pattern. From 48 px up it
is drawn from the app's own curves; 16 and 32 px are laid out on the pixel grid so the QR frame,
its gap and its centre stay apart.

Standard library only: `python3 tools/icons.py` from the repository root.
"""

import struct
import zlib

RED = (0xF9, 0x3E, 0x55)  # the app's dot; never recolored
WHITE = (0xFF, 0xFF, 0xFF)
BLACK = (0x00, 0x00, 0x00)

# --- The app's geometry (108-unit viewport of ic_launcher_foreground.xml, before its group
# translate). ---
DOT = (57.989449, 62.337734, 5.33737, 5.004139)  # cx, cy, rx, ry
BAR_1 = [
    ("M", (75.787626, 45.274354)),
    ("C", (73.713295, 45.274354), (71.808296, 46.586686), (71.130963, 48.576351)),
    ("C", (68.125299, 57.170011), (60.420638, 79.437328), (60.420638, 79.437328)),
    ("L", (66.770633, 79.437328)),
    ("C", (68.844965, 79.437328), (70.665297, 78.167329), (71.34263, 76.177664)),
    ("C", (74.348294, 67.66867), (82.137622, 45.274354), (82.137622, 45.274354)),
]
BAR_2 = [
    ("M", (88.635671, 45.274354)),
    ("C", (86.561339, 45.274354), (84.656341, 46.586686), (83.979008, 48.576351)),
    ("C", (80.973343, 57.170011), (73.268682, 79.437328), (73.268682, 79.437328)),
    ("L", (79.618678, 79.437328)),
    ("C", (81.693009, 79.437328), (83.513341, 78.167329), (84.190674, 76.177664)),
    ("C", (87.196339, 67.66867), (94.985666, 45.274354), (94.985666, 45.274354)),
]
TOP, BASE = 45.274354, 79.437328
# The QR finder pattern ("Q2"): 7 modules, frame 1.25 modules, centre 3, bottom on the baseline.
QR_SIZE, QR_X = 19.0, 78.0
MODULE = QR_SIZE / 7
FRAME = 1.25 * MODULE
QR_TOP = BASE - QR_SIZE


def flatten(path, steps=24):
    points = []
    for segment in path:
        if segment[0] in ("M", "L"):
            points.append(segment[1])
        else:
            p0 = points[-1]
            (p1, p2, p3) = segment[1:]
            for i in range(1, steps + 1):
                t = i / steps
                u = 1 - t
                points.append(tuple(
                    u**3 * a + 3 * u * u * t * b + 3 * u * t * t * c + t**3 * d
                    for a, b, c, d in zip(p0, p1, p2, p3)))
    return points


def vector_shapes():
    """Shapes in app units, painted in order. The second bar stops inside the frame's top."""
    bar_2 = [(x, y) for (x, y) in flatten(BAR_2)]
    cut = QR_TOP + FRAME * 0.9
    return [
        ("ellipse", DOT, RED),
        ("poly", flatten(BAR_1), WHITE),
        ("poly_above", (bar_2, cut), WHITE),
        ("rect", (QR_X, QR_TOP, QR_X + QR_SIZE, BASE), WHITE),
        ("rect", (QR_X + FRAME, QR_TOP + FRAME, QR_X + QR_SIZE - FRAME, BASE - FRAME), BLACK),
        ("rect", (QR_X + 2 * MODULE, QR_TOP + 2 * MODULE, QR_X + 5 * MODULE, BASE - 2 * MODULE), WHITE),
    ]


def pixel_shapes(u):
    """The 16-px layout (u = 1), doubled for 32 px (u = 2). Units are pixels."""
    s = lambda pts: [(x * u, y * u) for x, y in pts]
    return [
        ("ellipse", (2.6 * u, 7.9 * u, 1.5 * u, 1.5 * u), RED),
        ("poly", s([(7.0, 3), (9.0, 3), (5.6, 13), (3.6, 13)]), WHITE),
        ("poly", s([(10.6, 3), (12.6, 3), (11.24, 7), (9.24, 7)]), WHITE),
        # 1 px frame, 1 px gap, 2 px centre (doubled at 32 px).
        ("rect", (9 * u, 7 * u, 15 * u, 13 * u), WHITE),
        ("rect", (10 * u, 8 * u, 14 * u, 12 * u), BLACK),
        ("rect", (11 * u, 9 * u, 13 * u, 11 * u), WHITE),
    ]


def inside_poly(x, y, pts):
    inside = False
    for (x1, y1), (x2, y2) in zip(pts, pts[1:] + pts[:1]):
        if (y1 > y) != (y2 > y) and x < x1 + (y - y1) * (x2 - x1) / (y2 - y1):
            inside = not inside
    return inside


def hit(shape, x, y):
    kind, geo, _ = shape
    if kind == "ellipse":
        cx, cy, rx, ry = geo
        return ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1
    if kind == "rect":
        return geo[0] <= x < geo[2] and geo[1] <= y < geo[3]
    if kind == "poly":
        return inside_poly(x, y, geo)
    if kind == "poly_above":
        pts, cut = geo
        return y < cut and inside_poly(x, y, pts)
    raise ValueError(kind)


def render(n, shapes, to_shape, corner, ss):
    """RGBA rows: a black rounded square with the shapes, `ss`×`ss` samples per pixel.
    `to_shape` maps a pixel position to the shapes' units."""
    rows = []
    for py in range(n):
        row = []
        for px in range(n):
            total, covered = [0, 0, 0], 0
            for sy in range(ss):
                for sx in range(ss):
                    x, y = px + (sx + 0.5) / ss, py + (sy + 0.5) / ss
                    cx, cy = min(max(x, corner), n - corner), min(max(y, corner), n - corner)
                    if (x - cx) ** 2 + (y - cy) ** 2 > corner**2:
                        continue
                    color = BLACK
                    ux, uy = to_shape(x, y)
                    for shape in shapes:
                        if hit(shape, ux, uy):
                            color = shape[2]
                    total = [t + c for t, c in zip(total, color)]
                    covered += 1
            rgb = tuple(round(t / covered) for t in total) if covered else (0, 0, 0)
            row.append(rgb + (round(255 * covered / ss / ss),))
        rows.append(row)
    return rows


def vector_icon(n):
    left = DOT[0] - DOT[2]
    right = max(94.985666, QR_X + QR_SIZE)
    width, height = right - left, BASE - TOP
    k = n * 0.74 / width  # the mark fills 74 % of the width
    ox = (n - width * k) / 2 - left * k
    oy = (n - height * k) / 2 - TOP * k
    return render(n, vector_shapes(), lambda x, y: ((x - ox) / k, (y - oy) / k), n * 52 / 256,
                  4 if n > 64 else 8)


def pixel_icon(n):
    u = n // 16
    return render(n, pixel_shapes(u), lambda x, y: (x, y), 3 * u, 8)


def png(rows):
    n = len(rows)
    raw = b"".join(b"\0" + bytes(v for p in row for v in p) for row in rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    header = struct.pack(">IIBBBBB", n, n, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9))
            + chunk(b"IEND", b""))


def ico(images):
    """PNG-compressed entries (Windows Vista and later)."""
    out = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    data = b""
    for n, blob in images:
        out += struct.pack("<BBBBHHII", n % 256, n % 256, 0, 0, 1, 32, len(blob), offset + len(data))
        data += blob
    return out + data


def wm_icon(icons):
    """_NET_WM_ICON: width, height, then ARGB pixels, for each size."""
    words = []
    for rows in icons:
        words += [len(rows), len(rows)]
        words += [(a << 24) | (r << 16) | (g << 8) | b for row in rows for (r, g, b, a) in row]
    return struct.pack(f"<{len(words)}I", *words)


def main():
    sizes = {16: pixel_icon(16), 32: pixel_icon(32)}
    for n in (48, 64, 128, 256):
        sizes[n] = vector_icon(n)
    pngs = {n: png(rows) for n, rows in sizes.items()}
    for n, blob in pngs.items():
        open(f"assets/icon/wole-link-{n}.png", "wb").write(blob)
    open("assets/icon/wole-link.ico", "wb").write(ico(sorted(pngs.items())))
    # Window managers pick the closest size; 128 and 256 would only make the property heavy.
    open("assets/icon/wm-icon.bin", "wb").write(wm_icon([sizes[n] for n in (16, 32, 48, 64)]))


if __name__ == "__main__":
    main()

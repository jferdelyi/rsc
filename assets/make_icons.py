"""RSC's logo, from one geometry: rsc.svg, rsc-256.png (the window's icon) and rsc.ico (the Windows
executable's). A connector drawn by hand, a little crooked, and a signal on it, in one of three
styles:

- "hand": the connector filled with a gradient, its rim, on a dark tile;
- "neon": the connector's outline in the gradient, glowing, the signal in white, on a deep tile;
- "flat": the connector's outline and the signal in white, on a tile in the gradient.

    python make_icons.py                  the style of STYLE, into this folder
    python make_icons.py --preview <dir>  every style into <dir>, to compare

Pillow is needed.
"""

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

HERE = Path(__file__).resolve().parent
STYLE = "flat"
# Drawn at this many times the size of the SVG (512), for smooth edges once reduced.
SCALE = 4

# The connector's face, a little crooked on purpose: four cubic curves, each (control, control, end).
FACE_START = (104, 168)
FACE_CURVES = [
    ((200, 140), (330, 140), (414, 170)),
    ((404, 230), (378, 290), (342, 340)),
    ((290, 362), (210, 368), (162, 350)),
    ((140, 300), (116, 240), (104, 168)),
]
WAVE_POINTS = [(150, 271), (204, 271), (204, 209), (259, 209), (259, 271), (312, 271), (312, 209), (366, 209)]
SIGNAL = [(0.0, "#FF9F1C"), (0.5, "#E0457B"), (1.0, "#8B2CFF")]

STYLES = {
    "hand": {
        "tile": [(0.0, "#313131"), (1.0, "#232323")], "tile_diagonal": False,
        "shrink": 1.0, "wave_shrink": 1.0, "rim": 64, "outline": None, "fill_face": True,
        "wave": "#640D02", "wave_width": 23, "glow": False, "shadow": True,
    },
    "neon": {
        "tile": [(0.0, "#1B1830"), (1.0, "#0E0D18")], "tile_diagonal": True,
        "shrink": 0.86, "wave_shrink": 0.7, "rim": None, "outline": 26, "fill_face": False,
        "wave": "#FFFFFF", "wave_width": 24, "glow": True, "shadow": False,
    },
    "flat": {
        "tile": SIGNAL, "tile_diagonal": True,
        "shrink": 0.86, "wave_shrink": 0.7, "rim": None, "outline": 26, "fill_face": False,
        "wave": "#FFFFFF", "wave_width": 24, "glow": False, "shadow": True,
    },
}
CENTER = (258, 256)


def rgb(color):
    return tuple(int(color[i:i + 2], 16) for i in (1, 3, 5)) + (255,)


def mix(stops, t):
    t = min(max(t, 0.0), 1.0)
    for (a, ca), (b, cb) in zip(stops, stops[1:]):
        if a <= t <= b:
            k = (t - a) / (b - a)
            ca, cb = rgb(ca), rgb(cb)
            return tuple(round(ca[i] + (cb[i] - ca[i]) * k) for i in range(4))
    return rgb(stops[-1][1])


def shrink(point, k):
    return (CENTER[0] + (point[0] - CENTER[0]) * k, CENTER[1] + (point[1] - CENTER[1]) * k)


def cubic(start, c1, c2, end, steps=40):
    points = []
    for i in range(1, steps + 1):
        t = i / steps
        u = 1 - t
        x = u ** 3 * start[0] + 3 * u * u * t * c1[0] + 3 * u * t * t * c2[0] + t ** 3 * end[0]
        y = u ** 3 * start[1] + 3 * u * u * t * c1[1] + 3 * u * t * t * c2[1] + t ** 3 * end[1]
        points.append((x, y))
    return points


def geometry(look):
    """The face and the signal, brought towards the centre: an outline needs room inside it."""
    k = look["shrink"]
    start = shrink(FACE_START, k)
    curves = [tuple(shrink(p, k) for p in curve) for curve in FACE_CURVES]
    wave = [shrink(p, look["wave_shrink"]) for p in WAVE_POINTS]
    return start, curves, wave


def face_points(start, curves):
    points, at = [start], start
    for c1, c2, end in curves:
        points += cubic(at, c1, c2, end)
        at = end
    return points[:-1]


def fmt(point):
    return f"{point[0]:.1f} {point[1]:.1f}"


def svg(style):
    look = STYLES[style]
    start, curves, wave = geometry(look)
    face = f"M{fmt(start)} " + " ".join(f"C{fmt(c1)} {fmt(c2)} {fmt(e)}" for c1, c2, e in curves) + " Z"
    wave_path = f"M{fmt(wave[0])} " + " ".join(f"L{fmt(p)}" for p in wave[1:])
    stops = lambda stops: "".join(f'<stop offset="{o}" stop-color="{c}"/>' for o, c in stops)
    tile_axis = 'x1="0" y1="0" x2="1" y2="1"' if look["tile_diagonal"] else 'x1="0" y1="0" x2="1" y2="0"'
    parts = [f'<rect x="16" y="16" width="480" height="480" rx="112" fill="url(#tile)"/>']
    if look["shadow"]:
        width = look["rim"] or look["outline"]
        fill = "#000" if look["fill_face"] else "none"
        parts.append(f'<path d="{face}" transform="translate(5 10)" fill="{fill}" fill-opacity="0.3" stroke="#000" stroke-opacity="0.3" stroke-width="{width}" stroke-linejoin="round" filter="url(#blur)"/>')
    if look["glow"]:
        parts.append(f'<path d="{face}" fill="none" stroke="url(#signal)" stroke-width="{look["outline"] + 10}" stroke-linejoin="round" opacity="0.75" filter="url(#glow)"/>')
    if look["rim"]:
        parts.append(f'<path d="{face}" fill="#3D3D3D" stroke="#3D3D3D" stroke-width="{look["rim"]}" stroke-linejoin="round"/>')
    if look["fill_face"]:
        parts.append(f'<path d="{face}" fill="url(#signal)" stroke="url(#signal)" stroke-width="12" stroke-linejoin="round"/>')
    else:
        stroke = "url(#signal)" if style == "neon" else "#FFFFFF"
        parts.append(f'<path d="{face}" fill="none" stroke="{stroke}" stroke-width="{look["outline"]}" stroke-linejoin="round"/>')
    parts.append(f'<path d="{wave_path}" fill="none" stroke="{look["wave"]}" stroke-width="{look["wave_width"]}" stroke-linecap="round" stroke-linejoin="round"/>')
    body = "\n  ".join(parts)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <title>RSC</title>
  <defs>
    <linearGradient id="tile" {tile_axis}>{stops(look["tile"])}</linearGradient>
    <linearGradient id="signal" gradientUnits="userSpaceOnUse" x1="104" y1="0" x2="414" y2="0">{stops(SIGNAL)}</linearGradient>
    <filter id="blur" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="9"/></filter>
    <filter id="glow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="14"/></filter>
  </defs>
  {body}
</svg>
"""


def s(points):
    return [(x * SCALE, y * SCALE) for x, y in points]


def outline_mask(points, width, filled):
    """A closed shape, filled or not, stroked with round joins, as a mask."""
    size = 512 * SCALE
    layer = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(layer)
    points = s(points)
    if filled:
        draw.polygon(points, fill=255)
    draw.line(points + [points[0]], fill=255, width=round(width * SCALE), joint="curve")
    r = width * SCALE / 2
    for x, y in points:
        draw.ellipse([x - r, y - r, x + r, y + r], fill=255)
    return layer


def gradient(stops, start, end, diagonal):
    """A gradient over the whole picture: left to right from `start` to `end`, or along the diagonal."""
    size = 512 * SCALE
    image = Image.new("RGBA", (size, size))
    draw = ImageDraw.Draw(image)
    if diagonal:
        for k in range(2 * size):
            draw.line([(k, 0), (0, k)], fill=mix(stops, (k / 2 / SCALE - start) / (end - start)), width=3)
    else:
        for x in range(size):
            draw.line([(x, 0), (x, size)], fill=mix(stops, (x / SCALE - start) / (end - start)))
    return image


def png(style):
    look = STYLES[style]
    size = 512 * SCALE
    start, curves, wave = geometry(look)
    face = face_points(start, curves)
    tile = Image.new("L", (size, size), 0)
    ImageDraw.Draw(tile).rounded_rectangle([16 * SCALE, 16 * SCALE, 496 * SCALE, 496 * SCALE], radius=112 * SCALE, fill=255)
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    image.paste(gradient(look["tile"], 16, 496, look["tile_diagonal"]), (0, 0), tile)
    signal = gradient(SIGNAL, 104 * look["shrink"] + CENTER[0] * (1 - look["shrink"]), 414 * look["shrink"] + CENTER[0] * (1 - look["shrink"]), False)
    black = Image.new("RGBA", (size, size), (0, 0, 0, 255))
    if look["shadow"]:
        width = look["rim"] or look["outline"]
        shadow = outline_mask(face, width, look["fill_face"]).filter(ImageFilter.GaussianBlur(9 * SCALE)).point(lambda v: v * 0.3)
        image.paste(black, (5 * SCALE, 10 * SCALE), shadow)
    if look["glow"]:
        glow = outline_mask(face, look["outline"] + 10, False).filter(ImageFilter.GaussianBlur(14 * SCALE)).point(lambda v: v * 0.75)
        image.paste(signal, (0, 0), glow)
    if look["rim"]:
        image.paste(Image.new("RGBA", (size, size), rgb("#3D3D3D")), (0, 0), outline_mask(face, look["rim"], True))
    if look["fill_face"]:
        image.paste(signal, (0, 0), outline_mask(face, 12, True))
    else:
        color = signal if style == "neon" else Image.new("RGBA", (size, size), rgb("#FFFFFF"))
        image.paste(color, (0, 0), outline_mask(face, look["outline"], False))
    draw = ImageDraw.Draw(image)
    points = s(wave)
    width = look["wave_width"] * SCALE
    draw.line(points, fill=rgb(look["wave"]), width=width, joint="curve")
    for x, y in (points[0], points[-1]):
        draw.ellipse([x - width / 2, y - width / 2, x + width / 2, y + width / 2], fill=rgb(look["wave"]))
    # The tile's own edge, sharp again after the shadow and the glow.
    image.putalpha(Image.composite(image.getchannel("A"), Image.new("L", (size, size), 0), tile))
    return image


def write(style, folder, name):
    (folder / f"{name}.svg").write_text(svg(style), encoding="utf-8")
    big = png(style)
    big.resize((256, 256), Image.LANCZOS).save(folder / f"{name}-256.png", optimize=True)
    return big


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--preview":
        folder = Path(sys.argv[2])
        folder.mkdir(parents=True, exist_ok=True)
        for style in STYLES:
            write(style, folder, f"rsc-{style}")
        print("styles:", ", ".join(STYLES), "->", folder)
        return
    big = write(STYLE, HERE, "rsc")
    sizes = [16, 24, 32, 48, 64, 128, 256]
    frames = [big.resize((n, n), Image.LANCZOS) for n in sizes]
    frames[-1].save(HERE / "rsc.ico", sizes=[(n, n) for n in sizes], append_images=frames[:-1])
    print(f"{STYLE}: rsc.svg, rsc-256.png, rsc.ico:", ", ".join(map(str, sizes)))


if __name__ == "__main__":
    main()

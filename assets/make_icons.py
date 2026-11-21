"""RSC's logo: rsc.svg, rsc-256.png (the window's icon) and rsc.ico (the Windows executable's). A
connector drawn by hand, a little crooked, its outline and a signal on it in white, on a tile in the
signal's gradient, with a soft shadow.

    python make_icons.py

Pillow is needed.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

HERE = Path(__file__).resolve().parent
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
# The tile's gradient, along its diagonal.
SIGNAL = [(0.0, "#FF9F1C"), (0.5, "#E0457B"), (1.0, "#8B2CFF")]
# The face and the signal are brought towards the centre: the outline needs room inside the tile.
CENTER = (258, 256)
FACE_SHRINK = 0.86
WAVE_SHRINK = 0.7
OUTLINE = 26
WAVE_WIDTH = 24
WHITE = "#FFFFFF"


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


def geometry():
    """The face's start, its curves and the signal's points, brought towards the centre."""
    start = shrink(FACE_START, FACE_SHRINK)
    curves = [tuple(shrink(p, FACE_SHRINK) for p in curve) for curve in FACE_CURVES]
    wave = [shrink(p, WAVE_SHRINK) for p in WAVE_POINTS]
    return start, curves, wave


def face_points(start, curves):
    points, at = [start], start
    for c1, c2, end in curves:
        points += cubic(at, c1, c2, end)
        at = end
    return points[:-1]


def fmt(point):
    return f"{point[0]:.1f} {point[1]:.1f}"


def svg():
    start, curves, wave = geometry()
    face = f"M{fmt(start)} " + " ".join(f"C{fmt(c1)} {fmt(c2)} {fmt(e)}" for c1, c2, e in curves) + " Z"
    wave_path = f"M{fmt(wave[0])} " + " ".join(f"L{fmt(p)}" for p in wave[1:])
    stops = "".join(f'<stop offset="{o}" stop-color="{c}"/>' for o, c in SIGNAL)
    parts = [
        '<rect x="16" y="16" width="480" height="480" rx="112" fill="url(#tile)"/>',
        f'<path d="{face}" transform="translate(5 10)" fill="none" fill-opacity="0.3" stroke="#000" stroke-opacity="0.3" stroke-width="{OUTLINE}" stroke-linejoin="round" filter="url(#blur)"/>',
        f'<path d="{face}" fill="none" stroke="{WHITE}" stroke-width="{OUTLINE}" stroke-linejoin="round"/>',
        f'<path d="{wave_path}" fill="none" stroke="{WHITE}" stroke-width="{WAVE_WIDTH}" stroke-linecap="round" stroke-linejoin="round"/>',
    ]
    body = "\n  ".join(parts)
    # The glow filter is defined but not used: rsc.svg keeps the form it was first drawn in.
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <title>RSC</title>
  <defs>
    <linearGradient id="tile" x1="0" y1="0" x2="1" y2="1">{stops}</linearGradient>
    <linearGradient id="signal" gradientUnits="userSpaceOnUse" x1="104" y1="0" x2="414" y2="0">{stops}</linearGradient>
    <filter id="blur" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="9"/></filter>
    <filter id="glow" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="14"/></filter>
  </defs>
  {body}
</svg>
"""


def s(points):
    return [(x * SCALE, y * SCALE) for x, y in points]


def outline_mask(points, width):
    """A closed shape stroked with round joins, as a mask."""
    size = 512 * SCALE
    layer = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(layer)
    points = s(points)
    draw.line(points + [points[0]], fill=255, width=round(width * SCALE), joint="curve")
    r = width * SCALE / 2
    for x, y in points:
        draw.ellipse([x - r, y - r, x + r, y + r], fill=255)
    return layer


def gradient(stops, start, end):
    """A gradient over the whole picture, along its diagonal, from `start` to `end`."""
    size = 512 * SCALE
    image = Image.new("RGBA", (size, size))
    draw = ImageDraw.Draw(image)
    for k in range(2 * size):
        draw.line([(k, 0), (0, k)], fill=mix(stops, (k / 2 / SCALE - start) / (end - start)), width=3)
    return image


def png():
    size = 512 * SCALE
    start, curves, wave = geometry()
    face = face_points(start, curves)
    tile = Image.new("L", (size, size), 0)
    ImageDraw.Draw(tile).rounded_rectangle([16 * SCALE, 16 * SCALE, 496 * SCALE, 496 * SCALE], radius=112 * SCALE, fill=255)
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    image.paste(gradient(SIGNAL, 16, 496), (0, 0), tile)
    black = Image.new("RGBA", (size, size), (0, 0, 0, 255))
    shadow = outline_mask(face, OUTLINE).filter(ImageFilter.GaussianBlur(9 * SCALE)).point(lambda v: v * 0.3)
    image.paste(black, (5 * SCALE, 10 * SCALE), shadow)
    image.paste(Image.new("RGBA", (size, size), rgb(WHITE)), (0, 0), outline_mask(face, OUTLINE))
    draw = ImageDraw.Draw(image)
    points = s(wave)
    width = WAVE_WIDTH * SCALE
    draw.line(points, fill=rgb(WHITE), width=width, joint="curve")
    for x, y in (points[0], points[-1]):
        draw.ellipse([x - width / 2, y - width / 2, x + width / 2, y + width / 2], fill=rgb(WHITE))
    # The tile's own edge, sharp again after the shadow.
    image.putalpha(Image.composite(image.getchannel("A"), Image.new("L", (size, size), 0), tile))
    return image


def main(folder=HERE):
    (folder / "rsc.svg").write_text(svg(), encoding="utf-8")
    big = png()
    big.resize((256, 256), Image.LANCZOS).save(folder / "rsc-256.png", optimize=True)
    sizes = [16, 24, 32, 48, 64, 128, 256]
    frames = [big.resize((n, n), Image.LANCZOS) for n in sizes]
    frames[-1].save(folder / "rsc.ico", sizes=[(n, n) for n in sizes], append_images=frames[:-1])
    print("rsc.svg, rsc-256.png, rsc.ico:", ", ".join(map(str, sizes)))


if __name__ == "__main__":
    main()

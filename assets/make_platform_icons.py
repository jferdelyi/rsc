"""RSC's icons for Linux, taken from rsc.ico (which make_icons.py draws), with Python alone:

    python make_platform_icons.py

- linux/rsc-<size>.png, 16 to 256 pixels: the icons a .desktop file names (build.sh --install puts
  them in ~/.local/share/icons/hicolor/<size>x<size>/apps/rsc.png).

The icon of RSC.app under macOS, rsc.icns, is drawn by make_icons.py on Apple's grid.

Each image of rsc.ico is a PNG already: they are copied as they are, nothing is drawn again.
"""

import struct
from pathlib import Path

HERE = Path(__file__).parent
PNG = b"\x89PNG\r\n\x1a\n"


def ico_images(path):
    """The PNG images of an .ico file, by their size."""
    data = path.read_bytes()
    _, kind, count = struct.unpack("<HHH", data[:6])
    assert kind == 1, "not an icon"
    images = {}
    for index in range(count):
        width, _, _, _, _, _, size, offset = struct.unpack("<BBBBHHII", data[6 + 16 * index:22 + 16 * index])
        image = data[offset:offset + size]
        assert image.startswith(PNG), f"image {index} is not a PNG"
        images[width or 256] = image
    return images


def main():
    images = ico_images(HERE / "rsc.ico")
    folder = HERE / "linux"
    folder.mkdir(exist_ok=True)
    for size, image in sorted(images.items()):
        (folder / f"rsc-{size}.png").write_bytes(image)
    print(f"linux/rsc-{{{','.join(str(size) for size in sorted(images))}}}.png")


if __name__ == "__main__":
    main()

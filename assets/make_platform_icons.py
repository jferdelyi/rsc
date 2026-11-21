"""RSC's icon for Linux and macOS, taken from rsc.ico (which make_icons.py draws), with Python alone:

    python make_platform_icons.py

- linux/rsc-<size>.png, 16 to 256 pixels: the icons a .desktop file names (build.sh --install puts
  them in ~/.local/share/icons/hicolor/<size>x<size>/apps/rsc.png);
- rsc.icns: the icon of RSC.app under macOS, its images in PNG.

Each image of rsc.ico is a PNG already: they are copied as they are, nothing is drawn again.
"""

import struct
from pathlib import Path

HERE = Path(__file__).parent
PNG = b"\x89PNG\r\n\x1a\n"
# The types of an .icns image by its size; the @2x ones are the size above for a retina screen.
ICNS_TYPES = {16: [b"icp4"], 32: [b"icp5", b"ic11"], 64: [b"icp6", b"ic12"], 128: [b"ic07"], 256: [b"ic08", b"ic13"]}


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


def icns(images):
    """An .icns file holding the PNG images, by their size."""
    entries = b""
    for size, kinds in ICNS_TYPES.items():
        for kind in kinds:
            entries += kind + struct.pack(">I", 8 + len(images[size])) + images[size]
    return b"icns" + struct.pack(">I", 8 + len(entries)) + entries


def main():
    images = ico_images(HERE / "rsc.ico")
    folder = HERE / "linux"
    folder.mkdir(exist_ok=True)
    for size, image in sorted(images.items()):
        (folder / f"rsc-{size}.png").write_bytes(image)
    (HERE / "rsc.icns").write_bytes(icns(images))
    print(f"linux/rsc-{{{','.join(str(size) for size in sorted(images))}}}.png, rsc.icns")


if __name__ == "__main__":
    main()

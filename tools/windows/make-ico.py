#!/usr/bin/env python3
"""Pack PNG files into one Windows .ico (PNG-compressed entries, the form every
Windows since Vista reads) — no Pillow needed. Usage:

    make-ico.py OUTPUT.ico 16.png 32.png 48.png 256.png

Each PNG must be square and at most 256 px; the size is read from its header.
"""

import struct
import sys
from pathlib import Path


def png_size(data: bytes) -> int:
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise SystemExit("not a PNG")
    width, height = struct.unpack(">II", data[16:24])
    if width != height or width > 256 or width == 0:
        raise SystemExit(f"icon entries must be square and at most 256 px, got {width}x{height}")
    return width


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    output = Path(argv[1])
    images = sorted(
        ((png_size(data), data) for data in (Path(name).read_bytes() for name in argv[2:])),
        key=lambda entry: entry[0],
    )
    sizes = [size for size, _ in images]
    if len(set(sizes)) != len(sizes):
        raise SystemExit(f"duplicate icon sizes: {sizes}")
    header = struct.pack("<HHH", 0, 1, len(images))
    directory = b""
    offset = len(header) + 16 * len(images)
    for size, data in images:
        # 0 stands for 256 in the one-byte width/height fields.
        entry_size = 0 if size == 256 else size
        directory += struct.pack("<BBBBHHII", entry_size, entry_size, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    output.write_bytes(header + directory + b"".join(images))
    print(f"wrote {output} ({len(images)} entries)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))

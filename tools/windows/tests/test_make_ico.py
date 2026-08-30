"""Tests for the .ico packer. Run: python3 -m unittest discover tools/windows/tests"""

import struct
import subprocess
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

PACKER = Path(__file__).resolve().parents[1] / "make-ico.py"


def square_png(side: int) -> bytes:
    """A minimal opaque PNG, so the packer has something real to read a size from."""

    def chunk(kind: bytes, payload: bytes) -> bytes:
        return (
            struct.pack(">I", len(payload))
            + kind
            + payload
            + struct.pack(">I", zlib.crc32(kind + payload))
        )

    raw = b"".join(b"\x00" + b"\xff\xff\xff\xff" * side for _ in range(side))
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", side, side, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )


class MakeIcoTests(unittest.TestCase):
    def temporary_directory(self):
        # `addCleanup` rather than `enterContext`, which is 3.11+: the packer
        # itself only needs 3.9, and a test should not raise the floor.
        holder = tempfile.TemporaryDirectory()
        self.addCleanup(holder.cleanup)
        return Path(holder.name)

    def pack(self, sides):
        directory = self.temporary_directory()
        pages = []
        for side in sides:
            page = directory / f"{side}.png"
            page.write_bytes(square_png(side))
            pages.append(str(page))
        output = directory / "out.ico"
        result = subprocess.run(
            [sys.executable, str(PACKER), str(output), *pages],
            capture_output=True,
            text=True,
        )
        return result, output

    def test_the_packed_entries_hold_the_png_payloads_at_their_declared_offsets(self):
        # The regression this pins: the packer once joined the (size, data)
        # pairs it had sorted rather than the data, which is a TypeError at
        # best and a corrupt directory at worst.
        sides = [16, 48, 32]
        result, output = self.pack(sides)
        self.assertEqual(result.returncode, 0, result.stderr)

        blob = output.read_bytes()
        reserved, kind, count = struct.unpack_from("<HHH", blob, 0)
        self.assertEqual((reserved, kind), (0, 1))
        self.assertEqual(count, len(sides))

        for index, expected_side in enumerate(sorted(sides)):
            width, _, _, _, _, _, length, offset = struct.unpack_from(
                "<BBBBHHII", blob, 6 + 16 * index
            )
            self.assertEqual(width, expected_side, "entries are ordered by size")
            payload = blob[offset : offset + length]
            self.assertEqual(payload, square_png(expected_side))

    def test_a_256_px_entry_declares_itself_as_zero(self):
        # One byte holds the dimension, so 256 is written as 0 by convention.
        _, output = self.pack([256])
        width, height = struct.unpack_from("<BB", output.read_bytes(), 6)
        self.assertEqual((width, height), (0, 0))

    def test_duplicate_sizes_are_refused(self):
        directory = self.temporary_directory()
        first, second = directory / "a.png", directory / "b.png"
        first.write_bytes(square_png(32))
        second.write_bytes(square_png(32))
        result = subprocess.run(
            [sys.executable, str(PACKER), str(directory / "out.ico"), str(first), str(second)],
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("duplicate", result.stderr.lower())


if __name__ == "__main__":
    unittest.main()

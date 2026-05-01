#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 dictionary.csv 建立 association.bin

輸入：output/dictionary.csv
輸出：output/association.bin

Binary 格式（little-endian）：
  Header (20 bytes):
    magic:       4 bytes  "TKWA"
    version:     u32      1
    key_count:   u32      unique prev_word count
    entry_count: u32      total entry count
    build_ts:    u32      Unix timestamp (must match dictionary.bin)

  Key offset table (key_count × u32):
    Absolute byte offset from file start to each key entry

  Key section (sorted by prev_word UTF-8 bytes):
    Each key:
      prev_word_len: u8    UTF-8 byte count
      prev_word:     [u8]  UTF-8 bytes
      entry_offset:  u32   absolute byte offset to first entry
      entry_count:   u16   number of entries for this key

  Entry section (sorted by count DESC within each key group, NO tiebreaker —
  Codex pre-impl review Q6: adding a tiebreaker would drift the SHA256
  baseline against pre-refactor builds):
    Each entry:
      bitmask:       u16   9-bit source flags (kautian..khpoo)
      count:         u32   association count
      next_word_len: u8    UTF-8 byte count
      next_tl_len:   u8    UTF-8 byte count
      next_word:     [u8]  UTF-8 bytes
      next_tl:       [u8]  UTF-8 bytes

用法：
  python3 create_association_bin.py            # 建立 binary
  python3 create_association_bin.py --verify    # 建立並驗證
"""

import struct
import sys

from build.associations import AssociationEntry, compute_associations
from build.common import LOG_DIR, OUTPUT_DIR, read_shared_build_timestamp
from common.logging_utils import log_header, setup_logging
from common.source_bits import ASSOC_SOURCE_COLUMNS

CSV_FILE = OUTPUT_DIR / "dictionary.csv"
OUTPUT_FILE = OUTPUT_DIR / "association.bin"
SCRIPT_NAME = "create_association_bin"

MAGIC = b"TKWA"
VERSION = 1


def encode_assoc_bitmask(entry: AssociationEntry) -> int:
    """Encode association source flags into a u16 bitmask."""
    sources = entry.source_dict()
    mask = 0
    for bit, col in enumerate(ASSOC_SOURCE_COLUMNS):
        if sources[col]:
            mask |= 1 << bit
    return mask


def encode_entry(entry: AssociationEntry) -> bytes:
    """Encode a single association entry into binary bytes."""
    bitmask = encode_assoc_bitmask(entry)
    count = entry.count or 0

    next_word_bytes = entry.next_word.encode("utf-8")
    next_tl = entry.next_tl or ""
    next_tl_bytes = next_tl.encode("utf-8")

    return struct.pack(
        f"<HIBB{len(next_word_bytes)}s{len(next_tl_bytes)}s",
        bitmask,
        count,
        len(next_word_bytes),
        len(next_tl_bytes),
        next_word_bytes,
        next_tl_bytes,
    )


def build(logger):
    if not CSV_FILE.exists():
        logger.error(f"CSV not found: {CSV_FILE}")
        sys.exit(1)

    build_ts = read_shared_build_timestamp()
    logger.info(f"Build timestamp: {build_ts}")

    grouped = compute_associations(CSV_FILE)

    # Outer sort: prev_word UTF-8 bytes ascending. Inner sort already done
    # in compute_associations (count DESC stable).
    all_keys = sorted(grouped.keys(), key=lambda s: s.encode("utf-8"))

    total_entries = sum(len(grouped[k]) for k in all_keys)
    logger.info(f"Unique prev_words: {len(all_keys)}")
    logger.info(f"Total entries: {total_entries}")

    encoded_groups: dict[str, list[bytes]] = {}
    for key in all_keys:
        encoded_groups[key] = [encode_entry(e) for e in grouped[key]]

    key_count = len(all_keys)
    header_size = 20  # magic(4) + version(4) + key_count(4) + entry_count(4) + build_ts(4)
    key_offset_table_size = key_count * 4

    key_section_start = header_size + key_offset_table_size
    key_entries_data = []
    for key in all_keys:
        key_bytes = key.encode("utf-8")
        key_entries_data.append((key_bytes, len(encoded_groups[key])))

    key_section_size = sum(1 + len(kb) + 4 + 2 for kb, _ in key_entries_data)
    entry_section_start = key_section_start + key_section_size

    entry_offset = entry_section_start
    key_entry_offsets = []
    for key in all_keys:
        key_entry_offsets.append(entry_offset)
        for enc in encoded_groups[key]:
            entry_offset += len(enc)

    key_offsets = []
    pos = key_section_start
    for kb, _ in key_entries_data:
        key_offsets.append(pos)
        pos += 1 + len(kb) + 4 + 2

    with open(OUTPUT_FILE, "wb") as f:
        f.write(MAGIC)
        f.write(struct.pack("<IIII", VERSION, key_count, total_entries, build_ts))

        for offset in key_offsets:
            f.write(struct.pack("<I", offset))

        for i, (kb, entry_count) in enumerate(key_entries_data):
            f.write(struct.pack("B", len(kb)))
            f.write(kb)
            f.write(struct.pack("<IH", key_entry_offsets[i], entry_count))

        for key in all_keys:
            for enc in encoded_groups[key]:
                f.write(enc)

    file_size = OUTPUT_FILE.stat().st_size
    logger.info("\n  [output]")
    logger.info(f"    File:    {OUTPUT_FILE}")
    logger.info(f"    Size:    {file_size / 1024 / 1024:.2f} MB")
    logger.info(f"    Keys:    {key_count}")
    logger.info(f"    Entries: {total_entries}")

    return key_count, total_entries


def verify(logger):
    """Verify association.bin against compute_associations (round-trip check)."""
    logger.info(f"\n{'=' * 50}")
    logger.info("Verifying association.bin against dictionary.csv...")
    logger.info(f"{'=' * 50}")

    data = OUTPUT_FILE.read_bytes()

    magic = data[:4]
    assert magic == MAGIC, f"Bad magic: {magic}"
    version, key_count, entry_count, build_ts = struct.unpack_from("<IIII", data, 4)
    assert version == VERSION
    logger.info(
        f"  Header: version={version}, keys={key_count}, "
        f"entries={entry_count}, build_ts={build_ts}"
    )

    header_size = 20
    key_offsets = []
    for i in range(key_count):
        offset = struct.unpack_from("<I", data, header_size + i * 4)[0]
        key_offsets.append(offset)

    grouped = compute_associations(CSV_FILE)

    errors = 0
    verified_entries = 0

    for i in range(key_count):
        koff = key_offsets[i]
        prev_word_len = struct.unpack_from("B", data, koff)[0]
        prev_word = data[koff + 1: koff + 1 + prev_word_len].decode("utf-8")
        e_offset, e_count = struct.unpack_from("<IH", data, koff + 1 + prev_word_len)

        expected_entries = grouped.get(prev_word, [])

        if len(expected_entries) != e_count:
            logger.error(
                f"  Key '{prev_word}': count mismatch bin={e_count} csv={len(expected_entries)}"
            )
            errors += 1
            continue

        pos = e_offset
        for j, expected in enumerate(expected_entries):
            bitmask, count_val, nw_len, nt_len = struct.unpack_from("<HIBB", data, pos)
            pos += 8
            next_word = data[pos: pos + nw_len].decode("utf-8")
            pos += nw_len
            next_tl = data[pos: pos + nt_len].decode("utf-8")
            pos += nt_len

            expected_bitmask = encode_assoc_bitmask(expected)

            if next_word != expected.next_word:
                logger.error(f"  Key '{prev_word}' entry {j}: next_word mismatch")
                errors += 1
            if next_tl != (expected.next_tl or ""):
                logger.error(f"  Key '{prev_word}' entry {j}: next_tl mismatch")
                errors += 1
            if count_val != (expected.count or 0):
                logger.error(f"  Key '{prev_word}' entry {j}: count mismatch")
                errors += 1
            if bitmask != expected_bitmask:
                logger.error(f"  Key '{prev_word}' entry {j}: bitmask mismatch")
                errors += 1

            verified_entries += 1

    if errors == 0:
        logger.info(f"  Verified {key_count} keys, {verified_entries} entries — all match!")
    else:
        logger.error(f"  {errors} mismatches found!")
        sys.exit(1)


def main():
    logger = setup_logging(SCRIPT_NAME, log_dir=LOG_DIR)
    log_header(logger, SCRIPT_NAME, CSV_FILE, OUTPUT_FILE)

    build(logger)

    if "--verify" in sys.argv:
        verify(logger)

    logger.info("\nDone!")


if __name__ == "__main__":
    main()

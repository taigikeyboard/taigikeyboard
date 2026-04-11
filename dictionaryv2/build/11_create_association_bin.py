#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 dictionary.db 的 word_association 表建立 association.bin

輸入：output/dictionary.db
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

  Entry section (sorted by count DESC within each key group):
    Each entry:
      bitmask:       u16   9-bit source flags (kautian..khpoo)
      count:         u32   association count
      next_word_len: u8    UTF-8 byte count
      next_tl_len:   u8    UTF-8 byte count
      next_word:     [u8]  UTF-8 bytes
      next_tl:       [u8]  UTF-8 bytes

用法：
  python3 11_create_association_bin.py            # 建立 binary
  python3 11_create_association_bin.py --verify    # 建立並驗證
"""

import os
import sys
import struct
import sqlite3
import logging
import time
from datetime import datetime

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
BASE_DIR = os.path.dirname(SCRIPT_DIR)
OUTPUT_DIR = os.path.join(BASE_DIR, "output")
DB_FILE = os.path.join(OUTPUT_DIR, "dictionary.db")
OUTPUT_FILE = os.path.join(OUTPUT_DIR, "association.bin")
BUILD_TS_FILE = os.path.join(OUTPUT_DIR, ".build_ts")
LOG_DIR = os.path.join(BASE_DIR, "logs")
SCRIPT_NAME = "11_create_association_bin"

MAGIC = b"TKWA"
VERSION = 1

# Association source columns (9 flags, no is_variant/khiin/dev/lkk)
ASSOC_SOURCE_COLUMNS = [
    "kautian", "taigitv", "itaigi", "sitbut", "taihoa",
    "taijit", "kungge", "stti", "khpoo",
]


def setup_logging():
    os.makedirs(LOG_DIR, exist_ok=True)
    log_file = os.path.join(LOG_DIR, f"{SCRIPT_NAME}.log")
    logging.basicConfig(
        level=logging.INFO,
        format="%(message)s",
        handlers=[
            logging.FileHandler(log_file, mode="w", encoding="utf-8"),
            logging.StreamHandler(),
        ],
    )
    return logging.getLogger(__name__)


def get_build_timestamp():
    """Read shared build timestamp (must be created by 10_create_dictionary_bin.py first)."""
    if os.path.exists(BUILD_TS_FILE):
        with open(BUILD_TS_FILE) as f:
            return int(f.read().strip())
    ts = int(time.time())
    with open(BUILD_TS_FILE, "w") as f:
        f.write(str(ts))
    return ts


def encode_assoc_bitmask(row):
    """Encode association source flags into a u16 bitmask."""
    mask = 0
    for bit, col in enumerate(ASSOC_SOURCE_COLUMNS):
        if row[col]:
            mask |= 1 << bit
    return mask


def encode_entry(row):
    """Encode a single association entry into binary bytes."""
    bitmask = encode_assoc_bitmask(row)
    count = row["count"] or 0

    next_word_bytes = row["next_word"].encode("utf-8")
    next_tl = row["next_tl"] or ""
    next_tl_bytes = next_tl.encode("utf-8")

    return struct.pack(
        f"<HIbb{len(next_word_bytes)}s{len(next_tl_bytes)}s",
        bitmask,
        count,
        len(next_word_bytes),
        len(next_tl_bytes),
        next_word_bytes,
        next_tl_bytes,
    )


def build(logger):
    if not os.path.exists(DB_FILE):
        logger.error(f"Database not found: {DB_FILE}")
        sys.exit(1)

    build_ts = get_build_timestamp()
    logger.info(f"Build timestamp: {build_ts}")

    conn = sqlite3.connect(DB_FILE)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()

    # Get all unique prev_words sorted by UTF-8 bytes
    cursor.execute("SELECT DISTINCT prev_word FROM word_association ORDER BY prev_word")
    all_keys = [row["prev_word"] for row in cursor.fetchall()]

    # Sort by raw UTF-8 bytes (Python str sort matches UTF-8 byte sort for CJK)
    all_keys.sort(key=lambda s: s.encode("utf-8"))

    logger.info(f"Unique prev_words: {len(all_keys)}")

    # Build grouped entries (sorted by count DESC within each group)
    groups: dict[str, list] = {}
    total_entries = 0
    for key in all_keys:
        cursor.execute(
            "SELECT next_word, next_tl, count, "
            + ", ".join(ASSOC_SOURCE_COLUMNS)
            + " FROM word_association WHERE prev_word = ? ORDER BY count DESC",
            (key,),
        )
        entries = cursor.fetchall()
        groups[key] = entries
        total_entries += len(entries)

    logger.info(f"Total entries: {total_entries}")

    # Encode all entries per group
    encoded_groups: dict[str, list[bytes]] = {}
    for key in all_keys:
        encoded_groups[key] = [encode_entry(row) for row in groups[key]]

    # Calculate layout
    key_count = len(all_keys)
    header_size = 20  # magic(4) + version(4) + key_count(4) + entry_count(4) + build_ts(4)
    key_offset_table_size = key_count * 4

    # Key section: each key = prev_word_len(1) + prev_word(N) + entry_offset(4) + entry_count(2)
    key_section_start = header_size + key_offset_table_size
    key_entries_data = []
    for key in all_keys:
        key_bytes = key.encode("utf-8")
        # Placeholder for entry_offset (will be filled later)
        key_entries_data.append((key_bytes, len(encoded_groups[key])))

    # Calculate key section total size
    key_section_size = sum(1 + len(kb) + 4 + 2 for kb, _ in key_entries_data)
    entry_section_start = key_section_start + key_section_size

    # Calculate entry offsets (absolute from file start)
    entry_offset = entry_section_start
    key_entry_offsets = []
    for key in all_keys:
        key_entry_offsets.append(entry_offset)
        for enc in encoded_groups[key]:
            entry_offset += len(enc)

    # Calculate key offsets (absolute from file start)
    key_offsets = []
    pos = key_section_start
    for kb, _ in key_entries_data:
        key_offsets.append(pos)
        pos += 1 + len(kb) + 4 + 2

    # Write binary file
    with open(OUTPUT_FILE, "wb") as f:
        # Header
        f.write(MAGIC)
        f.write(struct.pack("<IIII", VERSION, key_count, total_entries, build_ts))

        # Key offset table
        for offset in key_offsets:
            f.write(struct.pack("<I", offset))

        # Key section
        for i, (kb, entry_count) in enumerate(key_entries_data):
            f.write(struct.pack("b", len(kb)))
            f.write(kb)
            f.write(struct.pack("<IH", key_entry_offsets[i], entry_count))

        # Entry section
        for key in all_keys:
            for enc in encoded_groups[key]:
                f.write(enc)

    file_size = os.path.getsize(OUTPUT_FILE)
    logger.info(f"\n  [output]")
    logger.info(f"    File:    {OUTPUT_FILE}")
    logger.info(f"    Size:    {file_size / 1024 / 1024:.2f} MB")
    logger.info(f"    Keys:    {key_count}")
    logger.info(f"    Entries: {total_entries}")

    conn.close()
    return key_count, total_entries


def verify(logger):
    """Verify association.bin against dictionary.db (round-trip check)."""
    logger.info(f"\n{'=' * 50}")
    logger.info("Verifying association.bin against dictionary.db...")
    logger.info(f"{'=' * 50}")

    with open(OUTPUT_FILE, "rb") as f:
        data = f.read()

    # Parse header
    magic = data[:4]
    assert magic == MAGIC, f"Bad magic: {magic}"
    version, key_count, entry_count, build_ts = struct.unpack_from("<IIII", data, 4)
    assert version == VERSION
    logger.info(
        f"  Header: version={version}, keys={key_count}, "
        f"entries={entry_count}, build_ts={build_ts}"
    )

    # Parse key offset table
    header_size = 20
    key_offsets = []
    for i in range(key_count):
        offset = struct.unpack_from("<I", data, header_size + i * 4)[0]
        key_offsets.append(offset)

    # Compare against SQLite
    conn = sqlite3.connect(DB_FILE)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()

    errors = 0
    verified_entries = 0

    for i in range(key_count):
        koff = key_offsets[i]
        prev_word_len = struct.unpack_from("b", data, koff)[0]
        prev_word = data[koff + 1 : koff + 1 + prev_word_len].decode("utf-8")
        e_offset, e_count = struct.unpack_from("<IH", data, koff + 1 + prev_word_len)

        # Get expected entries from SQLite
        cursor.execute(
            "SELECT next_word, next_tl, count, "
            + ", ".join(ASSOC_SOURCE_COLUMNS)
            + " FROM word_association WHERE prev_word = ? ORDER BY count DESC",
            (prev_word,),
        )
        db_rows = cursor.fetchall()

        if len(db_rows) != e_count:
            logger.error(
                f"  Key '{prev_word}': count mismatch bin={e_count} db={len(db_rows)}"
            )
            errors += 1
            continue

        # Parse and compare each entry
        pos = e_offset
        for j, db_row in enumerate(db_rows):
            bitmask, count_val, nw_len, nt_len = struct.unpack_from("<HIbb", data, pos)
            pos += 8
            next_word = data[pos : pos + nw_len].decode("utf-8")
            pos += nw_len
            next_tl = data[pos : pos + nt_len].decode("utf-8")
            pos += nt_len

            expected_bitmask = encode_assoc_bitmask(db_row)
            expected_next_tl = db_row["next_tl"] or ""

            if next_word != db_row["next_word"]:
                logger.error(f"  Key '{prev_word}' entry {j}: next_word mismatch")
                errors += 1
            if next_tl != expected_next_tl:
                logger.error(f"  Key '{prev_word}' entry {j}: next_tl mismatch")
                errors += 1
            if count_val != (db_row["count"] or 0):
                logger.error(f"  Key '{prev_word}' entry {j}: count mismatch")
                errors += 1
            if bitmask != expected_bitmask:
                logger.error(f"  Key '{prev_word}' entry {j}: bitmask mismatch")
                errors += 1

            verified_entries += 1

    conn.close()

    if errors == 0:
        logger.info(f"  Verified {key_count} keys, {verified_entries} entries — all match!")
    else:
        logger.error(f"  {errors} mismatches found!")
        sys.exit(1)


def main():
    logger = setup_logging()
    logger.info(f"{'=' * 50}")
    logger.info(f"{SCRIPT_NAME}")
    logger.info(f"Run: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    logger.info(f"{'=' * 50}")
    logger.info(f"Input:  {DB_FILE}")
    logger.info(f"Output: {OUTPUT_FILE}")
    logger.info("")

    build(logger)

    if "--verify" in sys.argv:
        verify(logger)

    logger.info("\nDone!")


if __name__ == "__main__":
    main()

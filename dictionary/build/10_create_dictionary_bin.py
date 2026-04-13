#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 dictionary.db 建立 dictionary.bin（binary mmap 格式）

輸入：output/dictionary.db
輸出：output/dictionary.bin

Binary 格式（little-endian）：
  Header (16 bytes):
    magic:    4 bytes  "TKDB"
    version:  u32      1
    count:    u32      record count
    build_ts: u32      Unix timestamp

  Offset table (count × 4 bytes):
    offsets[0..N-1]: u32  absolute byte offset from file start to record
    Mapping: rowId (1-based) → offsets[rowId - 1]

  Records (variable-length, one per rowid):
    bitmask:   u16   source flags
    frequency: u32   frequency value
    hanzi_len: u8    UTF-8 byte count (0 = NULL)
    tl_len:    u8    UTF-8 byte count
    hanzi:     [u8]  UTF-8 bytes
    tl:        [u8]  UTF-8 bytes

  Bitmask bit layout (u16):
    0=kautian  1=taigitv  2=itaigi   3=sitbut  4=taihoa   5=taijit
    6=kungge   7=stti     8=khpoo    9=khiin   10=dev     11=lkk
    12=is_variant  13-15=reserved

用法：
  python3 10_create_dictionary_bin.py            # 建立 binary
  python3 10_create_dictionary_bin.py --verify    # 建立並驗證
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
OUTPUT_FILE = os.path.join(OUTPUT_DIR, "dictionary.bin")
BUILD_TS_FILE = os.path.join(OUTPUT_DIR, ".build_ts")
LOG_DIR = os.path.join(BASE_DIR, "logs")
SCRIPT_NAME = "10_create_dictionary_bin"

MAGIC = b"TKDB"
VERSION = 1

# Bitmask bit layout — must match Swift reader
SOURCE_COLUMNS = [
    "kautian", "taigitv", "itaigi", "sitbut", "taihoa", "taijit",
    "kungge", "stti", "khpoo", "khiin", "dev", "lkk", "is_variant",
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
    """Get or create shared build timestamp for all binary files."""
    if os.path.exists(BUILD_TS_FILE):
        with open(BUILD_TS_FILE) as f:
            return int(f.read().strip())
    ts = int(time.time())
    with open(BUILD_TS_FILE, "w") as f:
        f.write(str(ts))
    return ts


def encode_bitmask(row):
    """Encode source flags into a u16 bitmask."""
    mask = 0
    for bit, col in enumerate(SOURCE_COLUMNS):
        if row[col]:
            mask |= 1 << bit
    return mask


def encode_record(row):
    """Encode a single dictionary row into binary bytes."""
    bitmask = encode_bitmask(row)
    frequency = row["frequency"] or 0

    hanzi = row["hanzi"]
    hanzi_bytes = hanzi.encode("utf-8") if hanzi else b""
    tl_bytes = row["tl"].encode("utf-8")

    return struct.pack(
        f"<HIBB{len(hanzi_bytes)}s{len(tl_bytes)}s",
        bitmask,
        frequency,
        len(hanzi_bytes),
        len(tl_bytes),
        hanzi_bytes,
        tl_bytes,
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

    # Verify rowids are contiguous 1..N
    cursor.execute("SELECT MIN(id), MAX(id), COUNT(*) FROM dictionary")
    min_id, max_id, count = cursor.fetchone()
    assert min_id == 1, f"Expected min id=1, got {min_id}"
    assert max_id == count, f"Expected max id={count}, got {max_id}"
    logger.info(f"Verified: {count} rows, ids 1..{count} (contiguous)")

    # Fetch all rows ordered by id
    cursor.execute(
        "SELECT id, hanzi, tl, frequency, "
        + ", ".join(SOURCE_COLUMNS)
        + " FROM dictionary ORDER BY id"
    )
    rows = cursor.fetchall()
    assert len(rows) == count

    # Encode all records
    records = []
    for row in rows:
        records.append(encode_record(row))

    # Calculate offsets (absolute from file start)
    header_size = 16  # magic(4) + version(4) + count(4) + build_ts(4)
    offset_table_size = count * 4
    data_start = header_size + offset_table_size

    offsets = []
    current_offset = data_start
    for rec in records:
        offsets.append(current_offset)
        current_offset += len(rec)

    # Verify offsets are monotonically increasing
    for i in range(1, len(offsets)):
        assert offsets[i] > offsets[i - 1], f"Non-monotonic offset at index {i}"

    # Write binary file
    with open(OUTPUT_FILE, "wb") as f:
        # Header
        f.write(MAGIC)
        f.write(struct.pack("<III", VERSION, count, build_ts))

        # Offset table
        for offset in offsets:
            f.write(struct.pack("<I", offset))

        # Records
        for rec in records:
            f.write(rec)

    file_size = os.path.getsize(OUTPUT_FILE)
    logger.info(f"\n  [output]")
    logger.info(f"    File:    {OUTPUT_FILE}")
    logger.info(f"    Size:    {file_size / 1024 / 1024:.2f} MB")
    logger.info(f"    Records: {count}")
    logger.info(f"    Header:  {header_size} bytes")
    logger.info(f"    Offsets: {offset_table_size} bytes")
    logger.info(f"    Data:    {current_offset - data_start} bytes")

    conn.close()
    return count, build_ts


def verify(logger):
    """Verify dictionary.bin against dictionary.db (round-trip check)."""
    logger.info(f"\n{'=' * 50}")
    logger.info("Verifying dictionary.bin against dictionary.db...")
    logger.info(f"{'=' * 50}")

    if not os.path.exists(OUTPUT_FILE):
        logger.error(f"Binary file not found: {OUTPUT_FILE}")
        sys.exit(1)

    with open(OUTPUT_FILE, "rb") as f:
        data = f.read()

    # Parse header
    magic = data[:4]
    assert magic == MAGIC, f"Bad magic: {magic}"
    version, count, build_ts = struct.unpack_from("<III", data, 4)
    assert version == VERSION, f"Bad version: {version}"
    logger.info(f"  Header: version={version}, count={count}, build_ts={build_ts}")

    # Parse offset table
    header_size = 16
    offsets = []
    for i in range(count):
        offset = struct.unpack_from("<I", data, header_size + i * 4)[0]
        offsets.append(offset)

    # Compare against SQLite
    conn = sqlite3.connect(DB_FILE)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    cursor.execute(
        "SELECT id, hanzi, tl, frequency, "
        + ", ".join(SOURCE_COLUMNS)
        + " FROM dictionary ORDER BY id"
    )
    rows = cursor.fetchall()
    assert len(rows) == count, f"Row count mismatch: bin={count}, db={len(rows)}"

    errors = 0
    for i, row in enumerate(rows):
        offset = offsets[i]
        # Determine record size
        if i + 1 < count:
            rec_end = offsets[i + 1]
        else:
            rec_end = len(data)
        rec_data = data[offset:rec_end]

        # Parse record
        bitmask, frequency, hanzi_len, tl_len = struct.unpack_from("<HIBB", rec_data, 0)
        pos = 8
        hanzi_bytes = rec_data[pos : pos + hanzi_len]
        pos += hanzi_len
        tl_bytes = rec_data[pos : pos + tl_len]

        hanzi = hanzi_bytes.decode("utf-8") if hanzi_len > 0 else None
        tl = tl_bytes.decode("utf-8")

        # Compare
        expected_hanzi = row["hanzi"] if row["hanzi"] else None
        expected_tl = row["tl"]
        expected_freq = row["frequency"] or 0
        expected_bitmask = encode_bitmask(row)

        if hanzi != expected_hanzi:
            logger.error(f"  Row {i + 1}: hanzi mismatch: '{hanzi}' vs '{expected_hanzi}'")
            errors += 1
        if tl != expected_tl:
            logger.error(f"  Row {i + 1}: tl mismatch: '{tl}' vs '{expected_tl}'")
            errors += 1
        if frequency != expected_freq:
            logger.error(f"  Row {i + 1}: freq mismatch: {frequency} vs {expected_freq}")
            errors += 1
        if bitmask != expected_bitmask:
            logger.error(
                f"  Row {i + 1}: bitmask mismatch: {bitmask:#06x} vs {expected_bitmask:#06x}"
            )
            errors += 1

    conn.close()

    if errors == 0:
        logger.info(f"  Verified {count} records — all match!")
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

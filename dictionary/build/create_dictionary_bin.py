#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 dictionary.csv 建立 dictionary.bin（binary mmap 格式）

輸入：output/dictionary.csv
輸出：output/dictionary.bin

Binary 格式（version 2,little-endian）：
  Header (16 bytes):
    magic:    4 bytes  "TKDB"
    version:  u32      2
    count:    u32      record count
    build_ts: u32      Unix timestamp

  Offset table (count × 4 bytes):
    offsets[0..N-1]: u32  absolute byte offset from file start to record
    Mapping: rowId (1-based) → offsets[rowId - 1]

  Records (variable-length, one per rowid):
    bitmask:        u16  source flags
    frequency:      u32  frequency value
    hanzi_len:      u8   UTF-8 byte count (0 = NULL)
    tl_len:         u8   UTF-8 byte count
    syllable_count: u8   TL syllable count (v2; 1..=MAX_SYLLABLES)
    hanzi:          [u8] UTF-8 bytes
    tl:             [u8] UTF-8 bytes

  Bitmask bit layout (u16):
    0=kautian  1=taigitv  2=itaigi   3=sitbut  4=taihoa   5=taijit
    6=kungge   7=stti     8=khpoo    9=khiin   10=dev     11=lkk
    12=is_variant  13-15=reserved

  v1 → v2 (v3.5.8 Phase 1): added per-record `syllable_count` u8 between
  `tl_len` and the `hanzi` payload. v1 binaries are NOT readable by the
  Rust v2 reader; rebuild + redeploy artifacts in lockstep.

用法：
  python3 create_dictionary_bin.py            # 建立 binary
  python3 create_dictionary_bin.py --verify    # 建立並驗證
"""

import struct
import sys

from build.common import LOG_DIR, OUTPUT_DIR, start_new_build_timestamp
from build.dictionary_records import DictionaryRecord, load_dictionary_records
from common.logging_utils import log_header, setup_logging
from common.source_bits import DICT_BIN_COLUMNS

CSV_FILE = OUTPUT_DIR / "dictionary.csv"
OUTPUT_FILE = OUTPUT_DIR / "dictionary.bin"
SCRIPT_NAME = "create_dictionary_bin"

MAGIC = b"TKDB"
VERSION = 2

# Bitmask bit layout — must match `engine/lexicon/src/dictionary_reader.rs`
# (CROSS-CRATE INVARIANT). Authoritative source for both bit positions and
# field semantics: `common/source_bits.py::SOURCE_BITS` + `IS_VARIANT_BIT`.
# iOS / Android no longer parse dictionary.bin directly post-Phase IV-B;
# the Rust reader is the sole consumer of this layout.
SOURCE_COLUMNS = DICT_BIN_COLUMNS


def encode_bitmask(record: DictionaryRecord) -> int:
    """Encode source flags into a u16 bitmask."""
    sources = record.source_dict()
    mask = 0
    for bit, col in enumerate(SOURCE_COLUMNS):
        if sources[col]:
            mask |= 1 << bit
    return mask


def encode_record(record: DictionaryRecord) -> bytes:
    """Encode a single dictionary record into binary bytes."""
    bitmask = encode_bitmask(record)
    frequency = record.frequency or 0

    hanzi_bytes = record.hanzi.encode("utf-8") if record.hanzi else b""
    tl_bytes = record.tl.encode("utf-8")

    return struct.pack(
        f"<HIBBB{len(hanzi_bytes)}s{len(tl_bytes)}s",
        bitmask,
        frequency,
        len(hanzi_bytes),
        len(tl_bytes),
        record.syllable_count,
        hanzi_bytes,
        tl_bytes,
    )


def build(logger):
    if not CSV_FILE.exists():
        logger.error(f"CSV not found: {CSV_FILE}")
        sys.exit(1)

    build_ts = start_new_build_timestamp()
    logger.info(f"Build timestamp: {build_ts}")

    records = load_dictionary_records(CSV_FILE)
    count = len(records)
    if count == 0:
        logger.error("load_dictionary_records returned 0 rows — CSV empty?")
        sys.exit(1)

    # Sanity: rowids are 1..N contiguous by construction; assert anyway.
    assert records[0].rowid == 1, f"Expected first rowid=1, got {records[0].rowid}"
    assert records[-1].rowid == count, (
        f"Expected last rowid={count}, got {records[-1].rowid}"
    )
    logger.info(f"Loaded {count} records, ids 1..{count} (contiguous)")

    encoded = [encode_record(r) for r in records]

    header_size = 16  # magic(4) + version(4) + count(4) + build_ts(4)
    offset_table_size = count * 4
    data_start = header_size + offset_table_size

    offsets = []
    current_offset = data_start
    for rec in encoded:
        offsets.append(current_offset)
        current_offset += len(rec)

    for i in range(1, len(offsets)):
        assert offsets[i] > offsets[i - 1], f"Non-monotonic offset at index {i}"

    with open(OUTPUT_FILE, "wb") as f:
        f.write(MAGIC)
        f.write(struct.pack("<III", VERSION, count, build_ts))
        for offset in offsets:
            f.write(struct.pack("<I", offset))
        for rec in encoded:
            f.write(rec)

    file_size = OUTPUT_FILE.stat().st_size
    logger.info("\n  [output]")
    logger.info(f"    File:    {OUTPUT_FILE}")
    logger.info(f"    Size:    {file_size / 1024 / 1024:.2f} MB")
    logger.info(f"    Records: {count}")
    logger.info(f"    Header:  {header_size} bytes")
    logger.info(f"    Offsets: {offset_table_size} bytes")
    logger.info(f"    Data:    {current_offset - data_start} bytes")

    return count, build_ts


def verify(logger):
    """Verify dictionary.bin against load_dictionary_records (round-trip check)."""
    logger.info(f"\n{'=' * 50}")
    logger.info("Verifying dictionary.bin against dictionary.csv...")
    logger.info(f"{'=' * 50}")

    if not OUTPUT_FILE.exists():
        logger.error(f"Binary file not found: {OUTPUT_FILE}")
        sys.exit(1)

    data = OUTPUT_FILE.read_bytes()

    magic = data[:4]
    assert magic == MAGIC, f"Bad magic: {magic}"
    version, count, build_ts = struct.unpack_from("<III", data, 4)
    assert version == VERSION, f"Bad version: {version}"
    logger.info(f"  Header: version={version}, count={count}, build_ts={build_ts}")

    header_size = 16
    offsets = []
    for i in range(count):
        offset = struct.unpack_from("<I", data, header_size + i * 4)[0]
        offsets.append(offset)

    records = load_dictionary_records(CSV_FILE)
    assert len(records) == count, f"Row count mismatch: bin={count}, csv={len(records)}"

    errors = 0
    for i, rec in enumerate(records):
        offset = offsets[i]
        rec_end = offsets[i + 1] if i + 1 < count else len(data)
        rec_data = data[offset:rec_end]

        bitmask, frequency, hanzi_len, tl_len, syllable_count = struct.unpack_from(
            "<HIBBB", rec_data, 0
        )
        pos = 9  # 2 bitmask + 4 freq + 1 hanzi_len + 1 tl_len + 1 syllable_count
        hanzi_bytes = rec_data[pos: pos + hanzi_len]
        pos += hanzi_len
        tl_bytes = rec_data[pos: pos + tl_len]

        hanzi = hanzi_bytes.decode("utf-8") if hanzi_len > 0 else None
        tl = tl_bytes.decode("utf-8")

        expected_hanzi = rec.hanzi
        expected_tl = rec.tl
        expected_freq = rec.frequency or 0
        expected_bitmask = encode_bitmask(rec)
        expected_syllables = rec.syllable_count

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
        if syllable_count != expected_syllables:
            logger.error(
                f"  Row {i + 1}: syllable_count mismatch: {syllable_count} vs {expected_syllables}"
            )
            errors += 1

    if errors == 0:
        logger.info(f"  Verified {count} records — all match!")
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

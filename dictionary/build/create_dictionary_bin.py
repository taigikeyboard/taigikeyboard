#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Build dictionary.bin (binary mmap format) from dictionary.csv

Input: output/dictionary.csv, shared/data/word_unigrams.tsv (build.walker_lm)
Output: output/dictionary.bin, output/walker_lm_stats.txt

Binary format (version 4, little-endian):
  Header (16 bytes):
    magic:    4 bytes  "TKDB"
    version:  u32      4
    count:    u32      record count
    build_ts: u32      build id — CRC-32 of the build inputs (common.build_id)

  Offset table (count × 4 bytes):
    offsets[0..N-1]: u32  absolute byte offset from file start to record
    Mapping: rowId (1-based) → offsets[rowId - 1]

  Records (variable-length, one per rowid):
    bitmask:        u16  source flags
    frequency:      u32  frequency value
    hanzi_len:      u8   UTF-8 byte count (0 = NULL)
    tl_len:         u8   UTF-8 byte count
    syllable_count: u8   TL syllable count (v2; 1..=MAX_SYLLABLES)
    kautian_subtag: u16  kautian subcollection provenance (v3)
    walker_cost:    u16  walker model −ln p in milli-nats (v4; build.walker_lm)
    hanzi:          [u8] UTF-8 bytes
    tl:             [u8] UTF-8 bytes

  Bitmask bit layout (u16):
    0=kautian  1=taigitv  2=itaigi   3=sitbut  4=taihoa   5=taijit
    6=kungge   7=stti     8=khpoo    9=khiin   10=dev     11=lkk
    12=is_variant  13-15=reserved

  kautian_subtag bit layout (u16; 0 for every non-kautian row):
    0=has_main  1..=10=accent_mask (10 dialect columns)  11=has_name
    12=has_alt_reading  13-15=reserved

  v1 → v2 (v3.5.8 Phase 1): added per-record `syllable_count` u8 between
  `tl_len` and the `hanzi` payload.
  v2 → v3 (kautian subcollections Phase 2): added per-record `kautian_subtag`
  u16 between `syllable_count` and the `hanzi` payload.
  v3 → v4 (E1 unified word frequency P2): added per-record `walker_cost` u16
  after `kautian_subtag`. Older binaries are NOT readable by the Rust v4
  reader; rebuild + redeploy artifacts in lockstep.

Usage:
  python3 create_dictionary_bin.py            # build the binary
  python3 create_dictionary_bin.py --verify    # build and verify
"""

import struct
import sys

from build.common import LOG_DIR, OUTPUT_DIR, build_id
from build.dictionary_records import DictionaryRecord, load_dictionary_records
from build.walker_lm import load_model, record_costs, stats_text
from common.logging_utils import log_header, setup_logging
from common.source_bits import DICT_BIN_COLUMNS, KAUTIAN_SUBTAG_USED_MASK

CSV_FILE = OUTPUT_DIR / "dictionary.csv"
OUTPUT_FILE = OUTPUT_DIR / "dictionary.bin"
WALKER_LM_STATS_FILE = OUTPUT_DIR / "walker_lm_stats.txt"
SCRIPT_NAME = "create_dictionary_bin"

MAGIC = b"TKDB"
VERSION = 4
# bitmask u16, frequency u32, hanzi_len u8, tl_len u8, syllable_count u8,
# kautian_subtag u16, walker_cost u16 — then the hanzi and tl bytes.
RECORD_PREFIX = "<HIBBBHH"
RECORD_PREFIX_SIZE = struct.calcsize(RECORD_PREFIX)

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


def encoded_frequency(record: DictionaryRecord) -> int:
    """Convention for emitting `record.frequency` to `dictionary.bin`.

    `DictionaryRecord.frequency` is `int | None` so the parity counter in
    `compare_baseline.py` can reproduce `WHERE frequency IS NULL`. The
    binary writer coerces `None → 0` through this single helper so the
    on-disk convention cannot drift between encode + verify call sites.
    """
    return record.frequency or 0


def encode_record(record: DictionaryRecord, walker_cost: int) -> bytes:
    """Encode a single dictionary record into binary bytes."""
    bitmask = encode_bitmask(record)
    frequency = encoded_frequency(record)

    hanzi_bytes = record.hanzi.encode("utf-8") if record.hanzi else b""
    tl_bytes = record.tl.encode("utf-8")

    assert record.kautian_subtag & ~KAUTIAN_SUBTAG_USED_MASK == 0, (
        f"kautian_subtag {record.kautian_subtag:#06x} sets reserved bits "
        f"(rowid={record.rowid})"
    )

    return struct.pack(
        RECORD_PREFIX,
        bitmask,
        frequency,
        len(hanzi_bytes),
        len(tl_bytes),
        record.syllable_count,
        record.kautian_subtag,
        walker_cost,
    ) + hanzi_bytes + tl_bytes


def build(logger):
    if not CSV_FILE.exists():
        logger.error(f"CSV not found: {CSV_FILE}")
        sys.exit(1)

    build_ts = build_id()
    logger.info(f"Build id: {build_ts}")

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

    model = load_model(records)
    walker_costs = record_costs(records, model)
    WALKER_LM_STATS_FILE.write_text(stats_text(records, model, walker_costs), encoding="utf-8")
    logger.info(
        f"Walker model: N={model.tokens} V={model.vocabulary} alpha={model.alpha} "
        f"unseen_cost={model.unseen_cost} → {WALKER_LM_STATS_FILE}"
    )

    encoded = [encode_record(r, cost) for r, cost in zip(records, walker_costs, strict=True)]

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
    assert build_ts == build_id(), f"Bad build id: {build_ts} != {build_id()}"
    logger.info(f"  Header: version={version}, count={count}, build_ts={build_ts}")

    header_size = 16
    offsets = []
    for i in range(count):
        offset = struct.unpack_from("<I", data, header_size + i * 4)[0]
        offsets.append(offset)

    records = load_dictionary_records(CSV_FILE)
    assert len(records) == count, f"Row count mismatch: bin={count}, csv={len(records)}"
    expected_walker_costs = record_costs(records, load_model(records))

    errors = 0
    for i, rec in enumerate(records):
        offset = offsets[i]
        rec_end = offsets[i + 1] if i + 1 < count else len(data)
        rec_data = data[offset:rec_end]

        (
            bitmask,
            frequency,
            hanzi_len,
            tl_len,
            syllable_count,
            kautian_subtag,
            walker_cost,
        ) = struct.unpack_from(RECORD_PREFIX, rec_data, 0)
        if len(rec_data) != RECORD_PREFIX_SIZE + hanzi_len + tl_len:
            logger.error(f"  Row {i + 1}: record length {len(rec_data)} != prefix + payload")
            errors += 1
        pos = RECORD_PREFIX_SIZE
        hanzi_bytes = rec_data[pos: pos + hanzi_len]
        pos += hanzi_len
        tl_bytes = rec_data[pos: pos + tl_len]

        hanzi = hanzi_bytes.decode("utf-8") if hanzi_len > 0 else None
        tl = tl_bytes.decode("utf-8")

        expected_hanzi = rec.hanzi
        expected_tl = rec.tl
        expected_freq = encoded_frequency(rec)
        expected_bitmask = encode_bitmask(rec)
        expected_syllables = rec.syllable_count
        expected_subtag = rec.kautian_subtag

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
        if kautian_subtag != expected_subtag:
            logger.error(
                f"  Row {i + 1}: kautian_subtag mismatch: "
                f"{kautian_subtag:#06x} vs {expected_subtag:#06x}"
            )
            errors += 1
        if walker_cost != expected_walker_costs[i]:
            logger.error(
                f"  Row {i + 1}: walker_cost mismatch: {walker_cost} vs {expected_walker_costs[i]}"
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

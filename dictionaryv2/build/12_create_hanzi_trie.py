#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 dictionary.db 建立 hanzi.trie（漢字前綴搜尋用 MARISA-trie）

輸入：output/dictionary.db
輸出：output/hanzi.trie

Key 格式：hanzi UTF-8 字串（不加 prefix）
Value：SQLite rowid（uint32 LE，RecordTrie 格式）

用法：
  python3 12_create_hanzi_trie.py            # 建立 trie
  python3 12_create_hanzi_trie.py --verify    # 建立並驗證
"""

import os
import sys
import sqlite3
import logging
from datetime import datetime
import marisa_trie

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
BASE_DIR = os.path.dirname(SCRIPT_DIR)
OUTPUT_DIR = os.path.join(BASE_DIR, "output")
DB_FILE = os.path.join(OUTPUT_DIR, "dictionary.db")
OUTPUT_FILE = os.path.join(OUTPUT_DIR, "hanzi.trie")
LOG_DIR = os.path.join(BASE_DIR, "logs")
SCRIPT_NAME = "12_create_hanzi_trie"


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


def build(logger):
    if not os.path.exists(DB_FILE):
        logger.error(f"Database not found: {DB_FILE}")
        sys.exit(1)

    conn = sqlite3.connect(DB_FILE)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()

    # Fetch all rows with non-empty hanzi
    cursor.execute(
        "SELECT id, hanzi FROM dictionary "
        "WHERE hanzi IS NOT NULL AND hanzi != '' "
        "ORDER BY id"
    )
    rows = cursor.fetchall()
    logger.info(f"Loaded {len(rows)} rows with hanzi")

    # Build key-value pairs: hanzi → rowid
    # Multiple rows can have the same hanzi (different romanizations)
    pairs = []
    seen = set()
    for row in rows:
        hanzi = row["hanzi"]
        rowid = row["id"]
        pair_key = (hanzi, rowid)
        if pair_key not in seen:
            seen.add(pair_key)
            pairs.append((hanzi, (rowid,)))

    logger.info(f"Generated {len(pairs)} key-value pairs")

    # Build MARISA RecordTrie (same format as dictionary.trie)
    trie = marisa_trie.RecordTrie("<I", pairs)
    trie.save(OUTPUT_FILE)

    file_size = os.path.getsize(OUTPUT_FILE)
    logger.info(f"\n  [output]")
    logger.info(f"    File: {OUTPUT_FILE}")
    logger.info(f"    Size: {file_size / 1024 / 1024:.2f} MB")

    # Test queries
    logger.info(f"\n  [test queries]")
    test_keys = ["早", "好", "台", "的"]
    for key in test_keys:
        if key in trie:
            results = trie[key]
            logger.info(f"    exact '{key}' -> {len(results)} rowids")
        prefix_results = trie.keys(key)
        logger.info(f"    prefix '{key}' -> {len(prefix_results)} keys")

    conn.close()
    return len(pairs)


def verify(logger):
    """Verify hanzi.trie against dictionary.db."""
    logger.info(f"\n{'=' * 50}")
    logger.info("Verifying hanzi.trie against dictionary.db...")
    logger.info(f"{'=' * 50}")

    trie = marisa_trie.RecordTrie("<I")
    trie.mmap(OUTPUT_FILE)

    conn = sqlite3.connect(DB_FILE)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()

    # Verify: every (hanzi, id) pair in DB should be in trie
    cursor.execute(
        "SELECT id, hanzi FROM dictionary "
        "WHERE hanzi IS NOT NULL AND hanzi != '' "
        "ORDER BY id"
    )
    rows = cursor.fetchall()

    errors = 0
    for row in rows:
        hanzi = row["hanzi"]
        rowid = row["id"]

        if hanzi not in trie:
            logger.error(f"  Missing key in trie: '{hanzi}' (id={rowid})")
            errors += 1
            continue

        rowids = [r[0] for r in trie[hanzi]]
        if rowid not in rowids:
            logger.error(f"  Missing rowid {rowid} for key '{hanzi}'")
            errors += 1

    # Verify: prefix search works for known patterns
    test_prefixes = [("早", "早安"), ("好", "好無"), ("台", "台灣")]
    for prefix, expected_word in test_prefixes:
        keys = trie.keys(prefix)
        matching_words = [k for k in keys if k.startswith(expected_word)]
        if matching_words:
            logger.info(f"  Prefix '{prefix}' -> found '{expected_word}' (OK)")
        else:
            logger.warning(f"  Prefix '{prefix}' -> '{expected_word}' not found (may not exist)")

    conn.close()

    if errors == 0:
        logger.info(f"  Verified {len(rows)} entries — all match!")
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

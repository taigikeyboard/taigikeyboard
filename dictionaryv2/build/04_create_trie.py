#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 trie.db + dictionary.db 建立 MARISA-trie

輸入：output/trie.db, output/dictionary.db
輸出：output/dictionary.trie

Key 格式（前綴式）：
- tl:<tl_num>：TL 數字聲調（如 tl:hoo2-se3）
- tl:<tl_notone>：TL 去調（如 tl:hoo-se）
- poj:<poj_num>：POJ 數字聲調（如 poj:ho2-se3）
- poj:<poj_notone>：POJ 去調（如 poj:ho-se）
- tl:<tl_abbrev>：TL 縮寫（如 tl:hs）
- poj:<poj_abbrev>：POJ 縮寫（如 poj:hs）
- hanzi:<hanzi>：漢字前綴搜尋（如 hanzi:好無）

Value：SQLite rowid（對應 dictionary.db 的 id）
"""

import os
import sys
import sqlite3
import logging
from datetime import datetime
import marisa_trie

# 基準目錄（dictionary2/）
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
BASE_DIR = os.path.dirname(SCRIPT_DIR)

OUTPUT_DIR = os.path.join(BASE_DIR, "output")
DB_FILE = os.path.join(OUTPUT_DIR, "trie.db")
DICT_DB_FILE = os.path.join(OUTPUT_DIR, "dictionary.db")
OUTPUT_FILE = os.path.join(OUTPUT_DIR, "dictionary.trie")
LOG_DIR = os.path.join(BASE_DIR, "logs")
SCRIPT_NAME = "03_create_trie"


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




def main():
    logger = setup_logging()
    logger.info(f"{'=' * 50}")
    logger.info(f"{SCRIPT_NAME}")
    logger.info(f"Run: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    logger.info(f"{'=' * 50}")
    logger.info(f"Input:  {DB_FILE}")
    logger.info(f"Output: {OUTPUT_FILE}")
    logger.info("")

    if not os.path.exists(DB_FILE):
        logger.error(f"Database not found: {DB_FILE}")
        return

    # 連接 SQLite
    conn = sqlite3.connect(DB_FILE)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()

    # 讀取所有欄位
    cursor.execute("SELECT * FROM dictionary")
    rows = cursor.fetchall()
    columns = rows[0].keys() if rows else []
    logger.info(f"Loaded {len(rows)} records from database")
    logger.info(f"Columns: {list(columns)}")

    # 建立 key-value pairs（前綴式）
    TL_PREFIX = "tl:"
    POJ_PREFIX = "poj:"
    HANZI_PREFIX = "hanzi:"

    pairs = []
    seen_keys = set()

    def add_pair(key, rowid):
        pair_key = (key, rowid)
        if pair_key not in seen_keys:
            seen_keys.add(pair_key)
            pairs.append((key, (rowid,)))

    for row in rows:
        rowid = row["id"]

        # TL keys (tl: prefix)
        for col in ["tl_num", "tl_notone", "tl_abbrev"]:
            if col in columns:
                val = row[col]
                if val:
                    add_pair(TL_PREFIX + val, rowid)

        # POJ keys (poj: prefix)
        for col in ["poj_num", "poj_notone", "poj_abbrev"]:
            if col in columns:
                val = row[col]
                if val:
                    add_pair(POJ_PREFIX + val, rowid)

    romanization_count = len(pairs)
    logger.info(f"Romanization pairs: {romanization_count}")

    # Hanzi keys（from dictionary.db）
    if not os.path.exists(DICT_DB_FILE):
        logger.error(f"Dictionary database not found: {DICT_DB_FILE}")
        sys.exit(1)

    dict_conn = sqlite3.connect(DICT_DB_FILE)
    dict_conn.row_factory = sqlite3.Row
    dict_cursor = dict_conn.cursor()

    dict_cursor.execute(
        "SELECT id, hanzi FROM dictionary "
        "WHERE hanzi IS NOT NULL AND hanzi != '' "
        "ORDER BY id"
    )
    hanzi_rows = dict_cursor.fetchall()

    for row in hanzi_rows:
        add_pair(HANZI_PREFIX + row["hanzi"], row["id"])

    dict_conn.close()

    hanzi_count = len(pairs) - romanization_count
    logger.info(f"Hanzi pairs: {hanzi_count}")
    logger.info(f"Total pairs: {len(pairs)}")

    # 建立 MARISA-trie
    trie = marisa_trie.RecordTrie("<I", pairs)

    # 儲存
    trie.save(OUTPUT_FILE)

    # 檔案大小
    file_size = os.path.getsize(OUTPUT_FILE)
    logger.info(f"\n  [output]")
    logger.info(f"    File: {OUTPUT_FILE}")
    logger.info(f"    Size: {file_size / 1024 / 1024:.2f} MB")

    # 測試查詢
    logger.info(f"\n  [test queries]")
    test_keys = ["tl:gua", "tl:gua2", "poj:goa", "poj:goa2", "tl:gh", "hanzi:好", "hanzi:台"]
    for key in test_keys:
        if key in trie:
            results = trie[key]
            logger.info(f"    '{key}' -> {len(results)} results")
        else:
            prefix_results = trie.keys(key)
            logger.info(f"    '{key}' -> prefix: {len(prefix_results)} keys")

    conn.close()
    logger.info(f"\nDone!")


if __name__ == "__main__":
    main()

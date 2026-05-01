#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 trie.db + dictionary.db 建立 fst 前綴索引

輸入：output/trie.db, output/dictionary.db
輸出：output/dictionary.fst

Key 格式（前綴式）：
- tl:<tl_num>：TL 數字聲調（如 tl:hoo2-se3）
- tl:<tl_notone>：TL 去調（如 tl:hoo-se）
- poj:<poj_num>：POJ 數字聲調（如 poj:ho2-se3）
- poj:<poj_notone>：POJ 去調（如 poj:ho-se）
- tl:<tl_abbrev>：TL 縮寫（如 tl:hs）
- poj:<poj_abbrev>：POJ 縮寫（如 poj:hs）
- hanzi:<hanzi>：漢字前綴搜尋（如 hanzi:好無）

Wire format (per docs/engine/lexicon-slice-plan.md §2.2):
    key_bytes (UTF-8) || 0xFF || rowid_le_4

實際 fst 建置由 Rust binary `engine/build-helpers/fst-builder` 完成；
本 Python 腳本只負責讀取 SQLite、組 stdin pairs、shell out 到 Rust。
"""

import shutil
import sqlite3
import subprocess
import sys
from pathlib import Path

from build.common import LOG_DIR, OUTPUT_DIR
from common.logging_utils import log_header, setup_logging

DB_FILE = OUTPUT_DIR / "trie.db"
DICT_DB_FILE = OUTPUT_DIR / "dictionary.db"
OUTPUT_FILE = OUTPUT_DIR / "dictionary.fst"
SCRIPT_NAME = "create_fst"

ENGINE_DIR = Path(__file__).resolve().parents[2] / "engine"
BUILDER_RELEASE = ENGINE_DIR / "target" / "release" / "fst-builder"
BUILDER_DEBUG = ENGINE_DIR / "target" / "debug" / "fst-builder"


def resolve_builder_bin() -> Path:
    """Locate the fst-builder Rust binary; build it if missing."""
    if BUILDER_RELEASE.exists():
        return BUILDER_RELEASE
    if BUILDER_DEBUG.exists():
        return BUILDER_DEBUG
    cargo = shutil.which("cargo")
    if not cargo:
        raise RuntimeError(
            "fst-builder binary missing and `cargo` not in PATH; build it manually "
            f"with `cargo build --release -p fst-builder` from {ENGINE_DIR}"
        )
    subprocess.run(
        [cargo, "build", "--release", "-p", "fst-builder"],
        cwd=ENGINE_DIR,
        check=True,
    )
    if not BUILDER_RELEASE.exists():
        raise RuntimeError(
            f"cargo build succeeded but {BUILDER_RELEASE} still missing"
        )
    return BUILDER_RELEASE


def collect_pairs(logger) -> list[tuple[str, int]]:
    """Yield (key, rowid) pairs from trie.db + dictionary.db hanzi rows."""
    if not DB_FILE.exists():
        raise RuntimeError(f"Database not found: {DB_FILE}")
    if not DICT_DB_FILE.exists():
        raise RuntimeError(f"Dictionary database not found: {DICT_DB_FILE}")

    pairs: list[tuple[str, int]] = []
    seen: set[tuple[str, int]] = set()

    def add(key: str, rowid: int) -> None:
        pair = (key, rowid)
        if pair not in seen:
            seen.add(pair)
            pairs.append(pair)

    conn = sqlite3.connect(DB_FILE)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM dictionary")
    rows = cursor.fetchall()
    columns = rows[0].keys() if rows else []
    logger.info(f"Loaded {len(rows)} records from {DB_FILE.name}")
    logger.info(f"Columns: {list(columns)}")

    for row in rows:
        rowid = int(row["id"])
        for col in ("tl_num", "tl_notone", "tl_abbrev"):
            if col in columns:
                val = row[col]
                if val:
                    add(f"tl:{val}", rowid)
        for col in ("poj_num", "poj_notone", "poj_abbrev"):
            if col in columns:
                val = row[col]
                if val:
                    add(f"poj:{val}", rowid)

    romanization_count = len(pairs)
    logger.info(f"Romanization pairs: {romanization_count}")

    dict_conn = sqlite3.connect(DICT_DB_FILE)
    dict_conn.row_factory = sqlite3.Row
    dict_cursor = dict_conn.cursor()
    dict_cursor.execute(
        "SELECT id, hanzi FROM dictionary "
        "WHERE hanzi IS NOT NULL AND hanzi != '' "
        "ORDER BY id"
    )
    for row in dict_cursor.fetchall():
        add(f"hanzi:{row['hanzi']}", int(row["id"]))
    dict_conn.close()
    conn.close()

    hanzi_count = len(pairs) - romanization_count
    if hanzi_count <= 0:
        raise RuntimeError(
            "No hanzi keys generated — dictionary.db may be missing hanzi data"
        )
    logger.info(f"Hanzi pairs: {hanzi_count}")
    logger.info(f"Total pairs: {len(pairs)}")
    return pairs


def emit_to_builder(pairs: list[tuple[str, int]], builder_bin: Path) -> None:
    """Pipe TL\\tkey\\trowid lines into fst-builder; let it write OUTPUT_FILE."""
    payload_lines = (f"TL\t{key}\t{rowid}" for key, rowid in pairs)
    payload = ("\n".join(payload_lines) + "\n").encode("utf-8")
    proc = subprocess.run(
        [str(builder_bin), "build", str(OUTPUT_FILE)],
        input=payload,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"fst-builder failed (exit {proc.returncode}): "
            f"{proc.stderr.decode('utf-8', errors='replace')}"
        )
    sys.stderr.write(proc.stderr.decode("utf-8", errors="replace"))


def main() -> None:
    logger = setup_logging(SCRIPT_NAME, log_dir=LOG_DIR)
    log_header(logger, SCRIPT_NAME, DB_FILE, OUTPUT_FILE)

    builder_bin = resolve_builder_bin()
    logger.info(f"fst-builder: {builder_bin}")

    pairs = collect_pairs(logger)
    emit_to_builder(pairs, builder_bin)

    file_size = OUTPUT_FILE.stat().st_size
    logger.info("\n  [output]")
    logger.info(f"    File: {OUTPUT_FILE}")
    logger.info(f"    Size: {file_size / 1024 / 1024:.2f} MB")
    logger.info("\nDone!")


if __name__ == "__main__":
    main()

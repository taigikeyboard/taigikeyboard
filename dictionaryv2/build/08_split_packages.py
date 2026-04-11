#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Split merged dictionary into per-source packages.

Input:  output/dictionary.csv (from 01_merge_csv.py)
Output: output/packages/{id}/{id}.db + {id}.trie + manifest.json
        output/packages/{id}-v{version}.zip
        output/packages/remote-manifest.json

Each package contains entries filtered by source column(s).
Entries appearing in multiple sources are included in all matching packages.
"""

import csv
import hashlib
import json
import os
import sqlite3
import sys
import zipfile
from datetime import datetime

import marisa_trie
import pandas as pd

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
BASE_DIR = os.path.dirname(SCRIPT_DIR)

sys.path.insert(0, BASE_DIR)
from common.logging_utils import setup_logging, log_header

OUTPUT_DIR = os.path.join(BASE_DIR, "output")
PACKAGES_DIR = os.path.join(OUTPUT_DIR, "packages")
DICTIONARY_CSV = os.path.join(OUTPUT_DIR, "dictionary.csv")
SCRIPT_NAME = "08_split_packages"

# Package definitions: id, display name, source columns to filter by
PACKAGES = [
    {"id": "core", "name": "教育部臺灣台語常用詞辭典", "filter": ["kautian", "dev"]},
    {"id": "taigitv", "name": "台語新詞辭庫", "filter": ["taigitv"]},
    {"id": "itaigi", "name": "iTaigi 華台對照典", "filter": ["itaigi"]},
    {"id": "sitbut", "name": "台灣植物名彙", "filter": ["sitbut"]},
    {"id": "taihoa", "name": "台華線頂對照典", "filter": ["taihoa"]},
    {"id": "taijit", "name": "台日大辭典", "filter": ["taijit"]},
    {"id": "kungge", "name": "台語工藝詞庫", "filter": ["kungge"]},
    {"id": "stti", "name": "學科術語辭典", "filter": ["stti"]},
    {"id": "khpoo", "name": "齒盤補充辭典", "filter": ["khpoo"]},
    {"id": "khiin", "name": "Khiin 補充", "filter": ["khiin"]},
    {"id": "lkk", "name": "LKK 漢羅合用建議用字", "filter": ["lkk"]},
]

TL_PREFIX = "tl:"
POJ_PREFIX = "poj:"


def filter_rows(df: pd.DataFrame, filter_cols: list[str]) -> pd.DataFrame:
    """Filter DataFrame to rows where any of the filter columns is True."""
    mask = pd.Series(False, index=df.index)
    for col in filter_cols:
        if col in df.columns:
            mask = mask | df[col].astype(bool)
    return df[mask].copy()


def create_package_db(rows: pd.DataFrame, db_path: str, pkg_id: str, version: str) -> int:
    """Create per-package SQLite database. Returns entry count."""
    if os.path.exists(db_path):
        os.remove(db_path)

    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    cursor.executescript("""
        PRAGMA journal_mode = DELETE;
        PRAGMA synchronous = NORMAL;

        CREATE TABLE metadata (
            key TEXT PRIMARY KEY,
            value TEXT
        );

        CREATE TABLE dictionary (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            hanzi TEXT,
            tl TEXT NOT NULL,
            frequency INTEGER,
            is_variant INTEGER DEFAULT 0,
            UNIQUE(tl, hanzi)
        );

        CREATE INDEX idx_tl ON dictionary(tl);
        CREATE INDEX idx_hanzi ON dictionary(hanzi);
        CREATE INDEX idx_frequency ON dictionary(frequency);
    """)

    cursor.execute(
        "INSERT INTO metadata (key, value) VALUES ('version', ?), ('build_date', datetime('now')), ('source_id', ?)",
        (version, pkg_id),
    )

    count = 0
    for _, row in rows.iterrows():
        tl = row["tl"]
        if pd.isna(tl) or tl == "":
            continue

        # Syllable count filter (max 4)
        normalized = str(tl).replace(" ", "-")
        if normalized.count("-") + 1 > 4:
            continue

        hanzi = row["hanzi"] if pd.notna(row["hanzi"]) and row["hanzi"] != "" else None
        frequency = int(row["frequency"]) if pd.notna(row["frequency"]) else None
        is_variant = 1 if row.get("is_variant") in (True, "True", 1) else 0

        try:
            cursor.execute(
                "INSERT OR IGNORE INTO dictionary (hanzi, tl, frequency, is_variant) VALUES (?, ?, ?, ?)",
                (hanzi, tl, frequency, is_variant),
            )
            if cursor.rowcount > 0:
                count += 1
        except sqlite3.IntegrityError:
            pass

    cursor.execute("INSERT INTO metadata (key, value) VALUES ('entry_count', ?)", (str(count),))
    cursor.execute("ANALYZE")
    conn.commit()
    conn.close()
    return count


def create_package_trie(rows: pd.DataFrame, db_path: str, trie_path: str) -> int:
    """Create per-package MARISA trie. Returns key count.

    Row IDs come from the per-package db (not the merged db).
    """
    # Build (tl, hanzi) -> id mapping from per-package db
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    cursor.execute("SELECT id, tl, hanzi FROM dictionary")
    id_map: dict[tuple[str, str | None], int] = {}
    for row_id, tl, hanzi in cursor.fetchall():
        id_map[(tl, hanzi)] = row_id
    conn.close()

    pairs = []
    seen_keys: set[tuple[str, int]] = set()

    def add_pair(key: str, rowid: int):
        pair_key = (key, rowid)
        if pair_key not in seen_keys:
            seen_keys.add(pair_key)
            pairs.append((key, (rowid,)))

    for _, row in rows.iterrows():
        tl = row["tl"]
        if pd.isna(tl) or tl == "":
            continue

        hanzi = row["hanzi"] if pd.notna(row["hanzi"]) and row["hanzi"] != "" else None
        rowid = id_map.get((tl, hanzi))
        if rowid is None:
            continue

        # TL keys
        for col in ["tl_num", "tl_notone", "tl_abbrev"]:
            val = row.get(col)
            if pd.notna(val) and val != "":
                add_pair(TL_PREFIX + str(val), rowid)

        # POJ keys
        for col in ["poj_num", "poj_notone", "poj_abbrev"]:
            val = row.get(col)
            if pd.notna(val) and val != "":
                add_pair(POJ_PREFIX + str(val), rowid)

    if not pairs:
        # Create empty trie
        trie = marisa_trie.RecordTrie("<I", [])
    else:
        trie = marisa_trie.RecordTrie("<I", pairs)

    trie.save(trie_path)
    return len(pairs)


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def create_package_zip(pkg_dir: str, pkg_id: str, version: str) -> str:
    """Create zip archive for package. Returns zip path."""
    zip_name = f"{pkg_id}-v{version}.zip"
    zip_path = os.path.join(PACKAGES_DIR, zip_name)

    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as zf:
        for filename in [f"{pkg_id}.db", f"{pkg_id}.trie", "manifest.json"]:
            filepath = os.path.join(pkg_dir, filename)
            if os.path.exists(filepath):
                zf.write(filepath, filename)

    return zip_path


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, DICTIONARY_CSV, PACKAGES_DIR)

    if not os.path.exists(DICTIONARY_CSV):
        logger.error(f"Dictionary CSV not found: {DICTIONARY_CSV}")
        sys.exit(1)

    version = datetime.now().strftime("%Y%m%d")
    df = pd.read_csv(DICTIONARY_CSV)
    logger.info(f"Loaded {len(df)} records from dictionary.csv")

    os.makedirs(PACKAGES_DIR, exist_ok=True)

    remote_manifest: dict = {
        "format_version": 1,
        "version": version,
        "build_date": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
        "dictionaries": [],
    }

    for pkg in PACKAGES:
        pkg_id = pkg["id"]
        pkg_name = pkg["name"]
        filter_cols = pkg["filter"]

        logger.info(f"\n  [{pkg_id}] Filtering by: {filter_cols}")

        filtered = filter_rows(df, filter_cols)
        if filtered.empty:
            logger.warning(f"  [{pkg_id}] No entries found, skipping")
            continue

        logger.info(f"  [{pkg_id}] {len(filtered)} entries matched")

        # Create package directory
        pkg_dir = os.path.join(PACKAGES_DIR, pkg_id)
        os.makedirs(pkg_dir, exist_ok=True)

        # Create db
        db_path = os.path.join(pkg_dir, f"{pkg_id}.db")
        entry_count = create_package_db(filtered, db_path, pkg_id, version)
        logger.info(f"  [{pkg_id}] DB: {entry_count} entries")

        # Create trie
        trie_path = os.path.join(pkg_dir, f"{pkg_id}.trie")
        key_count = create_package_trie(filtered, db_path, trie_path)
        logger.info(f"  [{pkg_id}] Trie: {key_count} keys")

        # Per-package manifest
        db_size = os.path.getsize(db_path)
        trie_size = os.path.getsize(trie_path)
        pkg_manifest = {
            "id": pkg_id,
            "name": pkg_name,
            "version": version,
            "entries": entry_count,
            "db_size_bytes": db_size,
            "trie_size_bytes": trie_size,
            "db_sha256": sha256_file(db_path),
            "trie_sha256": sha256_file(trie_path),
        }

        manifest_path = os.path.join(pkg_dir, "manifest.json")
        with open(manifest_path, "w", encoding="utf-8") as f:
            json.dump(pkg_manifest, f, ensure_ascii=False, indent=2)

        # Create zip
        zip_path = create_package_zip(pkg_dir, pkg_id, version)
        zip_size = os.path.getsize(zip_path)
        logger.info(f"  [{pkg_id}] Zip: {zip_size / 1024:.1f} KB")

        # Add to remote manifest
        remote_manifest["dictionaries"].append({
            "id": pkg_id,
            "name": pkg_name,
            "version": version,
            "entries": entry_count,
            "download_size_bytes": zip_size,
            "installed_size_bytes": db_size + trie_size,
            "checksum_sha256": sha256_file(zip_path),
        })

    # Write remote manifest
    remote_manifest_path = os.path.join(PACKAGES_DIR, "remote-manifest.json")
    with open(remote_manifest_path, "w", encoding="utf-8") as f:
        json.dump(remote_manifest, f, ensure_ascii=False, indent=2)

    # Summary
    logger.info(f"\n  [summary]")
    logger.info(f"  {'Package':<12} {'Entries':>8} {'Installed':>12} {'Zip':>10}")
    logger.info(f"  {'-' * 44}")
    total_entries = 0
    total_zip = 0
    for d in remote_manifest["dictionaries"]:
        installed_kb = d["installed_size_bytes"] / 1024
        zip_kb = d["download_size_bytes"] / 1024
        logger.info(f"  {d['id']:<12} {d['entries']:>8} {installed_kb:>9.1f} KB {zip_kb:>7.1f} KB")
        total_entries += d["entries"]
        total_zip += d["download_size_bytes"]
    logger.info(f"  {'-' * 44}")
    logger.info(f"  {'Total':<12} {total_entries:>8} {'':>12} {total_zip / 1024:>7.1f} KB")
    logger.info(f"\n  Remote manifest: {remote_manifest_path}")
    logger.info("Done!")


if __name__ == "__main__":
    main()

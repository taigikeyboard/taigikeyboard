#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
測試 fst 前綴索引查詢

用法：
    python query_fst.py <prefix>
    python query_fst.py tl:gua
    python query_fst.py poj:goa
    python query_fst.py hanzi:好

支援前綴查詢；exact-match 由前綴查詢自然涵蓋（key 完全相符 = 前綴長度等於 key 長度）。
"""

import shutil
import sqlite3
import subprocess
import sys
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
FST_FILE = BASE_DIR / "output" / "dictionary.fst"
DB_FILE = BASE_DIR / "output" / "trie.db"
ENGINE_DIR = BASE_DIR.parent / "engine"
BUILDER_RELEASE = ENGINE_DIR / "target" / "release" / "fst-builder"
BUILDER_DEBUG = ENGINE_DIR / "target" / "debug" / "fst-builder"


def resolve_builder_bin() -> Path:
    if BUILDER_RELEASE.exists():
        return BUILDER_RELEASE
    if BUILDER_DEBUG.exists():
        return BUILDER_DEBUG
    cargo = shutil.which("cargo")
    if not cargo:
        sys.exit(
            "fst-builder binary missing and `cargo` not in PATH; build it manually "
            f"with `cargo build --release -p fst-builder` from {ENGINE_DIR}"
        )
    subprocess.run(
        [cargo, "build", "--release", "-p", "fst-builder"],
        cwd=ENGINE_DIR,
        check=True,
    )
    return BUILDER_RELEASE


def main() -> None:
    if len(sys.argv) < 2:
        print("用法: python query_fst.py <prefix>")
        print("範例: python query_fst.py tl:gua")
        return

    prefix = sys.argv[1]
    if not FST_FILE.exists():
        sys.exit(f"fst file not found: {FST_FILE} — run ./build.sh first")

    builder_bin = resolve_builder_bin()
    proc = subprocess.run(
        [str(builder_bin), "query", str(FST_FILE), prefix],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        sys.exit(
            f"fst-builder query failed: {proc.stderr.decode('utf-8', errors='replace')}"
        )

    lines = proc.stdout.decode("utf-8", errors="replace").splitlines()
    if not lines:
        print(f"No matches for prefix `{prefix}`")
        sys.stderr.write(proc.stderr.decode("utf-8", errors="replace"))
        return

    print(f"Query prefix: '{prefix}'")
    print("=" * 50)
    print(f"\n[hits] {len(lines)} entries (key\\trowid)")
    print("-" * 50)

    rowids: list[int] = []
    for line in lines[:50]:
        parts = line.split("\t")
        if len(parts) == 2:
            print(f"  {line}")
            try:
                rowids.append(int(parts[1]))
            except ValueError:
                sys.stderr.write(
                    f"warn: skipping malformed rowid in line: {line!r}\n"
                )
    if len(lines) > 50:
        print(f"  ... 還有 {len(lines) - 50} 筆")

    if rowids and DB_FILE.exists():
        conn = sqlite3.connect(str(DB_FILE))
        cursor = conn.cursor()
        sample = rowids[:20]
        placeholders = ",".join("?" * len(sample))
        cursor.execute(
            f"SELECT id, tl_num, tl_notone, tl_abbrev FROM dictionary "
            f"WHERE id IN ({placeholders}) LIMIT 20",
            sample,
        )
        print("\n[sample resolved rows]")
        for row_id, tl_num, tl_notone, tl_abbrev in cursor.fetchall():
            parts = [f"num={tl_num}"]
            if tl_notone:
                parts.append(f"notone={tl_notone}")
            if tl_abbrev:
                parts.append(f"abbrev={tl_abbrev}")
            print(f"  [{row_id}] {', '.join(parts)}")
        conn.close()

    sys.stderr.write(proc.stderr.decode("utf-8", errors="replace"))


if __name__ == "__main__":
    main()

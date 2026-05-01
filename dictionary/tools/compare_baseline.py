#!/usr/bin/env python3
"""Capture or verify parity baseline for dictionary pipeline outputs.

Compares `output/dictionary.{csv,trie,bin,db}` + `output/association.bin` against a
baseline JSON captured before any refactor step. build_ts bytes in `.bin` files are
masked to zero so content-identical rebuilds match even when timestamps differ.

Usage:
  python3 tools/compare_baseline.py capture              # → dictionary/baseline.json
  python3 tools/compare_baseline.py verify               # diff current output vs baseline.json
  python3 tools/compare_baseline.py verify --json out    # verify, also emit diff JSON
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sqlite3
import sys
from pathlib import Path
from typing import Any

BASE_DIR = Path(__file__).resolve().parent.parent
OUTPUT_DIR = BASE_DIR / "output"
BASELINE_PATH = BASE_DIR / "baseline.json"

DICT_BIN_TS_OFFSET = 12  # bytes 12..15 in dictionary.bin
ASSOC_BIN_TS_OFFSET = 16  # bytes 16..19 in association.bin

ASSOC_SOURCE_COLS = (
    "kautian", "taigitv", "itaigi", "sitbut",
    "taihoa", "taijit", "kungge", "stti", "khpoo",
)


def hash_bytes_masked(data: bytes, mask_offset: int | None) -> str:
    if mask_offset is not None:
        data = data[:mask_offset] + b"\x00\x00\x00\x00" + data[mask_offset + 4 :]
    return hashlib.sha256(data).hexdigest()


def hash_file(path: Path, mask_offset: int | None = None) -> str:
    return hash_bytes_masked(path.read_bytes(), mask_offset)


def db_counters(db_path: Path) -> dict[str, Any]:
    conn = sqlite3.connect(db_path)
    try:
        cur = conn.cursor()
        result: dict[str, Any] = {}

        result["dictionary_rowcount"] = cur.execute(
            "SELECT COUNT(*) FROM dictionary"
        ).fetchone()[0]

        source_cols = [
            "kautian", "taigitv", "itaigi", "sitbut", "taihoa", "taijit",
            "kungge", "stti", "khpoo", "khiin", "dev", "lkk", "is_variant",
        ]
        sums_sql = ", ".join(f"SUM({c})" for c in source_cols)
        sums = cur.execute(f"SELECT {sums_sql} FROM dictionary").fetchone()
        result["dictionary_source_sums"] = dict(zip(source_cols, sums))

        result["dictionary_null_tl"] = cur.execute(
            "SELECT COUNT(*) FROM dictionary WHERE tl IS NULL OR tl = ''"
        ).fetchone()[0]
        result["dictionary_null_freq"] = cur.execute(
            "SELECT COUNT(*) FROM dictionary WHERE frequency IS NULL"
        ).fetchone()[0]
        result["dictionary_dupe_hanzi_tl"] = cur.execute(
            "SELECT COUNT(*) FROM ("
            "  SELECT hanzi, tl FROM dictionary GROUP BY hanzi, tl HAVING COUNT(*) > 1"
            ")"
        ).fetchone()[0]

        freq_hist = cur.execute(
            "SELECT (frequency / 1000) * 1000 AS bucket, COUNT(*) "
            "FROM dictionary GROUP BY bucket ORDER BY bucket"
        ).fetchall()
        result["dictionary_freq_hist"] = {str(b): c for b, c in freq_hist}

        combo = cur.execute(
            "SELECT kautian||taigitv||itaigi||sitbut||taihoa||taijit||kungge||stti||khpoo"
            "||khiin||dev||lkk||is_variant AS combo, COUNT(*) "
            "FROM dictionary GROUP BY combo ORDER BY combo"
        ).fetchall()
        result["dictionary_source_combos"] = {c: n for c, n in combo}

        result["word_association_rowcount"] = cur.execute(
            "SELECT COUNT(*) FROM word_association"
        ).fetchone()[0]
        result["word_association_count_sum"] = cur.execute(
            "SELECT COALESCE(SUM(count), 0) FROM word_association"
        ).fetchone()[0]
        # word_association has UNIQUE(prev_word, next_word, next_tl) so dupe is always 0,
        # but we still record it so any schema regression surfaces explicitly.
        result["word_association_dupe"] = cur.execute(
            "SELECT COUNT(*) FROM ("
            "  SELECT prev_word, next_word, next_tl FROM word_association"
            "  GROUP BY prev_word, next_word, next_tl HAVING COUNT(*) > 1"
            ")"
        ).fetchone()[0]
        assoc_source_sums = cur.execute(
            "SELECT SUM(kautian), SUM(taigitv), SUM(itaigi), SUM(sitbut),"
            "       SUM(taihoa),  SUM(taijit),  SUM(kungge), SUM(stti), SUM(khpoo) "
            "FROM word_association"
        ).fetchone()
        result["word_association_source_sums"] = dict(zip(
            ASSOC_SOURCE_COLS, assoc_source_sums,
        ))

        return result
    finally:
        conn.close()


def capture() -> dict[str, Any]:
    if not OUTPUT_DIR.is_dir():
        sys.exit(f"error: {OUTPUT_DIR} does not exist — run ./build.sh first")

    csv_path = OUTPUT_DIR / "dictionary.csv"
    fst_path = OUTPUT_DIR / "dictionary.fst"
    dict_bin = OUTPUT_DIR / "dictionary.bin"
    assoc_bin = OUTPUT_DIR / "association.bin"
    dict_db = OUTPUT_DIR / "dictionary.db"

    missing = [p for p in (csv_path, fst_path, dict_bin, assoc_bin, dict_db) if not p.exists()]
    if missing:
        sys.exit("error: missing output files: " + ", ".join(str(m) for m in missing))

    return {
        "files": {
            "dictionary.csv": {"sha256": hash_file(csv_path)},
            "dictionary.fst": {"sha256": hash_file(fst_path)},
            "dictionary.bin": {
                "sha256_masked_ts": hash_file(dict_bin, DICT_BIN_TS_OFFSET),
                "size": dict_bin.stat().st_size,
            },
            "association.bin": {
                "sha256_masked_ts": hash_file(assoc_bin, ASSOC_BIN_TS_OFFSET),
                "size": assoc_bin.stat().st_size,
            },
        },
        "db": db_counters(dict_db),
    }


def diff(baseline: dict[str, Any], current: dict[str, Any]) -> list[str]:
    diffs: list[str] = []

    def walk(prefix: str, a: Any, b: Any) -> None:
        if isinstance(a, dict) and isinstance(b, dict):
            keys = sorted(set(a) | set(b))
            for k in keys:
                walk(f"{prefix}.{k}" if prefix else k, a.get(k), b.get(k))
        elif a != b:
            diffs.append(f"{prefix}: baseline={a!r} current={b!r}")

    walk("", baseline, current)
    return diffs


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("capture", help="write baseline.json from current output/")
    v = sub.add_parser("verify", help="compare current output/ against baseline.json")
    v.add_argument("--json", type=Path, help="write full diff JSON here")
    v.add_argument("--baseline", type=Path, default=BASELINE_PATH)
    args = ap.parse_args()

    if args.cmd == "capture":
        data = capture()
        BASELINE_PATH.write_text(json.dumps(data, indent=2, ensure_ascii=False))
        print(f"baseline captured → {BASELINE_PATH}")
        print(f"  dictionary rows: {data['db']['dictionary_rowcount']:,}")
        print(f"  word_association rows: {data['db']['word_association_rowcount']:,}")
        return 0

    if args.cmd == "verify":
        if not args.baseline.exists():
            sys.exit(f"error: baseline not found: {args.baseline}")
        baseline = json.loads(args.baseline.read_text())
        current = capture()
        mismatches = diff(baseline, current)
        if args.json:
            args.json.write_text(json.dumps(
                {"baseline": baseline, "current": current, "diff": mismatches},
                indent=2, ensure_ascii=False,
            ))
        if mismatches:
            print(f"FAIL: {len(mismatches)} mismatch(es)")
            for m in mismatches[:40]:
                print(f"  {m}")
            if len(mismatches) > 40:
                print(f"  ... ({len(mismatches) - 40} more)")
            return 1
        print("OK: output matches baseline")
        return 0

    return 2


if __name__ == "__main__":
    sys.exit(main())

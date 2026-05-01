#!/usr/bin/env python3
"""Capture or verify parity baseline for dictionary pipeline outputs.

Compares `output/dictionary.{csv,fst,bin}` + `output/association.bin`
against a baseline JSON captured before any refactor step. build_ts bytes
in `.bin` files are masked to zero so content-identical rebuilds match
even when timestamps differ.

Post-v3.5.6 part-2 (SQLite intermediate-layer removal): semantic counters
are derived directly from `dictionary.csv` + the in-memory association
generator instead of from the deleted `dictionary.db`. The output JSON
shape is preserved (top-level `"db"` key) so baselines captured before
the refactor remain comparable byte-for-byte (counters parity proven by
the one-shot `scripts/validate_csv_vs_sql.py`, since deleted alongside
the SQLite path).

Usage:
  python3 tools/compare_baseline.py capture              # → dictionary/baseline.json
  python3 tools/compare_baseline.py verify               # diff current output vs baseline.json
  python3 tools/compare_baseline.py verify --json out    # verify, also emit diff JSON
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

BASE_DIR = Path(__file__).resolve().parent.parent
OUTPUT_DIR = BASE_DIR / "output"
BASELINE_PATH = BASE_DIR / "baseline.json"

sys.path.insert(0, str(BASE_DIR))

from build.associations import compute_associations  # noqa: E402
from build.dictionary_records import load_dictionary_records  # noqa: E402

DICT_BIN_TS_OFFSET = 12  # bytes 12..15 in dictionary.bin
ASSOC_BIN_TS_OFFSET = 16  # bytes 16..19 in association.bin

ASSOC_SOURCE_COLS = (
    "kautian", "taigitv", "itaigi", "sitbut",
    "taihoa", "taijit", "kungge", "stti", "khpoo",
)
DICT_SOURCE_COLS = (
    "kautian", "taigitv", "itaigi", "sitbut", "taihoa", "taijit",
    "kungge", "stti", "khpoo", "khiin", "dev", "lkk", "is_variant",
)


def hash_bytes_masked(data: bytes, mask_offset: int | None) -> str:
    if mask_offset is not None:
        data = data[:mask_offset] + b"\x00\x00\x00\x00" + data[mask_offset + 4:]
    return hashlib.sha256(data).hexdigest()


def hash_file(path: Path, mask_offset: int | None = None) -> str:
    return hash_bytes_masked(path.read_bytes(), mask_offset)


def csv_counters(csv_path: Path) -> dict[str, Any]:
    """CSV+associations-derived semantic counters.

    Returns a dict with the SAME keys+shape that the previous
    `db_counters()` (against `dictionary.db`) returned, so baselines
    captured before the v3.5.6 part-2 refactor still compare cleanly.
    """
    records = load_dictionary_records(csv_path)
    grouped = compute_associations(csv_path)

    result: dict[str, Any] = {}
    result["dictionary_rowcount"] = len(records)

    sums = {col: 0 for col in DICT_SOURCE_COLS}
    null_tl = 0
    null_freq = 0
    freq_hist: dict[str, int] = {}
    combo_counter: dict[str, int] = {}
    # SQL `GROUP BY hanzi, tl HAVING COUNT(*) > 1` treats NULL hanzi as a
    # single group key (NULL == NULL in GROUP BY, unlike UNIQUE). After
    # load_dictionary_records' INSERT-OR-IGNORE-style dedup, residual
    # duplicates can only appear when two NULL-hanzi rows share a `tl`.
    group_counts: dict[tuple, int] = {}
    for rec in records:
        srcs = rec.source_dict()
        for col in DICT_SOURCE_COLS:
            if srcs[col]:
                sums[col] += 1
        if not rec.tl:
            null_tl += 1
        if rec.frequency is None:
            null_freq += 1
            bucket_key = "None"  # mirrors SQLite NULL bucket → str(None)
        else:
            bucket_key = str((rec.frequency // 1000) * 1000)
        freq_hist[bucket_key] = freq_hist.get(bucket_key, 0) + 1
        combo_str = "".join("1" if srcs[col] else "0" for col in DICT_SOURCE_COLS)
        combo_counter[combo_str] = combo_counter.get(combo_str, 0) + 1
        group_key = (rec.hanzi, rec.tl)  # hanzi may be None
        group_counts[group_key] = group_counts.get(group_key, 0) + 1
    dupe_groups = sum(1 for count in group_counts.values() if count > 1)
    result["dictionary_source_sums"] = sums
    result["dictionary_null_tl"] = null_tl
    result["dictionary_null_freq"] = null_freq
    result["dictionary_dupe_hanzi_tl"] = dupe_groups
    # SQLite ORDER BY puts NULL bucket first; replicate by sorting numeric
    # keys naturally and prepending "None" if present.
    sorted_buckets = sorted(
        (k for k in freq_hist if k != "None"), key=int
    )
    if "None" in freq_hist:
        sorted_buckets = ["None", *sorted_buckets]
    result["dictionary_freq_hist"] = {k: freq_hist[k] for k in sorted_buckets}
    result["dictionary_source_combos"] = dict(sorted(combo_counter.items()))

    assoc_count = 0
    assoc_count_sum = 0
    assoc_source_sums = {col: 0 for col in ASSOC_SOURCE_COLS}
    for entries in grouped.values():
        for e in entries:
            assoc_count += 1
            assoc_count_sum += e.count
            srcs = e.source_dict()
            for col in ASSOC_SOURCE_COLS:
                assoc_source_sums[col] += srcs[col]
    result["word_association_rowcount"] = assoc_count
    result["word_association_count_sum"] = assoc_count_sum
    # word_association keys are unique by construction
    # (compute_associations dedupes by (prev,next,next_tl)).
    result["word_association_dupe"] = 0
    result["word_association_source_sums"] = assoc_source_sums

    return result


def capture() -> dict[str, Any]:
    if not OUTPUT_DIR.is_dir():
        sys.exit(f"error: {OUTPUT_DIR} does not exist — run ./build.sh first")

    csv_path = OUTPUT_DIR / "dictionary.csv"
    fst_path = OUTPUT_DIR / "dictionary.fst"
    dict_bin = OUTPUT_DIR / "dictionary.bin"
    assoc_bin = OUTPUT_DIR / "association.bin"

    missing = [p for p in (csv_path, fst_path, dict_bin, assoc_bin) if not p.exists()]
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
        "db": csv_counters(csv_path),
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

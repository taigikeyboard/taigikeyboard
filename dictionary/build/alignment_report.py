# -*- coding: utf-8 -*-
"""Build-time non-fatal Hanji/TL 1:1 syllable alignment report.

Lists every `output/dictionary.csv` entry where `hanzi` is non-empty, not
kana-only, and cannot be aligned 1:1 with `count_syllables(tl)` by the
Hanji/Han-Lo syllable tokenizer.
"""

from __future__ import annotations

import csv
from pathlib import Path

from build.common import MERGED_CSV, OUTPUT_DIR
from common.cleanup import count_syllables, is_kana_only_hanzi, tokenize_hanji_syllables
from common.source_bits import MAIN_SOURCE_COLUMNS

REPORT_FILE = OUTPUT_DIR / "alignment_report.txt"
ALL_SOURCE_COLS = (*MAIN_SOURCE_COLUMNS, "dev", "lkk", "khiin")


def find_unalignable_rows(dictionary_csv: Path = MERGED_CSV) -> list[tuple[str, str, str]]:
    """Return sorted `(hanzi, tl, sources)` rows that cannot align 1:1."""
    unalignable: list[tuple[str, str, str]] = []
    with dictionary_csv.open(encoding="utf-8", newline="") as f:
        for row in csv.DictReader(f):
            hanzi = row.get("hanzi", "")
            tl = row.get("tl", "")
            if not hanzi or is_kana_only_hanzi(hanzi):
                continue
            parsed = tokenize_hanji_syllables(hanzi)
            if parsed is None or len(parsed[0]) != count_syllables(tl):
                sources = [col for col in ALL_SOURCE_COLS if row.get(col) == "True"]
                unalignable.append((hanzi, tl, ",".join(sources)))
    unalignable.sort(key=lambda item: (item[0], item[1], item[2]))
    return unalignable


def write_alignment_report(
    dictionary_csv: Path = MERGED_CSV,
    report_path: Path = REPORT_FILE,
) -> list[tuple[str, str, str]]:
    """Write deterministic alignment report and return the unalignable rows."""
    rows = find_unalignable_rows(dictionary_csv)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "# Non-fatal Hanji/TL 1:1 syllable alignment report",
        f"# unalignable_count={len(rows)}",
        "hanzi\ttl\tsources",
        *(f"{hanzi}\t{tl}\t{sources}" for hanzi, tl, sources in rows),
    ]
    report_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return rows


def main(argv: list[str] | None = None) -> int:
    _ = argv
    if not MERGED_CSV.exists():
        print(f"[WARN] {MERGED_CSV} not found — skipping alignment report")
        return 0
    rows = write_alignment_report(MERGED_CSV, REPORT_FILE)
    print(f"Alignment report: {len(rows)} unalignable entries → {REPORT_FILE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

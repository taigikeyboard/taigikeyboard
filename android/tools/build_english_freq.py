#!/usr/bin/env python3
"""Generates the bundled English frequency wordlist asset for EN-mode autocomplete.

Deterministic — same source + same TOP_N always produce a byte-identical asset.

Source:  SymSpell frequency_dictionary_en_82_765.txt (Wolf Garbe, MIT license).
         https://github.com/wolfgarbe/SymSpell  — frequency derived from
         Google Books Ngram + SCOWL, freely redistributable.
Output:  android/app/src/main/assets/english_freq.txt  (word<TAB>count, freq-desc)

Filters: keep lowercase a-z words with optional internal apostrophes (don't,
         you're); drop numerics/symbols/hyphenated compounds. Emits the TOP_N
         most frequent valid words (source is already freq-sorted).

Usage:   android/tools/build_english_freq.py [path-to-symspell-source.txt]
         (no arg → downloads the source via urllib)
"""

from __future__ import annotations

import re
import sys
import urllib.request
from pathlib import Path

TOP_N = 30000
SRC_URL = (
    "https://raw.githubusercontent.com/wolfgarbe/SymSpell/master/"
    "SymSpell/frequency_dictionary_en_82_765.txt"
)
OUT_PATH = Path(__file__).resolve().parent.parent / "app/src/main/assets/english_freq.txt"

# Lowercase letters with optional internal apostrophe groups: don't, you're.
WORD_RE = re.compile(r"^[a-z]+(?:'[a-z]+)*$")


def read_source(arg: str | None) -> list[str]:
    if arg:
        # utf-8-sig transparently strips a leading BOM if present.
        return Path(arg).read_text(encoding="utf-8-sig").splitlines()
    with urllib.request.urlopen(SRC_URL, timeout=30) as response:  # noqa: S310 (trusted host)
        return response.read().decode("utf-8-sig").splitlines()


def main() -> int:
    lines = read_source(sys.argv[1] if len(sys.argv) > 1 else None)

    out_lines: list[str] = []
    for line in lines:
        parts = line.split()
        if len(parts) != 2:
            continue
        word, count = parts
        if not WORD_RE.match(word):
            continue
        out_lines.append(f"{word}\t{count}")
        if len(out_lines) >= TOP_N:
            break

    OUT_PATH.write_text("\n".join(out_lines) + "\n", encoding="utf-8")
    total_bytes = OUT_PATH.stat().st_size
    print(f"Wrote {OUT_PATH}: {len(out_lines)} words, {total_bytes} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

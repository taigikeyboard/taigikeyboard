# -*- coding: utf-8 -*-
"""Generate NextWord bigram + char-to-phrase associations from dictionary.csv.

Replaces the SQLite-backed `word_association` table that
build/generate_association.py used to write to `dictionary.db`.

The eligibility filter here is INTENTIONALLY DIFFERENT from
dictionary_records.load_dictionary_records: associations include 2-5
character entries, while runtime dictionary records are filtered at
<=4 syllables. Folding them onto the same filter would silently drop
valid associations (Codex pre-impl review Q1b).

Output is grouped by prev_word; entries within each group are sorted
by count DESC ONLY (no tie-breaker — adding one would drift the
association.bin SHA256, see Codex pre-impl review Q6).
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

import pandas as pd

from common.source_bits import ASSOC_SOURCE_COLUMNS

MIN_WORD_LEN = 2
MAX_WORD_LEN = 5         # 2-5 char hanzi entries seed associations
MAX_NEXT_WORD_LEN = 3    # phrase associations cap next_word at 3 chars


@dataclass(frozen=True)
class AssociationEntry:
    prev_word: str
    next_word: str
    next_tl: str
    count: int
    sources: tuple[tuple[str, int], ...]   # 0/1 per ASSOC_SOURCE_COLUMNS

    def source_dict(self) -> dict[str, int]:
        return dict(self.sources)


def _is_cjk(char: str) -> bool:
    code = ord(char)
    return (
        0x4E00 <= code <= 0x9FFF
        or 0x3400 <= code <= 0x4DBF
        or 0x20000 <= code <= 0x2CEAF
        or 0xF900 <= code <= 0xFAFF
    )


def _split_tl_syllables(tl) -> list[str]:
    if tl is None or pd.isna(tl):
        return []
    return [p for p in re.split(r"-+", str(tl)) if p]


def _generate_bigrams(
    hanzi: str, tl: str, frequency: int, sources: dict[str, int],
) -> list[dict]:
    tl_parts = _split_tl_syllables(tl)
    hanzi_chars = [c for c in hanzi if _is_cjk(c)]

    if len(hanzi_chars) < 2:
        return []
    if tl_parts and len(hanzi_chars) != len(tl_parts):
        return []

    out = []
    for i in range(len(hanzi_chars) - 1):
        next_tl = tl_parts[i + 1] if i + 1 < len(tl_parts) else ""
        out.append({
            "prev_word": hanzi_chars[i],
            "next_word": hanzi_chars[i + 1],
            "next_tl": next_tl,
            "count": frequency,
            "sources": sources.copy(),
        })
    return out


def _generate_phrase_associations(
    hanzi: str, tl: str, frequency: int, sources: dict[str, int],
) -> list[dict]:
    tl_parts = _split_tl_syllables(tl)
    hanzi_chars = [c for c in hanzi if _is_cjk(c)]

    if len(hanzi_chars) < 3:
        return []
    if tl_parts and len(hanzi_chars) != len(tl_parts):
        return []

    out = []
    for i in range(len(hanzi_chars) - 2):
        remaining = hanzi_chars[i + 1:]
        if len(remaining) > MAX_NEXT_WORD_LEN:
            continue
        next_phrase = "".join(remaining)
        next_tl = "-".join(tl_parts[i + 1:]) if len(tl_parts) > i + 1 else ""
        out.append({
            "prev_word": hanzi_chars[i],
            "next_word": next_phrase,
            "next_tl": next_tl,
            "count": frequency,
            "sources": sources.copy(),
        })
    return out


def compute_associations(
    csv_path: Path,
) -> dict[str, list[AssociationEntry]]:
    """Read dictionary.csv → return {prev_word: [entries sorted count DESC]}.

    Outer key iteration order is dict insertion order (= first-seen
    prev_word in CSV). Callers MUST re-sort keys by UTF-8 bytes for
    binary stability (kept as caller responsibility so this module
    doesn't dictate output binary layout).

    Inner sort within each group is `count DESC` with no tiebreaker —
    Python's sort is stable, so entries with equal counts retain their
    accumulation order. This matches SQLite's `ORDER BY count DESC`
    rowid-implicit ordering (entries inserted in CSV iteration order).
    """
    from common import read_dictionary_csv
    df = read_dictionary_csv(csv_path)

    df_multi = df[df["hanzi"].apply(
        lambda x: isinstance(x, str) and MIN_WORD_LEN <= len(x) <= MAX_WORD_LEN
    )]

    accum: dict[tuple[str, str, str], dict] = {}

    for _, row in df_multi.iterrows():
        hanzi = row["hanzi"]
        if pd.isna(hanzi) or len(hanzi) < MIN_WORD_LEN:
            continue
        tl = row.get("tl", "") or ""

        # NaN-safe frequency parse — `int(NaN)` raises ValueError. Preserve
        # the SQLite-era end-to-end semantics, NOT the literal source
        # (Codex PR #200 r3173989541): the old `... or 1` short-circuit in
        # generate_association.py did NOT fire for NaN (NaN is Python-truthy
        # → `NaN or 1` is NaN), so blank cells flowed through as NaN, were
        # stored as SQLite NULL, then encoded by create_association_bin's
        # `count = row["count"] or 0` to 0. The `or 1` only normalised
        # explicit zero. Mirror that here.
        freq_raw = row.get("frequency")
        if pd.isna(freq_raw):
            frequency = 0
        else:
            frequency = int(freq_raw) or 1

        # Explicit truthy check — `bool(NaN)` is True, so a missing source
        # cell would otherwise be promoted to a set bit. Match the original
        # generate_association.py logic: only literal True / 1 / "1" count.
        sources = {}
        for col in ASSOC_SOURCE_COLUMNS:
            val = row.get(col, 0)
            sources[col] = 1 if (val == 1 or val == "1" or val is True) else 0

        all_assocs = (
            _generate_bigrams(hanzi, tl, frequency, sources)
            + _generate_phrase_associations(hanzi, tl, frequency, sources)
        )

        for assoc in all_assocs:
            key = (assoc["prev_word"], assoc["next_word"], assoc["next_tl"])
            if key in accum:
                existing = accum[key]
                merged_sources = {
                    col: max(existing["sources"][col], assoc["sources"][col])
                    for col in ASSOC_SOURCE_COLUMNS
                }
                accum[key] = {
                    "count": existing["count"] + assoc["count"],
                    "sources": merged_sources,
                }
            else:
                accum[key] = {
                    "count": assoc["count"],
                    "sources": assoc["sources"],
                }

    grouped: dict[str, list[AssociationEntry]] = {}
    for (prev, nxt, nxt_tl), val in accum.items():
        grouped.setdefault(prev, []).append(AssociationEntry(
            prev_word=prev,
            next_word=nxt,
            next_tl=nxt_tl,
            count=val["count"],
            sources=tuple((col, val["sources"][col]) for col in ASSOC_SOURCE_COLUMNS),
        ))

    for prev in grouped:
        grouped[prev].sort(key=lambda e: e.count, reverse=True)

    return grouped

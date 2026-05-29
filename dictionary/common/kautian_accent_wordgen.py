# -*- coding: utf-8 -*-
"""kautian req2 — word-level accent variant generation.

Phase 5 (final phase) of the kautian subcollections feature. The 語音差異
sheet is single-character only (407 rows): each 漢字 carries one TL reading per
dialect accent (鹿港偏泉腔 … 臺中偏漳腔). Multi-character headwords only ever
carry the one standard 教育部 reading. This module generates WORD-LEVEL accent
variants by substituting each syllable of a multi-char main entry with that
char's per-accent reading, then tags the generated row with the accent bits
that produced it.

Two-syllable example — 八月 (base "peh-gue̍h", 八∈語音差異 as pueh/peh, 月∉):
the 泉腔 accents read 八 as "pueh" → generate "pueh-gue̍h" tagged with those
accents' bits; the 漳腔 accents read 八 as "peh" == base → no row (the base
headword already covers it, gated independently by the main toggle, DD6).

Design (USER-locked decisions, see project memory `project_kautian_subcollections`):

- **DD9 same-morpheme gate** — a syllable is only substituted when its base
  reading is itself one of that char's listed dialect readings. 八卦's 八/pat
  (文讀) is NOT in 八's 語音差異 set {pueh, peh} (白讀), so 八卦 generates
  nothing — different morpheme, 一字多音.
- **DD4 conservative skip** — only `漢字數==音節數` tokenizable rows; a
  comma-multi accent cell (新竹 女="lír,lí") skips that one accent bit, not the
  whole word; generated readings that fail strict TL→POJ conversion are skipped
  + reported (prevents the downstream `poj` stage's whole-source abort).
- **DD4b equal rank** — generated rows inherit the base word's `frequency` and
  the kautian source tier; accents are equal, none demoted. Candidate flood is
  managed by the per-accent toggles, not by ranking.
- **DD1 accent bitset** — two accents producing the same reading share one row
  whose `kautian_accent_mask` is the OR of their bits.

The core (`generate_variants_for_word`, `apply_word_accent_generation`) is
bridge-free; the POJ-convertibility check is injected so tests stay offline.
"""

from __future__ import annotations

from dataclasses import dataclass, field
import re
from typing import Callable

import pandas as pd

from common.cleanup import hanzi_chars
from common.kautian_provenance import (
    ACCENT_COLUMN_COUNT,
    COL_ACCENT_MASK,
    COL_MAIN,
    COL_NAME,
    ProvenanceKey,
)

# Splits a TL reading into syllables while KEEPING the separators, so a
# generated reading can be rejoined with the base's original hyphen/space
# word boundaries: re.split with a capture group interleaves tokens and
# separators (["peh", "-", "gue̍h"]).
_SEPARATOR_SPLIT = re.compile(r"([\s\-]+)")

CharAccentTable = dict[str, dict[str, int]]


@dataclass
class WordGenResult:
    """Per-word generation outcome.

    `variants` maps each generated reading → OR of the accent bits that
    produced it (only readings that differ from the base are kept). `aligned`
    is False when 漢字數≠音節數 or the row is not tokenizable (skipped).
    `ambiguous_accents` counts accent bits skipped because the char's reading
    for that accent was comma-multi.
    """

    variants: dict[str, int] = field(default_factory=dict)
    aligned: bool = True
    ambiguous_accents: int = 0


@dataclass
class WordGenReport:
    """Run-wide counters for the stage log."""

    main_rows_scanned: int = 0
    misaligned_rows: int = 0
    words_generating: int = 0
    rows_emitted: int = 0
    collisions_merged: int = 0
    skipped_nonconvertible: int = 0
    ambiguous_accents_skipped: int = 0


def build_char_accent_table(
    accent_map: dict[ProvenanceKey, int],
) -> CharAccentTable:
    """Group the Phase-1 accent map `(hanzi, reading) → bits` by hanzi into
    `hanzi → {reading: bits}`, the inverse lookup req2 substitution needs."""
    table: CharAccentTable = {}
    for (hanzi, reading), bits in accent_map.items():
        table.setdefault(hanzi, {})[reading] = bits
    return table


def _split_syllables(tl: str) -> tuple[list[str], list[str]]:
    """Tokenize a TL reading into (syllables, separators) preserving boundaries.

    `separators[i]` is the boundary between `syllables[i]` and
    `syllables[i+1]`, so the reading rejoins as
    `syllables[0] + separators[0] + syllables[1] + …`.
    """
    parts = _SEPARATOR_SPLIT.split(tl.strip())
    syllables = parts[0::2]
    separators = parts[1::2]
    # A trailing separator (empty final syllable) would desync the alignment.
    if syllables and syllables[-1] == "":
        syllables = syllables[:-1]
    return syllables, separators


def _rejoin(syllables: list[str], separators: list[str]) -> str:
    out = syllables[0]
    for sep, syl in zip(separators, syllables[1:]):
        out += sep + syl
    return out


# Sentinel: this char's reading for the requested accent is comma-multi, so the
# whole word-accent variant is skipped (DD4 conservative).
_AMBIGUOUS = object()


def _accent_reading(
    char_table: CharAccentTable, char: str, base_syllable: str, accent_bit: int
) -> object | None:
    """The reading `char` takes under `accent_bit`, or None to keep the base.

    Returns None when the char is not in 語音差異, when its base syllable is not
    one of its listed dialect readings (DD9 same-morpheme gate — different
    morpheme, keep base), or when that accent has no specific reading for the
    char. Returns `_AMBIGUOUS` when the accent's cell was comma-multi (>1
    reading carries this bit).
    """
    readings = char_table.get(char)
    if not readings or base_syllable not in readings:
        return None
    candidates = [r for r, bits in readings.items() if bits & (1 << accent_bit)]
    if not candidates:
        return None
    if len(candidates) > 1:
        return _AMBIGUOUS
    return candidates[0]


def generate_variants_for_word(
    hanzi: str,
    tl: str,
    char_table: CharAccentTable,
    accent_count: int = ACCENT_COLUMN_COUNT,
) -> WordGenResult:
    """Generate word-level accent readings for one multi-char main entry."""
    chars = hanzi_chars(hanzi)
    syllables, separators = _split_syllables(tl)
    if not chars or not syllables or len(chars) != len(syllables):
        return WordGenResult(aligned=False)
    if len(chars) < 2:
        # Single-char main entry — nothing to generate (req2 is word-level),
        # but it is aligned, not a 漢羅 mismatch.
        return WordGenResult(aligned=True)

    result = WordGenResult()
    for accent_bit in range(accent_count):
        new_syllables: list[str] = []
        changed = False
        ambiguous = False
        for char, base_syllable in zip(chars, syllables):
            sub = _accent_reading(char_table, char, base_syllable, accent_bit)
            if sub is _AMBIGUOUS:
                ambiguous = True
                break
            if sub is None:
                new_syllables.append(base_syllable)
            else:
                new_syllables.append(sub)  # type: ignore[arg-type]
                if sub != base_syllable:
                    changed = True
        if ambiguous:
            result.ambiguous_accents += 1
            continue
        if not changed:
            continue
        reading = _rejoin(new_syllables, separators)
        result.variants[reading] = result.variants.get(reading, 0) | (1 << accent_bit)
    return result


def apply_word_accent_generation(
    df: pd.DataFrame,
    accent_map: dict[ProvenanceKey, int],
    poj_convertible: Callable[[str], bool],
    accent_count: int = ACCENT_COLUMN_COUNT,
) -> tuple[pd.DataFrame, WordGenReport]:
    """Append word-level accent variant rows to a cleaned kautian DataFrame.

    For every multi-char `kautian_main` row, generate per-accent readings
    (`generate_variants_for_word`), validate POJ-convertibility (`poj_convertible`
    is cached per reading), then either OR the accent mask into an existing
    `(hanzi, tl)` row or append a new row inheriting the base `frequency`
    (DD4b). Collision is resolved HERE rather than at the cross-source merge
    because the `variants` stage runs in between and would duplicate-replicate a
    same-`(hanzi, tl)` row. Returns the new DataFrame + a run report.
    """
    char_table = build_char_accent_table(accent_map)
    report = WordGenReport()

    # (hanzi, tl) → df index for in-place mask OR on collision. merge dedup'd
    # the kautian df on (hanzi, tl), so this is 1:1.
    existing_index: dict[ProvenanceKey, int] = {
        (str(h), str(t)): idx
        for idx, h, t in zip(df.index, df["hanzi"], df["tl"])
    }
    existing_mask_add: dict[int, int] = {}
    new_rows: dict[ProvenanceKey, dict] = {}
    poj_cache: dict[str, bool] = {}

    # One vectorized extraction of the main rows (avoids ~tens-of-thousands of
    # per-row df.loc Series builds); each record is a plain dict we copy on emit.
    main_mask = df[COL_MAIN].fillna(False).astype(bool)
    for base_row in df.loc[main_mask].to_dict("records"):
        hanzi = str(base_row["hanzi"])
        base_tl = str(base_row["tl"])
        report.main_rows_scanned += 1

        gen = generate_variants_for_word(hanzi, base_tl, char_table, accent_count)
        if not gen.aligned:
            report.misaligned_rows += 1
            continue
        report.ambiguous_accents_skipped += gen.ambiguous_accents
        if not gen.variants:
            continue

        emitted_any = False
        for reading, mask in gen.variants.items():
            ok = poj_cache.get(reading)
            if ok is None:
                ok = poj_convertible(reading)
                poj_cache[reading] = ok
            if not ok:
                report.skipped_nonconvertible += 1
                continue

            key = (hanzi, reading)
            if key in existing_index:
                target = existing_index[key]
                existing_mask_add[target] = existing_mask_add.get(target, 0) | mask
                report.collisions_merged += 1
            elif key in new_rows:
                new_rows[key][COL_ACCENT_MASK] |= mask
            else:
                new_row = dict(base_row)
                new_row["tl"] = reading
                new_row[COL_MAIN] = False
                new_row[COL_ACCENT_MASK] = mask
                new_row[COL_NAME] = False
                new_rows[key] = new_row
                report.rows_emitted += 1
            emitted_any = True
        if emitted_any:
            report.words_generating += 1

    for target, add in existing_mask_add.items():
        current = df.at[target, COL_ACCENT_MASK]
        base = 0 if pd.isna(current) else int(current)
        df.at[target, COL_ACCENT_MASK] = base | add

    if new_rows:
        df = pd.concat(
            [df, pd.DataFrame(list(new_rows.values()))], ignore_index=True
        )
    # Deterministic order (matches the frequency stage) for reproducible output.
    df = df.sort_values(
        ["frequency", "hanzi", "tl"],
        ascending=[False, True, True],
        ignore_index=True,
    )
    return df, report

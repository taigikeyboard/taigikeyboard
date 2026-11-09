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

import csv
import logging
import re
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

import pandas as pd

from build.dictionary_records import load_dictionary_records
from common.cjk import is_cjk
from common.source_bits import ASSOC_SOURCE_COLUMNS
from common.variants import VARIANTS_CSV, read_variant_rows

MIN_WORD_LEN = 2
MAX_WORD_LEN = 5         # 2-5 char hanzi entries seed associations
MAX_NEXT_WORD_LEN = 3    # phrase associations cap next_word at 3 chars

# --- word namespace (association.bin v2, bigram LM roadmap P3) ---------------
# Key = `prev_hanji + WORD_KEY_SEPARATOR + prev_tl` (display TL, as the
# dictionary and the user store carry it).
# Mirrors `engine/lexicon/src/association_reader.rs` (`word_key`).
WORD_KEY_SEPARATOR = "\x01"
WORD_TOP_K = 30
WORD_MIN_COUNT = 2
# Per-source multipliers over the raw `word_bigrams.tsv` columns (USER
# 2026-09-28): Bible phrasing dominated some keys (`$→耶穌`), everything else
# counts once.
SOURCE_WEIGHTS = {"taigi_bible_nt": 0.5}
# `variants.csv` has no 個/个 row (6676637c: the classifier is deliberately not
# marked a 的 variant); the corpus writes the POJ-era 個 for 22k pairs.
EXTRA_VARIANT_FOLDS = {("個", "ê"): "个"}

log = logging.getLogger(__name__)


def word_key(hanji: str, tl: str) -> str:
    return hanji + WORD_KEY_SEPARATOR + tl


@dataclass(frozen=True)
class AssociationEntry:
    prev_word: str
    next_word: str
    next_tl: str
    count: int
    sources: tuple[tuple[str, int], ...]   # 0/1 per ASSOC_SOURCE_COLUMNS

    def source_dict(self) -> dict[str, int]:
        return dict(self.sources)


def _split_tl_syllables(tl) -> list[str]:
    if tl is None or pd.isna(tl):
        return []
    return [p for p in re.split(r"-+", str(tl)) if p]


def _generate_bigrams(
    hanzi: str, tl: str, frequency: int, sources: dict[str, int],
) -> list[dict]:
    tl_parts = _split_tl_syllables(tl)
    hanzi_chars = [c for c in hanzi if is_cjk(c)]

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
    hanzi_chars = [c for c in hanzi if is_cjk(c)]

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

    for row in df_multi.to_dict("records"):
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


# --------------------------------------------------------------- word pairs
def load_variant_folds(variants_csv: Path = VARIANTS_CSV) -> dict[tuple[str, str], str]:
    """`(variant hanji, tl)` → 教典 recommended hanji, whole-word match only.

    A `(variant, tl)` that `variants.csv` maps to more than one recommended
    form (青/tshenn → 生 or 腥) is ambiguous and is not folded. Folding is one
    hop: 到/kah → 甲 stops there even though 甲/kah → 佮 exists (two words
    sharing one `(hanji, tl)` row — the chain would merge them).
    """
    targets: dict[tuple[str, str], set[str]] = defaultdict(set)
    for hanzi, variant, tl in read_variant_rows(variants_csv):
        targets[(variant, tl)].add(hanzi)
    folds = {key: next(iter(hanzi)) for key, hanzi in targets.items() if len(hanzi) == 1}
    folds.update(EXTRA_VARIANT_FOLDS)
    return folds


def weighted_count(counts_by_source: dict[str, int]) -> int:
    """Σ weight × per-source count, rounded half-up (never banker's rounding)."""
    total = sum(SOURCE_WEIGHTS.get(source, 1.0) * count for source, count in counts_by_source.items())
    return int(total + 0.5)


def compute_word_associations(
    bigrams_tsv: Path,
    dictionary_csv: Path,
    variants_csv: Path = VARIANTS_CSV,
) -> dict[str, list[AssociationEntry]]:
    """`word_bigrams.tsv` → {word key: [entries, count DESC, top-K]}.

    Both words of a pair are folded to their recommended hanji so variant
    spellings merge; the next word must be a dictionary `(hanzi, tl)` row
    (its source flags become the entry bitmask, so the source toggles keep
    filtering). Counts are the weighted sums; `WORD_MIN_COUNT` applies after
    weighting and merging.
    """
    folds = load_variant_folds(variants_csv)
    sources_by_word: dict[tuple[str, str], dict[str, int]] = {}
    for record in load_dictionary_records(dictionary_csv):
        if record.hanzi is not None:
            flags = record.source_dict()
            sources_by_word[(record.hanzi, record.tl)] = {
                col: int(flags[col]) for col in ASSOC_SOURCE_COLUMNS
            }
    # 34 `variants.csv` targets (毋好, 啉水, 袂使 …) are not dictionary rows;
    # the variant itself is one, so such a word stays unfolded.
    usable_folds = {
        (variant, tl): target
        for (variant, tl), target in folds.items()
        if (target, tl) in sources_by_word
    }
    unfoldable = sorted(
        f"{variant}/{tl}→{target}"
        for (variant, tl), target in folds.items()
        if (variant, tl) in sources_by_word and (variant, tl) not in usable_folds
    )
    if unfoldable:
        log.info("word associations: %d fold targets missing from dictionary.csv, kept as written: %s",
                 len(unfoldable), " ".join(unfoldable))

    def fold(hanji: str, tl: str) -> tuple[str, str]:
        return usable_folds.get((hanji, tl), hanji), tl

    accum: dict[tuple[str, tuple[str, str]], int] = defaultdict(int)
    dropped = 0
    with bigrams_tsv.open(encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        source_columns = [col for col in reader.fieldnames if col not in
                          ("prev_hanji", "prev_tl", "next_hanji", "next_tl", "count")]
        for row in reader:
            following = fold(row["next_hanji"], row["next_tl"])
            if following not in sources_by_word:
                dropped += 1
                continue
            key = word_key(*fold(row["prev_hanji"], row["prev_tl"]))
            accum[(key, following)] += weighted_count({col: int(row[col]) for col in source_columns})
    if dropped:
        log.info("word associations: %d rows whose next word is not a dictionary row dropped", dropped)

    by_key: dict[str, list[AssociationEntry]] = defaultdict(list)
    for (key, (next_word, next_tl)), count in accum.items():
        if count < WORD_MIN_COUNT:
            continue
        if count > 0xFFFF_FFFF:
            raise ValueError(f"count {count} for {key!r}→{next_word} exceeds u32")
        flags = sources_by_word[(next_word, next_tl)]
        by_key[key].append(AssociationEntry(
            prev_word=key,
            next_word=next_word,
            next_tl=next_tl,
            count=count,
            sources=tuple((col, flags[col]) for col in ASSOC_SOURCE_COLUMNS),
        ))
    def rank(entry: AssociationEntry) -> tuple:
        return (-entry.count, entry.next_word.encode("utf-8"), entry.next_tl.encode("utf-8"))

    return {key: sorted(entries, key=rank)[:WORD_TOP_K] for key, entries in by_key.items()}

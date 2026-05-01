# -*- coding: utf-8 -*-
"""Load dictionary.csv into runtime records with stable rowids.

Replaces the SQLite intermediate (`dictionary.db`) that previously held
dictionary records as the source-of-truth for `dictionary.bin` and the
hanzi half of `dictionary.fst`.

Filter + dedup mirror the original `INSERT OR IGNORE INTO dictionary` from
build/create_app_db.sh:
  - Drop rows where `tl` is empty or > MAX_SYLLABLES.
  - UNIQUE(tl, hanzi) — but NULL hanzi rows are NEVER deduped (SQLite
    UNIQUE allows multiple NULLs; collapsing them with a Python tuple key
    would silently drop valid pure-romanization rows).

Rowid is assigned by enumerate (1..N) in post-filter CSV order. This order
must match what create_app_db.sh produced via INSERT, otherwise the rowids
embedded in dictionary.fst values won't line up with dictionary.bin offsets
(see INV-1 / INV-2 in the part-2 pre-impl plan).
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pandas as pd

from common.source_bits import DICT_BIN_COLUMNS

REQUIRED_COLUMNS = (
    "hanzi", "tl", "frequency",
    "tl_num", "tl_notone", "tl_abbrev",
    "poj_num", "poj_notone", "poj_abbrev",
    *DICT_BIN_COLUMNS,
)

MAX_SYLLABLES = 4


@dataclass(frozen=True)
class DictionaryRecord:
    rowid: int
    hanzi: str | None
    tl: str
    # `frequency` stays `None` when the CSV cell was empty/NaN so the
    # parity counter in compare_baseline.py can faithfully reproduce
    # `SELECT COUNT(*) FROM dictionary WHERE frequency IS NULL`. Binary
    # writers coerce `None → 0` at encode time (see
    # create_dictionary_bin.py:encode_record).
    frequency: int | None
    tl_num: str | None
    tl_notone: str | None
    tl_abbrev: str | None
    poj_num: str | None
    poj_notone: str | None
    poj_abbrev: str | None
    sources: tuple[tuple[str, bool], ...]

    def source_dict(self) -> dict[str, bool]:
        return dict(self.sources)


def _syllable_count(tl: str) -> int:
    """Mirror SQL: count syllables after normalising spaces to hyphens."""
    return tl.replace(" ", "-").count("-") + 1


def _normalise_optional(value) -> str | None:
    if pd.isna(value):
        return None
    s = str(value)
    return s if s != "" else None


def load_dictionary_records(csv_path: Path) -> list[DictionaryRecord]:
    """Read dictionary.csv → return filtered, deduped records with rowids.

    The returned list's order MUST match what create_app_db.sh produced via
    INSERT OR IGNORE — both for byte-for-byte parity of dictionary.bin and
    for fst rowid alignment.
    """
    df = pd.read_csv(csv_path)
    missing = [c for c in REQUIRED_COLUMNS if c not in df.columns]
    if missing:
        raise RuntimeError(f"dictionary.csv missing columns: {missing}")

    records: list[DictionaryRecord] = []
    seen_non_null: set[tuple[str, str]] = set()

    for _, row in df.iterrows():
        tl_raw = row["tl"]
        if pd.isna(tl_raw):
            continue
        tl = str(tl_raw)
        if tl == "" or _syllable_count(tl) > MAX_SYLLABLES:
            continue

        hanzi = _normalise_optional(row["hanzi"])

        if hanzi is not None:
            key = (tl, hanzi)
            if key in seen_non_null:
                continue
            seen_non_null.add(key)
        # else: NULL hanzi — SQLite UNIQUE distinct-null semantics, never deduped.

        rowid = len(records) + 1
        sources = tuple((col, bool(row[col])) for col in DICT_BIN_COLUMNS)
        records.append(DictionaryRecord(
            rowid=rowid,
            hanzi=hanzi,
            tl=tl,
            frequency=int(row["frequency"]) if pd.notna(row["frequency"]) else None,
            tl_num=_normalise_optional(row["tl_num"]),
            tl_notone=_normalise_optional(row["tl_notone"]),
            tl_abbrev=_normalise_optional(row["tl_abbrev"]),
            poj_num=_normalise_optional(row["poj_num"]),
            poj_notone=_normalise_optional(row["poj_notone"]),
            poj_abbrev=_normalise_optional(row["poj_abbrev"]),
            sources=sources,
        ))

    return records

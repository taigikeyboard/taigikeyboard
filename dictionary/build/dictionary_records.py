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

from common.kautian_provenance import COL_ACCENT_MASK, COL_MAIN, COL_NAME
from common.source_bits import DICT_BIN_COLUMNS, encode_kautian_subtag

# kautian subcollection provenance columns (Phase 1 output). Present on every
# row of a freshly-built dictionary.csv (merge_csv fills 0/False for non-kautian
# rows); the loader defaults a missing column to "no provenance" but fails loud
# if kautian rows exist without them (stale CSV — `make dict` not re-run).
_PROVENANCE_COLUMNS = (COL_MAIN, COL_ACCENT_MASK, COL_NAME)

REQUIRED_COLUMNS = (
    "hanzi", "tl", "frequency",
    "tl_num", "tl_notone", "tl_abbrev",
    "poj_num", "poj_notone", "poj_abbrev",
    "tps_num", "tps_notone", "tps_abbrev",
    # C-3a er↔or dialect dual-emit: ㄜ→ㄛ variants of the three tps_*
    # columns. Empty when the source has no ㄜ (i.e. no `er`/`or` in TL).
    # Used by `create_fst.py` to emit `tps:<variant>` keys at the same
    # rowid, retiring the runtime `tps_or_mapped_to_er` branch.
    "tps_num_var", "tps_notone_var", "tps_abbrev_var",
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
    tps_num: str | None
    tps_notone: str | None
    tps_abbrev: str | None
    # C-3a er↔or dialect variants of the three `tps_*` columns. None when
    # the source row has no ㄜ to swap (i.e. no `er`/`or` in `tl_num`).
    tps_num_var: str | None
    tps_notone_var: str | None
    tps_abbrev_var: str | None
    sources: tuple[tuple[str, bool], ...]
    # Number of TL syllables in `tl`, computed from hyphen / space count
    # (see `_syllable_count`). Always 1..=MAX_SYLLABLES — out-of-range rows
    # are filtered before construction. Encoded as `u8` in dictionary.bin v2.
    syllable_count: int
    # kautian subcollection provenance packed into a u16 (dictionary.bin v3).
    # 0 for every non-kautian row. Layout (`source_bits.encode_kautian_subtag`):
    # bit 0 = has_main, bits 1..=10 = accent_mask, bit 11 = has_name.
    kautian_subtag: int

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


def _as_bool(value) -> bool:
    """Coerce a CSV cell to bool, robust to pandas dtype inference.

    `read_dictionary_csv` infers dtype, so a clean flag column arrives as
    `numpy.bool_`; a column with stray NaN arrives as object strings. Treat
    only the literal "true" (case-insensitive) / truthy non-string as True.
    """
    if pd.isna(value):
        return False
    if isinstance(value, str):
        return value.strip().lower() == "true"
    return bool(value)


def _kautian_subtag(row) -> int:
    accent_raw = row[COL_ACCENT_MASK]
    accent_mask = 0 if pd.isna(accent_raw) else int(float(accent_raw))
    return encode_kautian_subtag(
        has_main=_as_bool(row[COL_MAIN]),
        accent_mask=accent_mask,
        has_name=_as_bool(row[COL_NAME]),
    )


def load_dictionary_records(csv_path: Path) -> list[DictionaryRecord]:
    """Read dictionary.csv → return filtered, deduped records with rowids.

    The returned list's order MUST match what create_app_db.sh produced via
    INSERT OR IGNORE — both for byte-for-byte parity of dictionary.bin and
    for fst rowid alignment.
    """
    from common import read_dictionary_csv
    df = read_dictionary_csv(csv_path)
    missing = [c for c in REQUIRED_COLUMNS if c not in df.columns]
    if missing:
        raise RuntimeError(f"dictionary.csv missing columns: {missing}")

    has_provenance = all(c in df.columns for c in _PROVENANCE_COLUMNS)
    if (
        not has_provenance
        and "kautian" in df.columns
        and any(_as_bool(v) for v in df["kautian"])
    ):
        raise RuntimeError(
            "dictionary.csv has kautian rows but no kautian subcollection "
            f"provenance columns {_PROVENANCE_COLUMNS} — regenerate via "
            "`make dict` (Phase 1 kautian_provenance stage)."
        )

    records: list[DictionaryRecord] = []
    seen_non_null: set[tuple[str, str]] = set()

    for _, row in df.iterrows():
        tl_raw = row["tl"]
        if pd.isna(tl_raw):
            continue
        tl = str(tl_raw)
        syllable_count = _syllable_count(tl)
        if tl == "" or syllable_count > MAX_SYLLABLES:
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
            tps_num=_normalise_optional(row["tps_num"]),
            tps_notone=_normalise_optional(row["tps_notone"]),
            tps_abbrev=_normalise_optional(row["tps_abbrev"]),
            tps_num_var=_normalise_optional(row["tps_num_var"]),
            tps_notone_var=_normalise_optional(row["tps_notone_var"]),
            tps_abbrev_var=_normalise_optional(row["tps_abbrev_var"]),
            sources=sources,
            syllable_count=syllable_count,
            kautian_subtag=_kautian_subtag(row) if has_provenance else 0,
        ))

    return records

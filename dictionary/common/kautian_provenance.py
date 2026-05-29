# -*- coding: utf-8 -*-
"""kautian subcollection provenance — accent / name / main membership maps.

Phase 1 of the kautian subcollections feature. The kautian ODS carries three
gateable subcollections that the build pipeline used to flatten into one
undifferentiated `kautian` source bit:

- ``main`` — headword sheets: 詞目 / 又唸作 / 合音唸作 / 俗唸作 / 詞彙比較
- ``accent_differences`` — the 語音差異 sheet, one reading per dialect column
- ``name_appendix`` — the 名 / 姓 sheets (姓名附錄)

This module builds, from the raw sheets, per-``(hanzi, normalized-reading)``
membership and applies it to the cleaned DataFrame as three columns:
``kautian_main`` (bool), ``kautian_accent_mask`` (10-bit int), ``kautian_name``
(bool). The Phase 2 binary encoder consumes them; Phase 1 only emits them.

Keys use the SAME normalization the cleanup stage applies to ``tl``
(`normalize_roman`), so the maps join onto the post-cleanup rows even when the
raw reading was Capitalized (姓 "Káu" → "káu") or hyphen/space-variant.
"""

from __future__ import annotations

import pandas as pd

from common.cleanup import (
    clean_brackets,
    normalize_roman,
    remove_ellipsis,
    remove_prefix_hyphens,
)
from common.expand import split_variants

# ctx side-channel key for the maps stashed at `select`, applied at the
# `kautian_provenance` stage.
PROVENANCE_META_KEY = "kautian_provenance"

# Emitted CSV column names (also the Phase 2 encoder input names).
COL_MAIN = "kautian_main"
COL_ACCENT_MASK = "kautian_accent_mask"
COL_NAME = "kautian_name"

# Max accent bits the Phase 2 binary encoder reserves; the dialect-column
# list (config.yaml select.dialect_columns) is in bit order, index → bit.
ACCENT_COLUMN_COUNT = 10

ProvenanceKey = tuple[str, str]


# CROSS-REF: these mirror common/cleanup.py::cleanup_dataframe's per-column key
# transforms (steps 1-3 brackets/prefix/ellipsis + step 9 normalize_roman). The
# sequence is duplicated, not shared, because cleanup_dataframe applies it
# column-wise inline rather than via a composable. If cleanup's tl/hanzi
# transform changes, update here too or the maps silently fail to join.
def _clean_hanzi(hanzi: object) -> str:
    """Mirror cleanup_dataframe's hanzi key transform (clean_brackets +
    remove_ellipsis) so map keys match the post-cleanup `hanzi`."""
    return remove_ellipsis(clean_brackets(hanzi))


def _clean_reading(reading: object, preserve_spaces: bool) -> str:
    """Mirror cleanup_dataframe's `tl` key transform (clean_brackets →
    remove_prefix_hyphens → remove_ellipsis → normalize_roman) so map keys
    match the post-cleanup `tl`. Headword readings carry 【文】/【白】 bracket
    annotations that cleanup strips — without this, an annotated headword
    (八 = 【文】pat) would never match `main_set` and lose `kautian_main`."""
    text = clean_brackets(reading)
    text = remove_prefix_hyphens(text)
    text = remove_ellipsis(text)
    return normalize_roman(text, preserve_spaces=preserve_spaces)


def _key(hanzi: object, reading: object, preserve_spaces: bool) -> ProvenanceKey:
    return (_clean_hanzi(hanzi), _clean_reading(reading, preserve_spaces))


def build_accent_mask_map(
    dialect_df: pd.DataFrame,
    dialect_columns: list[str],
    preserve_spaces: bool,
    hanzi_col: str = "漢字",
) -> dict[ProvenanceKey, int]:
    """Map ``(hanzi, cleaned reading) → OR of accent bits``.

    `dialect_columns` is in bit order (index 0 → bit 0). A cell may hold
    comma/slash-separated readings (e.g. 新竹 女 = "lír,lí"); each gets the
    column's bit. The same reading shared across columns ORs their bits
    (八/pueh ∈ 鹿港/三峽/臺北/金門/馬公/新竹 → bits 0,1,2,6,7,8).
    """
    if len(dialect_columns) > ACCENT_COLUMN_COUNT:
        raise ValueError(
            f"dialect_columns has {len(dialect_columns)} entries; the Phase 2 "
            f"encoder reserves only {ACCENT_COLUMN_COUNT} accent bits"
        )
    out: dict[ProvenanceKey, int] = {}
    for _, row in dialect_df.iterrows():
        hanzi = row[hanzi_col]
        for bit, col in enumerate(dialect_columns):
            cell = row.get(col)
            if pd.isna(cell):
                continue
            for reading in split_variants(cell):
                key = _key(hanzi, reading, preserve_spaces)
                out[key] = out.get(key, 0) | (1 << bit)
    return out


def build_membership_set(
    sheets: dict[str, pd.DataFrame],
    sheet_names: list[str],
    preserve_spaces: bool,
    hanzi_col: str = "漢字",
    reading_col: str = "羅馬字",
) -> set[ProvenanceKey]:
    """Collect ``(hanzi, cleaned reading)`` keys from the named sheets."""
    out: set[ProvenanceKey] = set()
    for name in sheet_names:
        df = sheets.get(name)
        if df is None:
            continue
        for _, row in df.iterrows():
            cell = row.get(reading_col)
            if pd.isna(cell):
                continue
            for reading in split_variants(cell):
                out.add(_key(row.get(hanzi_col), reading, preserve_spaces))
    return out


def apply_provenance(
    df: pd.DataFrame,
    accent_map: dict[ProvenanceKey, int],
    name_set: set[ProvenanceKey],
    main_set: set[ProvenanceKey],
) -> pd.DataFrame:
    """Add the three provenance columns to a cleaned kautian DataFrame.

    `df["tl"]` is already cleanup-normalized, so it keys directly into the
    maps. Every kautian row originates from a main / accent / name sheet, so a
    row that is neither accent nor name is a headword by construction — hence
    `has_main` is True for those without consulting `main_set`. `main_set` is
    consulted only to confirm headword-ness for accent/name OVERLAP rows
    (a reading that is both a headword and an accent/name entry). This keeps
    DD6: such a word stays visible when its accent/name toggle is off. The
    default-True for the non-overlap case is also fail-safe — a headword is
    never hidden by a map miss.
    """
    df = df.copy()
    keys = list(
        zip(df["hanzi"].astype(str).str.strip(), df["tl"].astype(str))
    )
    masks = [accent_map.get(key, 0) for key in keys]
    names = [key in name_set for key in keys]
    # headword by construction unless it's purely an accent/name reading;
    # main_set re-confirms headword-ness for accent/name overlaps (DD6).
    mains = [
        (not mask and not name) or (key in main_set)
        for key, mask, name in zip(keys, masks, names)
    ]
    df[COL_MAIN] = mains
    df[COL_ACCENT_MASK] = masks
    df[COL_NAME] = names
    return df

# -*- coding: utf-8 -*-
"""
dictionary2 shared modules
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pandas as pd
import polars as pl

from .logging_utils import setup_logging
from .frequency import load_frequency_map, get_frequency, BASE_DEFAULT_FREQUENCY
from .abbrev import extract_abbrev
from .romanization import convert_tl_to_poj, add_roman_columns


def read_dictionary_csv(path: str | Path, **kwargs: Any) -> pd.DataFrame:
    """Read a dictionary-pipeline CSV preserving the literal string `"nan"`.

    Pandas' default `na_values` includes `"nan"`, which collides with valid
    POJ first-letter abbreviations (e.g. `niû-á-nn̄g` → `nan`). Empty cells
    stay NaN so int/bool dtypes still round-trip.
    """
    return pd.read_csv(path, keep_default_na=False, na_values=[""], **kwargs)


def read_dictionary_csv_polars(path: str | Path) -> pl.DataFrame:
    """Polars twin of `read_dictionary_csv` for a per-source or merged CSV.

    No type inference: every column is a string (`"nan"` included), null for
    an empty cell, except the source / provenance flags (bool) and
    `frequency` / the kautian accent mask (Int64).
    """
    from .kautian_provenance import COL_ACCENT_MASK, COL_ALT_READING, COL_MAIN, COL_NAME
    from .source_bits import DICT_BIN_COLUMNS

    bool_columns = (*DICT_BIN_COLUMNS, COL_MAIN, COL_ALT_READING, COL_NAME)
    int_columns = ("frequency", COL_ACCENT_MASK)
    df = pl.read_csv(path, infer_schema=False)
    return df.with_columns(
        *(pl.col(c) == "True" for c in bool_columns if c in df.columns),
        *(pl.col(c).cast(pl.Int64) for c in int_columns if c in df.columns),
    )


__all__ = [
    "setup_logging",
    "load_frequency_map",
    "get_frequency",
    "BASE_DEFAULT_FREQUENCY",
    "extract_abbrev",
    "convert_tl_to_poj",
    "add_roman_columns",
    "read_dictionary_csv",
    "read_dictionary_csv_polars",
]

# -*- coding: utf-8 -*-
"""
dictionary2 共用模組
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pandas as pd

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


__all__ = [
    "setup_logging",
    "load_frequency_map",
    "get_frequency",
    "BASE_DEFAULT_FREQUENCY",
    "extract_abbrev",
    "convert_tl_to_poj",
    "add_roman_columns",
    "read_dictionary_csv",
]

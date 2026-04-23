# -*- coding: utf-8 -*-
"""Frequency lookups for the pipeline + build/ scripts."""

from __future__ import annotations

import csv
from pathlib import Path

import pandas as pd

BASE_DEFAULT_FREQUENCY = 50


def get_default_frequency(roman: str) -> int:
    """Fallback frequency derived from syllable count (fewer → more frequent)."""
    syllable_count = len(roman.replace(" ", "-").split("-"))
    return max(1, BASE_DEFAULT_FREQUENCY // syllable_count)


def load_frequency_map(filepath: str | Path) -> dict[tuple[str, str], int]:
    """Load the merged (hanzi, romanisation) → frequency map.

    Combines two sources:
      1. `char_freq_merged.txt` — per-character counts (Taigi corpus).
      2. `khiin_frequency.csv` + `khiin_conversions.csv` — multi-syllable
         word counts from the Khiin IME, deduped via POJ→TL conversion.

    `filepath` points at (1); (2) is resolved as siblings in the same
    directory. Missing sidecar files are silently tolerated.
    """
    filepath = Path(filepath)
    freq_map: dict[tuple[str, str], int] = {}

    if filepath.exists():
        df = pd.read_csv(filepath, sep="\t")
        for _, row in df.iterrows():
            name = str(row.get("Name", "")).strip()
            pronunciation = str(row.get("Pronunciation", "")).strip().lower()
            freq = int(row.get("New_Freq", 0))

            if name and pronunciation and name != "nan":
                freq_map[(name, pronunciation)] = freq

    khiin_freq_path = filepath.parent / "khiin_frequency.csv"
    khiin_conv_path = filepath.parent / "khiin_conversions.csv"

    if khiin_freq_path.exists() and khiin_conv_path.exists():
        for key, freq in _load_khiin_frequency(khiin_freq_path, khiin_conv_path).items():
            freq_map.setdefault(key, freq)

    return freq_map


def _load_khiin_frequency(freq_path: Path, conv_path: Path) -> dict[tuple[str, str], int]:
    """Load Khiin word-freq pairs, convert POJ→TL, pair with hanzi outputs."""
    from .taigi_bridge import convert_poj_to_tl

    freq: dict[str, int] = {}
    with freq_path.open() as f:
        for row in csv.DictReader(f):
            freq[row["input"]] = int(row["freq"])

    # Collect every hanzi output for each POJ input.
    all_hanzi: dict[str, set[str]] = {}
    with conv_path.open() as f:
        for row in csv.DictReader(f):
            inp = row["input"]
            output = row["output"]
            has_hanzi = any(
                "一" <= c <= "鿿"
                or "㐀" <= c <= "䶿"
                or ord(c) > 0x20000
                for c in output
            )
            if has_hanzi:
                all_hanzi.setdefault(inp, set()).add(output)

    # Each (hanzi, TL) may appear under multiple POJ variants; keep the max freq.
    result: dict[tuple[str, str], int] = {}
    for inp, f in freq.items():
        hanzi_set = all_hanzi.get(inp)
        if not hanzi_set:
            continue

        try:
            tl = convert_poj_to_tl(inp).lower().replace(" ", "-")
        except Exception:
            continue

        for hanzi in hanzi_set:
            key = (hanzi, tl)
            if key not in result or f > result[key]:
                result[key] = f

    return result


def get_frequency(hanzi: str, roman: str, freq_map: dict[tuple[str, str], int]) -> int:
    """Exact-match lookup with a syllable-count fallback when absent."""
    key = (hanzi, roman.lower())
    if key in freq_map:
        return freq_map[key]
    return get_default_frequency(roman)

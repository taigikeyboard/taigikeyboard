# -*- coding: utf-8 -*-
"""Authoritative source-bitmask and tier constants for the dictionary pipeline.

CROSS-PLATFORM INVARIANT — the values in this module MUST mirror:
  - ios/Sources/TaigiKeyboard/Lexicon/Database/DictionaryBinaryReader.swift
  - ios/Sources/TaigiKeyboard/Lexicon/Utils/CandidateProcessor.swift
  - android/.../dictionary/DictionaryBinaryReader.kt
  - android/.../dictionary/CandidateProcessor.kt

Drift causes silent divergence in dictionary filtering and candidate ranking.
When changing ANY bit position or tier numerator, update this file AND both
platform sources in the SAME PR (see rules/cross-platform-alignment.md §3a).

`scripts/check_source_bits_parity.py` guards the invariant in CI.
"""

from __future__ import annotations

from typing import Final

# Dictionary source bit positions (stored in the u16 bitmask of dictionary.bin records).
SOURCE_BITS: Final[dict[str, int]] = {
    "kautian": 0,   # 教育部臺灣台語常用詞辭典 (MoE)
    "taigitv": 1,   # 公視台語新詞辭庫
    "itaigi":  2,   # iTaigi 華台對照典 (ChhoeTaigi)
    "sitbut":  3,   # 台灣植物名彙 (ChhoeTaigi)
    "taihoa":  4,   # 台華線頂對照典 (ChhoeTaigi)
    "taijit":  5,   # 台日大辭典 (ChhoeTaigi)
    "kungge":  6,   # 台語工藝詞庫 (MoE)
    "stti":    7,   # 學科術語辭典 (MoE)
    "khpoo":   8,   # 齒盤補充辭典
    "khiin":   9,   # Khiin 頻率/轉換
    "dev":    10,   # 開發者補充辭典
    "lkk":    11,   # LKK 漢羅合用建議用字
}

IS_VARIANT_BIT: Final[int] = 12  # bit 12 in dictionary.bin u16 bitmask
# Bits 13..15 are reserved and must remain 0.

# Column order for dictionary.bin encoding — `bit = index in this list`.
DICT_BIN_COLUMNS: Final[list[str]] = list(SOURCE_BITS.keys()) + ["is_variant"]

# The 9-flag subset used by association.bin, build/merge_csv.py's main-source
# selection, and build/generate_association.py. khiin/dev/lkk and is_variant
# are excluded — those sources do not participate in NextWord scoring and
# supplementary flags are derived at merge time.
ASSOC_SOURCE_COLUMNS: Final[list[str]] = list(SOURCE_BITS.keys())[:9]
# Alias retained for the main-source merge pipeline (same 9 columns).
MAIN_SOURCE_COLUMNS: Final[list[str]] = ASSOC_SOURCE_COLUMNS

# Tier multipliers — MUST mirror CandidateProcessor on both platforms.
# baseFreqScore = rawFrequency * tierNumerator(sourceBitmask) / TIER_DENOMINATOR
# tierNumerator(bitmask) = first-match in the order below, else DEFAULT_TIER_NUMERATOR.
SOURCE_TIERS: Final[list[tuple[str, int]]] = [
    ("kautian", 15),
    ("taigitv", 13),
    ("stti",    12),
    ("kungge",  11),
]
DEFAULT_TIER_NUMERATOR: Final[int] = 10
TIER_DENOMINATOR: Final[int] = 10

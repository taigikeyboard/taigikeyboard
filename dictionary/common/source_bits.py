# -*- coding: utf-8 -*-
"""Authoritative source-bitmask and tier constants for the dictionary pipeline.

CROSS-PLATFORM INVARIANT — drift here causes silent divergence in dictionary
filtering and candidate ranking. When changing ANY constant, update this
file AND every mirror listed below in the SAME PR (see
rules/cross-platform-alignment.md §3a).

`SOURCE_BITS` + `IS_VARIANT_BIT` (bit-layout owners):
  - engine/lexicon/src/dictionary_reader.rs (binary-reader filter mask)
  - ios/Sources/TaigiKeyboard/Lexicon/Utils/LexiconBitmask.swift (decoder)
  - android/.../dictionary/LexiconBitmask.kt (decoder)

`SOURCE_TIERS` + `DEFAULT_TIER_NUMERATOR` + `TIER_DENOMINATOR` (ranking owners):
  - engine/ranking/src/score.rs (`tier_numerator` + `TIER_DENOMINATOR`)
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

# --- kautian subcollection subtag (dictionary.bin v3) ------------------------
# A SEPARATE u16 field (NOT part of the source bitmask above) records which
# kautian subcollections a row belongs to, so the kautian source bit (bit 0)
# stays a single undifferentiated badge/ranking signal while the filter can
# independently gate main / accent / name. Bit-layout owner — mirrors:
#   - engine/lexicon/src/dictionary_reader.rs (KAUTIAN_SUBTAG_* consts + reader)
#   - docs/engine/binary-format.md §1 (record layout)
# The wire `enabled_sources_bitmask` carries the user's subcollection ENABLE
# bits in the SAME 12-bit layout (main | accent[10] | name) at a high offset;
# see lexicon.proto + dictionary_reader.rs `Filter::from_enabled_bitmask`.
KAUTIAN_SUBTAG_MAIN_BIT: Final[int] = 0  # headword (主條目)
KAUTIAN_SUBTAG_ACCENT_SHIFT: Final[int] = 1  # accent_mask occupies bits 1..=10
KAUTIAN_SUBTAG_ACCENT_COUNT: Final[int] = 10  # 10 dialect columns (語音差異)
KAUTIAN_SUBTAG_NAME_BIT: Final[int] = 11  # 姓名附錄 (名 + 姓)
# Bits 12..15 are reserved and must remain 0.
_KAUTIAN_SUBTAG_ACCENT_MASK: Final[int] = (1 << KAUTIAN_SUBTAG_ACCENT_COUNT) - 1
# All subtag bits in use (main | accent[10] | name); bits 12-15 reserved. The
# encoder asserts `subtag & ~KAUTIAN_SUBTAG_USED_MASK == 0`; the Rust reader
# masks reads with the mirror constant.
KAUTIAN_SUBTAG_USED_MASK: Final[int] = (1 << (KAUTIAN_SUBTAG_NAME_BIT + 1)) - 1


def encode_kautian_subtag(has_main: bool, accent_mask: int, has_name: bool) -> int:
    """Pack kautian subcollection provenance into the v3 dictionary.bin u16.

    `accent_mask` is the Phase 1 10-bit value (`kautian_accent_mask`,
    bit order = config.yaml select.dialect_columns). It is masked to 10 bits
    so an out-of-range value can never spill into the reserved bits.
    """
    subtag = 0
    if has_main:
        subtag |= 1 << KAUTIAN_SUBTAG_MAIN_BIT
    subtag |= (accent_mask & _KAUTIAN_SUBTAG_ACCENT_MASK) << KAUTIAN_SUBTAG_ACCENT_SHIFT
    if has_name:
        subtag |= 1 << KAUTIAN_SUBTAG_NAME_BIT
    return subtag

# The 9-flag subset used by association.bin, build/merge_csv.py's main-source
# selection, and build/associations.py (NextWord bigram generator). khiin /
# dev / lkk and is_variant are excluded — those sources do not participate in
# NextWord scoring and supplementary flags are derived at merge time.
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

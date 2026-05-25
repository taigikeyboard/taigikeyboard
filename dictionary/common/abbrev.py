# -*- coding: utf-8 -*-
"""
縮寫相關函數
"""

import unicodedata
import pandas as pd


def remove_diacritics(char: str) -> str:
    """移除字元的聲調符號（組合字元）"""
    # NFD 分解：將 á 分解為 a + ́
    decomposed = unicodedata.normalize("NFD", char)
    # 過濾掉組合字元（Mn = Mark, Nonspacing）
    return "".join(c for c in decomposed if unicodedata.category(c) != "Mn")


def extract_abbrev(syllable_string: str) -> str:
    """
    提取音節首字母縮寫（使用帶連字符或空白的原始欄位）

    Args:
        syllable_string: 以 - 或空白分隔的音節字串（如 guá-sī 或 m̄ bat）

    Returns:
        所有音節的首字母組合（小寫、無聲調），若少於兩個音節則回傳空字串

    範例:
        "guá-sī" → "gs"
        "guá-tsá-goá" → "gtg"
        "m̄ bat" → "mb"
        "á" → "" (單音節不產生縮寫)
    """
    if not syllable_string or pd.isna(syllable_string):
        return ""

    import re
    syllables = re.split(r"[-\s]+", str(syllable_string))
    syllables = [s for s in syllables if s]

    if len(syllables) < 2:
        return ""

    # 取每個音節的首字母，移除聲調，轉小寫
    return "".join(remove_diacritics(s[0]).lower() for s in syllables)


def extract_tps_abbrev(tl_syllable_string: str, tps_per_syllable: list[str]) -> str:
    """Per-syllable TPS initial-char concatenation; "" for <2 syllables.

    Mirrors `extract_abbrev` (first ASCII char per TL syllable, minus
    diacritics) for the TPS family. The first char of every TPS
    syllable is its leading initial / vowel Bopomofo glyph by
    construction (per `to_zhuyin` output shape — see
    `engine/phonetics/src/tps.rs:210`), so no diacritic stripping is
    needed; we take the first `char` directly.

    Args:
        tl_syllable_string: hyphenated TL (used only for syllable-count
            gate — at least 2 syllables to emit an abbrev).
        tps_per_syllable: list of TPS strings, one per TL syllable
            (caller already converted via `convert_tl_to_tps_strict`
            on each TL syllable). Same length as the TL split.

    Returns:
        Concatenated first-char-per-syllable TPS abbrev, or "" when the
        TL has fewer than 2 syllables OR any per-syllable TPS is empty.
    """
    if not tl_syllable_string or pd.isna(tl_syllable_string):
        return ""

    import re
    tl_syllables = re.split(r"[-\s]+", str(tl_syllable_string))
    tl_syllables = [s for s in tl_syllables if s]

    if len(tl_syllables) < 2:
        return ""
    if len(tps_per_syllable) != len(tl_syllables):
        return ""
    if any(not s for s in tps_per_syllable):
        return ""

    return "".join(s[0] for s in tps_per_syllable)

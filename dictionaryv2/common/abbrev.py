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

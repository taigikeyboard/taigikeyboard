# -*- coding: utf-8 -*-
"""
詞頻相關函數
"""

import os
import pandas as pd

BASE_DEFAULT_FREQUENCY = 50


def get_default_frequency(roman: str) -> int:
    """
    根據音節數計算預設詞頻

    音節數越少，詞頻越高

    Args:
        roman: 羅馬字（以 - 或空格分隔）

    Returns:
        預設詞頻
    """
    syllable_count = len(roman.replace(" ", "-").split("-"))
    return max(1, BASE_DEFAULT_FREQUENCY // syllable_count)


def load_frequency_map(filepath: str) -> dict:
    """
    載入詞頻資料

    Args:
        filepath: char_freq_merged.txt 路徑

    Returns:
        dict[(漢字, 發音), 頻率]
    """
    freq_map = {}

    if not os.path.exists(filepath):
        return freq_map

    df = pd.read_csv(filepath, sep="\t")
    for _, row in df.iterrows():
        name = str(row.get("Name", "")).strip()
        pronunciation = str(row.get("Pronunciation", "")).strip().lower()
        freq = int(row.get("New_Freq", 0))

        if name and pronunciation and name != "nan":
            freq_map[(name, pronunciation)] = freq

    return freq_map


def get_frequency(hanzi: str, roman: str, freq_map: dict) -> int:
    """
    取得詞頻

    策略：
    1. 嘗試精確匹配 (漢字, 羅馬字)
    2. 無匹配時根據音節數回傳預設值

    Args:
        hanzi: 漢字
        roman: 羅馬字
        freq_map: 詞頻對照表

    Returns:
        詞頻
    """
    key = (hanzi, roman.lower())
    if key in freq_map:
        return freq_map[key]

    return get_default_frequency(roman)

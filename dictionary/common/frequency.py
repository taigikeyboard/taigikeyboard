# -*- coding: utf-8 -*-
"""
詞頻相關函數
"""

import csv
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

    合併兩個來源：
    1. char_freq_merged.txt — 單字字頻（台語語料庫統計）
    2. khiin_frequency.csv + khiin_conversions.csv — 多音節詞頻（起引輸入法語料）

    Args:
        filepath: char_freq_merged.txt 路徑

    Returns:
        dict[(漢字, 發音), 頻率]
    """
    freq_map = {}

    # 1. 載入單字字頻
    if os.path.exists(filepath):
        df = pd.read_csv(filepath, sep="\t")
        for _, row in df.iterrows():
            name = str(row.get("Name", "")).strip()
            pronunciation = str(row.get("Pronunciation", "")).strip().lower()
            freq = int(row.get("New_Freq", 0))

            if name and pronunciation and name != "nan":
                freq_map[(name, pronunciation)] = freq

    # 2. 載入 Khiin 多音節詞頻
    khiin_dir = os.path.join(os.path.dirname(filepath), "")
    khiin_freq_path = os.path.join(os.path.dirname(filepath), "khiin_frequency.csv")
    khiin_conv_path = os.path.join(os.path.dirname(filepath), "khiin_conversions.csv")

    if os.path.exists(khiin_freq_path) and os.path.exists(khiin_conv_path):
        khiin_map = _load_khiin_frequency(khiin_freq_path, khiin_conv_path)
        for key, freq in khiin_map.items():
            if key not in freq_map:
                freq_map[key] = freq

    return freq_map


def _load_khiin_frequency(freq_path: str, conv_path: str) -> dict:
    """
    載入 Khiin 詞頻資料，轉換 POJ → TL 並配對漢字

    Args:
        freq_path: khiin_frequency.csv 路徑
        conv_path: khiin_conversions.csv 路徑

    Returns:
        dict[(漢字, TL發音), 頻率]
    """
    from .taigi_bridge import convert_poj_to_tl

    # 載入詞頻
    freq = {}
    with open(freq_path) as f:
        for row in csv.DictReader(f):
            freq[row["input"]] = int(row["freq"])

    # 載入轉換表：收集每個 input 的所有漢字輸出
    all_hanzi = {}  # input → set of hanzi strings
    with open(conv_path) as f:
        for row in csv.DictReader(f):
            inp = row["input"]
            output = row["output"]
            has_hanzi = any(
                "\u4e00" <= c <= "\u9fff"
                or "\u3400" <= c <= "\u4dbf"
                or ord(c) > 0x20000
                for c in output
            )
            if has_hanzi:
                all_hanzi.setdefault(inp, set()).add(output)

    # 轉換 POJ → TL，建立 (漢字, TL) → 頻率 對照表
    # 同一個 romanization 的所有漢字變體都加入，共享同一個頻率
    result = {}
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
            # 同一個 (漢字, TL) 可能有多個 POJ 變體，取最大頻率
            if key not in result or f > result[key]:
                result[key] = f

    return result


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

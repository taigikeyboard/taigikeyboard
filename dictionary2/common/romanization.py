# -*- coding: utf-8 -*-
"""
羅馬字轉換相關函數
"""

from kesi import Ku
from kesi.susia.kongke import tsuan_sooji_tiau


def convert_tl_to_poj(tl: str) -> str:
    """
    將 TL 轉換為 POJ

    Args:
        tl: TL 羅馬字

    Returns:
        POJ 羅馬字（轉換失敗時回傳原值）
    """
    try:
        ku = Ku(tl)
        return ku.POJ().hanlo
    except Exception:
        return tl


def normalize_roman(text: str) -> str:
    """
    正規化羅馬字：空格轉連字符

    Args:
        text: 羅馬字

    Returns:
        正規化後的羅馬字
    """
    return text.replace(" ", "-").replace("\u3000", "-")


def to_numeric_tone(roman: str, ascii_only: bool = False) -> str:
    """
    轉換為數字聲調版本（去除連字符）

    Args:
        roman: 羅馬字（含調號）
        ascii_only: 是否只使用 ASCII 字元

    Returns:
        數字聲調版本（無連字符）
    """
    result = tsuan_sooji_tiau(roman, ascii=ascii_only).lower()
    return result.replace("-", "")


def add_roman_columns(hanzi: str, tl: str) -> dict:
    """
    產生完整的羅馬字欄位

    Args:
        hanzi: 漢字
        tl: TL 羅馬字

    Returns:
        包含所有羅馬字欄位的 dict

    Raises:
        Exception: 轉換失敗時拋出
    """
    # 轉換 TL → POJ
    poj = convert_tl_to_poj(tl)

    # 正規化：空格轉連字符
    tl = normalize_roman(tl)
    poj = normalize_roman(poj)

    # 產生數字聲調版本
    tl_num = to_numeric_tone(tl)
    poj_num = to_numeric_tone(poj, ascii_only=True)

    return {
        "hanzi": hanzi,
        "tl": tl.lower(),
        "poj": poj.lower(),
        "tl_num": tl_num,
        "poj_num": poj_num,
    }

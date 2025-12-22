# -*- coding: utf-8 -*-
"""
資料清理共用函數

清理步驟：
1. 移除括號標註：【白】【文】【替】（俗）[背] 等
2. 移除前綴連字符：--xxx -> xxx
3. 移除省略號：tsû...... -> tsû
4. 剔除俚語（漢字含全形標點符號）
5. 剔除含非法字符的 TL（使用 KeSi kam_haphuat 驗證）
6. 剔除無效的 hanzi（含 '.'、連續空格、'?'、Tab、大括號、單獨括號）
7. 移除空白列
8. 移除過長詞條（使用 KeSi thianji() 計算音節數，預設 4+ 音節）
9. 正規化羅馬字（空格轉連字符、轉小寫）
10. 去重複（hanzi + tl）
11. 若 hanzi 為空且 tl 已有對應漢字版本，則刪除該筆
12. 剔除漢羅字數不符的資料（使用 KeSi TuiBeTse 驗證）
"""

import re
import pandas as pd
from kesi import kam_haphuat, Ku, TuiBeTse, normalize_taibun

BRACKET_PATTERN = re.compile(r"[（(〈《「『【\[].*?[）)〉》」』】\]]")
PROVERB_PUNCTUATION = "，。！；？、"
DEFAULT_MAX_SYLLABLES = 5

# 所有可能含有括號標註的欄位
BRACKET_COLUMNS = ["hanzi", "tl", "poj", "tl_num", "poj_num", "tl_notone", "poj_notone", "tl_abbrev", "poj_abbrev"]


def count_syllables(text):
    """使用 KeSi thianji() 計算音節數"""
    if pd.isna(text) or not str(text).strip():
        return 0
    try:
        ku = Ku(str(text).strip())
        return len(list(ku.thianji()))
    except Exception:
        # KeSi 無法解析時，fallback 到簡單分割
        syllables = re.split(r"[\s\-]+", str(text).strip())
        return len([s for s in syllables if s])


def clean_brackets(text):
    """移除括號標註"""
    if pd.isna(text):
        return ""
    text = str(text)
    text = BRACKET_PATTERN.sub("", text)
    return text.strip()


def remove_prefix_hyphens(text):
    """移除前綴 --"""
    if pd.isna(text):
        return ""
    text = str(text)
    if text.startswith("--"):
        return text[2:]
    return text


def remove_ellipsis(text):
    """移除省略號 (連續的點)，例如 tsû...... -> tsû"""
    if pd.isna(text):
        return ""
    text = str(text)
    # 移除連續 2 個以上的點
    text = re.sub(r"\.{2,}", "", text)
    return text.strip()


def normalize_roman(text):
    """正規化羅馬字：Unicode NFC 正規化、空格轉連字符、轉小寫"""
    if pd.isna(text):
        return text
    text = normalize_taibun(str(text))  # Unicode NFC + 教育部造字碼轉換
    return text.replace(" ", "-").replace("\u3000", "-").lower()


def is_proverb(hanzi):
    """判斷是否為俚語（漢字含全形標點符號）"""
    if pd.isna(hanzi):
        return False
    return any(p in str(hanzi) for p in PROVERB_PUNCTUATION)


def is_valid_tl(text):
    """
    使用 KeSi kam_haphuat 檢查 TL 欄位是否為合法羅馬字
    支援 KIP、POJ、數字調
    """
    if pd.isna(text) or str(text).strip() == "":
        return False
    # 分割音節後逐一驗證
    for syllable in re.split(r"[\s\-]+", str(text).strip()):
        if syllable and not kam_haphuat(syllable):
            return False
    return True


def is_hanlo_matched(hanzi, tl):
    """
    使用 KeSi 檢查漢字和羅馬字字數是否相符
    若字數不符會拋出 TuiBeTse 例外
    """
    if pd.isna(hanzi) or pd.isna(tl):
        return True
    hanzi_str = str(hanzi).strip()
    tl_str = str(tl).strip()
    if hanzi_str == "" or tl_str == "":
        return True
    try:
        Ku(hanlo=hanzi_str, lomaji=tl_str)
        return True
    except TuiBeTse:
        return False
    except Exception:
        # 其他例外（如格式錯誤）視為不符
        return False


def is_valid_hanzi(text):
    """
    檢查 hanzi 欄位是否有效
    無效情況：
    - 含有 '.'（如「雞仔腸.鳥仔肚」）
    - 含有連續空格（如「峭脊    台日大辭典」）
    - 含有 '?'（如「khû-?」「土m̀?」）
    - 含有單獨括號（如「事)」「[以外」「屈死]」）
    - 含有 Tab 字符
    - 含有大括號（如「{查看覓}」）
    """
    if pd.isna(text):
        return True  # 空值由其他步驟處理
    text = str(text)
    # 含有 '.' 則無效
    if '.' in text:
        return False
    # 含有連續空格則無效
    if '  ' in text:
        return False
    # 含有 '?' 則無效
    if '?' in text:
        return False
    # 含有 Tab 字符則無效
    if '\t' in text:
        return False
    # 含有大括號則無效
    if '{' in text or '}' in text:
        return False
    # 含有單獨括號則無效（只有一邊括號）
    for open_b, close_b in [('(', ')'), ('[', ']'), ('（', '）'), ('【', '】')]:
        has_open = open_b in text
        has_close = close_b in text
        if has_open != has_close:  # 只有一邊括號
            return False
    return True


def cleanup_dataframe(df, logger=None, max_syllables=DEFAULT_MAX_SYLLABLES):
    """
    執行完整的資料清理流程

    Args:
        df: DataFrame（需有 hanzi, tl 欄位）
        logger: Logger 實例（可選）
        max_syllables: 最大音節數，超過則移除（預設 3）

    Returns:
        清理後的 DataFrame
    """
    total_count = len(df)
    if logger:
        logger.info(f"Loaded {total_count} records")

    # 1. 移除括號標註（所有相關欄位）
    for col in BRACKET_COLUMNS:
        if col in df.columns:
            df[col] = df[col].apply(clean_brackets)

    # 2. 移除前綴 --
    if "tl" in df.columns:
        df["tl"] = df["tl"].apply(remove_prefix_hyphens)

    # 3. 移除省略號 (例如 tsû...... -> tsû)
    for col in ["tl", "hanzi"]:
        if col in df.columns:
            df[col] = df[col].apply(remove_ellipsis)

    # 4. 剔除俚語
    proverb_mask = df["hanzi"].apply(is_proverb)
    proverb_count = proverb_mask.sum()
    if proverb_count > 0 and logger:
        logger.info(f"  Removed proverbs: {proverb_count}")
    df = df[~proverb_mask]

    # 5. 剔除含非法字符的 TL（日文、中文等）
    valid_tl_mask = df["tl"].apply(is_valid_tl)
    invalid_tl_count = (~valid_tl_mask).sum()
    if invalid_tl_count > 0 and logger:
        logger.info(f"  Removed invalid TL (non-roman chars): {invalid_tl_count}")
    df = df[valid_tl_mask]

    # 6. 剔除無效的 hanzi（含 '.' 或連續空格）
    valid_hanzi_mask = df["hanzi"].apply(is_valid_hanzi)
    invalid_hanzi_count = (~valid_hanzi_mask).sum()
    if invalid_hanzi_count > 0 and logger:
        logger.info(f"  Removed invalid hanzi (dot or consecutive spaces): {invalid_hanzi_count}")
    df = df[valid_hanzi_mask]

    # 7. 移除空白列
    df = df[df["tl"].str.strip() != ""]
    df = df[df["hanzi"].str.strip() != ""]

    # 8. 移除過長詞條
    before_long = len(df)
    df = df[df["tl"].apply(count_syllables) <= max_syllables]
    long_removed = before_long - len(df)
    if long_removed > 0 and logger:
        logger.info(f"  Removed long entries ({max_syllables + 1}+ syllables): {long_removed}")

    # 9. 正規化羅馬字
    df["tl"] = df["tl"].apply(normalize_roman)

    # 10. 去重複（hanzi + tl）
    before_dedup = len(df)
    df = df.drop_duplicates(subset=["hanzi", "tl"], keep="first")
    after_dedup = len(df)
    removed = before_dedup - after_dedup
    if removed > 0 and logger:
        logger.info(f"  Removed duplicates (hanzi+tl): {removed}")

    # 11. 若 hanzi 為空，檢查 tl 是否重複（保留有漢字的，刪除無漢字的重複項）
    before_empty_hanzi = len(df)
    # 找出所有 tl 值
    all_tl_values = set(df["tl"])
    # 找出有漢字的 tl 值
    has_hanzi_tl = set(df[df["hanzi"].str.strip() != ""]["tl"])
    # 對於 hanzi 為空的資料，若其 tl 已有對應的漢字版本，則刪除
    empty_hanzi_duplicate_mask = (df["hanzi"].str.strip() == "") & (df["tl"].isin(has_hanzi_tl))
    empty_hanzi_removed = empty_hanzi_duplicate_mask.sum()
    if empty_hanzi_removed > 0:
        df = df[~empty_hanzi_duplicate_mask]
        if logger:
            logger.info(f"  Removed empty hanzi duplicates: {empty_hanzi_removed}")

    # 12. 剔除漢羅字數不符的資料（使用 KeSi TuiBeTse 驗證）
    hanlo_matched_mask = df.apply(lambda row: is_hanlo_matched(row["hanzi"], row["tl"]), axis=1)
    hanlo_mismatch_count = (~hanlo_matched_mask).sum()
    if hanlo_mismatch_count > 0 and logger:
        logger.info(f"  Removed hanlo mismatch (KeSi TuiBeTse): {hanlo_mismatch_count}")
    df = df[hanlo_matched_mask]

    if logger:
        logger.info(f"  Final: {len(df)} records")

    return df

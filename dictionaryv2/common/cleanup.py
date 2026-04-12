# -*- coding: utf-8 -*-
"""
資料清理共用函數

清理步驟：
1. 移除括號標註：【白】【文】【替】（俗）[背] 等
2. 移除前綴連字符：--xxx -> xxx
3. 移除省略號：tsû...... -> tsû
4. 剔除俚語（漢字含全形標點符號）
5. 剔除含非法字符的 TL（嘗試 TL→POJ 轉換驗證）
6. 剔除無效的 hanzi（含 '.'、連續空格、'?'、Tab、大括號、單獨括號）
6.5. 清空包含羅馬字的 hanzi（含拉丁字母或台語聲調符號，保留該筆資料）
7. 移除 tl 為空的列（保留 hanzi 為空的詞條，因為台語常見純羅馬字詞）
8. 移除過長詞條（預設 4+ 音節）
9. 正規化羅馬字（空格轉連字符、轉小寫）
10. 去重複（hanzi + tl）
11. 剔除漢羅字數不符的資料（漢字數 vs TL 音節數比對）
"""

import re
import unicodedata

import pandas as pd

from .taigi_bridge import is_valid_romanization, normalize_taibun

BRACKET_PATTERN = re.compile(r"[（(〈《「『【\[].*?[）)〉》」』】\]]")
PROVERB_PUNCTUATION = "，。！；？、"
DEFAULT_MAX_SYLLABLES = 4

# 所有可能含有括號標註的欄位
BRACKET_COLUMNS = ["hanzi", "tl", "poj", "tl_num", "poj_num", "tl_notone", "poj_notone", "tl_abbrev", "poj_abbrev"]


def count_syllables(text):
    """計算音節數（以連字符和空白分割）"""
    if pd.isna(text) or not str(text).strip():
        return 0
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


def normalize_roman(text, preserve_spaces=False):
    """
    正規化羅馬字：Unicode NFC 正規化、轉小寫

    Args:
        text: 羅馬字
        preserve_spaces: True=保留空白（官方辭典），False=空白轉連字符（非官方辭典）
    """
    if pd.isna(text):
        return text
    text = normalize_taibun(str(text))
    text = text.replace("\u3000", " " if preserve_spaces else "-")
    if not preserve_spaces:
        text = text.replace(" ", "-")
    return text.lower()


def is_proverb(hanzi):
    """判斷是否為俚語（漢字含全形標點符號）"""
    if pd.isna(hanzi):
        return False
    return any(p in str(hanzi) for p in PROVERB_PUNCTUATION)


def is_valid_tl(text):
    """
    使用 taigi-converter parseSyllable 驗證 TL 是否為合法羅馬字
    """
    if pd.isna(text) or str(text).strip() == "":
        return False
    return is_valid_romanization(str(text).strip())


def is_hanlo_matched(hanzi, tl):
    """
    檢查漢字和羅馬字字數是否相符
    漢字字數（不含標點空白）應等於 TL 音節數
    """
    if pd.isna(hanzi) or pd.isna(tl):
        return True
    hanzi_str = str(hanzi).strip()
    tl_str = str(tl).strip()
    if hanzi_str == "" or tl_str == "":
        return True
    # 計算漢字字數（排除空白和連字符）
    hanzi_chars = [c for c in hanzi_str if c not in " \u3000-"]
    hanzi_count = len(hanzi_chars)
    # 計算 TL 音節數
    tl_count = count_syllables(tl_str)
    return hanzi_count == tl_count


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


def contains_roman_in_hanzi(text):
    """
    檢查 hanzi 欄位是否包含羅馬字（拉丁字母或台語聲調符號）

    包含以下任一情況視為含羅馬字：
    - 基本拉丁字母 a-zA-Z
    - 台語羅馬字調號（如 ā, á, ǎ, à, ē, é, ě, è, ō, ó, ǒ, ò, m̄, ń 等）

    忽略：
    - 空值
    - 連字符號 '-'
    """
    if pd.isna(text):
        return False
    text = str(text).strip()
    if text == "":
        return False

    # 移除連字符號後檢查
    text_without_hyphen = text.replace('-', '')

    # 檢查是否包含基本拉丁字母 a-zA-Z
    if re.search(r'[a-zA-Z]', text_without_hyphen):
        return True

    # 檢查是否包含台語羅馬字常見的帶調號字母
    # POJ/TL 調號：ā á ǎ à ē é ě è ī í ǐ ì ō ó ǒ ò ū ú ǔ ù m̄ ḿ m̌ m̀ n̄ ń ň ǹ o͘ 等
    roman_tone_pattern = r'[āáǎàaⁿēéěèeⁿīíǐìiⁿōóǒòoⁿūúǔùuⁿm̄ḿm̌m̀n̄ńňǹṁṅⁿ]'
    if re.search(roman_tone_pattern, text_without_hyphen):
        return True

    return False


def cleanup_dataframe(df, logger=None, max_syllables=DEFAULT_MAX_SYLLABLES, check_roman_in_hanzi=False, preserve_spaces=False):
    """
    執行完整的資料清理流程

    Args:
        df: DataFrame（需有 hanzi, tl 欄位）
        logger: Logger 實例（可選）
        max_syllables: 最大音節數，超過則移除（預設 5）
        check_roman_in_hanzi: 是否檢查並清空 hanzi 欄位中的羅馬字（預設 False）
        preserve_spaces: 是否保留空白作為詞界（官方辭典），預設 False

    Returns:
        (cleaned_df, dropped_df) - cleaned_df 為清理後的 DataFrame，
        dropped_df 為被丟棄的列（含 drop_reason 欄位）
    """
    total_count = len(df)
    dropped_parts = []

    def collect_dropped(mask, reason):
        """Collect rows that will be dropped, with a reason tag."""
        dropped = df.loc[mask].copy()
        if len(dropped) > 0:
            dropped["drop_reason"] = reason
            dropped_parts.append(dropped)

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
    collect_dropped(proverb_mask, "proverb")
    df = df[~proverb_mask]

    # 5. 剔除含非法字符的 TL（日文、中文等）
    valid_tl_mask = df["tl"].apply(is_valid_tl)
    invalid_tl_count = (~valid_tl_mask).sum()
    if invalid_tl_count > 0 and logger:
        logger.info(f"  Removed invalid TL (non-roman chars): {invalid_tl_count}")
    collect_dropped(~valid_tl_mask, "invalid_tl")
    df = df[valid_tl_mask]

    # 6. 剔除無效的 hanzi（含 '.' 或連續空格）
    valid_hanzi_mask = df["hanzi"].apply(is_valid_hanzi)
    invalid_hanzi_count = (~valid_hanzi_mask).sum()
    if invalid_hanzi_count > 0 and logger:
        logger.info(f"  Removed invalid hanzi (dot or consecutive spaces): {invalid_hanzi_count}")
    collect_dropped(~valid_hanzi_mask, "invalid_hanzi")
    df = df[valid_hanzi_mask]

    # 6.5. 清空包含羅馬字的 hanzi（但保留該筆資料）
    if check_roman_in_hanzi:
        roman_mask = df["hanzi"].apply(contains_roman_in_hanzi)
        roman_in_hanzi_count = roman_mask.sum()
        if roman_in_hanzi_count > 0:
            df.loc[roman_mask, "hanzi"] = ""
            if logger:
                logger.info(f"  Cleared hanzi containing roman letters: {roman_in_hanzi_count}")

    # 7. 移除空白列（只檢查 tl，保留 hanzi 為空的詞條）
    empty_tl_mask = df["tl"].str.strip() == ""
    collect_dropped(empty_tl_mask, "empty_tl")
    df = df[~empty_tl_mask]

    # 8. 移除過長詞條
    before_long = len(df)
    long_mask = df["tl"].apply(count_syllables) > max_syllables
    collect_dropped(long_mask, f"too_long(>{max_syllables})")
    df = df[~long_mask]
    long_removed = before_long - len(df)
    if long_removed > 0 and logger:
        logger.info(f"  Removed long entries ({max_syllables + 1}+ syllables): {long_removed}")

    # 9. 正規化羅馬字
    df["tl"] = df["tl"].apply(lambda x: normalize_roman(x, preserve_spaces=preserve_spaces))

    # 10. 去重複（hanzi + tl）
    before_dedup = len(df)
    dup_mask = df.duplicated(subset=["hanzi", "tl"], keep="first")
    collect_dropped(dup_mask, "duplicate")
    df = df[~dup_mask]
    after_dedup = len(df)
    removed = before_dedup - after_dedup
    if removed > 0 and logger:
        logger.info(f"  Removed duplicates (hanzi+tl): {removed}")

    # 11. 剔除漢羅字數不符的資料（漢字數 vs TL 音節數比對）
    hanlo_matched_mask = df.apply(lambda row: is_hanlo_matched(row["hanzi"], row["tl"]), axis=1)
    hanlo_mismatch_count = (~hanlo_matched_mask).sum()
    if hanlo_mismatch_count > 0 and logger:
        logger.info(f"  Removed hanlo mismatch (KeSi TuiBeTse): {hanlo_mismatch_count}")
    collect_dropped(~hanlo_matched_mask, "hanlo_mismatch")
    df = df[hanlo_matched_mask]

    if logger:
        logger.info(f"  Final: {len(df)} records")

    dropped_df = pd.concat(dropped_parts, ignore_index=True) if dropped_parts else pd.DataFrame()
    return df, dropped_df

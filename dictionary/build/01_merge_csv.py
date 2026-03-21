#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
合併多個詞庫為單一 CSV

輸入：
- 1_教育部臺灣台語常用詞辭典/data/kautian.csv
- 2_台語新詞辭庫/data/taigitv.csv
- 3_iTaigi華台對照典/data/itaigi.csv
- 4_台灣植物名彙/data/sitbut.csv
- 5_台華線頂對照典/data/taihoa.csv
- 6_台日大辭典/data/taijit.csv
- 7_台語工藝詞庫/data/kungge.csv
- Khiin 詞頻資料（補充不在其他詞庫中的詞條）

輸出：
- output/dictionary.csv
"""

import csv
import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.frequency import load_frequency_map, get_frequency

# 基準目錄（dictionary2/）
BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

INPUT_FILES = [
    ("1_教育部臺灣台語常用詞辭典/data/kautian.csv", "kautian"),
    ("2_台語新詞辭庫/data/taigitv.csv", "taigitv"),
    ("3_iTaigi華台對照典/data/itaigi.csv", "itaigi"),
    ("4_台灣植物名彙/data/sitbut.csv", "sitbut"),
    ("5_台華線頂對照典/data/taihoa.csv", "taihoa"),
    ("6_台日大辭典/data/taijit.csv", "taijit"),
    ("7_台語工藝詞庫/data/kungge.csv", "kungge"),
    ("13_學科術語辭典/data/stti.csv", "stti"),
    ("14_齒盤補充辭典/data/khpoo.csv", "khpoo"),
]
OUTPUT_DIR = "output"
OUTPUT_FILE = "dictionary.csv"
SCRIPT_NAME = "01_merge_csv"

SOURCE_COLUMNS = ["kautian", "taigitv", "itaigi", "sitbut", "taihoa", "taijit", "kungge", "stti", "khpoo"]


def main():
    logger = setup_logging(SCRIPT_NAME)
    output_path = os.path.join(BASE_DIR, OUTPUT_DIR, OUTPUT_FILE)
    log_header(logger, SCRIPT_NAME, ", ".join([f[0] for f in INPUT_FILES]), output_path)

    os.makedirs(os.path.join(BASE_DIR, OUTPUT_DIR), exist_ok=True)

    all_dfs = []

    for filepath, source_col in INPUT_FILES:
        full_path = os.path.join(BASE_DIR, filepath)
        if not os.path.exists(full_path):
            logger.warning(f"  [skip] {filepath} not found")
            continue

        df = pd.read_csv(full_path)
        logger.info(f"  [load] {filepath}: {len(df)} records")
        all_dfs.append(df)

    if not all_dfs:
        logger.error("No input files found!")
        return

    # 合併
    merged_df = pd.concat(all_dfs, ignore_index=True)
    logger.info(f"\n  Total before merge: {len(merged_df)} records")

    # 建立正規化 key（空白→連字符）用於跨辭典去重
    # 官方辭典 tl 可能含空白（如 "m̄ bat"），非官方辭典為連字符（如 "m̄-bat"）
    # 去重時視為同一筆，但保留官方版本（含空白）的 tl
    OFFICIAL_SOURCES = ["kautian", "taigitv", "kungge"]
    merged_df["_tl_key"] = merged_df["tl"].str.replace(" ", "-", regex=False)
    merged_df["_is_official"] = merged_df[OFFICIAL_SOURCES].any(axis=1)

    # 排序：官方辭典排在前面，確保 groupby 的 "first" 取到官方版本
    merged_df = merged_df.sort_values("_is_official", ascending=False, ignore_index=True)

    # 去重複：相同 (hanzi, _tl_key) 合併來源欄位
    # dropna=False: 確保 groupby 不會自動排除含 NaN 的列
    agg_dict = {}
    for col in merged_df.columns:
        if col in ["hanzi", "_tl_key"]:
            continue  # groupby key
        elif col in ["_is_official"]:
            continue  # temp column
        elif col in SOURCE_COLUMNS:
            agg_dict[col] = "any"
        elif col == "frequency":
            agg_dict[col] = "max"
        else:
            agg_dict[col] = "first"
    result_df = merged_df.groupby(["hanzi", "_tl_key"], as_index=False, dropna=False).agg(agg_dict)

    # 移除臨時欄位
    result_df = result_df.drop(columns=["_tl_key"])

    # 排序：依 frequency 降序
    result_df = result_df.sort_values(
        ["frequency", "hanzi", "tl"],
        ascending=[False, True, True],
        ignore_index=True
    )

    logger.info(f"  After dedup: {len(result_df)} records")

    # 補入 Khiin 獨有的詞條（不屬於任何辭典來源）
    khiin_new = _load_khiin_new_entries(result_df, BASE_DIR, logger)
    if khiin_new is not None and len(khiin_new) > 0:
        result_df = pd.concat([result_df, khiin_new], ignore_index=True)
        # Re-sort
        result_df = result_df.sort_values(
            ["frequency", "hanzi", "tl"],
            ascending=[False, True, True],
            ignore_index=True
        )
        logger.info(f"  After Khiin supplement: {len(result_df)} records")

    # Ensure dev column exists before dev supplement (needed for marking existing entries)
    if "dev" not in result_df.columns:
        result_df["dev"] = False

    # 補入開發者補充辭典
    dev_new = _load_dev_supplement(result_df, BASE_DIR, logger)
    if dev_new is not None and len(dev_new) > 0:
        result_df = pd.concat([result_df, dev_new], ignore_index=True)
        result_df = result_df.sort_values(
            ["frequency", "hanzi", "tl"],
            ascending=[False, True, True],
            ignore_index=True
        )
        logger.info(f"  After dev supplement: {len(result_df)} records")

    # 補入 LKK 漢羅合用建議用字
    if "lkk" not in result_df.columns:
        result_df["lkk"] = False
    lkk_new = _load_lkk_entries(result_df, BASE_DIR, logger)
    if lkk_new is not None and len(lkk_new) > 0:
        result_df = pd.concat([result_df, lkk_new], ignore_index=True)
        result_df = result_df.sort_values(
            ["frequency", "hanzi", "tl"],
            ascending=[False, True, True],
            ignore_index=True
        )
        logger.info(f"  After LKK supplement: {len(result_df)} records")

    # Compute zairaiji column: entries not in any named source
    result_df["khiin"] = ~(result_df[SOURCE_COLUMNS].any(axis=1) | result_df["dev"] | result_df["lkk"])

    # 統計來源
    logger.info(f"\n  [source statistics]")
    for col in SOURCE_COLUMNS:
        total = result_df[col].sum()
        # 計算該來源獨有的數量
        other_cols = [c for c in SOURCE_COLUMNS if c != col]
        only_mask = result_df[col]
        for other in other_cols:
            only_mask = only_mask & ~result_df[other]
        only_count = only_mask.sum()
        logger.info(f"    {col}: {total} (only: {only_count})")

    # 儲存
    result_df.to_csv(output_path, index=False)

    logger.info(f"\n  [sample records]:")
    for _, row in result_df.head(10).iterrows():
        sources = [col for col in SOURCE_COLUMNS if row[col]]
        logger.info(f"    {row['hanzi']}: {row['tl']} ({row['frequency']}) [{', '.join(sources)}]")

    logger.info(f"\nSaved: {output_path}")


def _load_khiin_new_entries(existing_df: pd.DataFrame, base_dir: str, logger) -> pd.DataFrame:
    """
    載入 Khiin 詞頻資料中不在現有詞庫的詞條

    這些詞條的所有 source column 都是 False，不會出現在 App 的辭典開關設定中。
    """
    from kesi import Ku
    from common.romanization import normalize_roman, to_numeric_tone, convert_tl_to_poj
    from common.notone import remove_tone
    from common.abbrev import extract_abbrev

    freq_path = os.path.join(base_dir, "0_其他資料", "khiin_frequency.csv")
    conv_path = os.path.join(base_dir, "0_其他資料", "khiin_conversions.csv")

    if not os.path.exists(freq_path) or not os.path.exists(conv_path):
        logger.warning("  [skip] Khiin frequency files not found")
        return None

    # Load frequency
    freq = {}
    with open(freq_path) as f:
        for row in csv.DictReader(f):
            freq[row["input"]] = int(row["freq"])

    # Load all hanzi variants per input
    all_hanzi = {}
    with open(conv_path) as f:
        for row in csv.DictReader(f):
            inp = row["input"]
            output = row["output"]
            if any(
                "\u4e00" <= c <= "\u9fff"
                or "\u3400" <= c <= "\u4dbf"
                or ord(c) > 0x20000
                for c in output
            ):
                all_hanzi.setdefault(inp, set()).add(output)

    # Build set of existing (hanzi, tl_normalized) pairs
    # Normalize spaces to hyphens for comparison
    existing_keys = set()
    for _, row in existing_df.iterrows():
        tl_key = str(row["tl"]).lower().replace(" ", "-")
        existing_keys.add((str(row["hanzi"]), tl_key))

    # Load frequency map for get_frequency
    freq_map_path = os.path.join(base_dir, "0_其他資料", "char_freq_merged.txt")
    freq_map = load_frequency_map(freq_map_path)

    # Find new entries
    new_rows = []
    for inp, f in freq.items():
        hanzi_set = all_hanzi.get(inp)
        if not hanzi_set:
            continue

        try:
            ku = Ku(inp)
            tl = ku.TL().hanlo.lower().replace(" ", "-")
            poj = ku.POJ().hanlo.lower().replace(" ", "-")
        except Exception:
            continue

        # Skip entries exceeding 4 syllables
        if len(tl.split("-")) > 4:
            continue

        for hanzi in hanzi_set:
            if (hanzi, tl) in existing_keys:
                continue

            try:
                tl_num = to_numeric_tone(tl)
                poj_num = to_numeric_tone(poj, ascii_only=True)
            except Exception:
                continue

            frequency = get_frequency(hanzi, tl, freq_map)

            tl_notone = remove_tone(tl_num)
            poj_notone = remove_tone(poj_num)
            tl_abbrev = extract_abbrev(tl)
            poj_abbrev = extract_abbrev(poj)

            new_rows.append({
                "tl": tl,
                "hanzi": hanzi,
                "frequency": frequency,
                "poj": poj,
                "tl_num": tl_num,
                "poj_num": poj_num,
                "tl_notone": tl_notone,
                "poj_notone": poj_notone,
                "tl_abbrev": tl_abbrev,
                "poj_abbrev": poj_abbrev,
                "is_variant": False,
                **{col: False for col in SOURCE_COLUMNS},
            })
            existing_keys.add((hanzi, tl))

    if new_rows:
        logger.info(f"  [khiin] Added {len(new_rows)} new entries from Khiin")

    return pd.DataFrame(new_rows) if new_rows else None


def _load_dev_supplement(existing_df: pd.DataFrame, base_dir: str, logger) -> pd.DataFrame:
    """
    Load developer supplement dictionary entries.

    Reads a minimal CSV (hanzi, tl) and auto-generates all romanization columns.
    No cleanup is applied — trusts user input.
    If a (hanzi, tl) pair already exists, marks it dev=True (always included).
    New pairs are added with dev=True and all source columns set to False.
    """
    from kesi import Ku
    from common.romanization import normalize_roman, to_numeric_tone, convert_tl_to_poj
    from common.notone import remove_tone
    from common.abbrev import extract_abbrev

    dev_path = os.path.join(base_dir, "15_開發者補充辭典", "data", "dev.csv")

    if not os.path.exists(dev_path):
        logger.warning("  [skip] dev supplement CSV not found")
        return None

    dev_df = pd.read_csv(dev_path)
    if dev_df.empty:
        logger.info("  [dev] dev.csv is empty, skipping")
        return None

    # Build map of existing (hanzi, tl_normalized) -> index for dedup
    existing_key_index = {}
    for idx, row in existing_df.iterrows():
        tl_key = str(row["tl"]).lower().replace(" ", "-")
        existing_key_index[(str(row["hanzi"]), tl_key)] = idx

    # Load frequency map
    freq_map_path = os.path.join(base_dir, "0_其他資料", "char_freq_merged.txt")
    freq_map = load_frequency_map(freq_map_path)

    marked_count = 0
    new_rows = []
    for _, row in dev_df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        tl_raw = str(row["tl"]).strip().lower()

        if not hanzi or not tl_raw:
            continue

        tl_key = tl_raw.replace(" ", "-")
        if (hanzi, tl_key) in existing_key_index:
            # Mark existing entry as dev so it's always included
            existing_df.at[existing_key_index[(hanzi, tl_key)], "dev"] = True
            logger.info(f"  [dev] marked existing: {hanzi} / {tl_raw}")
            marked_count += 1
            continue

        try:
            ku = Ku(tl_raw)
            tl = ku.TL().hanlo.lower().replace(" ", "-")
            poj = ku.POJ().hanlo.lower().replace(" ", "-")
            tl_num = to_numeric_tone(tl)
            poj_num = to_numeric_tone(poj, ascii_only=True)
        except Exception as e:
            logger.warning(f"  [dev] romanization failed for {hanzi}/{tl_raw}: {e}")
            continue

        tl_notone = remove_tone(tl_num)
        poj_notone = remove_tone(poj_num)
        tl_abbrev = extract_abbrev(tl)
        poj_abbrev = extract_abbrev(poj)

        # Use user-provided frequency, or compute from character frequency
        frequency = row.get("frequency") if "frequency" in dev_df.columns and pd.notna(row.get("frequency")) else None
        if frequency is None:
            frequency = get_frequency(hanzi, tl, freq_map)
        else:
            frequency = int(frequency)

        is_variant = row.get("is_variant", False) if "is_variant" in dev_df.columns else False

        new_rows.append({
            "tl": tl,
            "hanzi": hanzi,
            "frequency": frequency,
            "poj": poj,
            "tl_num": tl_num,
            "poj_num": poj_num,
            "tl_notone": tl_notone,
            "poj_notone": poj_notone,
            "tl_abbrev": tl_abbrev,
            "poj_abbrev": poj_abbrev,
            "is_variant": is_variant,
            **{col: False for col in SOURCE_COLUMNS},
            "dev": True,
        })
        existing_key_index[(hanzi, tl_key)] = -1  # sentinel: already added as new

    if marked_count:
        logger.info(f"  [dev] Marked {marked_count} existing entries as dev")
    if new_rows:
        logger.info(f"  [dev] Added {len(new_rows)} new entries from dev supplement")

    return pd.DataFrame(new_rows) if new_rows else None


def _load_lkk_entries(existing_df: pd.DataFrame, base_dir: str, logger) -> pd.DataFrame:
    """
    Load LKK 漢羅合用建議用字 dictionary entries.

    Reads a minimal CSV (hanzi, tl) and auto-generates all romanization columns.
    If a (hanzi, tl) pair already exists, marks it lkk=True.
    New pairs are added with lkk=True and all other source columns set to False.
    """
    from kesi import Ku
    from common.romanization import normalize_roman, to_numeric_tone, convert_tl_to_poj
    from common.notone import remove_tone
    from common.abbrev import extract_abbrev

    lkk_path = os.path.join(base_dir, "16_LKK漢羅合用建議用字", "data", "lkk.csv")

    if not os.path.exists(lkk_path):
        logger.warning("  [skip] LKK CSV not found")
        return None

    lkk_df = pd.read_csv(lkk_path)
    if lkk_df.empty:
        logger.info("  [lkk] lkk.csv is empty, skipping")
        return None

    # Build map of existing (hanzi, tl_normalized) -> index for dedup
    existing_key_index = {}
    for idx, row in existing_df.iterrows():
        tl_key = str(row["tl"]).lower().replace(" ", "-")
        existing_key_index[(str(row["hanzi"]), tl_key)] = idx

    # Load frequency map
    freq_map_path = os.path.join(base_dir, "0_其他資料", "char_freq_merged.txt")
    freq_map = load_frequency_map(freq_map_path)

    marked_count = 0
    new_rows = []
    for _, row in lkk_df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        tl_raw = str(row["tl"]).strip().lower()

        if not hanzi or not tl_raw:
            continue

        tl_key = tl_raw.replace(" ", "-")
        if (hanzi, tl_key) in existing_key_index:
            idx = existing_key_index[(hanzi, tl_key)]
            if idx >= 0:
                # Mark existing entry as lkk
                existing_df.at[idx, "lkk"] = True
                logger.info(f"  [lkk] marked existing: {hanzi} / {tl_raw}")
                marked_count += 1
            else:
                logger.info(f"  [lkk] skip duplicate: {hanzi} / {tl_raw}")
            continue

        try:
            ku = Ku(tl_raw)
            tl = ku.TL().hanlo.lower().replace(" ", "-")
            poj = ku.POJ().hanlo.lower().replace(" ", "-")
            tl_num = to_numeric_tone(tl)
            poj_num = to_numeric_tone(poj, ascii_only=True)
        except Exception as e:
            logger.warning(f"  [lkk] romanization failed for {hanzi}/{tl_raw}: {e}")
            continue

        tl_notone = remove_tone(tl_num)
        poj_notone = remove_tone(poj_num)
        tl_abbrev = extract_abbrev(tl)
        poj_abbrev = extract_abbrev(poj)

        frequency = get_frequency(hanzi, tl, freq_map)

        new_rows.append({
            "tl": tl,
            "hanzi": hanzi,
            "frequency": frequency,
            "poj": poj,
            "tl_num": tl_num,
            "poj_num": poj_num,
            "tl_notone": tl_notone,
            "poj_notone": poj_notone,
            "tl_abbrev": tl_abbrev,
            "poj_abbrev": poj_abbrev,
            "is_variant": False,
            **{col: False for col in SOURCE_COLUMNS},
            "dev": False,
            "lkk": True,
        })
        existing_key_index[(hanzi, tl_key)] = -1  # sentinel: already added as new

    if marked_count:
        logger.info(f"  [lkk] Marked {marked_count} existing entries as lkk")
    if new_rows:
        logger.info(f"  [lkk] Added {len(new_rows)} new entries from LKK")

    return pd.DataFrame(new_rows) if new_rows else None


if __name__ == "__main__":
    main()

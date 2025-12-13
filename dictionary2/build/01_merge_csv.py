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

輸出：
- output/dictionary.csv
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

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
]
OUTPUT_DIR = "output"
OUTPUT_FILE = "dictionary.csv"
SCRIPT_NAME = "01_merge_csv"

SOURCE_COLUMNS = ["kautian", "taigitv", "itaigi", "sitbut", "taihoa", "taijit", "kungge"]


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

    # 去重複：相同 (hanzi, tl) 合併來源欄位
    agg_dict = {}
    for col in merged_df.columns:
        if col in ["hanzi", "tl"]:
            continue  # groupby key
        elif col in SOURCE_COLUMNS:
            agg_dict[col] = "any"
        elif col == "frequency":
            agg_dict[col] = "max"
        else:
            agg_dict[col] = "first"
    result_df = merged_df.groupby(["hanzi", "tl"], as_index=False).agg(agg_dict)

    # 排序：依 frequency 降序
    result_df = result_df.sort_values(
        ["frequency", "hanzi", "tl"],
        ascending=[False, True, True],
        ignore_index=True
    )

    logger.info(f"  After dedup: {len(result_df)} records")

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


if __name__ == "__main__":
    main()

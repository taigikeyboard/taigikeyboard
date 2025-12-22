#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
加入異用字變體

讀取 12_異用字/data/variants.csv，比對 (hanzi, tl)，
若有對應的異用字則生成新的變體資料。

輸入：data/kautian.csv
輸出：data/12_variant/kautian.csv
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/12_source/kautian.csv"
VARIANTS_FILE = "../12_異用字/data/variants.csv"
OUTPUT_DIR = "data"
OUTPUT_FILE = "kautian.csv"
SCRIPT_NAME = "12_add_variants"


def load_variants_map(filepath, logger):
    """
    載入異用字對照表，建立兩個對應：
    1. 正字對照表: (hanzi, tl) -> [variant] 用於生成變體
    2. 異用字對照表: (variant, tl) -> True 用於標記原本資料是否為異用字

    variants.csv 的 tl 欄位可能有 / 分隔的多讀音，需要展開
    """
    variants_map = {}  # (hanzi, tl) -> [variant]
    is_variant_set = set()  # (variant, tl) 集合，用於判斷原本資料是否為異用字

    if not os.path.exists(filepath):
        logger.warning(f"Variants file not found: {filepath}")
        return variants_map, is_variant_set

    df = pd.read_csv(filepath)
    logger.info(f"Loaded variants: {len(df)} records")

    for _, row in df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        variant = str(row["variant"]).strip()
        tl_field = str(row["tl"]).strip()

        if not hanzi or not variant or not tl_field:
            continue

        # 展開 / 分隔的多讀音
        for tl in tl_field.split("/"):
            tl = tl.strip().lower()
            if tl:
                # 正字對照表
                key = (hanzi, tl)
                if key not in variants_map:
                    variants_map[key] = []
                if variant not in variants_map[key]:
                    variants_map[key].append(variant)

                # 異用字對照表
                is_variant_set.add((variant, tl))

    logger.info(f"Variants map: {len(variants_map)} unique (hanzi, tl) pairs")
    logger.info(f"Is-variant set: {len(is_variant_set)} unique (variant, tl) pairs")
    return variants_map, is_variant_set


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入異用字對照表
    variants_map, _ = load_variants_map(VARIANTS_FILE, logger)

    # 載入辭典資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded dictionary: {total_count} records")

    # 建立原始辭典的 (hanzi, tl) 集合，用於判斷變體是否已存在於主條目
    original_entries = set()
    for _, row in df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        tl = str(row["tl"]).strip().lower()
        original_entries.add((hanzi, tl))
    logger.info(f"Original entries set: {len(original_entries)} unique (hanzi, tl) pairs")

    # 原始辭典資料一律為 is_variant=False（它們都是主條目）
    df["is_variant"] = False

    # 生成變體資料（從正字複製並替換 hanzi 為 variant）
    # 但如果 (variant, tl) 已存在於原始辭典，則跳過（該字本身是主條目，非異用字）
    new_rows = []
    matched_count = 0
    skipped_count = 0

    for _, row in df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        tl = str(row["tl"]).strip().lower()

        key = (hanzi, tl)
        if key in variants_map:
            matched_count += 1
            for variant in variants_map[key]:
                # 檢查 (variant, tl) 是否已存在於原始辭典
                if (variant, tl) in original_entries:
                    skipped_count += 1
                    continue
                # 複製原本的資料，替換 hanzi 為 variant
                new_row = row.copy()
                new_row["hanzi"] = variant
                new_row["is_variant"] = True
                new_rows.append(new_row)

    logger.info(f"Matched (hanzi, tl) pairs for generation: {matched_count}")
    logger.info(f"Skipped (variant already in original): {skipped_count}")
    logger.info(f"Generated variant records: {len(new_rows)}")

    # 合併原本資料與新生成的變體
    if new_rows:
        variant_df = pd.DataFrame(new_rows)
        result_df = pd.concat([df, variant_df], ignore_index=True)
    else:
        result_df = df

    logger.info(f"Final: {len(result_df)} records")

    # 儲存
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    result_df.to_csv(output_path, index=False)

    # 顯示範例
    logger.info(f"\n  [sample variant records]:")
    sample_variants = result_df[result_df["is_variant"] == True].head(10)
    for _, row in sample_variants.iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
標記異用字變體

讀取 12_異用字/data/variants.csv，檢查 hanzi 是否為異用字變體。

輸入：data/09_source/itaigi.csv
輸出：data/itaigi.csv
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/10_source/itaigi.csv"
VARIANTS_FILE = "../12_異用字/data/variants.csv"
OUTPUT_DIR = "data"
OUTPUT_FILE = "itaigi.csv"
SCRIPT_NAME = "10_add_variants"


def load_variant_set(filepath, logger):
    """
    載入異用字對照表，取得所有 (variant, tl) 的配對集合
    """
    variant_set = set()

    if not os.path.exists(filepath):
        logger.warning(f"Variants file not found: {filepath}")
        return variant_set

    df = pd.read_csv(filepath)
    logger.info(f"Loaded variants: {len(df)} records")

    for _, row in df.iterrows():
        variant = str(row["variant"]).strip()
        tl_field = str(row["tl"]).strip()

        if not variant or not tl_field:
            continue

        # 展開 / 分隔的多讀音
        for tl in tl_field.split("/"):
            tl = tl.strip().lower()
            if tl:
                variant_set.add((variant, tl))

    logger.info(f"Unique (variant, tl) pairs: {len(variant_set)}")
    return variant_set


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入異用字集合
    variant_set = load_variant_set(VARIANTS_FILE, logger)

    # 載入辭典資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded dictionary: {total_count} records")

    # 標記 is_variant（需要 hanzi 和 tl 都符合）
    df["is_variant"] = df.apply(
        lambda row: (str(row["hanzi"]).strip(), str(row["tl"]).strip().lower()) in variant_set,
        axis=1
    )

    variant_count = df["is_variant"].sum()
    logger.info(f"Records marked as variant: {variant_count}")

    # 儲存
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # 顯示範例
    logger.info(f"\n  [sample variant records]:")
    sample_variants = df[df["is_variant"] == True].head(10)
    for _, row in sample_variants.iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

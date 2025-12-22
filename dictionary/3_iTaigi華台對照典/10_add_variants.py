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
    載入異用字對照表，取得：
    1. variant_set: 所有 (字, 音節) 的配對集合（逐字拆分）
    2. hanzi_set: 所有 (字, 音節) 的配對集合（正字/主條目，逐字拆分）

    若某 (字, 音節) 同時存在於兩個集合，表示它既是異用字也是正字，
    此時應視為正字（主條目），不標記為異用字。
    """
    variant_set = set()
    hanzi_set = set()

    if not os.path.exists(filepath):
        logger.warning(f"Variants file not found: {filepath}")
        return variant_set, hanzi_set

    df = pd.read_csv(filepath)
    logger.info(f"Loaded variants: {len(df)} records")

    for _, row in df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        variant = str(row["variant"]).strip()
        tl_field = str(row["tl"]).strip()

        if not variant or not tl_field:
            continue

        # 展開 / 分隔的多讀音
        for tl in tl_field.split("/"):
            tl = tl.strip().lower()
            if not tl:
                continue

            # 拆分漢字（逐字）
            variant_chars = list(variant)
            hanzi_chars = list(hanzi) if hanzi else []

            # 拆分音節（-- 先換成 -，再以 - 分隔）
            syllables = tl.replace("--", "-").split("-")

            # 逐字配對存入 variant_set
            for i, char in enumerate(variant_chars):
                if i < len(syllables):
                    variant_set.add((char, syllables[i]))

            # 逐字配對存入 hanzi_set
            for i, char in enumerate(hanzi_chars):
                if i < len(syllables):
                    hanzi_set.add((char, syllables[i]))

    logger.info(f"Unique (char, syllable) variant pairs: {len(variant_set)}")
    logger.info(f"Unique (char, syllable) hanzi pairs: {len(hanzi_set)}")
    return variant_set, hanzi_set


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入異用字集合
    variant_set, hanzi_set = load_variant_set(VARIANTS_FILE, logger)

    # 載入辭典資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded dictionary: {total_count} records")

    # 標記 is_variant（檢查詞條中是否包含任一異用字）
    def check_is_variant(row):
        hanzi = str(row["hanzi"]).strip()
        tl = str(row["tl"]).strip().lower()

        # 拆分漢字（逐字）
        chars = list(hanzi)

        # 拆分音節（-- 先換成 -，再以 - 分隔）
        syllables = tl.replace("--", "-").split("-")

        # 配對檢查：只要任一 (字, 音節) 是異用字就標記
        for i, char in enumerate(chars):
            if i < len(syllables):
                syllable = syllables[i]
                key = (char, syllable)
                # 在 variant_set 中，但不在 hanzi_set 中，才是異用字
                if key in variant_set and key not in hanzi_set:
                    return True

        return False

    df["is_variant"] = df.apply(check_is_variant, axis=1)

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

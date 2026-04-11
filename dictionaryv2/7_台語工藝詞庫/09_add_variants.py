#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
標記異用字變體

讀取 12_異用字/data/variants.csv，檢查 hanzi 是否為異用字變體。

輸入：data/08_source/kungge.csv
輸出：data/kungge.csv
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/09_source/kungge.csv"
VARIANTS_FILE = "../12_異用字/data/variants.csv"
OUTPUT_DIR = "data"
OUTPUT_FILE = "kungge.csv"
SCRIPT_NAME = "09_add_variants"


def load_variant_set(filepath, logger):
    """
    載入異用字對照表，以完整詞為單位儲存。

    回傳 variant_entries: list of (variant_word, syllables_tuple)
    例如 ("復元", ("ho̍k", "guân"))

    標記時以完整異用字詞做子字串比對，避免拆字造成 false positive。
    例如「復元」不會誤標「多元化」。
    """
    variant_entries = []

    if not os.path.exists(filepath):
        logger.warning(f"Variants file not found: {filepath}")
        return variant_entries

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
            if not tl:
                continue
            syllables = tuple(tl.replace("--", "-").split("-"))
            variant_entries.append((variant, syllables))

    logger.info(f"Variant entries: {len(variant_entries)}")
    return variant_entries


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入異用字集合
    variant_entries = load_variant_set(VARIANTS_FILE, logger)

    # 載入辭典資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded dictionary: {total_count} records")

    # 標記 is_variant（檢查詞條是否包含完整異用字詞）
    def check_is_variant(row):
        hanzi = str(row["hanzi"]).strip()
        tl = str(row["tl"]).strip().lower()
        syllables = tl.replace("--", "-").split("-")

        for variant_word, variant_syllables in variant_entries:
            variant_len = len(variant_word)
            # 搜尋異用字詞是否為 hanzi 的子字串
            idx = hanzi.find(variant_word)
            while idx != -1:
                end_idx = idx + variant_len
                # 比對對應位置的音節是否吻合
                if end_idx <= len(syllables):
                    if tuple(syllables[idx:end_idx]) == variant_syllables:
                        return True
                idx = hanzi.find(variant_word, idx + 1)

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

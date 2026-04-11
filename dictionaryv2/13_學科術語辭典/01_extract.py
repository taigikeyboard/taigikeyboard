#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
01_raw -> 02_extracted

從 CSV 提取資料
輸出欄位：hanzi, tl
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/01_raw/stti.csv"
OUTPUT_DIR = "data/02_extracted"
OUTPUT_FILE = "stti.csv"
SCRIPT_NAME = "01_extract"


def expand_variant_readings(hanzi, tl):
    """
    Detect and expand variant readings in TL field.

    Source data uses space to separate variant readings (e.g.,
    "tāu-hū-gâm tāu-hū-giâm" = two readings for 豆腐岩).
    Space also appears as word boundary within a single reading (e.g.,
    "Tiong-huâ Bîn-kok" = one reading for 中華).

    Uses hanzi character count to distinguish: greedily group TL tokens
    until cumulative syllable count matches hanzi count. Multiple groups
    = variant readings. Single group = word boundaries within one reading.

    Returns:
        list of TL strings (1 if no variants, N if variants detected)
    """
    tokens = tl.split()
    if len(tokens) <= 1:
        return [tl]

    hanzi_count = len(hanzi)
    if hanzi_count == 0:
        return [tl]

    groups = []
    current_group = []
    current_syllables = 0

    for token in tokens:
        syllables = len(token.split("-"))
        current_group.append(token)
        current_syllables += syllables

        if current_syllables == hanzi_count:
            # Join multi-token group with hyphen (cleanup normalizes spaces to hyphens anyway)
            groups.append("-".join(current_group))
            current_group = []
            current_syllables = 0
        elif current_syllables > hanzi_count:
            # Overshot — can't form clean groups, keep original
            return [tl]

    if current_group:
        # Leftover tokens — grouping failed, keep original
        return [tl]

    return groups if len(groups) > 1 else [tl]


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入 CSV
    df = pd.read_csv(INPUT_FILE)
    logger.info(f"Loaded {len(df)} entries from CSV")

    # 提取欄位（臺灣台語詞彙 -> hanzi, 臺羅 -> tl）
    # 處理複數資料（以空白分隔）
    results = []
    skipped_mismatch = 0
    expanded_count = 0

    for _, row in df.iterrows():
        hanzi_raw = str(row["臺灣台語詞彙"]).strip()
        tl_raw = str(row["臺羅"]).strip()

        if not tl_raw or tl_raw == "nan":
            continue

        # 判斷是否為複數詞彙（hanzi 包含空白）
        if " " in hanzi_raw:
            # 複數詞彙：以空白分隔多個詞彙
            hanzi_list = hanzi_raw.split()
            # tl 先依據已有的連字符分組，再用空白分隔
            tl_list = tl_raw.split()

            # 數量不一致時跳過（記錄警告）
            if len(hanzi_list) != len(tl_list):
                skipped_mismatch += 1
                continue

            # 配對後加入結果
            for hanzi, tl in zip(hanzi_list, tl_list):
                if hanzi and tl:
                    results.append({
                        "hanzi": hanzi,
                        "tl": tl,
                    })
        else:
            # 單一詞彙：展開變體讀音（若有）
            tl_variants = expand_variant_readings(hanzi_raw, tl_raw)
            if len(tl_variants) > 1:
                expanded_count += 1
            for tl in tl_variants:
                results.append({
                    "hanzi": hanzi_raw,
                    "tl": tl,
                })

    # 轉換為 DataFrame
    df = pd.DataFrame(results)

    # 去重複
    before_dedup = len(df)
    df = df.drop_duplicates(subset=["hanzi", "tl"], keep="first")
    after_dedup = len(df)

    logger.info(f"Extracted: {len(results)} records")
    if expanded_count > 0:
        logger.info(f"Expanded variant readings: {expanded_count}")
    if skipped_mismatch > 0:
        logger.warning(f"Skipped (hanzi/tl count mismatch): {skipped_mismatch}")
    logger.info(f"Duplicates removed: {before_dedup - after_dedup}")
    logger.info(f"Final: {after_dedup} records")

    # 儲存
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # 顯示前 10 筆
    logger.info(f"\n  [sample records]:")
    for _, row in df.head(10).iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

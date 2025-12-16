#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
01_raw -> 02_extracted

從 JSON 提取資料
輸出欄位：hanzi, tl
"""

import os
import sys
import json
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/01_raw/scrape-20251210.json"
OUTPUT_DIR = "data/02_extracted"
OUTPUT_FILE = "kungge.csv"
SCRIPT_NAME = "01_extract"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入 JSON
    with open(INPUT_FILE, "r", encoding="utf-8") as f:
        data = json.load(f)

    logger.info(f"Loaded {len(data)} entries from JSON")

    results = []

    for item in data:
        hanzi = item.get("taiwaneseCharacters", "")
        tl = item.get("romanizationSystem", "")

        if not tl:
            continue

        results.append({
            "hanzi": hanzi,
            "tl": tl,
        })

    # 轉換為 DataFrame
    df = pd.DataFrame(results)

    # 去重複
    before_dedup = len(df)
    df = df.drop_duplicates(subset=["hanzi", "tl"], keep="first")
    after_dedup = len(df)

    logger.info(f"Extracted: {len(results)} records")
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

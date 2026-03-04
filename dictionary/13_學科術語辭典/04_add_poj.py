#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
增加 POJ 欄位：將 TL 羅馬字轉換為 POJ
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.romanization import convert_tl_to_poj

INPUT_FILE = "data/04_frequency/stti.csv"
OUTPUT_DIR = "data/05_poj"
OUTPUT_FILE = "stti.csv"
SCRIPT_NAME = "04_add_poj"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")

    # TL → POJ
    error_count = 0
    def safe_convert(tl):
        nonlocal error_count
        try:
            return convert_tl_to_poj(str(tl))
        except Exception:
            error_count += 1
            return str(tl)

    df["poj"] = df["tl"].apply(safe_convert)

    if error_count > 0:
        logger.info(f"Conversion errors: {error_count}")

    # 儲存
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # 統計
    logger.info("")
    logger.info(f"Output columns: {list(df.columns)}")
    logger.info(f"Total records: {len(df)}")

    # 顯示範例
    logger.info(f"\n  [sample records]:")
    for _, row in df.head(10).iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl']} -> {row['poj']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
增加無聲調欄位：
1. tl_notone: TL 無聲調版本
2. poj_notone: POJ 無聲調版本
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.notone import remove_tone

INPUT_FILE = "data/09_numtone/kautian.csv"
OUTPUT_DIR = "data/10_notone"
OUTPUT_FILE = "kautian.csv"
SCRIPT_NAME = "09_add_notone"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")

    # 產生無聲調版本（只有數字在中間時才產生）
    df["tl_notone"] = df["tl_num"].apply(lambda x: remove_tone(str(x)))
    df["poj_notone"] = df["poj_num"].apply(lambda x: remove_tone(str(x)))

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
        logger.info(f"    {row['hanzi']}: {row['tl_num']} -> {row['tl_notone']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

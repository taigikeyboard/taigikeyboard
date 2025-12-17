#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
最終輸出：加入 sitbut 欄位 (True)
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/09_abbrev/sitbut.csv"
OUTPUT_DIR = "data/10_source"
OUTPUT_FILE = "sitbut.csv"
SCRIPT_NAME = "09_add_source"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")

    # 如果 hanzi 等於 tl 或 poj，代表沒有漢字，設為空
    no_hanzi_mask = (df["hanzi"] == df["tl"]) | (df["hanzi"] == df["poj"])
    no_hanzi_count = no_hanzi_mask.sum()
    df.loc[no_hanzi_mask, "hanzi"] = ""
    logger.info(f"Records without hanzi: {no_hanzi_count}")

    # 加入 sitbut 欄位
    df["sitbut"] = True

    # 儲存
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # 統計
    logger.info("")
    logger.info(f"Output columns: {list(df.columns)}")
    logger.info(f"Total records: {len(df)}")

    # 顯示前 10 筆
    logger.info(f"\n  [sample records]:")
    for _, row in df.head(10).iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl']} / {row['poj']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
新增縮寫欄位：
1. tl_abbrev: TL 音節首字母縮寫（兩個以上音節才產生）
2. poj_abbrev: POJ 音節首字母縮寫（兩個以上音節才產生）
3. 例如: "gua2-si7" → "gs"
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.abbrev import extract_abbrev

INPUT_FILE = "data/10_notone/kautian.csv"
OUTPUT_DIR = "data/11_abbrev"
OUTPUT_FILE = "kautian.csv"
SCRIPT_NAME = "10_add_abbrev"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")

    # 新增縮寫欄位（使用帶連字符的 tl/poj 欄位）
    df["tl_abbrev"] = df["tl"].apply(extract_abbrev)
    df["poj_abbrev"] = df["poj"].apply(extract_abbrev)

    # 統計有縮寫的數量
    abbrev_count = (df["tl_abbrev"] != "").sum()

    # 儲存
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # 統計
    logger.info("")
    logger.info(f"Total records: {total_count}")
    logger.info(f"Records with abbrev: {abbrev_count}")
    logger.info(f"Output columns: {list(df.columns)}")

    # 顯示縮寫範例
    logger.info(f"\n  [abbrev samples]:")
    abbrev_samples = df[df["tl_abbrev"] != ""].head(20)
    for _, row in abbrev_samples.iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl_num']} -> {row['tl_abbrev']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

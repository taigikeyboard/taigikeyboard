#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
加入詞頻資料
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.frequency import load_frequency_map, get_frequency

INPUT_FILE = "data/04_cleaned/sitbut.csv"
FREQ_FILE = "../0_其他資料/char_freq_merged.txt"
OUTPUT_DIR = "data/05_frequency"
OUTPUT_FILE = "sitbut.csv"
SCRIPT_NAME = "04_add_frequency"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入詞頻資料
    freq_map = load_frequency_map(FREQ_FILE)
    logger.info(f"Loaded {len(freq_map)} frequency entries")

    # 載入辭典資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} dictionary entries")

    # 加入詞頻欄位
    df["frequency"] = df.apply(
        lambda row: get_frequency(str(row["hanzi"]), str(row["tl"]), freq_map),
        axis=1
    )

    # 排序
    df = df.sort_values(
        ["frequency", "hanzi", "tl"],
        ascending=[False, True, True],
        ignore_index=True
    )

    # 儲存
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # 統計
    freq_series = df["frequency"]
    logger.info(f"\n  [frequency distribution]:")
    logger.info(f"    max: {freq_series.max()}")
    logger.info(f"    min: {freq_series.min()}")
    logger.info(f"    mean: {freq_series.mean():.0f}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

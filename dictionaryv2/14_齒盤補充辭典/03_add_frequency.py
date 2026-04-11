#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Add frequency data
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.frequency import load_frequency_map, get_frequency

INPUT_FILE = "data/03_cleaned/khpoo.csv"
FREQ_FILE = "../0_其他資料/char_freq_merged.txt"
OUTPUT_DIR = "data/04_frequency"
OUTPUT_FILE = "khpoo.csv"
SCRIPT_NAME = "03_add_frequency"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Load frequency data
    freq_map = load_frequency_map(FREQ_FILE)
    logger.info(f"Loaded {len(freq_map)} frequency entries")

    # Load dictionary data
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} dictionary entries")

    # Add frequency column
    df["frequency"] = df.apply(
        lambda row: get_frequency(str(row["hanzi"]), str(row["tl"]), freq_map),
        axis=1
    )

    # Sort
    df = df.sort_values(
        ["frequency", "hanzi", "tl"],
        ascending=[False, True, True],
        ignore_index=True
    )

    # Save
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # Stats
    freq_series = df["frequency"]
    logger.info(f"\n  [frequency distribution]:")
    logger.info(f"    max: {freq_series.max()}")
    logger.info(f"    min: {freq_series.min()}")
    logger.info(f"    mean: {freq_series.mean():.0f}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
資料清理
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.cleanup import cleanup_dataframe

INPUT_FILE = "data/03_expanded/itaigi.csv"
OUTPUT_DIR = "data/04_cleaned"
OUTPUT_FILE = "itaigi.csv"
SCRIPT_NAME = "03_cleanup"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    df = pd.read_csv(INPUT_FILE)
    df, dropped_df = cleanup_dataframe(df, logger=logger, check_roman_in_hanzi=True)

    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)
    logger.info(f"\nSaved: {output_path}")

    if len(dropped_df) > 0:
        drop_path = os.path.join(OUTPUT_DIR, "drop.csv")
        dropped_df.to_csv(drop_path, index=False)
        logger.info(f"Saved dropped: {drop_path} ({len(dropped_df)} records)")


if __name__ == "__main__":
    main()

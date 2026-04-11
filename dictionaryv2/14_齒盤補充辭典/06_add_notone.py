#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Add no-tone columns:
1. tl_notone: TL without tones
2. poj_notone: POJ without tones
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.notone import remove_tone

INPUT_FILE = "data/06_numtone/khpoo.csv"
OUTPUT_DIR = "data/07_notone"
OUTPUT_FILE = "khpoo.csv"
SCRIPT_NAME = "06_add_notone"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Load data
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")

    # Generate no-tone versions
    df["tl_notone"] = df["tl_num"].apply(lambda x: remove_tone(str(x)))
    df["poj_notone"] = df["poj_num"].apply(lambda x: remove_tone(str(x)))

    # Save
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # Stats
    logger.info("")
    logger.info(f"Output columns: {list(df.columns)}")
    logger.info(f"Total records: {len(df)}")

    # Show samples
    logger.info(f"\n  [sample records]:")
    for _, row in df.head(10).iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl_num']} -> {row['tl_notone']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Add numeric tone columns:
1. tl_num: TL numeric tone version
2. poj_num: POJ numeric tone version (ASCII)
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.romanization import to_numeric_tone

INPUT_FILE = "data/05_poj/khpoo.csv"
OUTPUT_DIR = "data/06_numtone"
OUTPUT_FILE = "khpoo.csv"
SCRIPT_NAME = "05_add_numtone"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Load data
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")

    # Generate numeric tone versions
    df["tl_num"] = df["tl"].apply(lambda x: to_numeric_tone(str(x)))
    df["poj_num"] = df["poj"].apply(lambda x: to_numeric_tone(str(x), ascii_only=True))

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
        logger.info(f"    {row['hanzi']}: {row['tl']} -> {row['tl_num']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

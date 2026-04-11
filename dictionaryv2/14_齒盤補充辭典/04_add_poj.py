#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Add POJ column: convert TL to POJ
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.romanization import convert_tl_to_poj

INPUT_FILE = "data/04_frequency/khpoo.csv"
OUTPUT_DIR = "data/05_poj"
OUTPUT_FILE = "khpoo.csv"
SCRIPT_NAME = "04_add_poj"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Load data
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")

    # TL -> POJ
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
        logger.info(f"    {row['hanzi']}: {row['tl']} -> {row['poj']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

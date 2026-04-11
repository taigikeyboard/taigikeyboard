#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
01_raw -> 02_extracted

From CSV extract data (no header, format: hanzi,tl)
Output columns: hanzi, tl
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/01_raw/khpoo.csv"
OUTPUT_DIR = "data/02_extracted"
OUTPUT_FILE = "khpoo.csv"
SCRIPT_NAME = "01_extract"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Load CSV (no header)
    df = pd.read_csv(INPUT_FILE, header=None, names=["hanzi", "tl"])
    logger.info(f"Loaded {len(df)} entries from CSV")

    # Remove rows with empty tl
    df = df.dropna(subset=["tl"])
    df = df[df["tl"].str.strip() != ""]

    # Deduplicate
    before_dedup = len(df)
    df = df.drop_duplicates(subset=["hanzi", "tl"], keep="first")
    after_dedup = len(df)

    logger.info(f"Extracted: {before_dedup} records")
    logger.info(f"Duplicates removed: {before_dedup - after_dedup}")
    logger.info(f"Final: {after_dedup} records")

    # Save
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # Show samples
    logger.info(f"\n  [sample records]:")
    for _, row in df.head(10).iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

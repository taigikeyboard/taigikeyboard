#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Final output: add khpoo source column (True)
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/08_abbrev/khpoo.csv"
OUTPUT_DIR = "data/09_source"
OUTPUT_FILE = "khpoo.csv"
SCRIPT_NAME = "08_add_source"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Load data
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")

    # If hanzi equals tl or poj, it means no hanzi, set to empty
    no_hanzi_mask = (df["hanzi"] == df["tl"]) | (df["hanzi"] == df["poj"])
    no_hanzi_count = no_hanzi_mask.sum()
    df.loc[no_hanzi_mask, "hanzi"] = ""
    logger.info(f"Records without hanzi: {no_hanzi_count}")

    # Add khpoo source column
    df["khpoo"] = True

    # Save
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    logger.info("")
    logger.info(f"Output columns: {list(df.columns)}")
    logger.info(f"Total records: {len(df)}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

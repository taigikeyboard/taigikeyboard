#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Add abbreviation columns:
1. tl_abbrev: TL syllable initial abbreviation (only for 2+ syllables)
2. poj_abbrev: POJ syllable initial abbreviation (only for 2+ syllables)
3. Example: "gua2-si7" -> "gs"
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.abbrev import extract_abbrev

INPUT_FILE = "data/07_notone/khpoo.csv"
OUTPUT_DIR = "data/08_abbrev"
OUTPUT_FILE = "khpoo.csv"
SCRIPT_NAME = "07_add_abbrev"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Load data
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")

    # Add abbreviation columns
    df["tl_abbrev"] = df["tl"].apply(extract_abbrev)
    df["poj_abbrev"] = df["poj"].apply(extract_abbrev)

    # Stats
    abbrev_count = (df["tl_abbrev"] != "").sum()

    # Save
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    logger.info("")
    logger.info(f"Total records: {total_count}")
    logger.info(f"Records with abbrev: {abbrev_count}")
    logger.info(f"Output columns: {list(df.columns)}")

    # Show abbreviation samples
    logger.info(f"\n  [abbrev samples]:")
    abbrev_samples = df[df["tl_abbrev"] != ""].head(20)
    for _, row in abbrev_samples.iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl_num']} -> {row['tl_abbrev']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

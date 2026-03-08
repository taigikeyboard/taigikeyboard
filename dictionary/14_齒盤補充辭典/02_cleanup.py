#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Data cleanup for 齒盤補充辭典 (khpoo)

This dictionary is pre-processed by the contributor and requires no
filtering or deduplication. Only romanization normalization is applied
(Unicode NFC, lowercase, space-to-hyphen).
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.cleanup import normalize_roman

INPUT_FILE = "data/02_extracted/khpoo.csv"
OUTPUT_DIR = "data/03_cleaned"
OUTPUT_FILE = "khpoo.csv"
SCRIPT_NAME = "02_cleanup"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")
    logger.info("  Skipping validation/dedup (pre-processed dictionary)")

    # Only normalize romanization (NFC, lowercase, space-to-hyphen)
    if "tl" in df.columns:
        df["tl"] = df["tl"].apply(normalize_roman)

    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)
    logger.info(f"  Final: {total_count} records")
    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

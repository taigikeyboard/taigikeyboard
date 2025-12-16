#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
01_raw_ods -> 02_raw_csv

Extract all sheets from ODS file to individual CSV files.
"""

import os
import logging
from datetime import datetime
import pandas as pd

INPUT_FILE = "data/01_raw/kautian.ods"
OUTPUT_DIR = "data/02_extracted"
LOG_DIR = "logs"
SCRIPT_NAME = "01_extract"


def setup_logging():
    os.makedirs(LOG_DIR, exist_ok=True)
    log_file = os.path.join(LOG_DIR, f"{SCRIPT_NAME}.log")

    logging.basicConfig(
        level=logging.INFO,
        format="%(message)s",
        handlers=[
            logging.FileHandler(log_file, mode="w", encoding="utf-8"),
            logging.StreamHandler(),
        ],
    )
    return logging.getLogger(__name__)


def main():
    logger = setup_logging()
    logger.info(f"{'=' * 50}")
    logger.info(f"{SCRIPT_NAME}")
    logger.info(f"Run: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    logger.info(f"{'=' * 50}")
    logger.info(f"Input:  {INPUT_FILE}")
    logger.info(f"Output: {OUTPUT_DIR}/")
    logger.info("")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    sheets = pd.read_excel(INPUT_FILE, sheet_name=None, engine="odf")

    total_rows = 0
    for sheet_name, df in sheets.items():
        output_path = os.path.join(OUTPUT_DIR, f"{sheet_name}.csv")
        df.to_csv(output_path, index=False)
        total_rows += len(df)
        cols = ", ".join(df.columns.tolist())
        logger.info(f"  {sheet_name}: {len(df)} rows")
        logger.info(f"    columns: [{cols}]")

    logger.info("")
    logger.info(f"Total: {len(sheets)} sheets, {total_rows} rows")


if __name__ == "__main__":
    main()

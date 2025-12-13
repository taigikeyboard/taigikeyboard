#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
raw_csv -> 02_selected

1. 讀取原始 CSV
2. 選取欄位：KipUnicode, HanLoTaibunKip
3. 重新命名：tl, hanzi
"""

import os
import logging
from datetime import datetime
import pandas as pd

INPUT_FILE = "data/01_raw/ChhoeTaigi_iTaigiHoataiTuichiautian.csv"
OUTPUT_DIR = "data/02_selected"
LOG_DIR = "logs"
SCRIPT_NAME = "01_select"

COLUMN_MAPPING = {
    "KipUnicode": "tl",
    "HanLoTaibunKip": "hanzi",
}


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

    df = pd.read_csv(INPUT_FILE)
    logger.info(f"Raw CSV: {len(df)} rows")
    logger.info(f"Original columns: {list(df.columns)}")
    logger.info("")

    # Select and rename columns
    selected_columns = list(COLUMN_MAPPING.keys())
    df_selected = df[selected_columns].copy()
    df_selected = df_selected.rename(columns=COLUMN_MAPPING)

    # Drop rows where tl is empty
    before_drop = len(df_selected)
    df_selected = df_selected.dropna(subset=["tl"])
    df_selected = df_selected[df_selected["tl"].str.strip() != ""]
    after_drop = len(df_selected)

    logger.info(f"Selected columns: {list(COLUMN_MAPPING.values())}")
    logger.info(f"Dropped {before_drop - after_drop} rows with empty tl")
    logger.info("")

    # Output
    output_file = os.path.join(OUTPUT_DIR, "itaigi.csv")
    df_selected.to_csv(output_file, index=False)

    logger.info(f"Output: {len(df_selected)} rows")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
raw_csv -> 02_selected

1. 讀取原始 CSV
2. 選取欄位：KipUnicode, HanLoTaibunPoj
3. 重新命名：tl, hanzi
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/01_raw/ChhoeTaigi_TaioanSitbutMialui.csv"
OUTPUT_DIR = "data/02_selected"
SCRIPT_NAME = "01_select"

COLUMN_MAPPING = {
    "KipUnicode": "tl",
    "HanLoTaibunPoj": "hanzi",
}


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/")

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
    output_file = os.path.join(OUTPUT_DIR, "sitbut.csv")
    df_selected.to_csv(output_file, index=False)

    logger.info(f"Output: {len(df_selected)} rows")


if __name__ == "__main__":
    main()

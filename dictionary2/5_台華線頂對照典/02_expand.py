#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
02_selected -> 03_expanded

展開含 "/" 或 "," 分隔符的欄位成多列
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.expand import expand_dataframe

INPUT_DIR = "data/02_selected"
OUTPUT_DIR = "data/03_expanded"
SCRIPT_NAME = "02_expand"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, f"{INPUT_DIR}/", f"{OUTPUT_DIR}/")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    input_path = os.path.join(INPUT_DIR, "taihoa.csv")
    output_path = os.path.join(OUTPUT_DIR, "taihoa.csv")

    df = pd.read_csv(input_path)
    original_count = len(df)
    logger.info(f"Input: {original_count} rows")

    df = expand_dataframe(df, logger=logger)
    df.to_csv(output_path, index=False)

    logger.info(f"Output: {len(df)} rows (+{len(df) - original_count})")


if __name__ == "__main__":
    main()

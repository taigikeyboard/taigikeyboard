#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
03_selected -> 04_expanded

Expand fields with "/" or "," separator into multiple rows.
Example: ji/li -> ji and li
Example: guan,gun -> guan and gun
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.expand import expand_dataframe

INPUT_DIR = "data/03_selected"
OUTPUT_DIR = "data/04_expanded"
SCRIPT_NAME = "03_expand"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, f"{INPUT_DIR}/", f"{OUTPUT_DIR}/")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    total_before = 0
    total_after = 0

    for filename in sorted(os.listdir(INPUT_DIR)):
        if not filename.endswith(".csv"):
            continue

        input_path = os.path.join(INPUT_DIR, filename)
        output_path = os.path.join(OUTPUT_DIR, filename)

        df = pd.read_csv(input_path)
        total_before += len(df)

        df = expand_dataframe(df, logger=logger)
        df.to_csv(output_path, index=False)
        total_after += len(df)

    logger.info("")
    logger.info(f"Total: {total_before} -> {total_after} (+{total_after - total_before})")


if __name__ == "__main__":
    main()

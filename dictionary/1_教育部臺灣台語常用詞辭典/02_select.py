#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
02_raw_csv -> 03_selected

1. Select relevant tables
2. Select relevant columns
3. Unpivot (pivot longer) for multi-column data
4. Rename columns: 漢字 -> hanzi, 羅馬字 -> tl
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_DIR = "data/02_extracted"
OUTPUT_DIR = "data/03_selected"
SCRIPT_NAME = "02_select"

# 統一欄位名稱
COLUMN_RENAME = {
    "漢字": "hanzi",
    "羅馬字": "tl",
    "異用字": "hanzi",  # 異用字表的漢字欄位
}

TABLES = {
    "詞目.csv": {
        "columns": ["漢字", "羅馬字"],
    },
    "又唸作.csv": {
        "columns": ["漢字", "羅馬字"],
    },
    "合音唸作.csv": {
        "columns": ["漢字", "羅馬字"],
    },
    "俗唸作.csv": {
        "columns": ["漢字", "羅馬字"],
    },
    "詞彙比較.csv": {
        "columns": ["漢字", "羅馬字"],
    },
    "名.csv": {
        "columns": ["漢字", "羅馬字"],
    },
    "姓.csv": {
        "columns": ["漢字", "羅馬字"],
    },
    "異用字.csv": {
        "columns": ["漢字", "異用字"],
    },
}

DIALECT_COLUMNS = [
    "鹿港偏泉腔",
    "三峽偏泉腔",
    "臺北偏泉腔",
    "宜蘭偏漳腔",
    "臺南混合腔",
    "高雄混合腔",
    "金門偏泉腔",
    "馬公偏泉腔",
    "新竹偏泉腔",
    "臺中偏漳腔",
]


def process_standard_table(input_file, config, logger):
    df = pd.read_csv(os.path.join(INPUT_DIR, input_file))
    columns = config["columns"]
    output_file = config.get("output_file", input_file)
    df_selected = df[columns].copy()

    # Rename columns
    rename_map = {}
    for col in columns:
        if col in COLUMN_RENAME:
            rename_map[col] = COLUMN_RENAME[col]
    df_selected = df_selected.rename(columns=rename_map)

    return df_selected, output_file


def process_dialect_table(logger):
    df = pd.read_csv(os.path.join(INPUT_DIR, "語音差異.csv"))
    df_long = df.melt(
        id_vars=["漢字"],
        value_vars=DIALECT_COLUMNS,
        var_name="腔調",
        value_name="羅馬字",
    )
    df_long = df_long.dropna(subset=["羅馬字"])
    df_long = df_long[df_long["羅馬字"].str.strip() != ""]
    df_long = df_long[["漢字", "羅馬字"]]

    # Rename columns
    df_long = df_long.rename(columns=COLUMN_RENAME)

    return df_long, "語音差異.csv"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, f"{INPUT_DIR}/", f"{OUTPUT_DIR}/")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    total_rows = 0
    for input_file, config in TABLES.items():
        df, output_file = process_standard_table(input_file, config, logger)
        output_path = os.path.join(OUTPUT_DIR, output_file)
        df.to_csv(output_path, index=False)
        total_rows += len(df)
        logger.info(f"  {output_file}: {len(df)} rows -> [{', '.join(df.columns)}]")

    df, output_file = process_dialect_table(logger)
    output_path = os.path.join(OUTPUT_DIR, output_file)
    df.to_csv(output_path, index=False)
    total_rows += len(df)
    logger.info(f"  {output_file}: {len(df)} rows -> [{', '.join(df.columns)}]")

    logger.info("")
    logger.info(f"Total: {len(TABLES) + 1} files, {total_rows} rows")


if __name__ == "__main__":
    main()

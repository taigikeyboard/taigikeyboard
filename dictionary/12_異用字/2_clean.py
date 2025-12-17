#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
資料清理：異用字表

輸入：data/3_異用字_眾人合作版.csv
輸出：data/variants.csv
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header
from common.cleanup import (
    clean_brackets,
    normalize_roman,
    is_valid_tl,
    is_valid_hanzi,
)

INPUT_FILE = "data/3_異用字_眾人合作版.csv"
OUTPUT_DIR = "data"
OUTPUT_FILE = "variants.csv"
SCRIPT_NAME = "2_clean"


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # 載入資料
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded {total_count} records")
    logger.info(f"Columns: {list(df.columns)}")

    # 統一欄位名稱
    df = df.rename(columns={
        "漢字": "hanzi",
        "異用字": "variant",
        "羅馬字": "tl",
    })

    # 1. 移除括號標註
    for col in ["hanzi", "variant", "tl"]:
        if col in df.columns:
            df[col] = df[col].apply(clean_brackets)

    # 2. 移除空白列
    before = len(df)
    df = df[df["hanzi"].str.strip() != ""]
    df = df[df["variant"].str.strip() != ""]
    df = df[df["tl"].str.strip() != ""]
    removed = before - len(df)
    if removed > 0:
        logger.info(f"  Removed empty rows: {removed}")

    # 3. 剔除無效的 hanzi
    valid_hanzi_mask = df["hanzi"].apply(is_valid_hanzi)
    valid_variant_mask = df["variant"].apply(is_valid_hanzi)
    invalid_count = (~valid_hanzi_mask | ~valid_variant_mask).sum()
    if invalid_count > 0:
        logger.info(f"  Removed invalid hanzi/variant: {invalid_count}")
    df = df[valid_hanzi_mask & valid_variant_mask]

    # 4. 驗證並正規化羅馬字（逐個音節檢查，支援 / 分隔的多讀音）
    def validate_and_normalize_tl(tl_str):
        """驗證並正規化羅馬字，回傳 (is_valid, normalized)"""
        if pd.isna(tl_str) or str(tl_str).strip() == "":
            return False, ""

        # 分割多個讀音
        readings = [r.strip() for r in str(tl_str).split("/") if r.strip()]
        valid_readings = []

        for reading in readings:
            if is_valid_tl(reading):
                valid_readings.append(normalize_roman(reading))

        if not valid_readings:
            return False, ""

        return True, "/".join(valid_readings)

    # 套用驗證與正規化
    validation_results = df["tl"].apply(validate_and_normalize_tl)
    df["tl_valid"] = validation_results.apply(lambda x: x[0])
    df["tl"] = validation_results.apply(lambda x: x[1])

    invalid_tl_count = (~df["tl_valid"]).sum()
    if invalid_tl_count > 0:
        logger.info(f"  Removed invalid TL: {invalid_tl_count}")
    df = df[df["tl_valid"]]
    df = df.drop(columns=["tl_valid"])

    # 5. 去重複（hanzi + variant + tl）
    before_dedup = len(df)
    df = df.drop_duplicates(subset=["hanzi", "variant", "tl"], keep="first")
    removed = before_dedup - len(df)
    if removed > 0:
        logger.info(f"  Removed duplicates: {removed}")

    # 6. 排序
    df = df.sort_values(["hanzi", "variant", "tl"], ignore_index=True)

    logger.info(f"  Final: {len(df)} records")

    # 儲存
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # 顯示前 10 筆
    logger.info(f"\n  [sample records]:")
    for _, row in df.head(10).iterrows():
        logger.info(f"    {row['hanzi']} → {row['variant']}: {row['tl']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

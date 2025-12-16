#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
05_cleaned -> 06_merged

Merge all cleaned datasets (except 異用字.csv)
into a single CSV file with deduplicated entries.
"""

import os
import sys
from collections import defaultdict
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_DIR = "data/04_expanded"
OUTPUT_DIR = "data/05_merged"
OUTPUT_FILE = "kautian.csv"
SCRIPT_NAME = "04_merge"

EXCLUDE_FILES = {"異用字.csv"}


def find_duplicates(all_records):
    """Find duplicates and group by source files."""
    seen = defaultdict(list)

    for record in all_records:
        key = (record["hanzi"], record["tl"])
        seen[key].append(record["source"])

    duplicates = {k: v for k, v in seen.items() if len(v) > 1}
    return duplicates


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, f"{INPUT_DIR}/", f"{OUTPUT_DIR}/{OUTPUT_FILE}")
    logger.info(f"Exclude: {', '.join(sorted(EXCLUDE_FILES))}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    all_dfs = []
    all_records = []
    total_rows = 0

    for filename in sorted(os.listdir(INPUT_DIR)):
        if not filename.endswith(".csv"):
            continue
        if filename in EXCLUDE_FILES:
            logger.info(f"  [skip] {filename}")
            continue

        input_path = os.path.join(INPUT_DIR, filename)
        df = pd.read_csv(input_path)
        row_count = len(df)
        total_rows += row_count
        all_dfs.append(df)

        for _, row in df.iterrows():
            all_records.append({
                "hanzi": row["hanzi"],
                "tl": row["tl"],
                "source": filename,
            })

        logger.info(f"  [load] {filename}: {row_count} rows")

    logger.info("")
    logger.info(f"Total loaded: {total_rows} rows")

    # Find and log duplicates
    duplicates = find_duplicates(all_records)
    if duplicates:
        by_sources = defaultdict(list)
        for (hanzi, tl), sources in duplicates.items():
            source_key = " + ".join(sorted(set(sources)))
            by_sources[source_key].append((hanzi, tl, sources))

        for source_key in sorted(by_sources.keys()):
            items = by_sources[source_key]
            logger.info(f"\n  [duplicates] {source_key} ({len(items)} records):")
            for hanzi, tl, sources in sorted(items)[:30]:
                logger.info(f"    {hanzi}: {tl}")
            if len(items) > 30:
                logger.info(f"    ... and {len(items) - 30} more")

    # Merge all dataframes
    merged_df = pd.concat(all_dfs, ignore_index=True)

    # Remove duplicates
    before_dedup = len(merged_df)
    merged_df = merged_df.drop_duplicates(subset=["hanzi", "tl"], keep="first")
    after_dedup = len(merged_df)
    removed = before_dedup - after_dedup

    logger.info("")
    logger.info(f"After dedup: {after_dedup} rows (-{removed} duplicates)")

    # Sort
    merged_df = merged_df.sort_values(["hanzi", "tl"], ignore_index=True)

    # Save
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    merged_df.to_csv(output_path, index=False)
    logger.info(f"Saved: {output_path}")


if __name__ == "__main__":
    main()

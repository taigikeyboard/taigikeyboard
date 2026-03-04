#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Mark variant characters

Read 12_異用字/data/variants.csv and check if hanzi is a variant.

Input: data/09_source/khpoo.csv
Output: data/khpoo.csv
"""

import os
import sys
import pandas as pd

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from common.logging_utils import setup_logging, log_header

INPUT_FILE = "data/09_source/khpoo.csv"
VARIANTS_FILE = "../12_異用字/data/variants.csv"
OUTPUT_DIR = "data"
OUTPUT_FILE = "khpoo.csv"
SCRIPT_NAME = "09_add_variants"


def load_variant_set(filepath, logger):
    """
    Load variant character mapping and get:
    1. variant_set: all (char, syllable) pairs (per-character split)
    2. hanzi_set: all (char, syllable) pairs (standard/main entries, per-character split)

    If a (char, syllable) exists in both sets, it is both a variant and a standard character,
    and should be treated as standard (not marked as variant).
    """
    variant_set = set()
    hanzi_set = set()

    if not os.path.exists(filepath):
        logger.warning(f"Variants file not found: {filepath}")
        return variant_set, hanzi_set

    df = pd.read_csv(filepath)
    logger.info(f"Loaded variants: {len(df)} records")

    for _, row in df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        variant = str(row["variant"]).strip()
        tl_field = str(row["tl"]).strip()

        if not variant or not tl_field:
            continue

        # Expand / separated multiple readings
        for tl in tl_field.split("/"):
            tl = tl.strip().lower()
            if not tl:
                continue

            # Split characters
            variant_chars = list(variant)
            hanzi_chars = list(hanzi) if hanzi else []

            # Split syllables (-- to -, then split by -)
            syllables = tl.replace("--", "-").split("-")

            # Per-character pair into variant_set
            for i, char in enumerate(variant_chars):
                if i < len(syllables):
                    variant_set.add((char, syllables[i]))

            # Per-character pair into hanzi_set
            for i, char in enumerate(hanzi_chars):
                if i < len(syllables):
                    hanzi_set.add((char, syllables[i]))

    logger.info(f"Unique (char, syllable) variant pairs: {len(variant_set)}")
    logger.info(f"Unique (char, syllable) hanzi pairs: {len(hanzi_set)}")
    return variant_set, hanzi_set


def main():
    logger = setup_logging(SCRIPT_NAME)
    log_header(logger, SCRIPT_NAME, INPUT_FILE, f"{OUTPUT_DIR}/{OUTPUT_FILE}")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    # Load variant set
    variant_set, hanzi_set = load_variant_set(VARIANTS_FILE, logger)

    # Load dictionary data
    df = pd.read_csv(INPUT_FILE)
    total_count = len(df)
    logger.info(f"Loaded dictionary: {total_count} records")

    # Mark is_variant
    def check_is_variant(row):
        hanzi = str(row["hanzi"]).strip()
        tl = str(row["tl"]).strip().lower()

        chars = list(hanzi)
        syllables = tl.replace("--", "-").split("-")

        for i, char in enumerate(chars):
            if i < len(syllables):
                syllable = syllables[i]
                key = (char, syllable)
                if key in variant_set and key not in hanzi_set:
                    return True

        return False

    df["is_variant"] = df.apply(check_is_variant, axis=1)

    variant_count = df["is_variant"].sum()
    logger.info(f"Records marked as variant: {variant_count}")

    # Save
    output_path = os.path.join(OUTPUT_DIR, OUTPUT_FILE)
    df.to_csv(output_path, index=False)

    # Show samples
    logger.info(f"\n  [sample variant records]:")
    sample_variants = df[df["is_variant"] == True].head(10)
    for _, row in sample_variants.iterrows():
        logger.info(f"    {row['hanzi']}: {row['tl']}")

    logger.info(f"\nSaved: {output_path}")


if __name__ == "__main__":
    main()

# -*- coding: utf-8 -*-
"""Shared helpers for expanding delimiter-separated fields."""

import re
import pandas as pd

DEFAULT_SEPARATORS = ["/", ","]


def split_variants(value, separators=None):
    """Split a delimiter-separated value into its variants (default
    separators "/" and ",").
    """
    if separators is None:
        separators = DEFAULT_SEPARATORS

    if pd.isna(value) or str(value).strip() == "":
        return []

    pattern = "|".join(re.escape(sep) for sep in separators)
    variants = [v.strip() for v in re.split(pattern, str(value)) if v.strip()]
    return variants


def expand_dataframe(df, separators=None, logger=None):
    """Expand and pair the `tl` and `hanzi` columns by index; if only one
    side has multiple values, its single counterpart repeats to match.
    `df` needs a `tl` column (`hanzi` optional). Returns the expanded
    DataFrame.
    """
    if separators is None:
        separators = DEFAULT_SEPARATORS

    expanded_rows = []
    expanded_count = 0

    for _, row in df.iterrows():
        tl_variants = split_variants(row.get("tl", ""), separators)
        hanzi_variants = split_variants(row.get("hanzi", ""), separators)

        if not tl_variants:
            continue

        max_variants = max(len(tl_variants), len(hanzi_variants)) if hanzi_variants else len(tl_variants)

        if len(tl_variants) == 1 and max_variants > 1:
            tl_variants = tl_variants * max_variants
        if len(hanzi_variants) == 1 and max_variants > 1:
            hanzi_variants = hanzi_variants * max_variants

        if max_variants > 1:
            expanded_count += 1

        for i in range(max_variants):
            new_row = {
                "tl": tl_variants[i] if i < len(tl_variants) else tl_variants[0],
                "hanzi": hanzi_variants[i] if i < len(hanzi_variants) else (hanzi_variants[0] if hanzi_variants else ""),
            }
            expanded_rows.append(new_row)

    if logger:
        logger.info(f"Expanded records: {expanded_count}")

    return pd.DataFrame(expanded_rows)

# -*- coding: utf-8 -*-
"""
展開分隔符欄位的共用函數
"""

import re
import pandas as pd

DEFAULT_SEPARATORS = ["/", ","]


def split_variants(value, separators=None):
    """
    將含分隔符的值拆成多個變體

    Args:
        value: 要拆分的值
        separators: 分隔符列表，預設為 ["/", ","]

    Returns:
        變體列表
    """
    if separators is None:
        separators = DEFAULT_SEPARATORS

    if pd.isna(value) or str(value).strip() == "":
        return []

    pattern = "|".join(re.escape(sep) for sep in separators)
    variants = [v.strip() for v in re.split(pattern, str(value)) if v.strip()]
    return variants


def expand_dataframe(df, separators=None, logger=None):
    """
    展開 tl 和 hanzi 欄位，並做配對

    規則：
    - 若兩欄位都有多個值，按索引配對
    - 若只有一方有多個值，另一方複製以配對

    Args:
        df: DataFrame（需有 tl 欄位，hanzi 欄位可選）
        separators: 分隔符列表
        logger: Logger 實例（可選）

    Returns:
        展開後的 DataFrame
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

        # 若只有一個值，複製以配對
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

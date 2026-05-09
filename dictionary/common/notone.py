# -*- coding: utf-8 -*-
"""
移除聲調相關函數
"""

import re


def remove_tone(text: str) -> str:
    """
    移除數字聲調版本中的聲調數字和連字符

    Args:
        text: 數字聲調版本的羅馬字 (如 gua2 或 gua2ho2)

    Returns:
        無聲調版本，或空字串（輸入為空時）

    Cross-system invariant — the `[\\d\\-]` character class strips BOTH
    digits AND hyphens, so multi-syllable `tl_num=tsu1a2` collapses to the
    fused `tl_notone=tsua`. The dictionary FST inherits that fused form
    via `dictionary/build/create_fst.py`, which is what lets toneless
    Roman input like `tsua` retrieve multi-syllable entries (e.g. `珠仔`).
    Pinned by `engine/lexicon/tests/fused_toneless_key.rs`. If you change
    this regex, update that test and the v3.5.8 roadmap §Phase 1b note.
    """
    if not text:
        return ""

    # 移除數字 (1-9) 和連字符
    return re.sub(r"[\d\-]", "", text)

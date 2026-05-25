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


# TPS tone-mark + separator class — mirrors `remove_tone` for the TPS
# family. Standalone tone marks from `ZHUYIN_TONES` (`\u{02c6}` ˆ tone-9,
# `\u{02c7}` ˇ tone-6, `\u{02ca}` ́ tone-5, `\u{02cb}` ̀ tone-2,
# `\u{02ea}` ˪ tone-3, `\u{02eb}` ˫ tone-7, `\u{0307}` combining dot
# for tone-8), the encode-safe `\u{02d9}` ˙ variant, hyphen, and
# whitespace are stripped. Bopomofo stop codas (`\u{31b4-7,b}`) stay
# because they're part of the syllable body, not the tone — tone-4 stop
# syllables carry no trailing mark and tone-8 combines a stop with a
# separate dot.
_TPS_TONE_AND_SEP_RE = re.compile(
    "[ˆˇˊˋ˙˪˫̇\\-\\s]"
)


def remove_tps_tone(text: str) -> str:
    """Strip TPS tone marks + separators → fused toneless TPS.

    Mirrors `remove_tone` (TL/POJ digit-tone stripping) for the TPS
    family. Input is a hyphenated or fused TPS string; output is the
    fused toneless Bopomofo body. Used by the `notone` stage to
    populate `tps_notone`, which becomes a `tps:<tps_notone>` family
    key in `dictionary.fst` for toneless TPS continuous input.
    """
    if not text:
        return ""
    return _TPS_TONE_AND_SEP_RE.sub("", text)

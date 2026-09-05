# -*- coding: utf-8 -*-
"""Tone-removal helpers for TL/POJ digit-tone stripping and TPS variants."""

import re


def remove_tone(text: str) -> str:
    """
    Strip tone digits and hyphens from a numeric-tone romanization.

    Args:
        text: Numeric-tone romanization (e.g. gua2 or gua2ho2).

    Returns:
        Toneless form, or an empty string if the input is empty.

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


# TPS dialect variant: bridge collapses TL `er` and `or` to ㄜ (U+311C).
# The `or_maps_to_er=false` form (iOS / Android user default) renders TL
# `or` as ㄛ (U+311B). Data audit confirms ㄜ in `tps_num` appears ONLY
# when `tl_num` contains an `er` or `or` token (1107 / 1107 rows; no
# other vowel maps to ㄜ in `taigi-converter/src/tables.js:68`), so a
# blanket ㄜ→ㄛ substitution produces the toggle-OFF variant without
# losing the er/or boundary information that pre-substitution at the TL
# level would lose (e.g. `tsherm` → `tshom` would collapse to ㄘㆱ rather
# than ㄘㄛㆬ).
_TPS_ER_GLYPH = "ㄜ"
_TPS_OR_GLYPH = "ㄛ"


def apply_or_dialect_variant(text: str) -> str:
    """Produce the `or_maps_to_er=false` TPS variant of a bridge output.

    Returns the original `text` with every ㄜ (U+311C) replaced by ㄛ
    (U+311B). Returns "" when no ㄜ is present so callers can skip
    emitting a redundant variant key. Used by `numtone` / `notone` /
    `abbrev` stages to populate `tps_*_var` columns that drive the
    er↔or dual-emit in `dictionary.fst` (PR C-3a).
    """
    if not text or _TPS_ER_GLYPH not in text:
        return ""
    return text.replace(_TPS_ER_GLYPH, _TPS_OR_GLYPH)

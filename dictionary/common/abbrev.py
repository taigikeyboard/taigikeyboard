# -*- coding: utf-8 -*-
"""Abbreviation helper functions."""

import unicodedata
import pandas as pd


def remove_diacritics(char: str) -> str:
    """Strip diacritics (combining marks) from a character via NFD decomposition."""
    decomposed = unicodedata.normalize("NFD", char)
    return "".join(c for c in decomposed if unicodedata.category(c) != "Mn")


def extract_abbrev(syllable_string: str) -> str:
    """First-letter abbreviation from a hyphen/space-separated syllable string
    (e.g. "guá-sī" -> "gs", "m̄ bat" -> "mb"), diacritics stripped and
    lowercased. Returns "" for a single syllable.
    """
    if not syllable_string or pd.isna(syllable_string):
        return ""

    import re
    syllables = re.split(r"[-\s]+", str(syllable_string))
    syllables = [s for s in syllables if s]

    if len(syllables) < 2:
        return ""

    return "".join(remove_diacritics(s[0]).lower() for s in syllables)


def extract_tps_abbrev(tl_syllable_string: str, tps_per_syllable: list[str]) -> str:
    """Per-syllable TPS initial-char concatenation; "" for <2 syllables.

    Mirrors `extract_abbrev` (first ASCII char per TL syllable, minus
    diacritics) for the TPS family. The first char of every TPS
    syllable is its leading initial / vowel Bopomofo glyph by
    construction (per `to_zhuyin` output shape — see
    `engine/phonetics/src/tps.rs:210`), so no diacritic stripping is
    needed; we take the first `char` directly.

    Args:
        tl_syllable_string: hyphenated TL (used only for syllable-count
            gate — at least 2 syllables to emit an abbrev).
        tps_per_syllable: list of TPS strings, one per TL syllable
            (caller already converted via `convert_tl_to_tps_strict`
            on each TL syllable). Same length as the TL split.

    Returns:
        Concatenated first-char-per-syllable TPS abbrev, or "" when the
        TL has fewer than 2 syllables OR any per-syllable TPS is empty.
    """
    if not tl_syllable_string or pd.isna(tl_syllable_string):
        return ""

    import re
    tl_syllables = re.split(r"[-\s]+", str(tl_syllable_string))
    tl_syllables = [s for s in tl_syllables if s]

    if len(tl_syllables) < 2:
        return ""
    if len(tps_per_syllable) != len(tl_syllables):
        return ""
    if any(not s for s in tps_per_syllable):
        return ""

    return "".join(s[0] for s in tps_per_syllable)

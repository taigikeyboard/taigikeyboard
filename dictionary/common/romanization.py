# -*- coding: utf-8 -*-
"""Romanization conversion helpers.

Uses taigi-converter (Node.js subprocess) for TL<->POJ conversion and tone processing.
"""

from .taigi_bridge import convert_tl_to_poj, to_tone_number, to_tone_number_ascii


def normalize_roman_spacing(text: str, preserve_spaces: bool = False) -> str:
    """Normalize whitespace inside a romanization string.

    Full-width space (U+3000) and ASCII spaces are converted to hyphens
    (or kept as spaces when `preserve_spaces=True`, for official dicts where
    word boundaries matter).

    NB: this is a subset of `common.cleanup.normalize_roman`, which also
    applies lowercase + NFC-via-taigi-converter. Callers that need the full
    normalisation should import the one from `common.cleanup`.
    """
    text = text.replace("\u3000", " " if preserve_spaces else "-")
    if not preserve_spaces:
        text = text.replace(" ", "-")
    return text


def _to_numeric_tone_word(roman: str, ascii_only: bool = False) -> str:
    """
    Convert a single word (syllables separated by "-") to its digit-tone form.

    Args:
        roman: A single word's romanization (e.g. "m̄-bat" or "phàu").
        ascii_only: Whether to use ASCII characters only (for POJ).

    Returns:
        The digit-tone form (no hyphens), e.g. "m7bat4" or "phau3".
    """
    converter = to_tone_number_ascii if ascii_only else to_tone_number
    result = converter(roman).lower()

    # Split into syllables and check whether each already carries a tone digit.
    syllables = result.split("-")
    processed = []
    for syllable in syllables:
        if syllable and not syllable[-1].isdigit():
            # Toneless syllable: infer the tone from the coda. A stop coda
            # (-p/-t/-k/-h) is tone 4 (checked); everything else is tone 1.
            if syllable[-1] in "ptkh":
                syllable = syllable + "4"
            else:
                syllable = syllable + "1"
        processed.append(syllable)

    return "".join(processed)


def to_numeric_tone(roman: str, ascii_only: bool = False) -> str:
    """
    Convert to the digit-tone form (hyphens removed).

    Supports space-separated multi-word input (e.g. "m̄ bat") — each word is
    processed separately then joined. A toneless syllable is marked tone 1.

    Args:
        roman: Romanization with tone marks; may contain spaces (word
            boundaries) or hyphens (syllable boundaries).
        ascii_only: Whether to use ASCII characters only.

    Returns:
        The digit-tone form (no hyphens or spaces), e.g. "m7bat4".
    """
    # Split by space (word boundary), process each word separately
    words = roman.split(" ")
    if len(words) <= 1:
        return _to_numeric_tone_word(roman, ascii_only)

    parts = []
    for word in words:
        if word:
            parts.append(_to_numeric_tone_word(word, ascii_only))
    return "".join(parts)


def add_roman_columns(hanzi: str, tl: str, preserve_spaces: bool = False) -> dict:
    """
    Generate the full set of romanization columns.

    Args:
        hanzi: The Hanji.
        tl: TL romanization.
        preserve_spaces: True keeps spaces (official dictionaries), False
            converts spaces to hyphens.

    Returns:
        A dict with all romanization columns.

    Raises:
        Exception: If conversion fails.
    """
    poj = convert_tl_to_poj(tl)

    tl = normalize_roman_spacing(tl, preserve_spaces=preserve_spaces)
    poj = normalize_roman_spacing(poj, preserve_spaces=preserve_spaces)

    tl_num = to_numeric_tone(tl)
    poj_num = to_numeric_tone(poj, ascii_only=True)

    return {
        "hanzi": hanzi,
        "tl": tl.lower(),
        "poj": poj.lower(),
        "tl_num": tl_num,
        "poj_num": poj_num,
    }

# -*- coding: utf-8 -*-
"""Shared dictionary data-cleaning helpers."""

import re
import unicodedata

import pandas as pd

from .cjk import is_cjk, is_kana
from .taigi_bridge import is_valid_romanization, normalize_taibun

BRACKET_PATTERN = re.compile(r"[（(〈《「『【\[].*?[）)〉》」』】\]]")
PROVERB_PUNCTUATION = "，。！；？、"
DEFAULT_MAX_SYLLABLES = 4

# ctx side-channel key for the `(hanzi, tl)` keys of `select.hanlo_exempt_sheets`,
# stashed at `select` (sheet identity is gone after `merge`), read at `cleanup`.
HANLO_EXEMPT_META_KEY = "hanlo_exempt"

BRACKET_COLUMNS = ["hanzi", "tl", "poj", "tl_num", "poj_num", "tl_notone", "poj_notone", "tl_abbrev", "poj_abbrev"]


def count_syllables(text):
    """Count syllables, splitting on hyphens and whitespace."""
    if pd.isna(text) or not str(text).strip():
        return 0
    syllables = re.split(r"[\s\-]+", str(text).strip())
    return len([s for s in syllables if s])


# Hanzi separators excluded from the syllable-bearing character count
# (ASCII space, full-width space U+3000, hyphen).
HANZI_SEPARATORS = " 　-"


def hanzi_chars(text):
    """Characters counted as hanzi length, with separators removed."""
    if pd.isna(text):
        return []
    return [c for c in str(text).strip() if c not in HANZI_SEPARATORS]


def clean_brackets(text):
    if pd.isna(text):
        return ""
    text = str(text)
    text = BRACKET_PATTERN.sub("", text)
    return text.strip()


def remove_prefix_hyphens(text):
    if pd.isna(text):
        return ""
    text = str(text)
    if text.startswith("--"):
        return text[2:]
    return text


def remove_ellipsis(text):
    """Strip runs of dots: tsû...... -> tsû."""
    if pd.isna(text):
        return ""
    text = str(text)
    text = re.sub(r"\.{2,}", "", text)
    return text.strip()


def normalize_roman(text, preserve_spaces=False):
    """NFC-normalize and lowercase a romanization. `preserve_spaces` keeps spaces as word
    boundaries (official dictionary); otherwise they become hyphens."""
    if pd.isna(text):
        return text
    text = normalize_taibun(str(text))
    text = text.replace("\u3000", " " if preserve_spaces else "-")
    if not preserve_spaces:
        text = text.replace(" ", "-")
    return text.lower()


def is_proverb(hanzi):
    """A proverb is a hanzi entry carrying full-width punctuation."""
    if pd.isna(hanzi):
        return False
    return any(p in str(hanzi) for p in PROVERB_PUNCTUATION)


def is_valid_tl(text):
    """Validate TL through taigi-converter's parseSyllable."""
    if pd.isna(text) or str(text).strip() == "":
        return False
    return is_valid_romanization(str(text).strip())


def is_hanlo_matched(hanzi, tl):
    """Hanzi length (separators excluded) must equal the TL syllable count."""
    if pd.isna(hanzi) or pd.isna(tl):
        return True
    hanzi_str = str(hanzi).strip()
    tl_str = str(tl).strip()
    if hanzi_str == "" or tl_str == "":
        return True
    hanzi_count = len(hanzi_chars(hanzi_str))
    tl_count = count_syllables(tl_str)
    return hanzi_count == tl_count


def is_valid_hanzi(text):
    """Reject hanzi carrying scraped-source noise: '.', consecutive spaces, '?', a tab,
    braces, or an unbalanced bracket pair."""
    if pd.isna(text):
        return True  # empty values are handled by the empty-tl step
    text = str(text)
    if '.' in text:
        return False
    if '  ' in text:
        return False
    if '?' in text:
        return False
    if '\t' in text:
        return False
    if '{' in text or '}' in text:
        return False
    for open_b, close_b in [('(', ')'), ('[', ']'), ('（', '）'), ('【', '】')]:
        has_open = open_b in text
        has_close = close_b in text
        if has_open != has_close:
            return False
    return True


def contains_roman_in_hanzi(text):
    """True if hanzi carries romanization — a-zA-Z or a Taiwanese tone-marked letter.
    Hyphens and empty values are ignored."""
    if pd.isna(text):
        return False
    text = str(text).strip()
    if text == "":
        return False

    text_without_hyphen = text.replace('-', '')

    if re.search(r'[a-zA-Z]', text_without_hyphen):
        return True

    roman_tone_pattern = r'[āáǎàaⁿēéěèeⁿīíǐìiⁿōóǒòoⁿūúǔùuⁿm̄ḿm̌m̀n̄ńňǹṁṅⁿ]'
    if re.search(roman_tone_pattern, text_without_hyphen):
        return True

    return False


def is_roman_only_hanzi(text):
    """True for a word written only in romanization: a Latin letter, no ideograph, no kana.

    The engine's `phonetics::api::is_roman_only_spelling` is this rule for the custom
    dictionary; both answer `tests/fixtures/roman_only_spelling_cases.tsv`, so change them together.
    """
    if pd.isna(text):
        return False
    # NFD: `ê` is a Latin letter `contains_roman_in_hanzi`'s tone list does not spell.
    letters = unicodedata.normalize("NFD", str(text))
    return contains_roman_in_hanzi(letters) and not any(is_cjk(c) or is_kana(c) for c in letters)


def blank_roman_only_hanzi(text):
    """A roman-only word has no hanzi: its spelling lives in `tl`, so POJ / TPS can render it."""
    return "" if is_roman_only_hanzi(text) else text


def is_kana_only_hanzi(text):
    """True for a word written only in kana, whose length counts morae, not syllables."""
    chars = hanzi_chars(text)
    return bool(chars) and all(is_kana(c) for c in chars)


def spelled_syllable_count(text):
    """Syllables a Han-Lo spelling writes: one per ideograph or kana, one per run of other
    letters between separators (`白phau-phau` = 3)."""
    count = 0
    in_run = False
    for char in str(text).strip():
        if char in HANZI_SEPARATORS:
            in_run = False
        elif is_cjk(char) or is_kana(char):
            count += 1
            in_run = False
        elif not in_run:
            count += 1
            in_run = True
    return count


def is_written_word_matched(hanzi, tl):
    """Hanlo check for sources keeping non-hanzi words: kana-only passes, a Han-Lo spelling
    counts syllables instead of characters."""
    return is_kana_only_hanzi(hanzi) or spelled_syllable_count(hanzi) == count_syllables(tl)


def cleanup_dataframe(df, logger=None, max_syllables=DEFAULT_MAX_SYLLABLES, check_roman_in_hanzi=False, preserve_spaces=False,
                      hanlo_exempt=frozenset(), keep_non_hanzi_words=False):
    """Run the full cleaning pipeline.

    Args:
        df: DataFrame with `hanzi` and `tl` columns.
        logger: optional logger for per-step counts.
        max_syllables: entries with more syllables than this are dropped.
        check_roman_in_hanzi: blank out hanzi containing romanization, keeping the row.
        preserve_spaces: keep spaces as word boundaries (official dictionary).
        hanlo_exempt: `(hanzi, tl)` keys (post-cleanup form) kept despite a hanlo mismatch.
        keep_non_hanzi_words: keep words not spelled in hanzi alone — roman-only ones lose
            the hanzi, kana-only ones skip the hanlo check, Han-Lo ones are checked by
            syllable and keep their spelling.

    Returns:
        (cleaned_df, dropped_df) — dropped rows carry a `drop_reason` column.
    """
    total_count = len(df)
    dropped_parts = []

    def collect_dropped(mask, reason):
        """Collect rows that will be dropped, with a reason tag."""
        dropped = df.loc[mask].copy()
        if len(dropped) > 0:
            dropped["drop_reason"] = reason
            dropped_parts.append(dropped)

    if logger:
        logger.info(f"Loaded {total_count} records")

    for col in BRACKET_COLUMNS:
        if col in df.columns:
            df[col] = df[col].apply(clean_brackets)

    if "tl" in df.columns:
        df["tl"] = df["tl"].apply(remove_prefix_hyphens)

    for col in ["tl", "hanzi"]:
        if col in df.columns:
            df[col] = df[col].apply(remove_ellipsis)

    proverb_mask = df["hanzi"].apply(is_proverb)
    proverb_count = proverb_mask.sum()
    if proverb_count > 0 and logger:
        logger.info(f"  Removed proverbs: {proverb_count}")
    collect_dropped(proverb_mask, "proverb")
    df = df[~proverb_mask]

    valid_tl_mask = df["tl"].apply(is_valid_tl)
    invalid_tl_count = (~valid_tl_mask).sum()
    if invalid_tl_count > 0 and logger:
        logger.info(f"  Removed invalid TL (non-roman chars): {invalid_tl_count}")
    collect_dropped(~valid_tl_mask, "invalid_tl")
    df = df[valid_tl_mask]

    valid_hanzi_mask = df["hanzi"].apply(is_valid_hanzi)
    invalid_hanzi_count = (~valid_hanzi_mask).sum()
    if invalid_hanzi_count > 0 and logger:
        logger.info(f"  Removed invalid hanzi (dot or consecutive spaces): {invalid_hanzi_count}")
    collect_dropped(~valid_hanzi_mask, "invalid_hanzi")
    df = df[valid_hanzi_mask]

    if check_roman_in_hanzi:
        roman_mask = df["hanzi"].apply(contains_roman_in_hanzi)
        roman_in_hanzi_count = roman_mask.sum()
        if roman_in_hanzi_count > 0:
            df.loc[roman_mask, "hanzi"] = ""
            if logger:
                logger.info(f"  Cleared hanzi containing roman letters: {roman_in_hanzi_count}")

    if keep_non_hanzi_words:
        blanked = df["hanzi"].apply(blank_roman_only_hanzi)
        if logger:
            logger.info(f"  Cleared roman-only hanzi: {(blanked != df['hanzi']).sum()}")
        df = df.assign(hanzi=blanked)

    # Only `tl` must be non-empty; hanzi-less entries are legitimate roman-only words.
    empty_tl_mask = df["tl"].str.strip() == ""
    collect_dropped(empty_tl_mask, "empty_tl")
    df = df[~empty_tl_mask]

    before_long = len(df)
    long_mask = df["tl"].apply(count_syllables) > max_syllables
    collect_dropped(long_mask, f"too_long(>{max_syllables})")
    df = df[~long_mask]
    long_removed = before_long - len(df)
    if long_removed > 0 and logger:
        logger.info(f"  Removed long entries ({max_syllables + 1}+ syllables): {long_removed}")

    df["tl"] = df["tl"].apply(lambda x: normalize_roman(x, preserve_spaces=preserve_spaces))

    before_dedup = len(df)
    dup_mask = df.duplicated(subset=["hanzi", "tl"], keep="first")
    collect_dropped(dup_mask, "duplicate")
    df = df[~dup_mask]
    after_dedup = len(df)
    removed = before_dedup - after_dedup
    if removed > 0 and logger:
        logger.info(f"  Removed duplicates (hanzi+tl): {removed}")

    hanlo_matched_mask = pd.Series(
        [is_hanlo_matched(h, t) or (h, t) in hanlo_exempt for h, t in zip(df["hanzi"], df["tl"])],
        index=df.index,
        dtype=bool,
    )
    if keep_non_hanzi_words:
        hanlo_matched_mask |= pd.Series(
            [is_written_word_matched(h, t) for h, t in zip(df["hanzi"], df["tl"])],
            index=df.index,
            dtype=bool,
        )
    hanlo_mismatch_count = (~hanlo_matched_mask).sum()
    if hanlo_mismatch_count > 0 and logger:
        logger.info(f"  Removed hanlo mismatch (KeSi TuiBeTse): {hanlo_mismatch_count}")
    collect_dropped(~hanlo_matched_mask, "hanlo_mismatch")
    df = df[hanlo_matched_mask]

    if logger:
        logger.info(f"  Final: {len(df)} records")

    dropped_df = pd.concat(dropped_parts, ignore_index=True) if dropped_parts else pd.DataFrame()
    return df, dropped_df

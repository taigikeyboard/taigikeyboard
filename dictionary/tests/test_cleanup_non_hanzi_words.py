"""Tests for `cleanup.keep_non_hanzi_words` — official words with no hanzi spelling survive cleanup.

Roman-only words lose the hanzi and keep the row; kana-only words keep the kana past the
hanlo check; Han-Lo spellings are checked by syllable and keep their spelling.
`cleanup_dataframe` validates TL via the taigi-converter bridge (started lazily).
"""

from __future__ import annotations

import pandas as pd

from common.cleanup import cleanup_dataframe
from common.kautian_provenance import apply_provenance, build_membership_set


def _rows(df: pd.DataFrame) -> set[tuple[str, str]]:
    return set(zip(df["hanzi"], df["tl"]))


def _cleanup(hanzi: list[str], tl: list[str], keep: bool = True):
    df = pd.DataFrame({"hanzi": hanzi, "tl": tl})
    return cleanup_dataframe(df, preserve_spaces=True, keep_non_hanzi_words=keep)


def test_words_dropped_without_option():
    # trace: a-lú-mih = 6 letters vs 3 syllables; にんじん = 4 kana vs 2 syllables.
    cleaned, dropped = _cleanup(["a-lú-mih", "にんじん"], ["a-lú-mih", "jīn-jîn"], keep=False)
    assert cleaned.empty
    assert set(dropped["drop_reason"]) == {"hanlo_mismatch"}


def test_roman_only_word_keeps_row_without_hanzi():
    cleaned, dropped = _cleanup(["a-lú-mih", "bá-suh bá-tah"], ["a-lú-mih", "bá-suh bá-tah"])
    assert _rows(cleaned) == {("", "a-lú-mih"), ("", "bá-suh bá-tah")}
    assert dropped.empty


def test_single_precomposed_vowel_is_roman_only():
    # trace: ê has no ASCII letter until NFD splits it into e + U+0302.
    cleaned, dropped = _cleanup(["ê"], ["ê"])
    assert _rows(cleaned) == {("", "ê")}
    assert dropped.empty


def test_two_roman_spellings_of_one_reading_collapse():
    cleaned, dropped = _cleanup(["a̋i-sat-tsuh", "a̋i-sat-tsirh"], ["a̋i-sat-tsirh", "a̋i-sat-tsirh"])
    assert _rows(cleaned) == {("", "a̋i-sat-tsirh")}
    assert list(dropped["drop_reason"]) == ["duplicate"]


def test_kana_only_word_keeps_kana():
    cleaned, dropped = _cleanup(["にんじん", "ビール"], ["jīn-jîn", "bì-lù"])
    assert _rows(cleaned) == {("にんじん", "jīn-jîn"), ("ビール", "bì-lù")}
    assert dropped.empty


def test_han_lo_spellings_kept_as_written():
    # trace: 白 + phau + phau = 3 = pe̍h-phau-phau; gá + suh + 窯 = 3; a + 小 + 調 = 3.
    cleaned, dropped = _cleanup(
        ["白phau-phau", "gá-suh窯", "a小調"], ["pe̍h-phau-phau", "gá-suh-iô", "a-sió-tiāu"]
    )
    assert _rows(cleaned) == {
        ("白phau-phau", "pe̍h-phau-phau"),
        ("gá-suh窯", "gá-suh-iô"),
        ("a小調", "a-sió-tiāu"),
    }
    assert dropped.empty


def test_han_lo_spelling_with_wrong_syllable_count_dropped():
    # trace: jazz + 樂 = 2 vs jiah-sū-ga̍k = 3.
    cleaned, dropped = _cleanup(["jazz樂"], ["jiah-sū-ga̍k"])
    assert cleaned.empty
    assert list(dropped["drop_reason"]) == ["hanlo_mismatch"]


def test_hanzi_words_unchanged():
    cleaned, dropped = _cleanup(["台語", "查某囝"], ["tâi-gí", "tsa̋u-kiánn"])
    assert _rows(cleaned) == {("台語", "tâi-gí")}
    assert list(dropped["drop_reason"]) == ["hanlo_mismatch"]


def test_provenance_joins_blanked_row():
    sheets = {"又唸作.csv": pd.DataFrame({"漢字": ["a-lú-mih"], "羅馬字": ["a-lú-mih"]})}
    alt_reading_set = build_membership_set(sheets, ["又唸作.csv"], preserve_spaces=True)
    cleaned, _ = _cleanup(["a-lú-mih"], ["a-lú-mih"])
    joined = apply_provenance(
        cleaned, accent_map={}, name_set=frozenset(), main_set=frozenset(), alt_reading_set=alt_reading_set
    )
    assert joined["kautian_alt_reading"].tolist() == [True]

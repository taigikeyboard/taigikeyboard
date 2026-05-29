"""Tests for kautian req2 word-level accent generation (Phase 5).

Covers the same-morpheme gate (DD9), comma-multi skip + mask OR (DD1/DD4),
separator preservation, frequency inheritance (DD4b), existing-(hanzi,tl)
collision merge, POJ-convertibility skip, and the `BridgeDeadError` re-raise.

The core (`generate_variants_for_word`, `apply_word_accent_generation`) is
bridge-free; `apply_*` takes an injected `poj_convertible` so these stay
offline. Accent bit order (matches config.yaml select.dialect_columns):
0 鹿港 / 1 三峽 / 2 臺北 / 3 宜蘭 / 4 臺南 / 5 高雄 / 6 金門 / 7 馬公 / 8 新竹 / 9 臺中.
"""

from __future__ import annotations

import pandas as pd
import pytest

from common.cleanup import hanzi_chars
from common.kautian_accent_wordgen import (
    _rejoin,
    _split_syllables,
    apply_word_accent_generation,
    build_char_accent_table,
    generate_variants_for_word,
)
from common.kautian_provenance import COL_ACCENT_MASK, COL_MAIN, COL_NAME

# 八: pueh in 泉腔 (bits 0,1,2,6,7,8 = 455); peh in 漳/混 (bits 3,4,5,9 = 568).
EIGHT_TABLE = {"八": {"pueh": 455, "peh": 568}}


def _always_convertible(_reading: str) -> bool:
    return True


def test_dd9_文讀_reading_generates_nothing():
    # 八卦's 八 is read /pat/ (文讀), NOT in 八's 語音差異 set {pueh, peh}
    # (白讀) → different morpheme → no accent variant (DD9).
    result = generate_variants_for_word("八卦", "pat-kuà", EIGHT_TABLE)
    assert result.aligned
    assert result.variants == {}


def test_single_variable_char_ors_matching_accents():
    # 八字 base "pueh-jī": 字 is fixed (∉ table); the 漳/混腔 accents read 八 as
    # "peh" → all four produce "peh-jī" sharing one row with OR'd bits = 568.
    # The 泉腔 accents read 八 as "pueh" == base → no change → no row.
    result = generate_variants_for_word("八字", "pueh-jī", EIGHT_TABLE)
    assert result.variants == {"peh-jī": 568}


def test_two_variable_chars_distinct_readings():
    # 重 reads tāng/tiōng split across accents; 山 reads suann/sann split.
    # bit0 → tāng-suann, bit1 → tiōng-sann: two distinct readings, distinct masks.
    table = {
        "重": {"tāng": 0b01, "tiōng": 0b10},
        "山": {"suann": 0b01, "sann": 0b10},
    }
    result = generate_variants_for_word("重山", "tāng-suann", table, accent_count=2)
    # bit0 reproduces the base (no change → skipped); bit1 changes both syllables.
    assert result.variants == {"tiōng-sann": 0b10}


def test_comma_multi_skips_that_accent_only():
    # 新竹 (bit8) cell was comma-multi "lír,lí" → both readings carry bit8.
    # Generating bit8 is ambiguous → skipped (counted), other accents proceed.
    table = {"女": {"lír": (1 << 8) | (1 << 0), "lí": (1 << 8) | (1 << 3)}}
    # base "lír-X": bit0 → lír (no change); bit3 → lí (change → "lí-X"); bit8 → ambiguous.
    result = generate_variants_for_word("女X", "lír-tsú", table, accent_count=10)
    assert result.ambiguous_accents == 1  # bit8 only
    assert result.variants == {"lí-tsú": (1 << 3)}


def test_separator_preserved_space_and_hyphen():
    table = {"重": {"tāng": 0b01, "tiōng": 0b10}}
    # base "tāng X" (space word-boundary) — only 重 variable, X fixed.
    result = generate_variants_for_word("重X", "tāng a", table, accent_count=2)
    assert result.variants == {"tiōng a": 0b10}


def test_misaligned_hanlo_not_generated():
    # 漢字數 (1) != 音節數 (2) → not aligned, skipped.
    result = generate_variants_for_word("八", "pueh-jī", EIGHT_TABLE)
    assert not result.aligned
    assert result.variants == {}


@pytest.mark.parametrize(
    "tl,syllables,separators",
    [
        ("peh-gue̍h", ["peh", "gue̍h"], ["-"]),
        ("tâi-uân tâi-gí", ["tâi", "uân", "tâi", "gí"], ["-", " ", "-"]),
        ("phàu", ["phàu"], []),
        ("  ", [], []),
        ("a--b", ["a", "b"], ["--"]),
    ],
)
def test_split_syllables_and_rejoin_roundtrip(tl, syllables, separators):
    out_syl, out_sep = _split_syllables(tl)
    assert out_syl == syllables
    assert out_sep == separators
    if out_syl:
        assert _rejoin(out_syl, out_sep) == tl.strip()


def test_hanzi_chars_excludes_separators():
    # Spaces (ASCII + full-width) and hyphens are not syllable-bearing.
    assert hanzi_chars("臺 灣-字　á") == ["臺", "灣", "字", "á"]
    assert hanzi_chars("八") == ["八"]
    assert hanzi_chars("") == []


def test_build_char_accent_table_groups_by_hanzi():
    accent_map = {("八", "pueh"): 455, ("八", "peh"): 568, ("刀", "to"): 1}
    table = build_char_accent_table(accent_map)
    assert table["八"] == {"pueh": 455, "peh": 568}
    assert table["刀"] == {"to": 1}


def _base_df() -> pd.DataFrame:
    return pd.DataFrame(
        {
            "hanzi": ["八字"],
            "tl": ["pueh-jī"],
            "frequency": [777],
            COL_MAIN: [True],
            COL_ACCENT_MASK: [0],
            COL_NAME: [False],
        }
    )


def test_apply_emits_row_with_inherited_frequency_and_flags():
    df, report = apply_word_accent_generation(
        _base_df(), {("八", "pueh"): 455, ("八", "peh"): 568}, _always_convertible
    )
    gen = df[df["tl"] == "peh-jī"]
    assert len(gen) == 1
    assert int(gen.iloc[0]["frequency"]) == 777  # DD4b inherit base frequency
    assert not bool(gen.iloc[0][COL_MAIN])
    assert not bool(gen.iloc[0][COL_NAME])
    assert int(gen.iloc[0][COL_ACCENT_MASK]) == 568
    assert report.rows_emitted == 1
    assert report.words_generating == 1


def test_apply_collision_ors_into_existing_row_no_duplicate():
    # An existing kautian row already carries (八字, peh-jī) as a headword
    # (main=True). The generated accent variant must OR its mask into that row,
    # NOT append a duplicate (which `variants` would later replicate). DD6:
    # main stays True.
    df = pd.DataFrame(
        {
            "hanzi": ["八字", "八字"],
            "tl": ["pueh-jī", "peh-jī"],
            "frequency": [777, 500],
            COL_MAIN: [True, True],
            COL_ACCENT_MASK: [0, 0],
            COL_NAME: [False, False],
        }
    )
    out, report = apply_word_accent_generation(
        df, {("八", "pueh"): 455, ("八", "peh"): 568}, _always_convertible
    )
    peh = out[out["tl"] == "peh-jī"]
    assert len(peh) == 1  # no duplicate appended
    assert bool(peh.iloc[0][COL_MAIN])  # existing main preserved
    assert int(peh.iloc[0][COL_ACCENT_MASK]) == 568  # mask OR'd in
    assert int(peh.iloc[0]["frequency"]) == 500  # existing freq untouched
    # Both main rows generate each other's reading (pueh-jī→peh-jī and
    # peh-jī→pueh-jī), so both collide into the existing rows — symmetric.
    pueh = out[out["tl"] == "pueh-jī"]
    assert len(pueh) == 1
    assert int(pueh.iloc[0][COL_ACCENT_MASK]) == 455
    assert report.collisions_merged == 2
    assert report.rows_emitted == 0


def test_apply_skips_nonconvertible_reading():
    def reject_peh(reading: str) -> bool:
        return reading != "peh-jī"

    df, report = apply_word_accent_generation(
        _base_df(), {("八", "pueh"): 455, ("八", "peh"): 568}, reject_peh
    )
    assert "peh-jī" not in set(df["tl"])
    assert report.skipped_nonconvertible == 1
    assert report.rows_emitted == 0


def test_apply_propagates_bridge_dead_error():
    from common.taigi_bridge import BridgeDeadError

    def dead(_reading: str) -> bool:
        raise BridgeDeadError("subprocess died")

    with pytest.raises(BridgeDeadError):
        apply_word_accent_generation(
            _base_df(), {("八", "pueh"): 455, ("八", "peh"): 568}, dead
        )


def test_stage_poj_convertible_reraises_bridge_dead(monkeypatch):
    # The stage's validator must re-raise BridgeDeadError (fail loud) but
    # return False on a per-row RuntimeError (skip the reading).
    from common.stages import kautian_accent_wordgen as stage
    from common.taigi_bridge import BridgeDeadError

    def boom(_reading: str) -> str:
        raise BridgeDeadError("dead")

    monkeypatch.setattr(stage, "convert_tl_to_poj_strict", boom)
    with pytest.raises(BridgeDeadError):
        stage._poj_convertible("tshut")

    def per_row(_reading: str) -> str:
        raise RuntimeError("JS conversion error")

    monkeypatch.setattr(stage, "convert_tl_to_poj_strict", per_row)
    assert stage._poj_convertible("tshut") is False

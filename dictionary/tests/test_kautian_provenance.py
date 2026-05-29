"""Tests for kautian subcollection provenance (Phase 1).

Covers the membership-map builders (accent bitset / name set), the
`apply_provenance` join logic (esp. DD6: a headword that is also an accent or
name reading keeps `kautian_main=True`), and the cross-source OR aggregation
used by `build/merge_csv.py`.

`build_*` helpers normalize readings via the taigi-converter bridge (started
lazily on first call); `apply_provenance` and the agg helpers are bridge-free.
"""

from __future__ import annotations

import pandas as pd
import pytest

from build.merge_csv import _or_mask_agg
from common.cleanup import normalize_roman
from common.kautian_provenance import (
    COL_ACCENT_MASK,
    COL_MAIN,
    COL_NAME,
    apply_provenance,
    build_accent_mask_map,
    build_membership_set,
)

# Accent columns in bit order (matches config.yaml select.dialect_columns).
ACCENT_COLUMNS = [
    "鹿港偏泉腔",
    "三峽偏泉腔",
    "臺北偏泉腔",
    "宜蘭偏漳腔",
    "臺南混合腔",
    "高雄混合腔",
    "金門偏泉腔",
    "馬公偏泉腔",
    "新竹偏泉腔",
    "臺中偏漳腔",
]


def test_build_accent_mask_map_shared_reading_ors_bits():
    # 八: pueh in 鹿港/三峽/臺北/金門/馬公/新竹 (bits 0,1,2,6,7,8) = 455;
    #     peh in 宜蘭/臺南/高雄/臺中 (bits 3,4,5,9) = 568.
    df = pd.DataFrame(
        {
            "漢字": ["八"],
            "鹿港偏泉腔": ["pueh"],
            "三峽偏泉腔": ["pueh"],
            "臺北偏泉腔": ["pueh"],
            "宜蘭偏漳腔": ["peh"],
            "臺南混合腔": ["peh"],
            "高雄混合腔": ["peh"],
            "金門偏泉腔": ["pueh"],
            "馬公偏泉腔": ["pueh"],
            "新竹偏泉腔": ["pueh"],
            "臺中偏漳腔": ["peh"],
        }
    )
    accent_map = build_accent_mask_map(df, ACCENT_COLUMNS, preserve_spaces=True)
    assert accent_map[("八", normalize_roman("pueh"))] == 455
    assert accent_map[("八", normalize_roman("peh"))] == 568


def test_build_accent_mask_map_comma_multi_reading_splits():
    # 新竹 女 = "lír,lí" → both sub-readings carry that column's bit (bit 8).
    df = pd.DataFrame({"漢字": ["女"], **{c: [None] for c in ACCENT_COLUMNS}})
    df.loc[0, "新竹偏泉腔"] = "lír,lí"
    accent_map = build_accent_mask_map(df, ACCENT_COLUMNS, preserve_spaces=True)
    assert accent_map[("女", normalize_roman("lír"))] == (1 << 8)
    assert accent_map[("女", normalize_roman("lí"))] == (1 << 8)


def test_build_accent_mask_map_rejects_too_many_columns():
    df = pd.DataFrame({"漢字": ["八"]})
    with pytest.raises(ValueError, match="accent bits"):
        build_accent_mask_map(df, [f"c{i}" for i in range(11)], preserve_spaces=True)


def test_build_membership_set_lowercases_capitalized_surname():
    # 姓 readings are Capitalized ("Káu"); cleanup lowercases tl, so the map
    # key must normalize the same way to join.
    sheets = {
        "姓.csv": pd.DataFrame({"漢字": ["九"], "羅馬字": ["Káu"]}),
        "名.csv": pd.DataFrame({"漢字": ["九"], "羅馬字": ["káu"]}),
    }
    name_set = build_membership_set(sheets, ["名.csv", "姓.csv"], preserve_spaces=True)
    assert normalize_roman("Káu") == normalize_roman("káu")
    assert ("九", normalize_roman("Káu")) in name_set


def test_build_membership_set_strips_bracket_annotations():
    # 詞目 headword readings carry 【文】/【白】 annotations that cleanup strips;
    # the map key must strip them too, else annotated headwords lose main-ness.
    sheets = {
        "詞目.csv": pd.DataFrame(
            {"漢字": ["八", "八"], "羅馬字": ["【文】pat", "【白】peh/pueh"]}
        )
    }
    main_set = build_membership_set(sheets, ["詞目.csv"], preserve_spaces=True)
    assert ("八", "pat") in main_set
    assert ("八", "peh") in main_set
    assert ("八", "pueh") in main_set


def test_apply_provenance_bracketed_headword_overlapping_name_stays_main():
    # Regression (Codex BLOCK): 八/pat is a headword (詞目 【文】pat) AND a name
    # reading. With a cleanup-normalized main_set it must stay kautian_main=True
    # so it survives when the 姓名附錄 toggle is off (DD6).
    df = pd.DataFrame({"hanzi": ["八"], "tl": ["pat"]})
    out = apply_provenance(
        df, accent_map={}, name_set={("八", "pat")}, main_set={("八", "pat")}
    )
    assert out.loc[0, COL_MAIN]
    assert out.loc[0, COL_NAME]


def test_apply_provenance_main_independent_of_accent_dd6():
    accent_map = {("八", "pueh"): 455, ("八", "peh"): 568}
    name_set = {("九", "kiu")}
    main_set = {("八", "pueh"), ("食", "tsia̍h")}
    df = pd.DataFrame(
        {
            "hanzi": ["八", "八", "食", "九", "王"],
            "tl": ["pueh", "peh", "tsia̍h", "kiu", "ông"],
        }
    )
    out = apply_provenance(df, accent_map, name_set, main_set)

    # 八/pueh: headword AND accent → stays main (DD6).
    assert out.loc[0, COL_MAIN]
    assert out.loc[0, COL_ACCENT_MASK] == 455
    assert not out.loc[0, COL_NAME]

    # 八/peh: accent-only (not a headword) → main False.
    assert not out.loc[1, COL_MAIN]
    assert out.loc[1, COL_ACCENT_MASK] == 568

    # 食/tsia̍h: neither accent nor name → main defaults True (may have been
    # bracket-stripped; never matched against main_set).
    assert out.loc[2, COL_MAIN]
    assert out.loc[2, COL_ACCENT_MASK] == 0

    # 九/kiu: name appendix only → name True, not a headword.
    assert out.loc[3, COL_NAME]
    assert not out.loc[3, COL_MAIN]

    # 王/ông: no provenance entry → defaults to main headword.
    assert out.loc[4, COL_MAIN]
    assert out.loc[4, COL_ACCENT_MASK] == 0
    assert not out.loc[4, COL_NAME]


def test_or_mask_agg_unions_across_nan():
    assert _or_mask_agg(pd.Series([455, None, 8, None])) == (455 | 8)
    assert _or_mask_agg(pd.Series([None, None])) == 0


def test_cross_source_dedup_preserves_kautian_mask():
    # A kautian accent row + a same-(hanzi, tl) other-source row (NaN
    # provenance) must merge WITHOUT dropping the kautian mask/flags. Bool
    # flags use the built-in "any" (NaN-skipping) — same as SOURCE_COLUMNS.
    merged = pd.DataFrame(
        {
            "hanzi": ["八", "八"],
            "_tl_key": ["pueh", "pueh"],
            COL_ACCENT_MASK: [455, None],
            COL_MAIN: [True, None],
            COL_NAME: [False, None],
        }
    )
    agg = {COL_ACCENT_MASK: _or_mask_agg, COL_MAIN: "any", COL_NAME: "any"}
    result = merged.groupby(["hanzi", "_tl_key"], as_index=False, dropna=False).agg(agg)
    assert len(result) == 1
    assert result.loc[0, COL_ACCENT_MASK] == 455
    assert bool(result.loc[0, COL_MAIN]) is True
    assert bool(result.loc[0, COL_NAME]) is False

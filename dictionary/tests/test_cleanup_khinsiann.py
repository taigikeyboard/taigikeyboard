"""Tests for triple-hyphen cleanup, pure-Hanji hyphen stripping, and taihoa Han-Lo normalization."""

from __future__ import annotations

import csv
from pathlib import Path

import pandas as pd

from common.cleanup import (
    cleanup_dataframe,
    format_khinsiann_hanji,
    is_pure_hanji,
    normalize_hanzi_khinsiann,
    normalize_roman,
    strip_pure_hanji_khinsiann,
)

DICTIONARY_DIR = Path(__file__).resolve().parents[1]


def test_normalize_roman_collapses_triple_hyphens_without_breaking_double_spaces():
    assert normalize_roman("guá-hūn --lí") == "guá-hūn--lí"
    assert normalize_roman("lak-khì   --ah") == "lak-khì--ah"
    assert normalize_roman("guá-hūn   lí") == "guá-hūn--lí"
    assert normalize_roman("tshiànn -tsha̍t   kòo-kenn") == "tshiànn--tsha̍t--kòo-kenn"
    assert normalize_roman("tsí-sī  hû-hō") == "tsí-sī--hû-hō"
    assert normalize_roman("sia̍p---khì") == "sia̍p--khì"
    assert normalize_roman("káu --ge̍h  tiông-iông", preserve_spaces=True) == "káu--ge̍h tiông-iông"


def test_strip_pure_hanji_khinsiann_strips_all_hyphens_from_pure_hanji():
    assert strip_pure_hanji_khinsiann("--轉-去") == "轉去"
    assert strip_pure_hanji_khinsiann("記--起-來") == "記起來"
    assert strip_pure_hanji_khinsiann("有--的-無--的") == "有的無的"
    assert strip_pure_hanji_khinsiann("九--月重陽") == "九月重陽"
    assert strip_pure_hanji_khinsiann("一-六") == "一六"
    assert strip_pure_hanji_khinsiann("圍-布-城") == "圍布城"
    # Non-pure-Hanji (Han-Lo / roman-only) entries are preserved verbatim.
    assert strip_pure_hanji_khinsiann("大--ê") == "大--ê"
    assert strip_pure_hanji_khinsiann("邊--a") == "邊--a"
    assert strip_pure_hanji_khinsiann("tso̍h--日") == "tso̍h--日"
    assert strip_pure_hanji_khinsiann("--tńg-去") == "--tńg-去"
    assert strip_pure_hanji_khinsiann("ē-kì--tit") == "ē-kì--tit"


def test_format_khinsiann_hanji_cases():
    assert format_khinsiann_hanji("會記得", "ē-kì--tit") == "會記--得"
    assert format_khinsiann_hanji("轉去", "--tńg-khì") == "--轉-去"
    assert format_khinsiann_hanji("轉去", "tńg--khì") == "轉--去"
    assert format_khinsiann_hanji("轉去", "tńg-khì") == "轉去"
    assert format_khinsiann_hanji("記起來", "kì--khí-lâi") == "記--起-來"
    assert format_khinsiann_hanji("九月重陽", "káu--ge̍h tiông-iông") == "九--月重陽"
    assert format_khinsiann_hanji("做人矣", "tsuè--lâng--ah") == "做--人--矣"
    assert format_khinsiann_hanji("有的無的", "ū--ê-bô--ê") == "有--的-無--的"
    assert format_khinsiann_hanji("用sai的", "iōng sai--ê") == "用sai--的"
    assert format_khinsiann_hanji("tso̍h--日", "tso̍h--ji̍t") == "tso̍h--日"
    assert format_khinsiann_hanji("邊--a", "pinn--a") == "邊--a"


def test_normalize_hanzi_khinsiann_cleanup_cases():
    assert normalize_hanzi_khinsiann("寒---人", "kuânn--lâng") == "寒人"
    assert normalize_hanzi_khinsiann("攝----去", "sia̍p--khì") == "攝去"
    assert normalize_hanzi_khinsiann("唯--哉", "ui̍h-tsài") == "唯哉"
    assert normalize_hanzi_khinsiann("爺--哥舍--哥", "iâ-ko-sià-ko") == "爺哥舍哥"
    assert normalize_hanzi_khinsiann("下--落去", "hē--lo̍h-khì") == "下落去"
    assert normalize_hanzi_khinsiann("選--出來", "suán--tshut--lâi") == "選出來"
    assert normalize_hanzi_khinsiann("孽--人撇--人", "gia̍t--lâng-phiat--lâng") == "孽人撇人"
    assert normalize_hanzi_khinsiann("一-六", "it-lio̍k") == "一六"
    assert normalize_hanzi_khinsiann("轉去", "tńg--khì") == "轉去"
    # Han-Lo <Hanzi>ê where TL has --ê
    assert normalize_hanzi_khinsiann("大ê", "tuā--ê") == "大--ê"
    assert normalize_hanzi_khinsiann("用sai的", "iōng sai--ê") == "用sai的"
    assert is_pure_hanji("大--ê") is False


def test_cleanup_dataframe_strips_pure_hanji_hyphens_and_drops_corrupt_ids_when_enabled():
    df = pd.DataFrame({
        "hanzi": ["我恨你", "攝----去", "唯--哉", "下--落去", "一-六", "大-片", "大ê", "用sai的"],
        "tl": ["guá-hūn --lí", "sia̍p---khì", "ui̍h-tsài", "hē--lo̍h-khì", "it-lio̍k", "tuā-pue", "tuā--ê", "iōng sai--ê"],
    })
    cleaned, dropped = cleanup_dataframe(
        df,
        preserve_spaces=False,
        keep_non_hanzi_words=True,
        drop_hyphen_in_pure_hanzi=True,
    )
    assert list(dropped["hanzi"]) == ["一-六", "大-片"]
    assert set(dropped["drop_reason"]) == {"corrupt_ids_hyphen"}

    cleaned2, dropped2 = cleanup_dataframe(df.iloc[:5].copy(), preserve_spaces=False)
    assert dropped2.empty
    assert list(zip(cleaned2["hanzi"], cleaned2["tl"])) == [
        ("我恨你", "guá-hūn--lí"),
        ("攝去", "sia̍p--khì"),
        ("唯哉", "ui̍h-tsài"),
        ("下落去", "hē--lo̍h-khì"),
        ("一六", "it-lio̍k"),
    ]
    assert list(zip(cleaned["hanzi"], cleaned["tl"]))[-2:] == [
        ("大--ê", "tuā--ê"),
        ("用sai的", "iōng-sai--ê"),
    ]


def test_non_lkk_source_csvs_have_no_hyphens_in_pure_hanji():
    offending = []
    for csv_path in sorted((DICTIONARY_DIR / "sources").glob("*/*/data/*.csv")):
        with csv_path.open(encoding="utf-8", newline="") as f:
            for row in csv.DictReader(f):
                hanzi = row.get("hanzi", "")
                if "-" in hanzi and is_pure_hanji(hanzi):
                    offending.append((str(csv_path.relative_to(DICTIONARY_DIR)), hanzi, row.get("tl", "")))
    assert not offending, f"Pure-Hanji entries with hyphens outside lkk.csv: {offending[:10]}"

"""Validation and exclusion rules for the manually curated LKK supplement."""

import csv
import unicodedata
from pathlib import Path

import pytest

from common.cleanup import normalize_roman
from common.cjk import is_cjk
from common.taigi_bridge import (
    convert_poj_to_tl_strict,
    convert_tl_to_poj_strict,
    is_valid_romanization,
)

DICTIONARY_DIR = Path(__file__).resolve().parents[1]
CURATED_PATH = DICTIONARY_DIR / "supplementary/lkk/data/lkk.csv"
MOE_PATH = DICTIONARY_DIR / "sources/official/kautian/data/kautian.csv"


def read_pairs(path):
    with path.open(encoding="utf-8", newline="") as source:
        reader = csv.DictReader(source)
        assert reader.fieldnames == ["hanzi", "tl"]
        return list(reader)


def key(row):
    return unicodedata.normalize("NFC", row["hanzi"].strip()), normalize_roman(row["tl"])


@pytest.fixture(scope="module")
def curated_rows():
    return read_pairs(CURATED_PATH)


def test_curated_pairs_have_explicit_canonical_readings(curated_rows):
    assert curated_rows
    for row in curated_rows:
        assert row["hanzi"] and row["hanzi"] == row["hanzi"].strip(), row
        assert row["tl"] and row["tl"] == normalize_roman(row["tl"]), row
        assert is_valid_romanization(row["tl"]), row
        canonical = convert_poj_to_tl_strict(convert_tl_to_poj_strict(row["tl"]))
        assert row["tl"] == unicodedata.normalize("NFC", canonical), row


def test_curated_pairs_are_unique_by_spelling_and_reading(curated_rows):
    pairs = [key(row) for row in curated_rows]
    assert len(pairs) == len(set(pairs)), "Duplicate LKK spelling/reading pair"


def test_curated_spellings_mix_hanji_and_romanization(curated_rows):
    for row in curated_rows:
        spelling = row["hanzi"]
        assert any(is_cjk(char) for char in spelling), row
        assert any(
            char.isalpha() and "LATIN" in unicodedata.name(char, "")
            for char in spelling
        ), row


def test_curated_pairs_exclude_exact_moe_recommendations(curated_rows):
    # The generated MoE source already carries canonical TL. Its generated
    # variant spellings do not count as recommendations for this exclusion.
    with MOE_PATH.open(encoding="utf-8", newline="") as source:
        moe_pairs = {
            key(row) for row in csv.DictReader(source)
            if row["is_variant"].lower() == "false"
        }
    overlap = {key(row) for row in curated_rows} & moe_pairs
    assert not overlap, f"LKK pairs duplicate MoE recommendations: {sorted(overlap)}"


def test_curated_pairs_exclude_leading_neutral_tone_markers(curated_rows):
    excluded = [
        key(row) for row in curated_rows
        if row["hanzi"].startswith("--") or row["tl"].startswith("--")
    ]
    assert not excluded, f"Standalone neutral-tone suffixes: {excluded}"

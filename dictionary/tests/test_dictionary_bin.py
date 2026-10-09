"""Tests for the dictionary.bin record: kautian_subtag (v3) and walker_cost (v4).

Covers the bit-packing (`source_bits.encode_kautian_subtag`), the loader
coercion from CSV cells (`dictionary_records._kautian_subtag` /
`_as_bool`), and the v4 record byte layout (`create_dictionary_bin`).
"""

from __future__ import annotations

import struct

import pandas as pd

from build.create_dictionary_bin import VERSION, encode_record
from build.dictionary_records import DictionaryRecord, _as_bool, _kautian_subtag
from common.kautian_provenance import COL_ACCENT_MASK, COL_ALT_READING, COL_MAIN, COL_NAME
from common.source_bits import (
    DICT_BIN_COLUMNS,
    KAUTIAN_SUBTAG_ALT_READING_BIT,
    KAUTIAN_SUBTAG_NAME_BIT,
    encode_kautian_subtag,
)


# --- encode_kautian_subtag (bit packing) --------------------------------


def test_encode_subtag_main_only():
    assert encode_kautian_subtag(has_main=True, accent_mask=0, has_name=False, has_alt_reading=False) == 0b0000_0000_0001


def test_encode_subtag_name_only():
    expected = 1 << KAUTIAN_SUBTAG_NAME_BIT  # bit 11
    assert encode_kautian_subtag(has_main=False, accent_mask=0, has_name=True, has_alt_reading=False) == expected
    assert expected == 0b1000_0000_0000


def test_encode_subtag_alt_reading_only():
    expected = 1 << KAUTIAN_SUBTAG_ALT_READING_BIT  # bit 12
    assert (
        encode_kautian_subtag(has_main=False, accent_mask=0, has_name=False, has_alt_reading=True)
        == expected
    )
    assert expected == 0b1_0000_0000_0000


def test_encode_subtag_accent_mask_shifted_to_bits_1_through_10():
    # accent bit 0 (Lukang) → subtag bit 1; accent bit 9 (Taichung) → subtag bit 10.
    assert encode_kautian_subtag(has_main=False, accent_mask=0b01, has_name=False, has_alt_reading=False) == 0b10
    assert encode_kautian_subtag(has_main=False, accent_mask=0b10_0000_0000, has_name=False, has_alt_reading=False) == (
        1 << 10
    )


def test_encode_subtag_combined_main_accent_name():
    # main + accent bit 0 + name → bit0 | bit1 | bit11.
    got = encode_kautian_subtag(has_main=True, accent_mask=0b01, has_name=True, has_alt_reading=False)
    assert got == (1 << 0) | (1 << 1) | (1 << 11)


def test_encode_subtag_masks_accent_overflow_into_reserved_bits():
    # An accent_mask wider than 10 bits must not spill into the name /
    # alt-reading bits (11, 12) or reserved bits 13-15.
    got = encode_kautian_subtag(has_main=False, accent_mask=0xFFFF, has_name=False, has_alt_reading=False)
    assert got & 0xF800 == 0, "bits above the accent region must stay 0"
    assert got == 0b0111_1111_1110, "only the 10 accent bits (1..=10) survive"


def test_encode_subtag_non_member_is_zero():
    assert encode_kautian_subtag(has_main=False, accent_mask=0, has_name=False, has_alt_reading=False) == 0


# --- loader coercion ----------------------------------------------------


def test_as_bool_handles_numpy_bool_string_and_nan():
    assert _as_bool(True) is True
    assert _as_bool(False) is False
    assert _as_bool("True") is True
    assert _as_bool("false") is False  # case-insensitive
    assert _as_bool("") is False
    assert _as_bool(float("nan")) is False
    assert _as_bool(1) is True
    assert _as_bool(0) is False


def test_kautian_subtag_from_row_int_accent_and_bool_flags():
    row = {COL_MAIN: True, COL_ACCENT_MASK: 0b11, COL_NAME: False, COL_ALT_READING: False}
    # main + accent bits 0,1 → bit0 | bit1 | bit2.
    assert _kautian_subtag(row) == (1 << 0) | (1 << 1) | (1 << 2)


def test_kautian_subtag_from_row_string_cells():
    # read_dictionary_csv may hand back string cells under dtype inference.
    row = {COL_MAIN: "False", COL_ACCENT_MASK: "455", COL_NAME: "True", COL_ALT_READING: "True"}
    expected = encode_kautian_subtag(has_main=False, accent_mask=455, has_name=True, has_alt_reading=True)
    assert _kautian_subtag(row) == expected


def test_kautian_subtag_nan_accent_defaults_zero():
    row = {COL_MAIN: True, COL_ACCENT_MASK: float("nan"), COL_NAME: False, COL_ALT_READING: False}
    assert _kautian_subtag(row) == (1 << 0)


# --- v3 record byte layout ----------------------------------------------


def _record(kautian_subtag: int) -> DictionaryRecord:
    """A minimal DictionaryRecord; only the encode-relevant fields matter."""
    return DictionaryRecord(
        rowid=1,
        hanzi="八",
        tl="pueh",
        frequency=42,
        tl_num="pueh4",
        tl_notone="pueh",
        tl_abbrev="p",
        poj_num="poeh4",
        poj_notone="poeh",
        poj_abbrev="p",
        tps_num=None,
        tps_notone=None,
        tps_abbrev=None,
        tps_num_var=None,
        tps_notone_var=None,
        tps_abbrev_var=None,
        sources=tuple((col, col == "kautian") for col in DICT_BIN_COLUMNS),
        syllable_count=1,
        kautian_subtag=kautian_subtag,
    )


def test_version_is_4():
    assert VERSION == 4


def test_encode_record_v4_layout_carries_subtag_and_walker_cost():
    subtag = encode_kautian_subtag(has_main=False, accent_mask=0b11, has_name=False, has_alt_reading=False)
    blob = encode_record(_record(subtag), walker_cost=12_345)
    # <HIBBBHH = bitmask, freq, hanzi_len, tl_len, syllable_count, kautian_subtag, walker_cost
    bitmask, freq, hanzi_len, tl_len, syll, got_subtag, walker_cost = struct.unpack_from(
        "<HIBBBHH", blob, 0
    )
    assert freq == 42
    assert syll == 1
    assert got_subtag == subtag
    assert walker_cost == 12_345
    # Fixed prefix is 13 bytes (11 in v3, 9 in v2); payload follows.
    assert hanzi_len == len("八".encode("utf-8"))
    assert tl_len == len(b"pueh")
    assert len(blob) == 13 + hanzi_len + tl_len
    assert blob[13:] == "八".encode() + b"pueh"


def test_encode_record_rejects_reserved_subtag_bits():
    import pytest

    with pytest.raises(AssertionError):
        encode_record(_record(0xE000), walker_cost=0)  # reserved bits 13-15 set


def test_encode_record_rejects_walker_cost_beyond_u16():
    import pytest

    with pytest.raises(struct.error):
        encode_record(_record(0), walker_cost=0x1_0000)


def test_pandas_isna_helpers_smoke():
    # Sanity: the coercion helpers tolerate real pandas NaN, not just float nan.
    s = pd.Series([None], dtype="object")
    assert _as_bool(s.iloc[0]) is False

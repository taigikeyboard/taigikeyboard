"""Tests for the dictionary.bin v3 kautian_subtag field (Phase 2).

Covers the bit-packing (`source_bits.encode_kautian_subtag`), the loader
coercion from CSV cells (`dictionary_records._kautian_subtag` /
`_as_bool`), and the v3 record byte layout (`create_dictionary_bin`).
"""

from __future__ import annotations

import struct

import pandas as pd

from build.create_dictionary_bin import VERSION, encode_record
from build.dictionary_records import DictionaryRecord, _as_bool, _kautian_subtag
from common.kautian_provenance import COL_ACCENT_MASK, COL_MAIN, COL_NAME
from common.source_bits import (
    DICT_BIN_COLUMNS,
    KAUTIAN_SUBTAG_NAME_BIT,
    encode_kautian_subtag,
)


# --- encode_kautian_subtag (bit packing) --------------------------------


def test_encode_subtag_main_only():
    assert encode_kautian_subtag(has_main=True, accent_mask=0, has_name=False) == 0b0000_0000_0001


def test_encode_subtag_name_only():
    expected = 1 << KAUTIAN_SUBTAG_NAME_BIT  # bit 11
    assert encode_kautian_subtag(has_main=False, accent_mask=0, has_name=True) == expected
    assert expected == 0b1000_0000_0000


def test_encode_subtag_accent_mask_shifted_to_bits_1_through_10():
    # accent bit 0 (鹿港) → subtag bit 1; accent bit 9 (臺中) → subtag bit 10.
    assert encode_kautian_subtag(has_main=False, accent_mask=0b01, has_name=False) == 0b10
    assert encode_kautian_subtag(has_main=False, accent_mask=0b10_0000_0000, has_name=False) == (
        1 << 10
    )


def test_encode_subtag_combined_main_accent_name():
    # main + accent bit 0 + name → bit0 | bit1 | bit11.
    got = encode_kautian_subtag(has_main=True, accent_mask=0b01, has_name=True)
    assert got == (1 << 0) | (1 << 1) | (1 << 11)


def test_encode_subtag_masks_accent_overflow_into_reserved_bits():
    # An accent_mask wider than 10 bits must not spill into reserved bits 12-15.
    got = encode_kautian_subtag(has_main=False, accent_mask=0xFFFF, has_name=False)
    assert got & 0xF000 == 0, "reserved bits must stay 0"
    assert got == 0b0111_1111_1110, "only the 10 accent bits (1..=10) survive"


def test_encode_subtag_non_member_is_zero():
    assert encode_kautian_subtag(has_main=False, accent_mask=0, has_name=False) == 0


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
    row = {COL_MAIN: True, COL_ACCENT_MASK: 0b11, COL_NAME: False}
    # main + accent bits 0,1 → bit0 | bit1 | bit2.
    assert _kautian_subtag(row) == (1 << 0) | (1 << 1) | (1 << 2)


def test_kautian_subtag_from_row_string_cells():
    # read_dictionary_csv may hand back string cells under dtype inference.
    row = {COL_MAIN: "False", COL_ACCENT_MASK: "455", COL_NAME: "True"}
    expected = encode_kautian_subtag(has_main=False, accent_mask=455, has_name=True)
    assert _kautian_subtag(row) == expected


def test_kautian_subtag_nan_accent_defaults_zero():
    row = {COL_MAIN: True, COL_ACCENT_MASK: float("nan"), COL_NAME: False}
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


def test_version_is_3():
    assert VERSION == 3


def test_encode_record_v3_layout_carries_subtag():
    subtag = encode_kautian_subtag(has_main=False, accent_mask=0b11, has_name=False)
    blob = encode_record(_record(subtag))
    # <HIBBBH = bitmask, freq, hanzi_len, tl_len, syllable_count, kautian_subtag
    bitmask, freq, hanzi_len, tl_len, syll, got_subtag = struct.unpack_from("<HIBBBH", blob, 0)
    assert freq == 42
    assert syll == 1
    assert got_subtag == subtag
    # Fixed prefix is 11 bytes (was 9 in v2); payload follows.
    assert hanzi_len == len("八".encode("utf-8"))
    assert tl_len == len(b"pueh")
    assert len(blob) == 11 + hanzi_len + tl_len


def test_encode_record_rejects_reserved_subtag_bits():
    import pytest

    with pytest.raises(AssertionError):
        encode_record(_record(0xF000))  # reserved bits set


def test_pandas_isna_helpers_smoke():
    # Sanity: the coercion helpers tolerate real pandas NaN, not just float nan.
    s = pd.Series([None], dtype="object")
    assert _as_bool(s.iloc[0]) is False

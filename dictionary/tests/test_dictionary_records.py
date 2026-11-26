"""Tests for the khinsiann `--` syllable count: `build.dictionary_records.load_dictionary_records`
(which dictionary.csv rows reach dictionary.bin, and the syllable count each record carries), the
default frequency, and the stage that splits a multi-reading TL into one reading per variant."""

from __future__ import annotations

import csv
from pathlib import Path

from build.dictionary_records import (
    _PROVENANCE_COLUMNS,
    MAX_SYLLABLES,
    REQUIRED_COLUMNS,
    load_dictionary_records,
)
from common.frequency import BASE_DEFAULT_FREQUENCY, get_default_frequency
from common.kautian_provenance import COL_ACCENT_MASK
from common.source_bits import DICT_BIN_COLUMNS
from common.stages.extract import _expand_variant_readings


def _write_csv(path: Path, rows: list[tuple[str, str]]) -> None:
    """A dictionary.csv with the columns the loader reads; every one but hanzi / tl is a placeholder."""
    header = list(dict.fromkeys((*REQUIRED_COLUMNS, *_PROVENANCE_COLUMNS)))
    with open(path, "w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=header)
        writer.writeheader()
        for hanzi, tl in rows:
            row = {column: "x" for column in header}
            row.update({column: "False" for column in (*DICT_BIN_COLUMNS, *_PROVENANCE_COLUMNS)})
            row.update({COL_ACCENT_MASK: "0", "hanzi": hanzi, "tl": tl, "frequency": "8"})
            writer.writerow(row)


def test_khinsiann_double_hyphen_is_one_separator_not_an_extra_syllable(tmp_path):
    # `--` marks the next syllable as neutral tone; it separates two syllables, like `-`.
    csv_path = tmp_path / "dictionary.csv"
    _write_csv(csv_path, [
        ("王哥柳哥", "ông--ko-liú--ko"),  # 4 syllables, 5 hyphens
        ("走來", "tsáu--lâi"),  # 2 syllables, 2 hyphens
        ("九月重陽", "káu--ge̍h tiông-iông"),  # 4 syllables, a space and 3 hyphens
    ])

    counts = {r.hanzi: r.syllable_count for r in load_dictionary_records(csv_path)}

    assert counts == {"王哥柳哥": 4, "走來": 2, "九月重陽": 4}


def test_rows_over_the_syllable_cap_are_still_dropped(tmp_path):
    csv_path = tmp_path / "dictionary.csv"
    _write_csv(csv_path, [
        ("一二三四五", "it-jī-sann-sì-ngóo"),
        ("一个一个一", "tsi̍t-ê--tsi̍t-ê--tsi̍t"),
        ("一个一个", "tsi̍t-ê--tsi̍t-ê"),
    ])

    kept = [r.hanzi for r in load_dictionary_records(csv_path)]

    assert MAX_SYLLABLES == 4
    assert kept == ["一个一个"]


def test_a_reading_of_separators_only_is_dropped(tmp_path):
    # Every record carries 1..=MAX_SYLLABLES syllables; a TL with no syllable has none to carry.
    csv_path = tmp_path / "dictionary.csv"
    _write_csv(csv_path, [("甲", "-"), ("乙", "--"), ("丙", " "), ("丁", "ting")])

    assert [r.hanzi for r in load_dictionary_records(csv_path)] == ["丁"]


def test_default_frequency_counts_khinsiann_as_one_separator():
    assert get_default_frequency("tsáu--lâi") == get_default_frequency("tsáu-lâi") == BASE_DEFAULT_FREQUENCY // 2
    assert get_default_frequency("ông--ko-liú--ko") == BASE_DEFAULT_FREQUENCY // 4


def test_variant_readings_with_khinsiann_split_one_per_variant():
    assert _expand_variant_readings("走來", "tsáu--lâi tsáu-lâi") == ["tsáu--lâi", "tsáu-lâi"]

"""Tests for `build.dictionary_records.load_dictionary_records` — the filter that decides which
dictionary.csv rows reach dictionary.bin, and the syllable count each record carries."""

from __future__ import annotations

import csv
from pathlib import Path

from build.dictionary_records import MAX_SYLLABLES, load_dictionary_records
from common.source_bits import DICT_BIN_COLUMNS

DICTIONARY_DIR = Path(__file__).resolve().parents[1]


def _write_csv(path: Path, rows: list[tuple[str, str]]) -> None:
    """A dictionary.csv with the production header; every column but hanzi / tl is a placeholder."""
    with open(DICTIONARY_DIR / "output" / "dictionary.csv", encoding="utf-8") as f:
        header = next(csv.reader(f))
    with open(path, "w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=header)
        writer.writeheader()
        for hanzi, tl in rows:
            row = {column: "" for column in header}
            row.update({column: "False" for column in DICT_BIN_COLUMNS})
            row.update(hanzi=hanzi, tl=tl, frequency="8", tl_num="x", kautian="True")
            row.update(kautian_main="True", kautian_alt_reading="False", kautian_accent_mask="0", kautian_name="False")
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

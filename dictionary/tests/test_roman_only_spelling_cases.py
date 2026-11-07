"""`cleanup.is_roman_only_hanzi` against the shared case table.

`fixtures/roman_only_spelling_cases.tsv` is also read by the engine's
`roman_only_spelling_parity` test against `phonetics::api::is_roman_only_spelling`,
the port the custom dictionary stores its Hanji through — so a word the
pipeline ships without hanzi is stored the same way when the user imports it.
"""

from __future__ import annotations

import csv
from pathlib import Path

import pytest

from common.cleanup import is_roman_only_hanzi

CASES_PATH = Path(__file__).resolve().parent / "fixtures/roman_only_spelling_cases.tsv"


def read_cases():
    with CASES_PATH.open(encoding="utf-8", newline="") as source:
        reader = csv.DictReader(source, delimiter="\t", quoting=csv.QUOTE_NONE)
        assert reader.fieldnames == ["hanzi", "roman_only"]
        return [(row["hanzi"], row["roman_only"] == "true") for row in reader]


@pytest.mark.parametrize(("hanzi", "expected"), read_cases())
def test_case_table(hanzi, expected):
    assert is_roman_only_hanzi(hanzi) is expected

"""Pipeline tests for pure-Hanji hyphen stripping at merge, dedup, and alignment report."""

from __future__ import annotations

import csv
from pathlib import Path

import pandas as pd

from build import alignment_report, merge_csv
from common.cleanup import is_pure_hanji
from common.source_bits import MAIN_SOURCE_COLUMNS

DICTIONARY_DIR = Path(__file__).resolve().parents[1]


def test_merge_strips_pure_hanji_and_merges_with_plain_twin(tmp_path, monkeypatch):
    inputs = [("taihoa.csv", "taihoa"), ("taijit.csv", "taijit")]

    def write_source(path, source, entries):
        rows = [
            {
                "hanzi": hanzi,
                "tl": tl,
                "poj": poj,
                "frequency": frequency,
                **{column: column == source for column in MAIN_SOURCE_COLUMNS},
            }
            for hanzi, tl, poj, frequency in entries
        ]
        pd.DataFrame(rows).to_csv(tmp_path / path, index=False)

    write_source("taihoa.csv", "taihoa", [
        ("走來", "tsáu--lâi", "cháu--lâi", 30),
        ("大--ê", "tuā--ê", "tōa--ê", 15),
    ])
    write_source("taijit.csv", "taijit", [
        ("走--來", "tsáu--lâi", "cháu--lâi", 16),
        ("一-六", "it-lio̍k", "it-lio̍k", 10),
        ("--起-來", "--khí-lâi", "--khí-lâi", 20),
        ("tso̍h--日", "tso̍h--ji̍t", "cho̍h--ji̍t", 12),
        ("--tńg-去", "--tńg-khì", "--tńg-khì", 12),
        ("邊--a", "pinn--a", "piⁿ--a", 12),
    ])

    monkeypatch.setattr(merge_csv, "BASE_DIR", tmp_path)
    monkeypatch.setattr(merge_csv, "LOG_DIR", tmp_path / "logs")
    monkeypatch.setattr(merge_csv, "INPUT_FILES", inputs)
    for loader in ("_load_khiin_new_entries", "_load_dev_supplement", "_load_lkk_entries"):
        monkeypatch.setattr(merge_csv, loader, lambda *args: None)

    merge_csv.main()

    result = pd.read_csv(tmp_path / "output/dictionary.csv", dtype=str, keep_default_na=False)
    tsau = result[(result.hanzi == "走來") & (result.tl == "tsáu--lâi")]
    assert len(tsau) == 1
    assert int(tsau.iloc[0]["frequency"]) == 30
    assert tsau.iloc[0]["taihoa"] == "True" and tsau.iloc[0]["taijit"] == "True"

    assert set(result["hanzi"]) == {"走來", "大--ê", "一六", "起來", "tso̍h--日", "--tńg-去", "邊--a"}


def test_merged_dictionary_csv_has_zero_hyphens_in_pure_hanji():
    merged_csv = DICTIONARY_DIR / "output" / "dictionary.csv"
    offending = []
    with merged_csv.open(encoding="utf-8", newline="") as f:
        for row in csv.DictReader(f):
            hanzi = row.get("hanzi", "")
            if "-" in hanzi and is_pure_hanji(hanzi):
                offending.append((hanzi, row.get("tl", "")))
    assert not offending, f"Pure-Hanji entries with hyphens in output/dictionary.csv: {offending[:10]}"


def test_alignment_report_writes_unalignable_entries_deterministically(tmp_path):
    csv_path = tmp_path / "dictionary.csv"
    report_path = tmp_path / "alignment_report.txt"
    rows = [
        {"hanzi": "會記得", "tl": "ē-kì--tit", **{c: "False" for c in alignment_report.ALL_SOURCE_COLS}},
        {"hanzi": "白phau-phau", "tl": "pe̍h-phau-phau", **{c: "False" for c in alignment_report.ALL_SOURCE_COLS}},
        {"hanzi": "ビール", "tl": "bì-lù", **{c: "False" for c in alignment_report.ALL_SOURCE_COLS}},
        {"hanzi": "", "tl": "lán", **{c: "False" for c in alignment_report.ALL_SOURCE_COLS}},
        {"hanzi": "甲乙", "tl": "kah", **{c: "False" for c in alignment_report.ALL_SOURCE_COLS}},
        {"hanzi": "有二步\uf777", "tl": "ū--lī-pōo--pueh", **{c: "False" for c in alignment_report.ALL_SOURCE_COLS}},
    ]
    rows[4]["taijit"] = "True"
    rows[5]["taihoa"] = "True"
    pd.DataFrame(rows).to_csv(csv_path, index=False)

    unalignable = alignment_report.write_alignment_report(csv_path, report_path)
    assert unalignable == [
        ("有二步\uf777", "ū--lī-pōo--pueh", "taihoa"),
        ("甲乙", "kah", "taijit"),
    ]
    text = report_path.read_text(encoding="utf-8")
    assert "# unalignable_count=2" in text
    assert "甲乙\tkah\ttaijit" in text

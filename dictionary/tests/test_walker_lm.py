"""build/walker_lm.py — the walker model's counts, smoothing, quantisation and input checks.

Run from `dictionary/`:
    PYTHONPATH=. python3 -m pytest tests/test_walker_lm.py -q
"""

from __future__ import annotations

import math
from dataclasses import replace
from pathlib import Path

import pytest

from build.dictionary_records import DictionaryRecord
from build.walker_lm import (
    HELD_OUT_SOURCE,
    WalkerModel,
    load_model,
    model_from_counts,
    record_costs,
)
from common.source_bits import DICT_BIN_COLUMNS

HEADER = ["hanji", "tl", "count", "moe_kautian", HELD_OUT_SOURCE]


def record(rowid: int, hanzi: str | None, tl: str, tl_num: str) -> DictionaryRecord:
    return DictionaryRecord(
        rowid=rowid,
        hanzi=hanzi,
        tl=tl,
        frequency=0,
        tl_num=tl_num,
        tl_notone=None,
        tl_abbrev=None,
        poj_num=None,
        poj_notone=None,
        poj_abbrev=None,
        tps_num=None,
        tps_notone=None,
        tps_abbrev=None,
        tps_num_var=None,
        tps_notone_var=None,
        tps_abbrev_var=None,
        sources=tuple((col, False) for col in DICT_BIN_COLUMNS),
        syllable_count=tl.replace(" ", "-").count("-") + 1,
        kautian_subtag=0,
    )


TSHUT_LAI = record(1, "出來", "tshut-lâi", "tshut4lai5")
TSHUT_NEUTRAL_LAI = record(2, "出來", "tshut--lâi", "tshut4lai5")
KAU_SIU = record(3, "教授", "kàu-siū", "kau3siu7")
KAU = record(4, "到", "kàu", "kau3")
NO_HANZI = record(5, None, "a", "a1")
RECORDS = [TSHUT_LAI, TSHUT_NEUTRAL_LAI, KAU_SIU, KAU, NO_HANZI]


@pytest.fixture
def unigrams(tmp_path: Path):
    """Writes a `word_unigrams.tsv` with the given rows (and header) and returns its path."""

    def write(rows: list[list[object]], header: list[str] = HEADER) -> Path:
        path = tmp_path / "word_unigrams.tsv"
        lines = ["\t".join(header), *("\t".join(str(cell) for cell in row) for row in rows)]
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return path

    return write


def test_counts_hold_out_typing_and_share_one_word_across_neutral_spellings(unigrams) -> None:
    path = unigrams([["出來", "tshut-lâi", 12, 10, 2], ["教授", "kàu-siū", 5, 5, 0]])
    model = load_model(RECORDS, path, alpha=1.0)
    # trace: 出來 12 − typing 2 = 10; 教授 5; N = 15. Model words: 出來/tshut4lai5 (two
    # records), 教授, 到, (None, a1) → V = 4.
    assert model.tokens == 15
    assert model.vocabulary == 4
    costs = record_costs(RECORDS, model)
    assert costs[0] == costs[1] == round(-math.log((10 + 1) / (15 + 4)) * 1000) == 547
    assert costs[2] == round(-math.log(6 / 19) * 1000) == 1153
    # Unseen words (到, the hanzi-less row) take the smoothed value, not a fallback.
    assert costs[3] == costs[4] == model.unseen_cost == round(-math.log(1 / 19) * 1000) == 2944


def test_model_from_counts_without_the_held_out_column() -> None:
    # A partial `corpus_bigrams --sources` run has no typing column; nothing is held out.
    model = model_from_counts({("教授", "kau3siu7"): {"moe_kautian": 4}}, vocabulary=3, alpha=1.0)
    assert (model.counts, model.tokens) == ({("教授", "kau3siu7"): 4}, 4)


def test_certain_word_costs_zero() -> None:
    model = WalkerModel(counts={}, tokens=0, vocabulary=1, alpha=1.0)
    assert model.cost_of_count(0) == 0  # p = 1


def test_count_beyond_the_corpus_is_capped_at_certainty() -> None:
    # Engine parity (`walker_model.rs` `unseen_and_certain_words_use_the_same_scale`):
    # p = (2000 + 1) / (15 + 4) > 1 → capped to 1, cost 0, never negative.
    model = WalkerModel(counts={}, tokens=15, vocabulary=4, alpha=1.0)
    assert model.cost_of_count(2_000) == 0


def test_cost_beyond_u16_fails_the_build() -> None:
    model = WalkerModel(counts={}, tokens=10**40, vocabulary=1, alpha=1.0)
    # −ln(1 / 1e40) ≈ 92.1 nats = 92,103 milli-nats > 65,535.
    with pytest.raises(ValueError, match="exceeds u16"):
        _ = model.unseen_cost


@pytest.mark.parametrize("alpha", [0.0, -1.0, math.inf, math.nan])
def test_alpha_must_be_finite_and_positive(unigrams, alpha: float) -> None:
    with pytest.raises(ValueError, match="alpha"):
        load_model(RECORDS, unigrams([]), alpha=alpha)


@pytest.mark.parametrize(
    ("rows", "header", "error"),
    [
        pytest.param([["教授", "kàu-siū", 6, 5, 0]], HEADER, "Σ sources", id="count-not-sum"),
        pytest.param([["教授", "kàu-siū", 4, 5, -1]], HEADER, "Σ sources", id="negative-source"),
        pytest.param([["教室", "kàu-sik", 1, 1, 0]], HEADER, "not a dictionary record", id="unknown-word"),
        pytest.param(
            [["出來", "tshut-lâi", 1, 1, 0], ["出來", "tshut--lâi", 1, 1, 0]],
            HEADER,
            "second row",
            id="two-rows-one-word",
        ),
        pytest.param([], HEADER[:-1], HELD_OUT_SOURCE, id="no-held-out-column"),
    ],
)
def test_malformed_unigrams_are_rejected(unigrams, rows, header, error: str) -> None:
    with pytest.raises(ValueError, match=error):
        load_model(RECORDS, unigrams(rows, header))


def test_model_word_lowercases_tl_num(unigrams) -> None:
    upper = replace(KAU, rowid=6, tl_num="KAU3")
    model = load_model([KAU, upper], unigrams([["到", "kàu", 3, 3, 0]]), alpha=1.0)
    assert model.vocabulary == 1
    assert record_costs([KAU, upper], model)[0] == record_costs([KAU, upper], model)[1]

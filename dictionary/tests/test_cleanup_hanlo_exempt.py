"""Tests for `select.hanlo_exempt_sheets` — kautian 合音唸作 contractions skip the hanlo check.

Exempt keys come from `build_membership_set` (built at `select`, before `merge`
drops sheet identity) and must join the post-cleanup `(hanzi, tl)` rows.
`cleanup_dataframe` validates TL via the taigi-converter bridge (started lazily).
"""

from __future__ import annotations

import logging
from pathlib import Path

import pandas as pd
import pytest

from common.cleanup import HANLO_EXEMPT_META_KEY, cleanup_dataframe
from common.kautian_provenance import build_membership_set
from common.stages import select
from pipeline.context import PipelineContext

CONTRACTION_SHEETS = {
    "合音唸作.csv": pd.DataFrame(
        {"漢字": ["查某囝", "落去"], "羅馬字": ["tsa̋u-kiánn", "lueh"]}
    )
}
EXEMPT_KEYS = build_membership_set(CONTRACTION_SHEETS, ["合音唸作.csv"], preserve_spaces=True)


def _rows(df: pd.DataFrame) -> set[tuple[str, str]]:
    return set(zip(df["hanzi"], df["tl"]))


def _cleanup(df: pd.DataFrame, hanlo_exempt=frozenset()):
    return cleanup_dataframe(df.copy(), preserve_spaces=True, hanlo_exempt=hanlo_exempt)


def test_contraction_rows_dropped_without_exemption():
    # trace: 查某囝 = 3 hanzi vs tsa̋u-kiánn = 2 syllables; 落去 = 2 vs lueh = 1 → hanlo_mismatch.
    df = pd.DataFrame({"hanzi": ["查某囝", "落去"], "tl": ["tsa̋u-kiánn", "lueh"]})
    cleaned, dropped = _cleanup(df)
    assert cleaned.empty
    assert set(dropped["drop_reason"]) == {"hanlo_mismatch"}


def test_exempt_contraction_rows_kept():
    df = pd.DataFrame({"hanzi": ["查某囝", "落去"], "tl": ["tsa̋u-kiánn", "lueh"]})
    cleaned, _ = _cleanup(df, hanlo_exempt=EXEMPT_KEYS)
    assert _rows(cleaned) == {("查某囝", "tsa̋u-kiánn"), ("落去", "lueh")}


def test_exemption_is_per_hanzi_tl_pair():
    # Same hanzi, a mismatched reading the sheet does not list → still dropped;
    # a matched reading (3 hanzi / 3 syllables) is kept as before.
    df = pd.DataFrame(
        {"hanzi": ["查某囝", "查某囝", "下昏"], "tl": ["tsa̋-kiánn", "tsa-bóo-kiánn", "i̋ng"]}
    )
    cleaned, dropped = _cleanup(df, hanlo_exempt=EXEMPT_KEYS)
    assert _rows(cleaned) == {("查某囝", "tsa-bóo-kiánn")}
    assert set(zip(dropped["hanzi"], dropped["drop_reason"])) == {
        ("查某囝", "hanlo_mismatch"),
        ("下昏", "hanlo_mismatch"),
    }


def test_exempt_rows_still_pass_other_checks():
    # Exemption covers only the hanlo step: a duplicate exempt row is still deduped.
    df = pd.DataFrame({"hanzi": ["落去", "落去"], "tl": ["lueh", "lueh"]})
    cleaned, dropped = _cleanup(df, hanlo_exempt=EXEMPT_KEYS)
    assert _rows(cleaned) == {("落去", "lueh")}
    assert list(dropped["drop_reason"]) == ["duplicate"]


def _select_ctx(hanlo_exempt_sheets: list[str]) -> PipelineContext:
    config = {
        "source_name": "kautian",
        "stage_options": {
            "select": {
                "sheets": {"合音唸作.csv": {"columns": ["漢字", "羅馬字"]}},
                "hanlo_exempt_sheets": hanlo_exempt_sheets,
            },
            "cleanup": {"preserve_spaces": True},
        },
    }
    ctx = PipelineContext(
        dict_dir=Path("."), base_dir=Path("."), config=config, _logger=logging.getLogger("test")
    )
    ctx.set_sheets({name: df.copy() for name, df in CONTRACTION_SHEETS.items()})
    return ctx


def test_select_stashes_exempt_keys():
    ctx = _select_ctx(["合音唸作.csv"])
    select.run(ctx)
    assert ctx.get_meta(HANLO_EXEMPT_META_KEY) == EXEMPT_KEYS == {("查某囝", "tsa̋u-kiánn"), ("落去", "lueh")}


def test_select_rejects_unknown_exempt_sheet():
    ctx = _select_ctx(["合音.csv"])
    with pytest.raises(ValueError, match="hanlo_exempt_sheets"):
        select.run(ctx)

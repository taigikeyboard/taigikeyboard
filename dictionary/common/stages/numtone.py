"""Numeric-tone stage — add `tl_num`, `poj_num`, `tps_num` from `tl` / `poj`."""

from __future__ import annotations

import logging

import pandas as pd

from pipeline.context import PipelineContext
from common.romanization import to_numeric_tone
from common.taigi_bridge import TpsResidueError, convert_tl_to_tps_strict

_logger = logging.getLogger(__name__)

# POJ non-ASCII characters that must be folded to ASCII in poj_num for trie
# lookup to match ASCII user input. Mirrors kesi `tsuan_ascii`. Regression
# guard: pipeline refuses to produce a poj_num column containing these.
_POJ_NON_ASCII_CHARS = "͘ⁿᴺ"  # o-dot, ⁿ, ᴺ

# Sample-log first N TPS derivation failures per pipeline run; total count
# is logged at stage end. Larger drifts get caught by the per-family
# non-empty gate in `fst-builder build-syllables` (run_build_rejects_empty_tps_input).
_TPS_LOG_SAMPLE_LIMIT = 10


def _derive_tps_num(tl: str, *, failure_state: dict) -> str:
    """Convert hyphenated TL to fused TPS via the `taigi-converter` bridge.

    Empty / NaN `tl` → "" (silent skip). On `TpsResidueError` (per-row
    residue / dialectal char / JS-side conversion failure on this
    token), sets `tps_num=""` for that row, logs the first few
    failures, and increments the run-wide counter so the stage can
    emit a summary line. Per-row failures don't abort the build —
    the `fst-builder` per-family non-empty gate catches a fully-broken
    TPS derivation downstream. `BridgeDeadError` (Node subprocess
    death) is NOT caught here so a dead bridge aborts the build loud
    (Codex PR #334 review).
    """
    if not tl or pd.isna(tl):
        return ""
    try:
        tps = convert_tl_to_tps_strict(str(tl))
    except TpsResidueError as exc:
        failure_state["count"] += 1
        if failure_state["count"] <= _TPS_LOG_SAMPLE_LIMIT:
            _logger.warning("tps_num derivation failed for tl=%r: %s", tl, exc)
        return ""
    # `taigi-converter` joins per-syllable TPS with ASCII space; FST keys
    # and user input use the fused form, so strip whitespace here.
    return tps.replace(" ", "")


def run(ctx: PipelineContext) -> None:
    df = ctx.current_df()
    df = df.copy()
    df["tl_num"] = df["tl"].apply(lambda x: to_numeric_tone(str(x)))
    df["poj_num"] = df["poj"].apply(lambda x: to_numeric_tone(str(x), ascii_only=True))

    tps_failure_state = {"count": 0}
    df["tps_num"] = df["tl"].apply(lambda x: _derive_tps_num(x, failure_state=tps_failure_state))
    if tps_failure_state["count"] > 0:
        _logger.info(
            "%s: tps_num derivation failed for %d / %d rows (logged first %d)",
            ctx.source_name,
            tps_failure_state["count"],
            len(df),
            min(tps_failure_state["count"], _TPS_LOG_SAMPLE_LIMIT),
        )

    pattern = f"[{_POJ_NON_ASCII_CHARS}]"
    bad = df[df["poj_num"].str.contains(pattern, regex=True, na=False)]
    if not bad.empty:
        samples = bad.head(5).apply(
            lambda r: f"{r['poj']!r} → {r['poj_num']!r}", axis=1
        ).tolist()
        raise AssertionError(
            f"{ctx.source_name}: {len(bad)} poj_num rows contain POJ non-ASCII "
            f"(o͘ / ⁿ / ᴺ) after ascii folding; samples: {samples}"
        )

    ctx.set_df(df)

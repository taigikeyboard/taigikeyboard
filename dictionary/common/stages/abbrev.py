"""Abbrev stage — add single-syllable-first-letter abbreviations."""

from __future__ import annotations

import re

import pandas as pd

from pipeline.context import PipelineContext
from common.abbrev import extract_abbrev, extract_tps_abbrev
from common.notone import apply_or_dialect_variant
from common.taigi_bridge import TpsResidueError, convert_tl_to_tps_strict


def _derive_tps_abbrev(tl: str) -> str:
    """Per-syllable TPS first-char concatenation; "" for <2 syllables.

    Calls the `taigi-converter` bridge once per TL syllable so each
    TPS syllable starts at its true initial / vowel glyph (fused
    `tps_num` loses the boundary for tone-1 syllables, which carry no
    Bopomofo tone mark).
    """
    if not tl or pd.isna(tl):
        return ""
    syllables = [s for s in re.split(r"[-\s]+", str(tl)) if s]
    if len(syllables) < 2:
        return ""
    tps_per_syllable: list[str] = []
    for syllable in syllables:
        try:
            tps_per_syllable.append(convert_tl_to_tps_strict(syllable))
        except TpsResidueError:
            # Per-row residue / JS-side conversion failure → skip
            # abbrev for this row. `BridgeDeadError` (subprocess
            # death) is a sibling RuntimeError subclass — NOT
            # `TpsResidueError` — so it propagates and aborts the
            # build (Codex PR #334 review).
            return ""
    return extract_tps_abbrev(str(tl), tps_per_syllable)


def run(ctx: PipelineContext) -> None:
    df = ctx.current_df()
    df = df.copy()
    df["tl_abbrev"] = df["tl"].apply(extract_abbrev)
    df["poj_abbrev"] = df["poj"].apply(extract_abbrev)
    df["tps_abbrev"] = df["tl"].apply(_derive_tps_abbrev)
    # C-3a er↔or dialect dual-emit: ㄜ→ㄛ variant of `tps_abbrev`. Only
    # populated when the abbrev has ㄜ (vowel-initial er/or syllables,
    # e.g. `or-á` → `ㄜㄚ` → `ㄛㄚ`); consonant-initial er/or syllables
    # contribute their consonant glyph and need no variant.
    df["tps_abbrev_var"] = df["tps_abbrev"].apply(lambda x: apply_or_dialect_variant(str(x)))
    ctx.set_df(df)

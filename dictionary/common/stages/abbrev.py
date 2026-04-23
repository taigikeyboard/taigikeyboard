"""Abbrev stage — add single-syllable-first-letter abbreviations."""

from __future__ import annotations

from pipeline.context import PipelineContext
from common.abbrev import extract_abbrev


def run(ctx: PipelineContext) -> None:
    df = ctx.current_df()
    df = df.copy()
    df["tl_abbrev"] = df["tl"].apply(extract_abbrev)
    df["poj_abbrev"] = df["poj"].apply(extract_abbrev)
    ctx.set_df(df)

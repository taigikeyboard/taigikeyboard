"""POJ stage — add `poj` column derived from TL via taigi-converter."""

from __future__ import annotations

from pipeline.context import PipelineContext
from common.taigi_bridge import convert_tl_to_poj


def run(ctx: PipelineContext) -> None:
    df = ctx.current_df()
    df = df.copy()
    df["poj"] = df["tl"].fillna("").map(lambda s: convert_tl_to_poj(s) if s else "")
    ctx.set_df(df)

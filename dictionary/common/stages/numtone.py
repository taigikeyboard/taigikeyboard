"""Numeric-tone stage — add `tl_num` and `poj_num` from diacritic forms."""

from __future__ import annotations

from pipeline.context import PipelineContext
from common.romanization import to_numeric_tone


def run(ctx: PipelineContext) -> None:
    df = ctx.current_df()
    df = df.copy()
    df["tl_num"] = df["tl"].apply(lambda x: to_numeric_tone(str(x)))
    df["poj_num"] = df["poj"].apply(lambda x: to_numeric_tone(str(x), ascii_only=True))
    ctx.set_df(df)

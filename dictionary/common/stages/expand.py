"""Expand stage — split fields with `/`, `,`, `.` separators into multiple rows.

Handles both single-sheet (`ctx._df`) and multi-sheet (`ctx._sheets`, kautian)
modes. Multi-sheet expands each sheet independently so kautian's downstream
`merge` stage sees the same per-sheet row shape it expects.
"""

from __future__ import annotations

from pipeline.context import PipelineContext
from common.expand import expand_dataframe


def run(ctx: PipelineContext) -> None:
    opts = ctx.get_stage_options("expand")
    separators = opts.get("separators")

    if ctx.has_sheets():
        out_sheets = {
            name: expand_dataframe(df, separators=separators, logger=ctx.logger)
            for name, df in ctx.current_sheets().items()
        }
        ctx.set_sheets(out_sheets)
        return

    df = ctx.current_df()
    df = expand_dataframe(df, separators=separators, logger=ctx.logger)
    ctx.set_df(df)

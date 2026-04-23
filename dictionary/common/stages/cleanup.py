"""Cleanup stage — brackets, punctuation, hanlo-mismatch, syllable cap.

Two modes:
- full (default): runs `cleanup_dataframe` — all 11 validation + dedup steps.
- minimal (`cleanup.minimal: true`, currently only khpoo): source is already
  curated upstream, so run `normalize_roman` on `tl` and skip the rest.
"""

from __future__ import annotations

from pipeline.context import PipelineContext
from common.cleanup import cleanup_dataframe, normalize_roman


def run(ctx: PipelineContext) -> None:
    opts = ctx.get_stage_options("cleanup")
    df = ctx.current_df()

    if opts.get("minimal"):
        ctx.logger.info(f"Loaded {len(df)} records")
        ctx.logger.info("  Skipping validation/dedup (pre-processed dictionary)")
        df = df.copy()
        if "tl" in df.columns:
            df["tl"] = df["tl"].apply(normalize_roman)
        ctx.logger.info(f"  Final: {len(df)} records")
        ctx.set_df(df)
        return

    df, _ = cleanup_dataframe(
        df,
        logger=ctx.logger,
        max_syllables=opts.get("max_syllables", 4),
        check_roman_in_hanzi=opts.get("check_roman_in_hanzi", False),
        preserve_spaces=opts.get("preserve_spaces", False),
    )
    ctx.set_df(df)

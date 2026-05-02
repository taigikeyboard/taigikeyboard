"""POJ stage — add `poj` column derived from TL via taigi-converter.

Uses the *strict* converter so a Node IPC failure aborts the per-source
pipeline immediately. The graceful converter (returns input on
subprocess failure) was the silent failure mode that let stale
`poj == tl` rows ship in the PR #175 + PR #184 incident — fixing the
loud-failure path here makes `build/audit.py 12_stale_poj` a backstop
rather than the only line of defense.
"""

from __future__ import annotations

from pipeline.context import PipelineContext
from common.taigi_bridge import convert_tl_to_poj_strict


def run(ctx: PipelineContext) -> None:
    df = ctx.current_df()
    df = df.copy()
    df["poj"] = df["tl"].fillna("").map(
        lambda s: convert_tl_to_poj_strict(s) if s else ""
    )
    ctx.set_df(df)

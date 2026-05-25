"""Notone stage — add tone-stripped variants `tl_notone` / `poj_notone` / `tps_notone`."""

from __future__ import annotations

from pipeline.context import PipelineContext
from common.notone import apply_or_dialect_variant, remove_tone, remove_tps_tone


def run(ctx: PipelineContext) -> None:
    df = ctx.current_df()
    df = df.copy()
    # Legacy strips tones from the *numeric-tone* column, not the diacritic one,
    # so the input here is tl_num / poj_num (always populated by numtone stage).
    df["tl_notone"] = df["tl_num"].apply(lambda x: remove_tone(str(x)))
    df["poj_notone"] = df["poj_num"].apply(lambda x: remove_tone(str(x)))
    df["tps_notone"] = df["tps_num"].apply(lambda x: remove_tps_tone(str(x)))
    # C-3a er↔or dialect dual-emit: ㄜ→ㄛ variant of `tps_notone`. Empty
    # when no ㄜ present; downstream emit dedups identical keys.
    df["tps_notone_var"] = df["tps_notone"].apply(lambda x: apply_or_dialect_variant(str(x)))
    ctx.set_df(df)

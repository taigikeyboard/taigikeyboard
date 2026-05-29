"""kautian_provenance stage — join subcollection membership onto cleaned rows.

Reads the provenance maps stashed by `select` (built from the raw ODS sheets
before the dialect melt drops 腔調) and adds `kautian_main` /
`kautian_accent_mask` / `kautian_name`. Runs after `cleanup` (so `tl` is
normalized) and before `frequency`. No-op when no maps are stashed — only the
kautian config wires this stage in.
"""

from __future__ import annotations

from common.kautian_provenance import PROVENANCE_META_KEY, apply_provenance
from common.kautian_provenance import COL_ACCENT_MASK, COL_MAIN, COL_NAME
from pipeline.context import PipelineContext


def run(ctx: PipelineContext) -> None:
    maps = ctx.get_meta(PROVENANCE_META_KEY)
    if not maps:
        ctx.logger.info("  [skip] no kautian provenance maps stashed")
        return

    df = apply_provenance(
        ctx.current_df(),
        accent_map=maps["accent"],
        name_set=maps["name"],
        main_set=maps["main"],
    )

    n_main = int(df[COL_MAIN].sum())
    n_name = int(df[COL_NAME].sum())
    n_accent = int((df[COL_ACCENT_MASK] > 0).sum())
    ctx.logger.info(
        f"  provenance: main={n_main} accent_rows={n_accent} name={n_name}"
    )
    ctx.set_df(df)

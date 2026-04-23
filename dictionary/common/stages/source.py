"""Source stage — tag every row with `<source_name> = True`.

No-hanzi policy (configurable per-source in `stage_options.source.no_hanzi`):

- ``clear`` (default; official dicts + khpoo): rows where hanzi equals tl or
  poj are romanization-only; clear hanzi in place and keep the row.
- ``drop`` (ChhoeTaigi community dicts): also drop rows where hanzi is NaN or
  empty, plus the equality cases — the row is removed entirely.
"""

from __future__ import annotations

from common.source_bits import SOURCE_BITS
from common.stages import csv_roundtrip
from pipeline.context import PipelineContext


def run(ctx: PipelineContext) -> None:
    name = ctx.source_name
    if name not in SOURCE_BITS:
        raise ValueError(f"source_name {name!r} not in SOURCE_BITS")

    policy = ctx.get_stage_options("source").get("no_hanzi", "clear")
    if policy not in ("clear", "drop"):
        raise ValueError(f"{name}: stage_options.source.no_hanzi must be 'clear' or 'drop', got {policy!r}")

    # Round-trip through CSV so abbrev-column strings that happen to match
    # NaN tokens (e.g. "nan" from syllables niû-á-nng) become actual NaN and
    # serialise back to empty on final write. Without this, taihoa emits
    # literal "nan" in the abbrev column.
    df = csv_roundtrip(ctx.current_df())
    eq_mask = (df["hanzi"] == df["tl"]) | (df["hanzi"] == df["poj"])

    if policy == "clear":
        df.loc[eq_mask, "hanzi"] = ""
    else:
        missing_mask = df["hanzi"].isna() | (df["hanzi"] == "")
        df = df[~(missing_mask | eq_mask)].reset_index(drop=True)

    df[name] = True
    ctx.set_df(df)

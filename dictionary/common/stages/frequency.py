"""Frequency stage — attach `frequency` column from char-frequency corpus."""

from __future__ import annotations

from pipeline.context import PipelineContext
from common.frequency import load_frequency_map, get_frequency

# Shared char-freq map cached per-process to avoid re-reading for every dict.
_FREQ_MAP: dict[str, int] | None = None


def _load(ctx: PipelineContext) -> dict:
    global _FREQ_MAP
    if _FREQ_MAP is None:
        freq_file = ctx.shared_dir / "char_freq_merged.txt"
        _FREQ_MAP = load_frequency_map(str(freq_file))
    return _FREQ_MAP


def run(ctx: PipelineContext) -> None:
    df = ctx.current_df()
    freq_map = _load(ctx)
    df = df.copy()
    df["frequency"] = df.apply(
        lambda row: get_frequency(str(row["hanzi"]), str(row["tl"]), freq_map),
        axis=1,
    )
    # Deterministic row order for downstream stages: freq desc, then hanzi/tl asc.
    df = df.sort_values(
        ["frequency", "hanzi", "tl"],
        ascending=[False, True, True],
        ignore_index=True,
    )
    ctx.set_df(df)

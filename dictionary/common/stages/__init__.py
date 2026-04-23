"""Reusable pipeline stage functions.

Each stage exposes a single `run(ctx: PipelineContext) -> None` that mutates
the shared `PipelineContext` (setting the current DataFrame or sheets dict).

`STAGE_RUNNERS` is the single source of truth for the stage name → runner
mapping. `pipeline/run.py` iterates it; `pipeline/config_schema.py` uses its
keys as the allowed-stage set during config validation.
"""

from __future__ import annotations

import io
from typing import Callable

import pandas as pd

from pipeline.context import PipelineContext


def csv_roundtrip(df: pd.DataFrame) -> pd.DataFrame:
    """Serialise a DataFrame to CSV and reparse it via `pd.read_csv`.

    Shared across stages that need the CSV-layer side effects pandas applies
    on read — specifically:

    - deduping clashing column names by suffixing (`hanzi` + `hanzi` →
      `hanzi` + `hanzi.1`), used by the kautian 異用字 sheet
    - re-inferring dtypes so numeric-looking strings become NaN and
      serialise back to empty rather than the literal "nan" string
    """
    buf = io.StringIO()
    df.to_csv(buf, index=False)
    buf.seek(0)
    return pd.read_csv(buf)


from . import (
    abbrev,
    cleanup,
    expand,
    extract,
    frequency,
    merge,
    notone,
    numtone,
    poj,
    select,
    source,
    variants,
)

StageFn = Callable[[PipelineContext], None]

STAGE_RUNNERS: dict[str, StageFn] = {
    "extract":   extract.run,
    "select":    select.run,
    "expand":    expand.run,
    "cleanup":   cleanup.run,
    "merge":     merge.run,
    "frequency": frequency.run,
    "poj":       poj.run,
    "numtone":   numtone.run,
    "notone":    notone.run,
    "abbrev":    abbrev.run,
    "source":    source.run,
    "variants":  variants.run,
}

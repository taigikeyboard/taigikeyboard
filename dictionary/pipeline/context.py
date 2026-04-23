# -*- coding: utf-8 -*-
"""PipelineContext — shared state threaded through every stage function."""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import pandas as pd

from common.logging_utils import setup_logging


@dataclass
class PipelineContext:
    """Shared state for a single dictionary's pipeline run.

    Stages receive one instance and mutate `df` via `set_df`. kautian's
    extract/select/expand operate on `_sheets` (dict of sheet-name → DataFrame)
    until the `merge` stage flattens them into a single `_df`. All subsequent
    stages use `_df`.
    """

    dict_dir: Path                         # e.g. dictionary/sources/official/kautian/
    base_dir: Path                         # dictionary/
    config: dict[str, Any]
    _df: pd.DataFrame | None = None
    _sheets: dict[str, pd.DataFrame] | None = None
    _logger: logging.Logger | None = field(default=None, repr=False)

    @property
    def source_name(self) -> str:
        return self.config["source_name"]

    @property
    def category(self) -> str:
        return self.config["category"]

    @property
    def logger(self) -> logging.Logger:
        if self._logger is None:
            log_dir = self.base_dir / "logs" / self.source_name
            log_dir.mkdir(parents=True, exist_ok=True)
            self._logger = setup_logging(
                f"pipeline.{self.source_name}",
                log_dir=str(log_dir),
            )
        return self._logger

    @property
    def shared_dir(self) -> Path:
        return self.base_dir / "shared" / "data"

    def input_path(self) -> Path:
        return self.dict_dir / self.config["input_path"]

    def get_stage_options(self, stage: str) -> dict[str, Any]:
        return (self.config.get("stage_options") or {}).get(stage) or {}

    def current_df(self) -> pd.DataFrame:
        if self._df is None:
            raise RuntimeError(
                f"{self.source_name}: no DataFrame in context — earlier stage "
                "must produce one before this stage runs"
            )
        return self._df

    def set_df(self, df: pd.DataFrame) -> None:
        self._df = df

    def current_sheets(self) -> dict[str, pd.DataFrame]:
        if self._sheets is None:
            raise RuntimeError(
                f"{self.source_name}: no multi-sheet data in context — this "
                "stage expected a prior extract/select/expand to populate _sheets"
            )
        return self._sheets

    def set_sheets(self, sheets: dict[str, pd.DataFrame]) -> None:
        self._sheets = sheets

    def has_sheets(self) -> bool:
        """True while kautian's multi-sheet carriage is still live."""
        return self._sheets is not None

    def clear_sheets(self) -> None:
        """Drop the multi-sheet carriage — used by `merge` once it flattens to `_df`."""
        self._sheets = None

    def save_final_csv(self, df: pd.DataFrame) -> Path:
        """Write the canonical `data/<source>.csv` that build/merge_csv.py consumes."""
        path = self.dict_dir / "data" / f"{self.source_name}.csv"
        path.parent.mkdir(parents=True, exist_ok=True)
        df.to_csv(path, index=False)
        return path

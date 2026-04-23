# -*- coding: utf-8 -*-
"""Shared logging setup for pipeline + build scripts."""

from __future__ import annotations

import logging
from datetime import datetime
from pathlib import Path


def setup_logging(script_name: str, log_dir: str | Path = "logs") -> logging.Logger:
    """Return a named logger that writes both to file and stderr.

    Unlike an earlier version that called `logging.basicConfig` and mutated
    the root logger, this attaches handlers to a *named* logger and sets
    `propagate = False` so callers can coexist without stepping on each
    other's handler chains.
    """
    log_dir = Path(log_dir)
    log_dir.mkdir(parents=True, exist_ok=True)

    logger = logging.getLogger(script_name)
    logger.setLevel(logging.INFO)
    logger.propagate = False
    # Reset handlers if the logger was previously configured (idempotent).
    for h in list(logger.handlers):
        logger.removeHandler(h)

    formatter = logging.Formatter("%(message)s")
    file_handler = logging.FileHandler(log_dir / f"{script_name}.log", mode="w", encoding="utf-8")
    file_handler.setFormatter(formatter)
    stream_handler = logging.StreamHandler()
    stream_handler.setFormatter(formatter)

    logger.addHandler(file_handler)
    logger.addHandler(stream_handler)
    return logger


def log_header(
    logger: logging.Logger,
    script_name: str,
    input_path: str | Path,
    output_path: str | Path,
) -> None:
    """Standard banner at the top of each script's log."""
    logger.info("=" * 50)
    logger.info(script_name)
    logger.info(f"Run: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    logger.info("=" * 50)
    logger.info(f"Input:  {input_path}")
    logger.info(f"Output: {output_path}")
    logger.info("")

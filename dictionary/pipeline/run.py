# -*- coding: utf-8 -*-
"""Pipeline entry point — run every discovered config through its stage list.

Usage:
  python3 -m pipeline.run
"""

from __future__ import annotations

import sys
from pathlib import Path

from common.stages import STAGE_RUNNERS
from pipeline.config_schema import CATEGORIES, validate_config
from pipeline.context import PipelineContext

BASE_DIR = Path(__file__).resolve().parent.parent


def discover_configs(base_dir: Path = BASE_DIR) -> list[tuple[Path, dict]]:
    """Find every sources/<category>/<key>/config.yaml and validate it."""
    found: list[tuple[Path, dict]] = []

    sources_dir = base_dir / "sources"
    if not sources_dir.exists():
        return found

    for cat in CATEGORIES:
        cat_dir = sources_dir / cat
        if not cat_dir.is_dir():
            continue
        for cfg in sorted(cat_dir.glob("*/config.yaml")):
            found.append((cfg.parent, validate_config(cfg)))

    return found


def run_dict(dict_dir: Path, config: dict) -> None:
    ctx = PipelineContext(dict_dir=dict_dir, base_dir=BASE_DIR, config=config)

    for stage in config["stages"]:
        ctx.logger.info(f"[{config['source_name']}] stage: {stage}")
        STAGE_RUNNERS[stage](ctx)

    # Canonical CSV output is a runner contract, not a stage side effect.
    # Any valid config — even one that ends at `source` or `abbrev` without
    # `variants` — must produce the `data/<key>.csv` that build/merge_csv.py
    # consumes.
    final_path = ctx.save_final_csv(ctx.current_df())
    ctx.logger.info(f"[{config['source_name']}] wrote {final_path.relative_to(BASE_DIR)}")


def main() -> int:
    configs = discover_configs()
    if not configs:
        sys.exit("error: no config.yaml found under sources/")

    for dict_dir, cfg in configs:
        run_dict(dict_dir, cfg)
    return 0


if __name__ == "__main__":
    sys.exit(main())

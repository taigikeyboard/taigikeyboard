# -*- coding: utf-8 -*-
"""Config schema + validator for per-source `config.yaml`.

A config describes *what to do* for one dictionary source; the stage modules under
`common.stages` consume it via `PipelineContext.get_stage_options(stage)`.

Run directly for a self-test:
    python3 -m pipeline.config_schema --selftest
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:
    sys.exit("error: pyyaml is required (see dictionary/requirements.txt)")

from common.source_bits import SOURCE_BITS
from common.stages import STAGE_RUNNERS

CATEGORIES = ("official", "community", "supplementary")

INPUT_FORMATS = (
    "ods",                  # MoE multi-sheet ODS (kautian)
    "json",                 # taigitv, kungge scrape JSON
    "csv",                  # headered CSV (ChhoeTaigi: itaigi, sitbut, taihoa, taijit; stti)
    "csv_noheader",         # headerless CSV with injected names (khpoo)
)

# Single source of truth for stage names lives in common/stages/__init__.py.
# config validation uses the key set; pipeline/run.py uses the runner mapping.
STAGES = tuple(STAGE_RUNNERS)

VARIANTS_MODES = ("generate", "mark")


class ConfigError(ValueError):
    pass


def _require(obj: dict[str, Any], key: str, where: str) -> Any:
    if key not in obj:
        raise ConfigError(f"{where}: missing required key '{key}'")
    return obj[key]


def validate_config(path: Path) -> dict[str, Any]:
    """Parse and validate a config.yaml. Raises ConfigError on any issue."""
    with path.open(encoding="utf-8") as f:
        raw = yaml.safe_load(f)
    if not isinstance(raw, dict):
        raise ConfigError(f"{path}: config must be a mapping at top level")

    source_name = _require(raw, "source_name", str(path))
    if source_name not in SOURCE_BITS:
        raise ConfigError(
            f"{path}: source_name={source_name!r} is not a known bit position. "
            f"Known: {sorted(SOURCE_BITS)}"
        )

    category = _require(raw, "category", str(path))
    if category not in CATEGORIES:
        raise ConfigError(f"{path}: category={category!r} not in {CATEGORIES}")

    _require(raw, "chinese_name", str(path))

    input_format = _require(raw, "input_format", str(path))
    if input_format not in INPUT_FORMATS:
        raise ConfigError(f"{path}: input_format={input_format!r} not in {INPUT_FORMATS}")

    input_path = _require(raw, "input_path", str(path))
    if not isinstance(input_path, str):
        raise ConfigError(f"{path}: input_path must be a string")

    stages = _require(raw, "stages", str(path))
    if not isinstance(stages, list) or not stages:
        raise ConfigError(f"{path}: stages must be a non-empty list")
    for s in stages:
        if s not in STAGES:
            raise ConfigError(f"{path}: unknown stage {s!r}; known: {STAGES}")

    # Ordering invariants
    if "source" in stages and "variants" in stages:
        si, vi = stages.index("source"), stages.index("variants")
        if si > vi:
            raise ConfigError(f"{path}: 'source' must come before 'variants'")
    if "variants" in stages and stages[-1] != "variants":
        raise ConfigError(f"{path}: 'variants' must be the final stage")
    if "merge" in stages and input_format != "ods":
        raise ConfigError(
            f"{path}: stage 'merge' only applies to ods input (kautian multi-sheet)"
        )

    # Variants mode must be set if the stage runs
    stage_options = raw.get("stage_options") or {}
    if "variants" in stages:
        mode = (stage_options.get("variants") or {}).get("mode")
        if mode not in VARIANTS_MODES:
            raise ConfigError(
                f"{path}: stage_options.variants.mode must be one of {VARIANTS_MODES}"
            )

    return raw


def _selftest() -> int:
    """Construct an in-memory sample + run validator; used in CI."""
    import io
    import textwrap

    good = textwrap.dedent(
        """
        source_name: kautian
        category: official
        chinese_name: 教育部臺灣台語常用詞辭典
        input_format: ods
        input_path: data/raw/kautian.ods
        stages: [extract, select, expand, cleanup, merge, frequency, poj, numtone, notone, abbrev, source, variants]
        stage_options:
          variants:
            mode: generate
            variants_csv: ../../../supplementary/variants/data/variants.csv
        """
    ).strip()

    tmp = Path("/tmp/_selftest_config.yaml")
    tmp.write_text(good)
    try:
        validate_config(tmp)
    except ConfigError as e:
        print(f"selftest FAIL (should have validated): {e}")
        return 1

    # Negative cases
    bad_cases = [
        ("source_name: bogus\n" + good.split("\n", 1)[1], "bogus source_name"),
        (good.replace("category: official", "category: foo"), "bad category"),
        (good.replace("input_format: ods", "input_format: xls"), "bad input_format"),
        (good.replace("mode: generate", "mode: transform"), "bad variants.mode"),
    ]
    for cfg, label in bad_cases:
        tmp.write_text(cfg)
        try:
            validate_config(tmp)
        except ConfigError:
            continue
        print(f"selftest FAIL: {label} should have raised")
        return 1

    print("selftest OK")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("config", nargs="?", type=Path)
    args = ap.parse_args()
    if args.selftest:
        return _selftest()
    if not args.config:
        ap.error("pass a config path or --selftest")
    try:
        cfg = validate_config(args.config)
    except ConfigError as e:
        print(f"FAIL: {e}", file=sys.stderr)
        return 1
    print(f"OK: {args.config} validates ({cfg['source_name']}, {cfg['category']})")
    return 0


if __name__ == "__main__":
    sys.exit(main())

"""Variants stage — mark (and optionally generate) variant-character (異用字) rows.

Two modes keyed off `stage_options.variants.mode`:

- **generate** (kautian): for each (hanzi, tl) pair in the dictionary that
  also appears in `variants.csv`, copy the row and swap `hanzi` for the
  variant character — unless (variant, tl) already exists as a primary entry.
  Original rows keep `is_variant=False`; generated rows get `is_variant=True`.

- **mark** (everyone else): load `variants.csv` as a list of
  `(variant_word, syllables_tuple)` pairs. For each dict row, search for
  `variant_word` as a substring of `hanzi`; if the corresponding syllable
  window in `tl` matches the recorded syllables, set `is_variant=True`.
"""

from __future__ import annotations

from pathlib import Path

import pandas as pd

from common.variants import read_variant_rows
from pipeline.context import PipelineContext


def run(ctx: PipelineContext) -> None:
    opts = ctx.get_stage_options("variants")
    mode = opts.get("mode")
    variants_csv = opts.get("variants_csv")
    if not variants_csv:
        raise ValueError(
            f"{ctx.source_name}: variants.variants_csv is required"
        )
    variants_path = (ctx.dict_dir / variants_csv).resolve()
    if mode == "generate":
        _run_generate(ctx, variants_path)
    elif mode == "mark":
        _run_mark(ctx, variants_path)
    else:
        raise ValueError(f"{ctx.source_name}: unknown variants.mode={mode!r}")


def _run_generate(ctx: PipelineContext, variants_path: Path) -> None:
    variants_map, _ = _load_variants_map(ctx, variants_path)
    df = ctx.current_df().copy()
    total = len(df)
    ctx.logger.info(f"Loaded dictionary: {total} records")

    original_entries: set[tuple[str, str]] = set()
    for _, row in df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        tl = str(row["tl"]).strip().lower()
        original_entries.add((hanzi, tl))
    ctx.logger.info(f"Original entries set: {len(original_entries)} unique (hanzi, tl) pairs")

    df["is_variant"] = False

    new_rows: list[pd.Series] = []
    matched = 0
    skipped = 0
    for _, row in df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        tl = str(row["tl"]).strip().lower()
        key = (hanzi, tl)
        if key in variants_map:
            matched += 1
            for variant in variants_map[key]:
                if (variant, tl) in original_entries:
                    skipped += 1
                    continue
                new_row = row.copy()
                new_row["hanzi"] = variant
                new_row["is_variant"] = True
                new_rows.append(new_row)

    ctx.logger.info(f"Matched (hanzi, tl) pairs for generation: {matched}")
    ctx.logger.info(f"Skipped (variant already in original): {skipped}")
    ctx.logger.info(f"Generated variant records: {len(new_rows)}")

    if new_rows:
        variant_df = pd.DataFrame(new_rows)
        result = pd.concat([df, variant_df], ignore_index=True)
    else:
        result = df

    ctx.logger.info(f"Final: {len(result)} records")
    ctx.set_df(result)


def _run_mark(ctx: PipelineContext, variants_path: Path) -> None:
    variant_entries = set(_load_variant_entries(ctx, variants_path))
    variant_lengths = sorted({len(word) for word, _ in variant_entries})
    df = ctx.current_df().copy()
    ctx.logger.info(f"Loaded dictionary: {len(df)} records")

    # Probe each window of `hanzi` against the entry set instead of scanning
    # every entry per row: hanzi is a few characters, the entry list is
    # thousands long.
    def check_is_variant(row) -> bool:
        hanzi = str(row["hanzi"]).strip()
        tl = str(row["tl"]).strip().lower()
        syllables = tl.replace("--", "-").split("-")
        for variant_len in variant_lengths:
            for idx in range(min(len(hanzi), len(syllables)) - variant_len + 1):
                end_idx = idx + variant_len
                if (hanzi[idx:end_idx], tuple(syllables[idx:end_idx])) in variant_entries:
                    return True
        return False

    df["is_variant"] = df.apply(check_is_variant, axis=1)
    ctx.logger.info(f"Records marked as variant: {int(df['is_variant'].sum())}")

    ctx.set_df(df)


def _load_variants_map(
    ctx: PipelineContext, path: Path
) -> tuple[dict[tuple[str, str], list[str]], set[tuple[str, str]]]:
    variants_map: dict[tuple[str, str], list[str]] = {}
    is_variant_set: set[tuple[str, str]] = set()
    if not path.exists():
        ctx.logger.warning(f"Variants file not found: {path}")
        return variants_map, is_variant_set

    for hanzi, variant, tl in read_variant_rows(path):
        key = (hanzi, tl)
        variants_map.setdefault(key, [])
        if variant not in variants_map[key]:
            variants_map[key].append(variant)
        is_variant_set.add((variant, tl))

    ctx.logger.info(f"Variants map: {len(variants_map)} unique (hanzi, tl) pairs")
    ctx.logger.info(f"Is-variant set: {len(is_variant_set)} unique (variant, tl) pairs")
    return variants_map, is_variant_set


def _load_variant_entries(
    ctx: PipelineContext, path: Path
) -> list[tuple[str, tuple[str, ...]]]:
    entries: list[tuple[str, tuple[str, ...]]] = []
    if not path.exists():
        ctx.logger.warning(f"Variants file not found: {path}")
        return entries

    for _hanzi, variant, tl in read_variant_rows(path):
        syllables = tuple(tl.replace("--", "-").split("-"))
        entries.append((variant, syllables))

    ctx.logger.info(f"Variant entries: {len(entries)}")
    return entries

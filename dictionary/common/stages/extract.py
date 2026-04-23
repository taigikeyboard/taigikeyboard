"""Extract stage — read the raw source file into the pipeline context.

Dispatches on `input_format`:

- `ods`   (kautian): read all sheets with `engine=odf`; populate `ctx._sheets`.
                     The `select` stage then pivots columns per-sheet.
- `json`  (taigitv, kungge): read JSON array, map configured fields to
                              (hanzi, tl). `tl_is_array: true` expands array
                              entries to multiple rows (taigitv).
- `csv`   (ChhoeTaigi 4 + stti): read CSV, optionally apply `column_mapping`
                                  to rename columns to (hanzi, tl).
                                  Set `mode: stti_variant_expand` for stti's
                                  custom space-based reading expansion.
- `csv_noheader` (khpoo): read CSV with `header=None, names=[...]`.

Options honoured across formats (per config):
- `drop_empty_tl`: drop rows where tl is NaN or whitespace.
- `dedup`: drop_duplicates(subset=[hanzi, tl], keep="first").
"""

from __future__ import annotations

import json
from pathlib import Path

import pandas as pd

from pipeline.context import PipelineContext


def run(ctx: PipelineContext) -> None:
    fmt = ctx.config["input_format"]
    path = ctx.input_path()
    opts = ctx.get_stage_options("extract")

    if fmt == "ods":
        _extract_ods(ctx, path, opts)
    elif fmt == "json":
        _extract_json(ctx, path, opts)
    elif fmt == "csv":
        _extract_csv(ctx, path, opts)
    elif fmt == "csv_noheader":
        _extract_csv_noheader(ctx, path, opts)
    else:
        raise ValueError(f"{ctx.source_name}: unsupported input_format={fmt!r}")


def _extract_ods(ctx: PipelineContext, path: Path, opts: dict) -> None:
    """kautian — read every sheet, stage them under `ctx._sheets` for `select`."""
    sheets = pd.read_excel(path, sheet_name=None, engine="odf")
    sheets_named = {f"{name}.csv": df for name, df in sheets.items()}
    ctx.logger.info(f"Loaded {len(sheets_named)} sheets from {path.name}")
    for name, df in sheets_named.items():
        ctx.logger.info(f"  {name}: {len(df)} rows  columns={list(df.columns)}")
    ctx.set_sheets(sheets_named)


def _extract_json(ctx: PipelineContext, path: Path, opts: dict) -> None:
    """taigitv, kungge — array-of-objects JSON with configurable field mapping."""
    hanzi_field = opts.get("hanzi_field")
    tl_field = opts.get("tl_field")
    tl_is_array = bool(opts.get("tl_is_array", False))
    if not hanzi_field or not tl_field:
        raise ValueError(
            f"{ctx.source_name}: json extract requires hanzi_field + tl_field"
        )

    with path.open(encoding="utf-8") as f:
        data = json.load(f)
    ctx.logger.info(f"Loaded {len(data)} entries from {path.name}")

    rows: list[dict] = []
    for item in data:
        hanzi = item.get(hanzi_field, "")
        if tl_is_array:
            for tl in item.get(tl_field, []) or []:
                if not tl:
                    continue
                rows.append({"hanzi": hanzi, "tl": tl})
        else:
            tl = item.get(tl_field, "")
            if not tl:
                continue
            rows.append({"hanzi": hanzi, "tl": tl})

    df = pd.DataFrame(rows)
    df = _finalize_csv_or_json(ctx, df, opts, initial_count=len(rows))
    ctx.set_df(df)


def _extract_csv(ctx: PipelineContext, path: Path, opts: dict) -> None:
    """ChhoeTaigi 4 + stti — headered CSV.

    - `column_mapping`: rename source columns to (hanzi, tl) and drop others.
    - `mode: stti_variant_expand`: invoke stti's custom per-row expansion.
    """
    df = pd.read_csv(path)
    ctx.logger.info(f"Loaded {len(df)} rows from {path.name}")

    mode = opts.get("mode")
    if mode == "stti_variant_expand":
        df = _extract_csv_stti(ctx, df, opts)
    else:
        mapping = opts.get("column_mapping")
        if mapping:
            selected = list(mapping.keys())
            missing = [c for c in selected if c not in df.columns]
            if missing:
                raise ValueError(
                    f"{ctx.source_name}: extract.column_mapping refs missing "
                    f"columns {missing}; available: {list(df.columns)}"
                )
            df = df[selected].copy()
            df = df.rename(columns=mapping)

    df = _finalize_csv_or_json(ctx, df, opts, initial_count=len(df))
    ctx.set_df(df)


def _extract_csv_stti(ctx: PipelineContext, df: pd.DataFrame, opts: dict) -> pd.DataFrame:
    """STTI extract — expand rows whose 漢字 carries multiple variant readings.

    For each row: if 漢字 contains spaces, split both 漢字/臺羅 by space and
    pair them up 1:1; otherwise run `expand_variant_readings` to detect
    space-separated variant readings by greedy-grouping TL tokens against
    the 漢字 character count.
    """
    hanzi_col = opts.get("hanzi_col", "臺灣台語詞彙")
    tl_col = opts.get("tl_col", "臺羅")

    results: list[dict] = []
    skipped_mismatch = 0
    expanded_count = 0

    for _, row in df.iterrows():
        hanzi_raw = str(row[hanzi_col]).strip()
        tl_raw = str(row[tl_col]).strip()

        if not tl_raw or tl_raw == "nan":
            continue

        if " " in hanzi_raw:
            hanzi_list = hanzi_raw.split()
            tl_list = tl_raw.split()
            if len(hanzi_list) != len(tl_list):
                skipped_mismatch += 1
                continue
            for hanzi, tl in zip(hanzi_list, tl_list):
                if hanzi and tl:
                    results.append({"hanzi": hanzi, "tl": tl})
        else:
            tl_variants = _expand_variant_readings(hanzi_raw, tl_raw)
            if len(tl_variants) > 1:
                expanded_count += 1
            for tl in tl_variants:
                results.append({"hanzi": hanzi_raw, "tl": tl})

    if expanded_count:
        ctx.logger.info(f"Expanded variant readings: {expanded_count}")
    if skipped_mismatch:
        ctx.logger.warning(f"Skipped (hanzi/tl count mismatch): {skipped_mismatch}")
    return pd.DataFrame(results)


def _expand_variant_readings(hanzi: str, tl: str) -> list[str]:
    """Greedy-group space-separated TL tokens against hanzi char count.

    Used by stti where one 漢字 phrase may ship with N variant readings in
    the 臺羅 column, concatenated by spaces. Returns one TL per variant.
    """
    tokens = tl.split()
    if len(tokens) <= 1:
        return [tl]

    hanzi_count = len(hanzi)
    if hanzi_count == 0:
        return [tl]

    groups: list[str] = []
    current_group: list[str] = []
    current_syllables = 0

    for token in tokens:
        syllables = len(token.split("-"))
        current_group.append(token)
        current_syllables += syllables

        if current_syllables == hanzi_count:
            groups.append("-".join(current_group))
            current_group = []
            current_syllables = 0
        elif current_syllables > hanzi_count:
            return [tl]

    if current_group:
        return [tl]

    return groups if len(groups) > 1 else [tl]


def _extract_csv_noheader(ctx: PipelineContext, path: Path, opts: dict) -> None:
    """khpoo — headerless CSV with explicit column names."""
    names = opts.get("names") or ["hanzi", "tl"]
    df = pd.read_csv(path, header=None, names=names)
    ctx.logger.info(f"Loaded {len(df)} rows from {path.name}")
    df = _finalize_csv_or_json(ctx, df, opts, initial_count=len(df))
    ctx.set_df(df)


def _finalize_csv_or_json(
    ctx: PipelineContext,
    df: pd.DataFrame,
    opts: dict,
    initial_count: int,
) -> pd.DataFrame:
    """Shared tail: optional drop-empty-tl and dedup, with log parity."""
    if opts.get("drop_empty_tl", False) and "tl" in df.columns:
        before = len(df)
        df = df.dropna(subset=["tl"])
        df = df[df["tl"].astype(str).str.strip() != ""]
        dropped = before - len(df)
        if dropped:
            ctx.logger.info(f"Dropped {dropped} rows with empty tl")

    if opts.get("dedup", False) and {"hanzi", "tl"}.issubset(df.columns):
        before = len(df)
        df = df.drop_duplicates(subset=["hanzi", "tl"], keep="first")
        removed = before - len(df)
        ctx.logger.info(f"Extracted: {initial_count} records")
        if removed:
            ctx.logger.info(f"Duplicates removed: {removed}")
        ctx.logger.info(f"Final: {len(df)} records")
    return df.reset_index(drop=True)

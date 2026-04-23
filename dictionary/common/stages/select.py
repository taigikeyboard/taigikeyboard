"""Select stage — kautian multi-sheet column selection + dialect melt.

For each sheet listed in `select.sheets`, pick the declared columns and
rename them to the canonical (`hanzi`, `tl`) schema. For the sheet named in
`select.dialect_sheet`, do a wide-to-long melt across `select.dialect_columns`
so each dialect reading becomes its own row.

The 異用字 sheet is intentionally preserved through select → expand and then
excluded at `merge` (its contents drive the `variants` stage separately via
`supplementary/variants/data/variants.csv`).
"""

from __future__ import annotations

import pandas as pd

from common.stages import csv_roundtrip
from pipeline.context import PipelineContext

_DEFAULT_COLUMN_RENAME: dict[str, str] = {
    "漢字": "hanzi",
    "羅馬字": "tl",
    "異用字": "hanzi",
}


def run(ctx: PipelineContext) -> None:
    opts = ctx.get_stage_options("select")
    sheet_specs: dict[str, dict] = opts.get("sheets") or {}
    dialect_sheet: str | None = opts.get("dialect_sheet")
    dialect_columns: list[str] = opts.get("dialect_columns") or []
    column_rename: dict[str, str] = {**_DEFAULT_COLUMN_RENAME, **(opts.get("column_rename") or {})}

    sheets = ctx.current_sheets()
    out_sheets: dict[str, pd.DataFrame] = {}
    total_rows = 0

    for sheet_name, spec in sheet_specs.items():
        if sheet_name not in sheets:
            raise ValueError(
                f"{ctx.source_name}: select.sheets references {sheet_name!r} "
                f"not in loaded sheets {list(sheets)}"
            )
        columns = spec["columns"]
        df = sheets[sheet_name][columns].copy()
        rename_map = {c: column_rename[c] for c in columns if c in column_rename}
        df = df.rename(columns=rename_map)
        out_sheets[sheet_name] = df
        total_rows += len(df)
        ctx.logger.info(
            f"  {sheet_name}: {len(df)} rows -> [{', '.join(df.columns.astype(str))}]"
        )

    if dialect_sheet:
        if dialect_sheet not in sheets:
            raise ValueError(
                f"{ctx.source_name}: select.dialect_sheet={dialect_sheet!r} not in "
                f"loaded sheets"
            )
        df = sheets[dialect_sheet]
        df_long = df.melt(
            id_vars=["漢字"],
            value_vars=dialect_columns,
            var_name="腔調",
            value_name="羅馬字",
        )
        df_long = df_long.dropna(subset=["羅馬字"])
        df_long = df_long[df_long["羅馬字"].str.strip() != ""]
        df_long = df_long[["漢字", "羅馬字"]].rename(columns=column_rename)
        out_sheets[dialect_sheet] = df_long
        total_rows += len(df_long)
        ctx.logger.info(
            f"  {dialect_sheet}: {len(df_long)} rows -> [{', '.join(df_long.columns)}]"
        )


    # Round-trip through CSV so clashing column names (kautian 異用字 renames
    # both 漢字 and 異用字 to "hanzi"; pd.read_csv disambiguates the second to
    # "hanzi.1") and dtypes get the same inference downstream stages expect.
    out_sheets = {name: csv_roundtrip(df) for name, df in out_sheets.items()}
    ctx.set_sheets(out_sheets)
    ctx.logger.info(f"Total: {len(out_sheets)} files, {total_rows} rows")

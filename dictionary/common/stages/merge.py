"""Merge stage — kautian multi-sheet concat → dedup → sort → single DataFrame.

Kautian-only: `select` + `expand` leave `ctx._sheets` populated with one
DataFrame per source sheet. This stage concatenates them into a single
DataFrame and drops the multi-sheet carrier.

Deterministic row ordering is load-bearing (output binaries hash over it):
- Sheets iterated in `sorted(sheets.keys())` order, excluding any name in
  `merge.exclude`.
- Post-concat, `drop_duplicates(subset=dedup_subset, keep=dedup_keep)`
  with configurable subset (default `[hanzi, tl]`).
- Final sort via `sort_values(sort_by, kind="mergesort")` for stability.
"""

from __future__ import annotations

from collections import defaultdict

import pandas as pd

from pipeline.context import PipelineContext


def run(ctx: PipelineContext) -> None:
    opts = ctx.get_stage_options("merge")
    exclude = set(opts.get("exclude") or [])
    dedup_subset = opts.get("dedup_subset") or ["hanzi", "tl"]
    dedup_keep = opts.get("dedup_keep", "first")
    sort_by = opts.get("sort_by") or ["hanzi", "tl"]
    sort_kind = opts.get("sort_kind", "mergesort")

    sheets = ctx.current_sheets()

    # Sort sheet names so final concat order is insertion-order-independent.
    ordered = [name for name in sorted(sheets.keys()) if name not in exclude]
    skipped = [name for name in sorted(sheets.keys()) if name in exclude]
    for name in skipped:
        ctx.logger.info(f"  [skip] {name}")

    all_dfs = []
    all_records = []
    total_rows = 0
    for name in ordered:
        df = sheets[name]
        all_dfs.append(df)
        total_rows += len(df)
        for _, row in df.iterrows():
            all_records.append({"hanzi": row["hanzi"], "tl": row["tl"], "source": name})
        ctx.logger.info(f"  [load] {name}: {len(df)} rows")

    ctx.logger.info(f"Total loaded: {total_rows} rows")

    _log_duplicates(ctx, all_records)

    merged = pd.concat(all_dfs, ignore_index=True)
    before = len(merged)
    merged = merged.drop_duplicates(subset=dedup_subset, keep=dedup_keep)
    removed = before - len(merged)
    ctx.logger.info(f"After dedup: {len(merged)} rows (-{removed} duplicates)")

    merged = merged.sort_values(sort_by, ignore_index=True, kind=sort_kind)

    ctx.set_df(merged)
    ctx.clear_sheets()


def _log_duplicates(ctx: PipelineContext, records: list[dict]) -> None:
    seen = defaultdict(list)
    for record in records:
        seen[(record["hanzi"], record["tl"])].append(record["source"])
    duplicates = {k: v for k, v in seen.items() if len(v) > 1}
    if not duplicates:
        return
    by_sources = defaultdict(list)
    for (hanzi, tl), sources in duplicates.items():
        by_sources[" + ".join(sorted(set(sources)))].append((hanzi, tl, sources))
    for source_key in sorted(by_sources.keys()):
        items = by_sources[source_key]
        ctx.logger.info(f"\n  [duplicates] {source_key} ({len(items)} records):")
        for hanzi, tl, _sources in sorted(items)[:30]:
            ctx.logger.info(f"    {hanzi}: {tl}")
        if len(items) > 30:
            ctx.logger.info(f"    ... and {len(items) - 30} more")

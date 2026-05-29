#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
合併多個詞庫為單一 CSV

輸入（依 INPUT_FILES 合併順序列出）：
- sources/official/kautian/data/kautian.csv
- sources/official/taigitv/data/taigitv.csv
- sources/community/itaigi/data/itaigi.csv
- sources/community/sitbut/data/sitbut.csv
- sources/community/taihoa/data/taihoa.csv
- sources/community/taijit/data/taijit.csv
- sources/official/kungge/data/kungge.csv
- sources/official/stti/data/stti.csv
- sources/supplementary/khpoo/data/khpoo.csv
- Khiin 詞頻資料（補充不在其他詞庫中的詞條）

輸出：
- output/dictionary.csv
"""

import csv
import json
from collections import Counter
from pathlib import Path

import pandas as pd

from build.common import BASE_DIR, LOG_DIR
from common.abbrev import extract_abbrev, extract_tps_abbrev
from common.frequency import load_frequency_map, get_frequency
from common.logging_utils import setup_logging, log_header
from common.notone import apply_or_dialect_variant, remove_tone, remove_tps_tone
from common.romanization import to_numeric_tone
from common.source_bits import MAIN_SOURCE_COLUMNS
from common.taigi_bridge import (
    BridgeDeadError,
    TpsResidueError,
    convert_poj_to_tl_strict,
    convert_tl_to_poj_strict,
    convert_tl_to_tps_strict,
)
from common import read_dictionary_csv

INPUT_FILES = [
    ("sources/official/kautian/data/kautian.csv", "kautian"),
    ("sources/official/taigitv/data/taigitv.csv", "taigitv"),
    ("sources/community/itaigi/data/itaigi.csv", "itaigi"),
    ("sources/community/sitbut/data/sitbut.csv", "sitbut"),
    ("sources/community/taihoa/data/taihoa.csv", "taihoa"),
    ("sources/community/taijit/data/taijit.csv", "taijit"),
    ("sources/official/kungge/data/kungge.csv", "kungge"),
    ("sources/official/stti/data/stti.csv", "stti"),
    ("sources/supplementary/khpoo/data/khpoo.csv", "khpoo"),
]
OUTPUT_DIR_NAME = "output"
OUTPUT_FILE = "dictionary.csv"
SCRIPT_NAME = "merge_csv"

SOURCE_COLUMNS = MAIN_SOURCE_COLUMNS


def main():
    logger = setup_logging(SCRIPT_NAME, log_dir=LOG_DIR)
    output_dir = BASE_DIR / OUTPUT_DIR_NAME
    output_path = output_dir / OUTPUT_FILE
    log_header(logger, SCRIPT_NAME, ", ".join(f[0] for f in INPUT_FILES), output_path)

    output_dir.mkdir(parents=True, exist_ok=True)

    all_dfs = []

    for filepath, _source_col in INPUT_FILES:
        full_path = BASE_DIR / filepath
        if not full_path.exists():
            logger.warning(f"  [skip] {filepath} not found")
            continue

        df = read_dictionary_csv(full_path)
        logger.info(f"  [load] {filepath}: {len(df)} records")
        all_dfs.append(df)

    if not all_dfs:
        logger.error("No input files found!")
        return

    # Build-drop stats consumed by build/version_snapshot.py for the build
    # summary. `filter_drops` accumulates entries discarded by supplement
    # filters (e.g. khiin >4-syllable cap / romanization failures).
    drop_stats: dict = {"filter_drops": Counter()}

    # 合併
    merged_df = pd.concat(all_dfs, ignore_index=True)
    drop_stats["raw_rows"] = len(merged_df)
    logger.info(f"\n  Total before merge: {len(merged_df)} records")

    # 建立正規化 key（空白→連字符）用於跨辭典去重
    # 官方辭典 tl 可能含空白（如 "m̄ bat"），非官方辭典為連字符（如 "m̄-bat"）
    # 去重時視為同一筆，但保留官方版本（含空白）的 tl
    OFFICIAL_SOURCES = ["kautian", "taigitv", "kungge"]
    merged_df["_tl_key"] = merged_df["tl"].str.replace(" ", "-", regex=False)
    merged_df["_is_official"] = merged_df[OFFICIAL_SOURCES].any(axis=1)

    # 排序：官方辭典排在前面，確保 groupby 的 "first" 取到官方版本
    merged_df = merged_df.sort_values("_is_official", ascending=False, ignore_index=True)

    # 去重複：相同 (hanzi, _tl_key) 合併來源欄位
    # dropna=False: 確保 groupby 不會自動排除含 NaN 的列
    agg_dict = {}
    for col in merged_df.columns:
        if col in ["hanzi", "_tl_key"]:
            continue  # groupby key
        elif col in ["_is_official"]:
            continue  # temp column
        elif col in SOURCE_COLUMNS:
            agg_dict[col] = "any"
        elif col == "frequency":
            agg_dict[col] = "max"
        else:
            agg_dict[col] = "first"
    result_df = merged_df.groupby(["hanzi", "_tl_key"], as_index=False, dropna=False).agg(agg_dict)

    # 移除臨時欄位
    result_df = result_df.drop(columns=["_tl_key"])

    # 排序：依 frequency 降序
    result_df = result_df.sort_values(
        ["frequency", "hanzi", "tl"],
        ascending=[False, True, True],
        ignore_index=True
    )

    drop_stats["after_dedup"] = len(result_df)
    logger.info(f"  After dedup: {len(result_df)} records")

    # 補入 Khiin 獨有的詞條（不屬於任何辭典來源）
    khiin_new = _load_khiin_new_entries(result_df, BASE_DIR, logger, drop_stats["filter_drops"])
    drop_stats["khiin_added"] = len(khiin_new) if khiin_new is not None else 0
    if khiin_new is not None and len(khiin_new) > 0:
        result_df = pd.concat([result_df, khiin_new], ignore_index=True)
        # Re-sort
        result_df = result_df.sort_values(
            ["frequency", "hanzi", "tl"],
            ascending=[False, True, True],
            ignore_index=True
        )
        logger.info(f"  After Khiin supplement: {len(result_df)} records")

    # Ensure dev column exists before dev supplement (needed for marking existing entries)
    if "dev" not in result_df.columns:
        result_df["dev"] = False

    # 補入開發者補充辭典
    dev_new = _load_dev_supplement(result_df, BASE_DIR, logger, drop_stats["filter_drops"])
    drop_stats["dev_added"] = len(dev_new) if dev_new is not None else 0
    if dev_new is not None and len(dev_new) > 0:
        result_df = pd.concat([result_df, dev_new], ignore_index=True)
        result_df = result_df.sort_values(
            ["frequency", "hanzi", "tl"],
            ascending=[False, True, True],
            ignore_index=True
        )
        logger.info(f"  After dev supplement: {len(result_df)} records")

    # 補入 LKK 漢羅合用建議用字
    if "lkk" not in result_df.columns:
        result_df["lkk"] = False
    lkk_new = _load_lkk_entries(result_df, BASE_DIR, logger, drop_stats["filter_drops"])
    drop_stats["lkk_added"] = len(lkk_new) if lkk_new is not None else 0
    if lkk_new is not None and len(lkk_new) > 0:
        result_df = pd.concat([result_df, lkk_new], ignore_index=True)
        result_df = result_df.sort_values(
            ["frequency", "hanzi", "tl"],
            ascending=[False, True, True],
            ignore_index=True
        )
        logger.info(f"  After LKK supplement: {len(result_df)} records")

    # `khiin` flags rows that came in via _load_khiin_new_entries and are not
    # tagged by any named source (or by dev/lkk supplements).
    result_df["khiin"] = ~(result_df[SOURCE_COLUMNS].any(axis=1) | result_df["dev"] | result_df["lkk"])

    # 統計來源
    logger.info(f"\n  [source statistics]")
    for col in SOURCE_COLUMNS:
        total = result_df[col].sum()
        # 計算該來源獨有的數量
        other_cols = [c for c in SOURCE_COLUMNS if c != col]
        only_mask = result_df[col]
        for other in other_cols:
            only_mask = only_mask & ~result_df[other]
        only_count = only_mask.sum()
        logger.info(f"    {col}: {total} (only: {only_count})")

    # 儲存
    result_df.to_csv(output_path, index=False)

    # Persist drop stats for build/version_snapshot.py's build summary.
    drop_stats["final"] = len(result_df)
    stats_path = output_dir / ".build_stats.json"
    stats_path.write_text(json.dumps(drop_stats, ensure_ascii=False, indent=2))
    logger.info(f"  Wrote drop stats: {stats_path}")

    logger.info(f"\n  [sample records]:")
    for _, row in result_df.head(10).iterrows():
        sources = [col for col in SOURCE_COLUMNS if row[col]]
        logger.info(f"    {row['hanzi']}: {row['tl']} ({row['frequency']}) [{', '.join(sources)}]")

    logger.info(f"\nSaved: {output_path}")


def _load_char_freq_map(base_dir: Path) -> dict[tuple[str, str], int]:
    """Load the per-character frequency map used by all supplement loaders."""
    return load_frequency_map(base_dir / "shared" / "data" / "char_freq_merged.txt")


def _build_existing_key_index(existing_df: pd.DataFrame) -> dict[tuple[str, str], int]:
    """Map `(hanzi, tl_normalised)` → row index of `existing_df`.

    `tl` is normalised by lowercasing + replacing spaces with hyphens, so
    official-dict entries (e.g. "m̄ bat") compare equal to supplement rows
    that spell them with hyphens ("m̄-bat").
    """
    index: dict[tuple[str, str], int] = {}
    for idx, row in existing_df.iterrows():
        tl_key = str(row["tl"]).lower().replace(" ", "-")
        index[(str(row["hanzi"]), tl_key)] = idx
    return index


def _assemble_supplement_row(
    *,
    hanzi: str,
    tl: str,
    poj: str,
    frequency: int,
    is_variant: bool = False,
    source_flags: dict[str, bool] | None = None,
) -> dict:
    """Build one supplement row with all derived romanization columns.

    Callers resolve the TL↔POJ direction themselves (khiin starts from POJ,
    dev/lkk start from TL) and pass both strings in normalised form (lowercase,
    hyphens). This helper derives `tl_num` / `poj_num` / `tps_num`,
    `tl_notone` / `poj_notone` / `tps_notone`, `tl_abbrev` / `poj_abbrev` /
    `tps_abbrev` and packs everything into a row dict together with the 9
    `MAIN_SOURCE_COLUMNS` flags (all False by default). Extra flags like
    `dev` / `lkk` come in via `source_flags`.
    """
    tl_num = to_numeric_tone(tl)
    poj_num = to_numeric_tone(poj, ascii_only=True)
    # `taigi-converter` joins per-syllable TPS with ASCII space; FST keys
    # and user input use the fused form, so strip whitespace here. A
    # TL row carrying a dialectal char without a Bopomofo equivalent
    # (e.g. `sṳ`) raises `TpsResidueError` in the strict bridge — fall
    # back to empty TPS columns so the supplement row still lands in
    # the CSV (the tl: / poj: indices stay intact even when tps:
    # cannot be derived). `BridgeDeadError` (Node subprocess death)
    # propagates so a dead bridge aborts the build (Codex PR #334
    # review).
    if tl:
        try:
            tps_num = convert_tl_to_tps_strict(tl).replace(" ", "")
        except TpsResidueError:
            tps_num = ""
    else:
        tps_num = ""

    tl_syllables = [s for s in tl.replace(" ", "-").split("-") if s] if tl else []
    if len(tl_syllables) >= 2:
        try:
            tps_per_syllable = [convert_tl_to_tps_strict(s) for s in tl_syllables]
        except TpsResidueError:
            tps_per_syllable = []
        tps_abbrev = extract_tps_abbrev(tl, tps_per_syllable) if tps_per_syllable else ""
    else:
        tps_abbrev = ""

    flags = {col: False for col in SOURCE_COLUMNS}
    if source_flags:
        flags.update(source_flags)

    tps_notone = remove_tps_tone(tps_num)
    return {
        "tl": tl,
        "hanzi": hanzi,
        "frequency": frequency,
        "poj": poj,
        "tl_num": tl_num,
        "poj_num": poj_num,
        "tps_num": tps_num,
        "tl_notone": remove_tone(tl_num),
        "poj_notone": remove_tone(poj_num),
        "tps_notone": tps_notone,
        "tl_abbrev": extract_abbrev(tl),
        "poj_abbrev": extract_abbrev(poj),
        "tps_abbrev": tps_abbrev,
        # C-3a er↔or dialect dual-emit variants. Same shared helper as
        # the stage pipeline so supplement rows (khiin / dev / lkk) get
        # identical coverage.
        "tps_num_var": apply_or_dialect_variant(tps_num),
        "tps_notone_var": apply_or_dialect_variant(tps_notone),
        "tps_abbrev_var": apply_or_dialect_variant(tps_abbrev),
        "is_variant": is_variant,
        **flags,
    }


def _load_khiin_new_entries(
    existing_df: pd.DataFrame, base_dir: Path, logger, filter_drops: Counter
) -> pd.DataFrame | None:
    """Load Khiin entries not already present in the merged dictionary.

    Khiin rows are anonymous contributors — none of the 9 main source bits
    (and no dev/lkk) is set on them, so they stay invisible in the app's
    source-toggle UI while still seeding vocabulary the user can type.

    `filter_drops` accumulates the count of entries discarded by a filter
    (>4 syllables) or a romanization failure, so the build drop summary is
    complete rather than silent.
    """
    freq_path = base_dir / "shared" / "data" / "khiin_frequency.csv"
    conv_path = base_dir / "shared" / "data" / "khiin_conversions.csv"

    if not freq_path.exists() or not conv_path.exists():
        logger.warning("  [skip] Khiin frequency files not found")
        return None

    freq: dict[str, int] = {}
    with freq_path.open() as f:
        for row in csv.DictReader(f):
            freq[row["input"]] = int(row["freq"])

    # Load all hanzi variants per input
    all_hanzi = {}
    with conv_path.open() as f:
        for row in csv.DictReader(f):
            inp = row["input"]
            output = row["output"]
            if any(
                "\u4e00" <= c <= "\u9fff"
                or "\u3400" <= c <= "\u4dbf"
                or ord(c) > 0x20000
                for c in output
            ):
                all_hanzi.setdefault(inp, set()).add(output)

    # Use a (hanzi, tl) set rather than the shared index-building helper —
    # khiin dedup only needs membership, never the row position.
    existing_keys = set(_build_existing_key_index(existing_df).keys())
    freq_map = _load_char_freq_map(base_dir)

    new_rows = []
    for inp, _f in freq.items():
        hanzi_set = all_hanzi.get(inp)
        if not hanzi_set:
            continue

        try:
            khiin_poj = inp.lower().replace(" ", "-")
            tl = convert_poj_to_tl_strict(khiin_poj).lower().replace(" ", "-")
            # Re-derive POJ from TL via the canonical converter so the
            # `poj == convert_tl_to_poj(tl)` invariant (enforced by
            # build/verify_poj_integrity.py) holds for khiin entries too.
            # Without this, khiin's idiosyncratic POJ encodings (e.g.,
            # `hoonn` → `hò͘ⁿ` instead of standard `hòⁿ`) would trip the gate.
            poj = convert_tl_to_poj_strict(tl).lower().replace(" ", "-")
        except BridgeDeadError:
            # Node subprocess died mid-IPC — fail loud, do NOT
            # continue iterating with a dead bridge (Codex PR #334
            # review).
            raise
        except Exception:
            filter_drops["khiin_romanization_fail"] += 1
            continue

        # Skip entries exceeding 4 syllables
        if len(tl.split("-")) > 4:
            filter_drops["khiin_over_4syl"] += 1
            continue

        for hanzi in hanzi_set:
            if (hanzi, tl) in existing_keys:
                continue

            try:
                new_rows.append(_assemble_supplement_row(
                    hanzi=hanzi,
                    tl=tl,
                    poj=poj,
                    frequency=get_frequency(hanzi, tl, freq_map),
                ))
            except BridgeDeadError:
                # _assemble_supplement_row also invokes the bridge
                # (for tps_num + per-syllable tps_abbrev) — bridge
                # death there must propagate too.
                raise
            except Exception:
                filter_drops["khiin_romanization_fail"] += 1
                continue
            existing_keys.add((hanzi, tl))

    if new_rows:
        logger.info(f"  [khiin] Added {len(new_rows)} new entries from Khiin")

    return pd.DataFrame(new_rows) if new_rows else None


def _load_dev_supplement(
    existing_df: pd.DataFrame, base_dir: Path, logger, filter_drops: Counter
) -> pd.DataFrame | None:
    """
    Load developer supplement dictionary entries.

    Reads a minimal CSV (hanzi, tl) and auto-generates all romanization columns.
    No cleanup is applied — trusts user input.
    If a (hanzi, tl) pair already exists, marks it dev=True (always included).
    New pairs are added with dev=True and all source columns set to False.
    """
    dev_path = base_dir / "supplementary" / "dev" / "data" / "dev.csv"

    if not dev_path.exists():
        logger.warning("  [skip] dev supplement CSV not found")
        return None

    dev_df = read_dictionary_csv(dev_path)
    if dev_df.empty:
        logger.info("  [dev] dev.csv is empty, skipping")
        return None

    existing_key_index = _build_existing_key_index(existing_df)
    freq_map = _load_char_freq_map(base_dir)

    marked_count = 0
    new_rows = []
    for _, row in dev_df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        tl_raw = str(row["tl"]).strip().lower()

        if not hanzi or not tl_raw:
            continue

        tl_key = tl_raw.replace(" ", "-")
        if (hanzi, tl_key) in existing_key_index:
            existing_df.at[existing_key_index[(hanzi, tl_key)], "dev"] = True
            logger.info(f"  [dev] marked existing: {hanzi} / {tl_raw}")
            marked_count += 1
            continue

        try:
            tl = tl_key
            poj = convert_tl_to_poj_strict(tl).lower().replace(" ", "-")
        except BridgeDeadError:
            # Node subprocess died mid-IPC — fail loud (Codex PR #334
            # review).
            raise
        except Exception as e:
            logger.warning(f"  [dev] romanization failed for {hanzi}/{tl_raw}: {e}")
            filter_drops["dev_romanization_fail"] += 1
            continue

        # user-provided frequency takes precedence over char-frequency derivation
        frequency_override = (
            row.get("frequency")
            if "frequency" in dev_df.columns and pd.notna(row.get("frequency"))
            else None
        )
        frequency = int(frequency_override) if frequency_override is not None else get_frequency(hanzi, tl, freq_map)

        is_variant = row.get("is_variant", False) if "is_variant" in dev_df.columns else False

        new_rows.append(_assemble_supplement_row(
            hanzi=hanzi,
            tl=tl,
            poj=poj,
            frequency=frequency,
            is_variant=is_variant,
            source_flags={"dev": True},
        ))
        existing_key_index[(hanzi, tl_key)] = -1  # sentinel: already added as new

    if marked_count:
        logger.info(f"  [dev] Marked {marked_count} existing entries as dev")
    if new_rows:
        logger.info(f"  [dev] Added {len(new_rows)} new entries from dev supplement")

    return pd.DataFrame(new_rows) if new_rows else None


def _load_lkk_entries(
    existing_df: pd.DataFrame, base_dir: Path, logger, filter_drops: Counter
) -> pd.DataFrame | None:
    """
    Load LKK 漢羅合用建議用字 dictionary entries.

    Reads a minimal CSV (hanzi, tl) and auto-generates all romanization columns.
    If a (hanzi, tl) pair already exists, marks it lkk=True.
    New pairs are added with lkk=True and all other source columns set to False.
    """
    lkk_path = base_dir / "supplementary" / "lkk" / "data" / "lkk.csv"

    if not lkk_path.exists():
        logger.warning("  [skip] LKK CSV not found")
        return None

    lkk_df = read_dictionary_csv(lkk_path)
    if lkk_df.empty:
        logger.info("  [lkk] lkk.csv is empty, skipping")
        return None

    existing_key_index = _build_existing_key_index(existing_df)
    freq_map = _load_char_freq_map(base_dir)

    marked_count = 0
    new_rows = []
    for _, row in lkk_df.iterrows():
        hanzi = str(row["hanzi"]).strip()
        tl_raw = str(row["tl"]).strip().lower()

        if not hanzi or not tl_raw:
            continue

        tl_key = tl_raw.replace(" ", "-")
        if (hanzi, tl_key) in existing_key_index:
            idx = existing_key_index[(hanzi, tl_key)]
            if idx >= 0:
                existing_df.at[idx, "lkk"] = True
                logger.info(f"  [lkk] marked existing: {hanzi} / {tl_raw}")
                marked_count += 1
            else:
                logger.info(f"  [lkk] skip duplicate: {hanzi} / {tl_raw}")
            continue

        try:
            tl = tl_key
            poj = convert_tl_to_poj_strict(tl).lower().replace(" ", "-")
        except BridgeDeadError:
            # Node subprocess died mid-IPC — fail loud (Codex PR #334
            # review).
            raise
        except Exception as e:
            logger.warning(f"  [lkk] romanization failed for {hanzi}/{tl_raw}: {e}")
            filter_drops["lkk_romanization_fail"] += 1
            continue

        new_rows.append(_assemble_supplement_row(
            hanzi=hanzi,
            tl=tl,
            poj=poj,
            frequency=get_frequency(hanzi, tl, freq_map),
            source_flags={"dev": False, "lkk": True},
        ))
        existing_key_index[(hanzi, tl_key)] = -1  # sentinel: already added as new

    if marked_count:
        logger.info(f"  [lkk] Marked {marked_count} existing entries as lkk")
    if new_rows:
        logger.info(f"  [lkk] Added {len(new_rows)} new entries from LKK")

    return pd.DataFrame(new_rows) if new_rows else None


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
從 dictionary.csv 建立 fst 前綴索引

輸入：output/dictionary.csv
輸出：output/dictionary.fst

Key 格式（前綴式）：
- tl:<tl_num>：TL 數字聲調（如 tl:hoo2se3，數字當 syllable separator）
- tl:<tl_notone>：TL 去調 fused（如 tl:hoose，連 hyphen 也已脫掉）
- poj:<poj_num>：POJ 數字聲調（如 poj:ho2se3）
- poj:<poj_notone>：POJ 去調 fused（如 poj:hoose）
- tl:<tl_abbrev>：TL 縮寫（如 tl:hs）
- poj:<poj_abbrev>：POJ 縮寫（如 poj:hs）
- tps:<tps_num>：TPS Bopomofo + 聲調符號（如 tps:ㄏㆦ˫ㄙㆤ˪）
- tps:<tps_notone>：TPS 去調 fused（如 tps:ㄏㆦㄙㆤ）
- tps:<tps_abbrev>：TPS 縮寫（如 tps:ㄏㄙ）
- tps:<tps_num_var>     C-3a er↔or 方言 always-on：TL `er`/`or` 兩種注音字形（ㄜ vs ㄛ）
- tps:<tps_notone_var>  同上,僅當源 `tps_*` 含 ㄜ 時 emit;A always-on,取代 runtime
- tps:<tps_abbrev_var>  `tps_or_mapped_to_er` toggle (per PR C-3a)。
- hanzi:<hanzi>：漢字前綴搜尋（如 hanzi:好無）

Fused-toneless invariant — `tl_notone` / `poj_notone` are produced
upstream by `dictionary/common/notone.py::remove_tone()` which strips
BOTH digits and hyphens from `tl_num` / `poj_num`. As a result the FST
already contains a fused toneless key for every multi-syllable entry,
so toneless Roman input like `tsua` retrieves both single-syllable
(`紙`) and multi-syllable (`珠仔`) candidates without any builder-side
derivation rule. v3.5.8 Phase 1b verified this on 2026-05-10 and was
marked N/A — see `docs/roadmap.md` §Phase 1b. Engine-side contract is
pinned by `engine/lexicon/tests/fused_toneless_key.rs`.

Wire format (per docs/engine/lexicon-slice-plan.md §2.2):
    key_bytes (UTF-8) || 0xFF || rowid_le_4

實際 fst 建置由 Rust binary `engine/build-helpers/fst-builder` 完成；
本 Python 腳本只負責讀取 dictionary.csv、組 stdin pairs、shell out 到 Rust。

Romanization-key filter mirrors the original create_trie_db.sh JOIN +
syllable cap on `tl_num` (Codex pre-impl review Q6): emit a romanization
key only when `tl_num` is non-empty AND `tl_num` syllable count <= 4.
Hanzi keys come from dictionary records with non-NULL hanzi.
"""

import shutil
import subprocess
import sys
from pathlib import Path

from build.common import LOG_DIR, OUTPUT_DIR
from build.dictionary_records import load_dictionary_records
from common.logging_utils import log_header, setup_logging

CSV_FILE = OUTPUT_DIR / "dictionary.csv"
OUTPUT_FILE = OUTPUT_DIR / "dictionary.fst"
SCRIPT_NAME = "create_fst"

ENGINE_DIR = Path(__file__).resolve().parents[2] / "engine"
BUILDER_RELEASE = ENGINE_DIR / "target" / "release" / "fst-builder"
BUILDER_DEBUG = ENGINE_DIR / "target" / "debug" / "fst-builder"

MAX_SYLLABLES_TL_NUM = 4


def resolve_builder_bin() -> Path:
    """Locate the fst-builder Rust binary; build it if missing."""
    if BUILDER_RELEASE.exists():
        return BUILDER_RELEASE
    if BUILDER_DEBUG.exists():
        return BUILDER_DEBUG
    cargo = shutil.which("cargo")
    if not cargo:
        raise RuntimeError(
            "fst-builder binary missing and `cargo` not in PATH; build it manually "
            f"with `cargo build --release -p fst-builder` from {ENGINE_DIR}"
        )
    subprocess.run(
        [cargo, "build", "--release", "-p", "fst-builder"],
        cwd=ENGINE_DIR,
        check=True,
    )
    if not BUILDER_RELEASE.exists():
        raise RuntimeError(
            f"cargo build succeeded but {BUILDER_RELEASE} still missing"
        )
    return BUILDER_RELEASE


def _tl_num_syllable_count(tl_num: str) -> int:
    """Count syllables in tl_num via hyphen-count + 1.

    Note: in current production data `tl_num` is always digit-separated
    (e.g. `hoo2se3`, `tsu1a2`) — the upstream `numtone` stage has already
    folded hyphens into tone digits — so this helper effectively returns
    1 for every record, and the `<= MAX_SYLLABLES_TL_NUM` gate at the
    call site is a redundant safety net behind the harder upstream filter
    in `dictionary_records.py::load_dictionary_records` (`syllable_count
    > MAX_SYLLABLES` rows are dropped before they ever reach here).
    """
    return tl_num.count("-") + 1


def collect_pairs(logger) -> list[tuple[str, int]]:
    """Yield (key, rowid) pairs from dictionary records."""
    if not CSV_FILE.exists():
        raise RuntimeError(f"CSV not found: {CSV_FILE}")

    records = load_dictionary_records(CSV_FILE)
    logger.info(f"Loaded {len(records)} records from {CSV_FILE.name}")

    pairs: list[tuple[str, int]] = []
    seen: set[tuple[str, int]] = set()

    def add(key: str, rowid: int) -> None:
        pair = (key, rowid)
        if pair not in seen:
            seen.add(pair)
            pairs.append(pair)

    for record in records:
        rowid = record.rowid
        # Romanization-key filter (mirrors original trie.db JOIN: tl_num
        # non-empty + syllables <= 4). All three TL variants share the
        # tl_num gate; same for POJ; TPS gated by the same TL syllable
        # count (TPS forms are derived from the same TL source row).
        if record.tl_num and _tl_num_syllable_count(record.tl_num) <= MAX_SYLLABLES_TL_NUM:
            for val in (record.tl_num, record.tl_notone, record.tl_abbrev):
                if val:
                    add(f"tl:{val}", rowid)
            # C-3a er↔or dual-emit: same rowid keyed by both ㄜ-form
            # (bridge default) and ㄛ-form (toggle-OFF variant) so a
            # TPS user typing either glyph hits the same dictionary
            # row. Empty `*_var` columns short-circuit.
            for val in (
                record.tps_num, record.tps_notone, record.tps_abbrev,
                record.tps_num_var, record.tps_notone_var, record.tps_abbrev_var,
            ):
                if val:
                    add(f"tps:{val}", rowid)
        if record.poj_num and _tl_num_syllable_count(record.poj_num) <= MAX_SYLLABLES_TL_NUM:
            for val in (record.poj_num, record.poj_notone, record.poj_abbrev):
                if val:
                    add(f"poj:{val}", rowid)

    romanization_count = len(pairs)
    logger.info(f"Romanization pairs: {romanization_count}")

    for record in records:
        if record.hanzi:
            add(f"hanzi:{record.hanzi}", record.rowid)

    hanzi_count = len(pairs) - romanization_count
    if hanzi_count <= 0:
        raise RuntimeError(
            "No hanzi keys generated — dictionary.csv may be missing hanzi data"
        )
    logger.info(f"Hanzi pairs: {hanzi_count}")
    logger.info(f"Total pairs: {len(pairs)}")
    return pairs


def emit_to_builder(pairs: list[tuple[str, int]], builder_bin: Path) -> None:
    """Pipe TL\\tkey\\trowid lines into fst-builder; let it write OUTPUT_FILE."""
    payload_lines = (f"TL\t{key}\t{rowid}" for key, rowid in pairs)
    payload = ("\n".join(payload_lines) + "\n").encode("utf-8")
    proc = subprocess.run(
        [str(builder_bin), "build", str(OUTPUT_FILE)],
        input=payload,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"fst-builder failed (exit {proc.returncode}): "
            f"{proc.stderr.decode('utf-8', errors='replace')}"
        )
    sys.stderr.write(proc.stderr.decode("utf-8", errors="replace"))


def main() -> None:
    logger = setup_logging(SCRIPT_NAME, log_dir=LOG_DIR)
    log_header(logger, SCRIPT_NAME, CSV_FILE, OUTPUT_FILE)

    builder_bin = resolve_builder_bin()
    logger.info(f"fst-builder: {builder_bin}")

    pairs = collect_pairs(logger)
    emit_to_builder(pairs, builder_bin)

    file_size = OUTPUT_FILE.stat().st_size
    logger.info("\n  [output]")
    logger.info(f"    File: {OUTPUT_FILE}")
    logger.info(f"    Size: {file_size / 1024 / 1024:.2f} MB")
    logger.info("\nDone!")


if __name__ == "__main__":
    main()

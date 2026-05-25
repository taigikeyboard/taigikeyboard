#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Build the v3.5.9 D / C-0 tagged-single-FST syllable inventory (`syllables.fst`).

Input:  output/dictionary.csv  (canonical dictionary records)
Output: output/syllables.fst   (fst::Set of TL + POJ + TPS syllables)

Pipeline
--------
1. Load filtered dictionary records via `dictionary_records.load_dictionary_records`.
2. Stage three temp files:
   - non-empty `tl_num` lines (one per record),
   - non-empty `poj_num` lines (one per record),
   - non-empty TPS syllables (one TPS syllable per line, derived by
     splitting each record's TL into per-syllable tokens and
     converting each via the `taigi-converter` bridge — fused
     `tps_num` is NOT used here because tone-1 syllables carry no
     Bopomofo tone mark, so the per-row fused form loses its syllable
     boundaries).
3. Invoke `engine/build-helpers/fst-builder build-syllables <out>
   --tl-input <tl-path> --poj-input <poj-path> --tps-input <tps-path>`
   once. The Rust binary:
     - splits TL / POJ lines on ASCII tone digits 1..=9; TPS lines
       are already one syllable per line and pass through unsplit,
     - canonicalizes TL tokens via `phonetics::canonicalize_syllable`
       (POJ→TL spelling fold + `o͘`→`oo` + `ⁿ`→`nn`) — emitted with
       `tl:` prefix;
     - canonicalizes POJ tokens via `phonetics::canonicalize_poj_syllable`
       (encoding-only fold; POJ ASCII spelling preserved) — emitted
       with `poj:` prefix;
     - canonicalizes TPS tokens via `phonetics::canonicalize_tps_syllable`
       (tone-mark split only — phonotactic validity is upstream from
       the TL CSV) — emitted with `tps:` prefix;
     - validates phonotactically through the shared TL initials × finals
       table; sample-logs invalid syllables to stderr but keeps building;
     - emits BOTH canonical numeric (`tl:tsua7` / `poj:choa7` /
       `tps:ㄗㄨㄚ˫`) AND canonical toneless (`tl:tsua` / `poj:choa` /
       `tps:ㄗㄨㄚ`) keys per family.

Why the tagged-single-FST split (v3.5.9 B-1 → D / C-0)
------------------------------------------------------
~80,000 of 159,000 dictionary rows have `tl_num != poj_num` (POJ-shaped
forms like `chit8`/`tsit8`, `goa2`/`gua2`, `toa7`/`tua7`). Folding the
POJ source rows into the TL inventory (the v3.5.8 behavior) loses the
POJ-side syllable boundaries, so a continuous POJ buffer like `chiah`
cannot be recognised as one valid syllable. Tagging the families into
`tl:` / `poj:` / `tps:` key prefixes in a single FST keeps the storage
shared (the three syllable inventories overlap heavily by prefix tree)
while letting `lexicon::SyllableInventory::contains_in(mode, syllable)`
serve the right family at runtime. Rationale + design alternatives in
`docs/reports/2026-05-20-v359-b-plan.md` §B-1; the TPS family extension
follows the same pattern (v3.5.9 D / C-0,
`memory/project_v359_d_tps_triindex_plan.md`).

Consumers
---------
The output FST is loaded by `engine/lexicon::SyllableInventory::open`
and consumed by the Phase 3 syllabifier (composing crate). This script
is invoked manually as part of the dictionary rebuild flow; it is not
yet wired into a master build orchestrator.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from build.common import LOG_DIR, OUTPUT_DIR
from build.dictionary_records import load_dictionary_records
from common.logging_utils import log_header, setup_logging
from common.taigi_bridge import TpsResidueError, convert_tl_to_tps_strict

CSV_FILE = OUTPUT_DIR / "dictionary.csv"
OUTPUT_FILE = OUTPUT_DIR / "syllables.fst"
SCRIPT_NAME = "create_syllables_fst"

ENGINE_DIR = Path(__file__).resolve().parents[2] / "engine"
BUILDER_RELEASE = ENGINE_DIR / "target" / "release" / "fst-builder"
BUILDER_DEBUG = ENGINE_DIR / "target" / "debug" / "fst-builder"


def resolve_builder_bin() -> Path:
    """Locate the fst-builder Rust binary; build it if missing.

    Mirrors `dictionary/build/create_fst.py::resolve_builder_bin` so both
    scripts share the same fallback behaviour during a clean build.
    """
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


_TL_SPLIT_RE = re.compile(r"[-\s]+")


def collect_family_lines(logger) -> tuple[list[str], list[str], list[str]]:
    """Yield non-empty `tl_num` / `poj_num` rows + per-syllable TPS lines.

    TPS lines are derived per-syllable from each record's `tl`:
    fused `tps_num` cannot be re-split into syllables at this stage
    because tone-1 syllables carry no Bopomofo tone mark (e.g.
    `ㄉㄞㄍㄧ` for `tai1-gi1`). We split the original hyphenated TL,
    convert each TL syllable to TPS via `convert_tl_to_tps_strict`,
    and emit each one as a separate line for the `--tps-input` temp
    file (the `fst-builder` TPS family is the one-syllable-per-line
    ingest path).
    """
    if not CSV_FILE.exists():
        raise RuntimeError(f"CSV not found: {CSV_FILE}")

    records = load_dictionary_records(CSV_FILE)
    logger.info(f"Loaded {len(records)} records from {CSV_FILE.name}")

    tl_lines = [record.tl_num for record in records if record.tl_num]
    poj_lines = [record.poj_num for record in records if record.poj_num]

    tps_seen: set[str] = set()
    tps_lines: list[str] = []
    tps_conversion_failures = 0
    for record in records:
        if not record.tl:
            continue
        syllables = [s for s in _TL_SPLIT_RE.split(record.tl) if s]
        for syllable in syllables:
            try:
                tps = convert_tl_to_tps_strict(syllable)
            except TpsResidueError:
                # Per-row residue (dialectal char etc.) — skip this
                # syllable. `BridgeDeadError` (subprocess death) is a
                # sibling RuntimeError subclass — NOT `TpsResidueError`
                # — so it propagates and aborts build (Codex PR #334
                # review).
                tps_conversion_failures += 1
                continue
            if tps and tps not in tps_seen:
                tps_seen.add(tps)
                tps_lines.append(tps)

    tl_skipped = len(records) - len(tl_lines)
    poj_skipped = len(records) - len(poj_lines)
    logger.info(
        f"tl_num lines emitted: {len(tl_lines)} (skipped {tl_skipped} empty); "
        f"poj_num lines emitted: {len(poj_lines)} (skipped {poj_skipped} empty); "
        f"tps syllable lines emitted: {len(tps_lines)} "
        f"(distinct; conversion failures: {tps_conversion_failures})"
    )
    return tl_lines, poj_lines, tps_lines


def invoke_builder(
    tl_lines: list[str],
    poj_lines: list[str],
    tps_lines: list[str],
    builder_bin: Path,
) -> None:
    """Stage each family to a temp file, invoke `build-syllables` once.

    The builder takes `--tl-input` / `--poj-input` / `--tps-input` so
    the tagged-single-FST is produced in one pass (avoiding fst::Set
    merges after the fact). Temp files are deleted in the `finally`
    block even on builder failure — they hold dictionary source rows,
    no secrets, but staying tidy keeps repeated invocations from
    leaking through `/tmp`.
    """
    tl_path: Path | None = None
    poj_path: Path | None = None
    tps_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", suffix=".tl-num.txt", delete=False
        ) as tl_f:
            tl_f.write("\n".join(tl_lines))
            tl_f.write("\n")
            tl_path = Path(tl_f.name)
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", suffix=".poj-num.txt", delete=False
        ) as poj_f:
            poj_f.write("\n".join(poj_lines))
            poj_f.write("\n")
            poj_path = Path(poj_f.name)
        with tempfile.NamedTemporaryFile(
            mode="w", encoding="utf-8", suffix=".tps-syllables.txt", delete=False
        ) as tps_f:
            tps_f.write("\n".join(tps_lines))
            tps_f.write("\n")
            tps_path = Path(tps_f.name)

        proc = subprocess.run(
            [
                str(builder_bin),
                "build-syllables",
                str(OUTPUT_FILE),
                "--tl-input",
                str(tl_path),
                "--poj-input",
                str(poj_path),
                "--tps-input",
                str(tps_path),
            ],
            capture_output=True,
            check=False,
        )
        if proc.returncode != 0:
            raise RuntimeError(
                f"fst-builder build-syllables failed (exit {proc.returncode}): "
                f"{proc.stderr.decode('utf-8', errors='replace')}"
            )
        sys.stderr.write(proc.stderr.decode("utf-8", errors="replace"))
    finally:
        for staged in (tl_path, poj_path, tps_path):
            if staged is not None and staged.exists():
                staged.unlink()


def main() -> None:
    logger = setup_logging(SCRIPT_NAME, log_dir=LOG_DIR)
    log_header(logger, SCRIPT_NAME, CSV_FILE, OUTPUT_FILE)

    builder_bin = resolve_builder_bin()
    logger.info(f"fst-builder: {builder_bin}")

    tl_lines, poj_lines, tps_lines = collect_family_lines(logger)
    invoke_builder(tl_lines, poj_lines, tps_lines, builder_bin)

    file_size = OUTPUT_FILE.stat().st_size
    logger.info("\n  [output]")
    logger.info(f"    File: {OUTPUT_FILE}")
    logger.info(f"    Size: {file_size / 1024:.2f} KB")
    logger.info("\nDone!")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Build the v3.5.9 B-1 tagged-single-FST syllable inventory (`syllables.fst`).

Input:  output/dictionary.csv  (canonical dictionary records)
Output: output/syllables.fst   (~tens of KB fst::Set of TL + POJ syllables)

Pipeline
--------
1. Load filtered dictionary records via `dictionary_records.load_dictionary_records`.
2. Stage the non-empty `tl_num` lines and the non-empty `poj_num` lines into
   two temp files.
3. Invoke `engine/build-helpers/fst-builder build-syllables <out>
   --tl-input <tl-path> --poj-input <poj-path>` once. The Rust binary:
     - splits each line on ASCII tone digits 1..=9,
     - canonicalizes TL tokens via `phonetics::canonicalize_syllable`
       (POJ→TL spelling fold + `o͘`→`oo` + `ⁿ`→`nn`) — emitted with
       `tl:` prefix;
     - canonicalizes POJ tokens via `phonetics::canonicalize_poj_syllable`
       (encoding-only fold; POJ ASCII spelling preserved) — emitted
       with `poj:` prefix;
     - validates phonotactically through the shared TL initials × finals
       table; sample-logs invalid syllables to stderr but keeps building;
     - emits BOTH canonical numeric (`tl:tsua7` / `poj:choa7`) AND
       canonical toneless (`tl:tsua` / `poj:choa`) keys per family.

Why the tagged-single-FST split (v3.5.9 B-1, from v3.5.8 single-family)
----------------------------------------------------------------------
~80,000 of 159,000 dictionary rows have `tl_num != poj_num` (POJ-shaped
forms like `chit8`/`tsit8`, `goa2`/`gua2`, `toa7`/`tua7`). Folding the
POJ source rows into the TL inventory (the v3.5.8 behavior) loses the
POJ-side syllable boundaries, so a continuous POJ buffer like `chiah`
cannot be recognised as one valid syllable. Tagging the two families
into `tl:` / `poj:` key prefixes in a single FST keeps the storage
shared (POJ + TL syllable inventories overlap heavily by prefix tree)
while letting `lexicon::SyllableInventory::contains_in(mode, syllable)`
serve the right family at runtime. Rationale + design alternatives in
`docs/reports/2026-05-20-v359-b-plan.md` §B-1.

Consumers
---------
The output FST is loaded by `engine/lexicon::SyllableInventory::open`
and consumed by the Phase 3 syllabifier (composing crate). This script
is invoked manually as part of the dictionary rebuild flow; it is not
yet wired into a master build orchestrator.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from build.common import LOG_DIR, OUTPUT_DIR
from build.dictionary_records import load_dictionary_records
from common.logging_utils import log_header, setup_logging

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


def collect_family_lines(logger) -> tuple[list[str], list[str]]:
    """Yield the non-empty `tl_num` and `poj_num` fields from every record."""
    if not CSV_FILE.exists():
        raise RuntimeError(f"CSV not found: {CSV_FILE}")

    records = load_dictionary_records(CSV_FILE)
    logger.info(f"Loaded {len(records)} records from {CSV_FILE.name}")

    tl_lines = [record.tl_num for record in records if record.tl_num]
    poj_lines = [record.poj_num for record in records if record.poj_num]
    tl_skipped = len(records) - len(tl_lines)
    poj_skipped = len(records) - len(poj_lines)
    logger.info(
        f"tl_num lines emitted: {len(tl_lines)} (skipped {tl_skipped} empty); "
        f"poj_num lines emitted: {len(poj_lines)} (skipped {poj_skipped} empty)"
    )
    return tl_lines, poj_lines


def invoke_builder(
    tl_lines: list[str],
    poj_lines: list[str],
    builder_bin: Path,
) -> None:
    """Stage each family to a temp file, invoke `build-syllables` once.

    The builder takes both `--tl-input` and `--poj-input` so the
    tagged-single-FST is produced in one pass (avoiding fst::Set merge
    after the fact). Temp files are deleted in the `finally` block even
    on builder failure — they hold dictionary source rows, no secrets,
    but staying tidy keeps repeated invocations from leaking through
    `/tmp`.
    """
    tl_path: Path | None = None
    poj_path: Path | None = None
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

        proc = subprocess.run(
            [
                str(builder_bin),
                "build-syllables",
                str(OUTPUT_FILE),
                "--tl-input",
                str(tl_path),
                "--poj-input",
                str(poj_path),
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
        for staged in (tl_path, poj_path):
            if staged is not None and staged.exists():
                staged.unlink()


def main() -> None:
    logger = setup_logging(SCRIPT_NAME, log_dir=LOG_DIR)
    log_header(logger, SCRIPT_NAME, CSV_FILE, OUTPUT_FILE)

    builder_bin = resolve_builder_bin()
    logger.info(f"fst-builder: {builder_bin}")

    tl_lines, poj_lines = collect_family_lines(logger)
    invoke_builder(tl_lines, poj_lines, builder_bin)

    file_size = OUTPUT_FILE.stat().st_size
    logger.info("\n  [output]")
    logger.info(f"    File: {OUTPUT_FILE}")
    logger.info(f"    Size: {file_size / 1024:.2f} KB")
    logger.info("\nDone!")


if __name__ == "__main__":
    main()

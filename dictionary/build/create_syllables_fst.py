#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Build the v3.5.8 Phase 2 TL syllable inventory FST (`syllables.fst`).

Input:  output/dictionary.csv  (canonical dictionary records)
Output: output/syllables.fst   (~9 KB fst::Set of valid TL syllables)

Pipeline
--------
1. Load filtered dictionary records via `dictionary_records.load_dictionary_records`.
2. Pipe each record's raw `tl_num` (one per line) into the Rust binary
   `engine/build-helpers/fst-builder build-syllables`.
3. The binary splits each line on ASCII tone digits 1..=9, canonicalizes
   each syllable through `phonetics::canonicalize_syllable` (POJ→TL
   spelling normalization + `ⁿ`→`nn` + `o͘`→`oo`), validates phonotactically,
   and emits BOTH canonical numeric (`tsua7`) AND canonical toneless
   (`tsua`) keys to `fst::Set`.

Why this split between Python and Rust
--------------------------------------
Python only collects raw `tl_num` strings; the full splitter +
canonicalizer lives in Rust so the inventory contains canonical TL keys
even when the source row carries POJ-shaped fragments (`choa7` → `tsua7`)
or non-ASCII forms (`peⁿ5` → `penn5`, `so͘3` → `soo3`). This avoids
duplicating phonetics tables between Python and Rust and pins the
canonicalization invariant to a single code path. Pre-impl review by
Codex (v3.5.8 Phase 2) flagged the alternative — Python regex
pre-splitting — as lossy for the 197 non-ASCII rows currently in
`dictionary.csv`.

POJ has no separate inventory: POJ input is normalized to TL via
`phonetics::poj::to_tl` at runtime in Phase 3+. See `docs/roadmap.md`
§Phase 2 line 193.

Consumers
---------
The output FST is loaded by `engine/lexicon::SyllableInventory::open`
and consumed by the Phase 3 syllabifier (composing crate, future slice).
This script is invoked manually as part of the dictionary rebuild flow;
it is not yet wired into a master build orchestrator.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
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


def collect_tl_num_lines(logger) -> list[str]:
    """Yield the non-empty `tl_num` field from every dictionary record."""
    if not CSV_FILE.exists():
        raise RuntimeError(f"CSV not found: {CSV_FILE}")

    records = load_dictionary_records(CSV_FILE)
    logger.info(f"Loaded {len(records)} records from {CSV_FILE.name}")

    lines = [record.tl_num for record in records if record.tl_num]
    skipped = len(records) - len(lines)
    logger.info(f"tl_num lines emitted: {len(lines)} (skipped {skipped} with empty tl_num)")
    return lines


def emit_to_builder(lines: list[str], builder_bin: Path) -> None:
    """Pipe one `tl_num` per line into `fst-builder build-syllables`."""
    payload = ("\n".join(lines) + "\n").encode("utf-8")
    proc = subprocess.run(
        [str(builder_bin), "build-syllables", str(OUTPUT_FILE)],
        input=payload,
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"fst-builder build-syllables failed (exit {proc.returncode}): "
            f"{proc.stderr.decode('utf-8', errors='replace')}"
        )
    sys.stderr.write(proc.stderr.decode("utf-8", errors="replace"))


def main() -> None:
    logger = setup_logging(SCRIPT_NAME, log_dir=LOG_DIR)
    log_header(logger, SCRIPT_NAME, CSV_FILE, OUTPUT_FILE)

    builder_bin = resolve_builder_bin()
    logger.info(f"fst-builder: {builder_bin}")

    lines = collect_tl_num_lines(logger)
    emit_to_builder(lines, builder_bin)

    file_size = OUTPUT_FILE.stat().st_size
    logger.info("\n  [output]")
    logger.info(f"    File: {OUTPUT_FILE}")
    logger.info(f"    Size: {file_size / 1024:.2f} KB")
    logger.info("\nDone!")


if __name__ == "__main__":
    main()

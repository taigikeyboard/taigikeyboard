#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Build the fst prefix index from dictionary.csv

Input: output/dictionary.csv
Output: output/dictionary.fst

Key format (prefixed):
- tl:<tl_num>: TL digit tones (e.g. tl:hoo2se3; the digits act as syllable separators)
- tl:<tl_notone>: TL toneless, fused (e.g. tl:hoose; even the hyphens are gone)
- poj:<poj_num>: POJ digit tones (e.g. poj:ho2se3)
- poj:<poj_notone>: POJ toneless, fused (e.g. poj:hoose)
- tps:<tps_num>: TPS Bopomofo + tone marks (e.g. tps:ㄏㆦ˫ㄙㆤ˪)
- tps:<tps_notone>: TPS toneless, fused (e.g. tps:ㄏㆦㄙㆤ)
- tps:<tps_num_var>     C-3a er↔or dialect, always-on: the two Bopomofo shapes of TL `er`/`or` (ㄜ vs ㄛ)
- tps:<tps_notone_var>  same as above, emitted only when the source `tps_*` holds ㄜ; A always-on, replaces runtime
- tl-abbrev:<tl_abbrev>: TL abbreviation (e.g. tl-abbrev:hs) — own family since §46 so a
  `tl:` prefix scan never meets an acronym key (engine reader:
  `lexicon::fetch_abbrev_candidates`; Tab3 `lexicon::search` unions both)
- poj-abbrev:<poj_abbrev>: POJ abbreviation (e.g. poj-abbrev:hs)
- tps-abbrev:<tps_abbrev>: TPS abbreviation (e.g. tps-abbrev:ㄏㄙ)
- tps-abbrev:<tps_abbrev_var>  `tps_or_mapped_to_er` toggle (per PR C-3a).
- tl:/poj: nasal oo alias: the num + notone keys after `onn` → `oonn` per syllable
  (e.g. tl:hoonn3 / tl:hoonn), same rowid. POJ `o͘ⁿ` (ASCII `oonn` in the engine)
  is the 台日大辭典-family spelling; dictionary columns are always normalized to
  `onn`, so this is the one place that emits the extra input key.
- hanzi:<hanzi>: Hanji prefix search (e.g. hanzi:好無)

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

The fst itself is built by the Rust binary `engine/build-helpers/fst-builder`;
this Python script only reads dictionary.csv, assembles the stdin pairs and
shells out to Rust.

Romanization-key filter mirrors the original create_trie_db.sh JOIN +
syllable cap on `tl_num` (Codex pre-impl review Q6): emit a romanization
key only when `tl_num` is non-empty AND `tl_num` syllable count <= 4.
Hanzi keys come from dictionary records with non-NULL hanzi.
"""

import re
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


# Abbreviation keys get their own family per romanization (`tl-abbrev:`,
# `poj-abbrev:`, `tps-abbrev:`) so a phonetic-family prefix scan never
# meets an acronym key. Mirrors `lexicon::key_normalizer::ABBREV_FAMILY_SUFFIX`.
ABBREV_FAMILY_SUFFIX = "-abbrev"

_NUM_SYLLABLE_RE = re.compile(r"[a-z]+[0-9]?")
_NASAL_OO_CANONICAL = "onn"
_NASAL_OO_ALIAS = "oonn"


def _nasal_oo_alias_num(num_value: str) -> str:
    """`onn` → `oonn`, applied per syllable of a `*_num` value.

    The alternate spelling of the nasal final /ɔ̃/ — POJ `o͘ⁿ`, which reaches
    the engine as ASCII `oonn`. Dictionary columns are always canonical
    (`knowledge/taigi-phonetics-reference.md` §5 fixes `onn`/`oⁿ`), so this
    exists only to index the input spelling.

    Splitting on tone digits FIRST is load-bearing. Inside one syllable the
    letters `onn` can only be the nasal final (`onn`, `onnh`, `ionn`, `ionnh`
    are the only finals holding them; no initial does). Across a seam the same
    letters are something else entirely — 滷卵 `loo2nng7` reads `loo`|`nng`,
    and a whole-string replace would emit `loo2onng7`, a key nobody types.
    Mirrors `phonetics::nasal_oo_alias_spelling`, which does the same job for
    `syllables.fst` where the caller already holds one syllable per token.

    Returns "" when there is nothing to respell, or when the value does not
    partition cleanly into `[a-z]+[0-9]?` syllables (defensive: a dirty row
    is skipped rather than mangled).
    """
    if _NASAL_OO_CANONICAL not in num_value:
        return ""
    syllables = _NUM_SYLLABLE_RE.findall(num_value)
    if "".join(syllables) != num_value:
        return ""
    return "".join(
        syllable.replace(_NASAL_OO_CANONICAL, _NASAL_OO_ALIAS) for syllable in syllables
    )


def _nasal_oo_alias_keys(num_value: str) -> tuple[str, ...]:
    """Alias `(num, notone)` key bodies for a `tl_num` / `poj_num` value.

    The toneless form is derived from the ALIAS num (digits stripped, mirroring
    `dictionary/common/notone.py::remove_tone`) rather than from the stored
    `*_notone` column: the stored column is already fused, so its syllable
    boundaries are gone and 滷卵's `loonng` would respell across the seam.

    `*_abbrev` carries initials only and can never hold `onn`, so it has no
    alias form.
    """
    if not num_value:
        return ()
    alias_num = _nasal_oo_alias_num(num_value)
    if not alias_num:
        return ()
    alias_notone = re.sub(r"[\d\-]", "", alias_num)
    return (alias_num, alias_notone) if alias_notone else (alias_num,)


def romanization_keys(record) -> list[str]:
    """Every romanization key body (family prefix included) one record is
    indexed under, in emission order. Pure — the unit under test for the
    family layout (`tests/test_fst_abbrev_family.py`).
    """
    keys: list[str] = []
    # Romanization-key filter (mirrors original trie.db JOIN: tl_num
    # non-empty + syllables <= 4). All three TL variants share the
    # tl_num gate; same for POJ; TPS gated by the same TL syllable
    # count (TPS forms are derived from the same TL source row).
    if record.tl_num and _tl_num_syllable_count(record.tl_num) <= MAX_SYLLABLES_TL_NUM:
        for val in (record.tl_num, record.tl_notone):
            if val:
                keys.append(f"tl:{val}")
        for val in _nasal_oo_alias_keys(record.tl_num):
            keys.append(f"tl:{val}")
        if record.tl_abbrev:
            keys.append(f"tl{ABBREV_FAMILY_SUFFIX}:{record.tl_abbrev}")
        # C-3a er↔or dual-emit: same rowid keyed by both ㄜ-form
        # (bridge default) and ㄛ-form (toggle-OFF variant) so a
        # TPS user typing either glyph hits the same dictionary
        # row. Empty `*_var` columns short-circuit.
        for val in (
            record.tps_num, record.tps_notone,
            record.tps_num_var, record.tps_notone_var,
        ):
            if val:
                keys.append(f"tps:{val}")
        for val in (record.tps_abbrev, record.tps_abbrev_var):
            if val:
                keys.append(f"tps{ABBREV_FAMILY_SUFFIX}:{val}")
    if record.poj_num and _tl_num_syllable_count(record.poj_num) <= MAX_SYLLABLES_TL_NUM:
        for val in (record.poj_num, record.poj_notone):
            if val:
                keys.append(f"poj:{val}")
        for val in _nasal_oo_alias_keys(record.poj_num):
            keys.append(f"poj:{val}")
        if record.poj_abbrev:
            keys.append(f"poj{ABBREV_FAMILY_SUFFIX}:{record.poj_abbrev}")
    return keys


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
        for key in romanization_keys(record):
            add(key, record.rowid)

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

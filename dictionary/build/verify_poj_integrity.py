#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Fatal POJ-integrity gate for the dictionary build.

Every row's `poj` (and the derived `poj_num` / `poj_notone` / `poj_abbrev`
columns the FST actually indexes) MUST equal what the canonical
`convert_tl_to_poj(tl)` produces. Any divergence means the FST ships
`poj:<wrong key>` and POJ-mode users silently miss those entries.

This module is the safety gate split out of the former `build/audit.py`
(removed with its verbose report in the v3.5.9 version-snapshot round).
Keep it a STANDALONE named step so "drop the audit report" never deletes
the data-integrity check by accident.

Incident it guards (PR #175 + PR #184): a converter submodule bump didn't
trigger a per-source rebuild and 4.8% of rows shipped with poj=<TL spelling>.

KeSi cross-validation is REPORT-ONLY — an independent Python TL→POJ engine
(vendored at `references/KeSi`) that flags rows where KeSi and
taigi-converter disagree. KeSi has no SYLLABLE_RE tokenization layer, so it
catches taigi-converter output bugs the single-engine self-check cannot.
Skipped silently when the vendored copy is absent.

Usage:
  python3 -m build.verify_poj_integrity        # exits 1 on any stale row
"""

from __future__ import annotations

import sys
import unicodedata
from pathlib import Path

import pandas as pd

BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))

from common import read_dictionary_csv  # noqa: E402
from common.abbrev import extract_abbrev  # noqa: E402
from common.notone import remove_tone  # noqa: E402
from common.romanization import to_numeric_tone  # noqa: E402
from common.taigi_bridge import convert_tl_to_poj_strict  # noqa: E402

INPUT_FILE = BASE_DIR / "output" / "dictionary.csv"

# Independent TL→POJ converter (vendored, gitignored — see CLAUDE.md project
# structure). Skipped if not present so the gate still runs without it.
_KESI_PATH = BASE_DIR.parent / "references" / "KeSi"
_kesi_ku: type | None = None
if _KESI_PATH.is_dir():
    sys.path.insert(0, str(_KESI_PATH))
    try:
        from kesi.butkian.ku import Ku as _kesi_ku  # type: ignore  # noqa: E402
    except ImportError:
        _kesi_ku = None

# How many sample mismatch rows to print per fatal category.
SAMPLE_ROWS = 10


def _norm(text: str) -> str:
    """Canonical form for comparing romanization columns: NFC + casefold.

    Pure `.lower()` would let mixed NFC/NFD source data trip the gate
    (precomposed `ó` vs `o` + combining acute). NFC + casefold is the safe
    boundary.
    """
    return unicodedata.normalize("NFC", text or "").casefold()


def _kesi_tl_to_poj(tl: str) -> str:
    """Independent TL→POJ via KeSi. Empty string on empty input / parse fail."""
    if not tl or _kesi_ku is None:
        return ""
    try:
        return _kesi_ku(lomaji=tl).POJ().lomaji
    except Exception:
        return ""


def _print_samples(df: pd.DataFrame, cols: list[str]) -> None:
    present = [c for c in cols if c in df.columns]
    for _, row in df.head(SAMPLE_ROWS).iterrows():
        fields = "  ".join(f"{c}={row[c]!r}" for c in present)
        print(f"    {fields}")
    if len(df) > SAMPLE_ROWS:
        print(f"    ... ({len(df) - SAMPLE_ROWS} more)")


def main() -> int:
    if not INPUT_FILE.exists():
        print(f"[ERROR] {INPUT_FILE} not found")
        return 1

    df = read_dictionary_csv(INPUT_FILE)
    fatal_errors: list[str] = []

    # Canonical expected POJ for every row, via the STRICT converter so a Node
    # IPC failure aborts the gate rather than silently returning input (which
    # could let stale `poj == tl` rows pass). Convert each DISTINCT `tl` once —
    # many rows share a `tl` (homophones, variants), and each conversion is a
    # synchronous Node IPC round-trip, so deduping collapses the dominant cost
    # from len(df) to nunique(tl).
    tl_series = df["tl"].fillna("").astype(str)
    poj_by_tl = {t: (convert_tl_to_poj_strict(t) if t else "") for t in tl_series.unique()}
    expected_poj = tl_series.map(poj_by_tl)
    expected_poj_norm = expected_poj.map(_norm)

    # ── poj column ──
    poj_norm = df["poj"].fillna("").astype(str).map(_norm)
    stale_mask = poj_norm != expected_poj_norm
    stale = df[stale_mask].copy()
    stale["expected_poj"] = expected_poj[stale_mask].values
    print(f"Stale POJ (tl→poj mismatch): {len(stale)}")
    if len(stale) > 0:
        _print_samples(stale, ["hanzi", "tl", "poj", "expected_poj"])
        fatal_errors.append(
            f"Stale POJ: {len(stale)} rows; poj column doesn't match "
            "convert_tl_to_poj(tl)."
        )

    # ── derived columns the FST indexes (poj_num / poj_notone / poj_abbrev) ──
    expected_poj_num = expected_poj.map(
        lambda s: to_numeric_tone(s, ascii_only=True) if s else ""
    )
    expected_poj_notone = expected_poj_num.map(remove_tone)
    expected_poj_abbrev = expected_poj.map(extract_abbrev)

    def _stale(actual_col: str, expected_series: pd.Series) -> pd.Series:
        actual_norm = df[actual_col].fillna("").astype(str).map(_norm)
        return actual_norm != expected_series.map(_norm)

    stale_num_mask = _stale("poj_num", expected_poj_num)
    stale_notone_mask = _stale("poj_notone", expected_poj_notone)
    stale_abbrev_mask = _stale("poj_abbrev", expected_poj_abbrev)
    derived_mask = stale_num_mask | stale_notone_mask | stale_abbrev_mask
    derived = df[derived_mask].copy()
    derived["expected_poj_num"] = expected_poj_num[derived_mask].values
    derived["expected_poj_notone"] = expected_poj_notone[derived_mask].values
    derived["expected_poj_abbrev"] = expected_poj_abbrev[derived_mask].values
    print(
        f"Stale poj_num/notone/abbrev: {len(derived)} "
        f"(num={int(stale_num_mask.sum())} notone={int(stale_notone_mask.sum())} "
        f"abbrev={int(stale_abbrev_mask.sum())})"
    )
    if len(derived) > 0:
        _print_samples(
            derived,
            ["hanzi", "tl", "poj_num", "expected_poj_num",
             "poj_notone", "expected_poj_notone",
             "poj_abbrev", "expected_poj_abbrev"],
        )
        fatal_errors.append(
            f"Stale poj derived columns: {len(derived)} rows; FST keys "
            "`poj:<num|notone|abbrev>` would not match user input."
        )

    # ── KeSi cross-validation (report-only) ──
    if _kesi_ku is not None:
        kesi_by_tl = {t: _kesi_tl_to_poj(t) for t in tl_series.unique()}
        kesi_poj_norm = tl_series.map(kesi_by_tl).map(_norm)
        divergence = int((kesi_poj_norm != expected_poj_norm).sum())
        print(f"KeSi vs taigi-converter divergence: {divergence}  (report-only)")
    else:
        rel = _KESI_PATH.relative_to(BASE_DIR.parent)
        print(f"KeSi cross-validation: SKIPPED (vendored copy not at {rel})")

    if fatal_errors:
        print()
        print("─" * 60)
        print("BUILD FAILED — fatal POJ-integrity errors:")
        for err in fatal_errors:
            print(f"  • {err}")
        return 1

    print("POJ integrity OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())

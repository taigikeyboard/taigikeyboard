#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
辭典健檢報告

輸入：output/dictionary.csv
輸出：
  - output/audit_report.txt      （摘要報告）
  - output/audit/                 （各檢查項目詳細 CSV）
    ├── 01_no_hanzi.csv           無漢字（純羅馬字）
    ├── 02_space_tl.csv           空白分隔的 tl
    ├── 03_no_source.csv          無詞庫來源
    ├── 04_variants.csv           異用字 / 語音變體
    ├── 05_freq_zero.csv          詞頻 = 0
    ├── 06_homographs.csv         同漢字多讀音
    ├── 07_homophones.csv         同讀音多漢字
    ├── 08_hanzi_digit.csv        漢字含數字
    ├── 09_tl_punctuation.csv     TL 含標點
    ├── 10_empty_poj.csv          POJ 為空
    ├── 11_empty_tl_num.csv       tl_num 為空
    ├── 12_stale_poj.csv          poj ≠ convert_tl_to_poj(tl) ❌ FATAL
    ├── 13_stale_poj_derived.csv  any of {poj_num, poj_notone, poj_abbrev} ≠ expected ❌ FATAL
    └── 14_kesi_divergence.csv    KeSi disagrees with taigi-converter (REPORT-ONLY)

Fatal categories halt the build (`sys.exit(1)`) so deploy/regen never ships
stale data. PR #175 + PR #184 incident: a converter submodule bump didn't
trigger a per-source rebuild and 4.8 % of rows shipped with poj=<TL spelling>.
The trie keys are built from `poj_num` / `poj_notone` / `poj_abbrev` (see
`build/create_fst.py`), so the audit checks `poj` itself AND every derived
column under one combined fatal category.

KeSi cross-validation (14_kesi_divergence) is REPORT-ONLY — it loads the
local KeSi vendored at `references/KeSi` and runs an independent TL→POJ
conversion to flag rows where KeSi and taigi-converter disagree. KeSi has
no SYLLABLE_RE-class tokenization, so divergence lights up taigi-converter
output bugs that 12_stale_poj alone (single-engine self-check) cannot
catch. Skipped silently if KeSi is unavailable.
"""

import re
import sys
import unicodedata
from datetime import datetime
from pathlib import Path

import pandas as pd

BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))
from common.source_bits import MAIN_SOURCE_COLUMNS  # noqa: E402
from common.taigi_bridge import convert_tl_to_poj_strict  # noqa: E402
from common.romanization import to_numeric_tone  # noqa: E402
from common.notone import remove_tone  # noqa: E402
from common.abbrev import extract_abbrev  # noqa: E402

# KeSi cross-validation — independent Python TL→POJ converter at
# `references/KeSi` (vendored, gitignored — see CLAUDE.md project structure).
# Used to cross-check taigi-converter output. Skipped if not present so the
# audit still runs on machines without the vendored copy.
_KESI_PATH = BASE_DIR.parent / "references" / "KeSi"
_kesi_ku: type | None = None
if _KESI_PATH.is_dir():
    sys.path.insert(0, str(_KESI_PATH))
    try:
        from kesi.butkian.ku import Ku as _kesi_ku  # type: ignore  # noqa: E402
    except ImportError:
        _kesi_ku = None


def _kesi_tl_to_poj(tl: str) -> str:
    """Independent TL→POJ via KeSi. Returns empty string on empty input or
    on KeSi parse failure (preserves report-only semantics)."""
    if not tl or _kesi_ku is None:
        return ""
    try:
        return _kesi_ku(lomaji=tl).POJ().lomaji
    except Exception:
        return ""


def _norm(text: str) -> str:
    """Canonical form for comparing romanization columns: NFC + casefold.

    Pure `.lower()` would let mixed NFC/NFD source data trip the audit
    (e.g. precomposed `ó` vs `o` + combining acute). NFC + casefold is
    the safe boundary recommended by Codex pre-impl review.
    """
    return unicodedata.normalize("NFC", text or "").casefold()

INPUT_FILE = BASE_DIR / "output" / "dictionary.csv"
OUTPUT_FILE = BASE_DIR / "output" / "audit_report.txt"
AUDIT_DIR = BASE_DIR / "output" / "audit"

SOURCE_COLUMNS = MAIN_SOURCE_COLUMNS

# Columns to include in detail CSVs
DETAIL_COLS = ["hanzi", "tl", "poj", "frequency", "is_variant"] + SOURCE_COLUMNS


def count_syllables(tl):
    if pd.isna(tl) or not str(tl).strip():
        return 0
    parts = re.split(r"[-\s]+", str(tl).strip())
    return len([p for p in parts if p])


def source_label(row):
    """Return comma-separated source names for a row."""
    return ",".join(c for c in SOURCE_COLUMNS if row.get(c))


def save_csv(subset, filename, extra_cols=None):
    """Save a subset to CSV under audit/ directory."""
    cols = list(DETAIL_COLS)
    if extra_cols:
        cols = extra_cols + [c for c in cols if c not in extra_cols]
    cols = [c for c in cols if c in subset.columns]
    path = AUDIT_DIR / filename
    subset[cols].to_csv(path, index=False)
    return path, len(subset)


def main():
    if not INPUT_FILE.exists():
        print(f"[ERROR] {INPUT_FILE} not found")
        return

    AUDIT_DIR.mkdir(parents=True, exist_ok=True)
    from common import read_dictionary_csv  # noqa: E402
    df = read_dictionary_csv(INPUT_FILE)

    # Fatal errors block the deploy step. Append a one-line summary; the
    # detail CSV is the row-level evidence.
    fatal_errors: list[str] = []

    # Pre-compute flags
    has_hanzi = df["hanzi"].notna() & (df["hanzi"].astype(str).str.strip() != "")
    has_space = df["tl"].astype(str).str.contains(" ", na=False)
    is_variant = df["is_variant"].astype(bool)
    no_source = ~df[SOURCE_COLUMNS].any(axis=1)
    df["_nsyl"] = df["tl"].apply(count_syllables)
    df["_sources"] = df.apply(source_label, axis=1)

    lines = []

    def w(text=""):
        lines.append(text)

    def section(title):
        w()
        w("=" * 60)
        w(title)
        w("=" * 60)

    # ── Header ──
    w("Taigi Keyboard Dictionary Audit Report")
    w(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    w(f"Input: {INPUT_FILE}")
    w(f"Total entries: {len(df)}")

    # ── 1. Overview ──
    section("1. Overview")
    w(f"  With hanzi:       {has_hanzi.sum():>7}")
    w(f"  Without hanzi:    {(~has_hanzi).sum():>7}  (pure romanization)")
    w(f"  Space in tl:      {has_space.sum():>7}  (word boundary)")
    w(f"  Variants:         {is_variant.sum():>7}")
    w(f"  No source:        {no_source.sum():>7}  (Khiin supplement etc.)")

    # ── 2. No Hanzi ──
    section("2. Entries Without Hanzi")
    no_hanzi = df[~has_hanzi].sort_values("frequency", ascending=False)
    path, cnt = save_csv(no_hanzi, "01_no_hanzi.csv")
    w(f"  Count: {cnt}")
    w(f"  CSV:   {path}")

    # ── 3. Space-separated TL ──
    section("3. Space-Separated TL (word boundary)")
    space_df = df[has_space].sort_values("frequency", ascending=False)
    path, cnt = save_csv(space_df, "02_space_tl.csv")
    w(f"  Count: {cnt}")
    w(f"  CSV:   {path}")
    if cnt > 0:
        w()
        w("  By source:")
        for col in SOURCE_COLUMNS:
            c = space_df[col].sum()
            if c > 0:
                w(f"    {col}: {c}")

    # ── 4. No Source ──
    section("4. Entries Without Source")
    no_src = df[no_source].sort_values("frequency", ascending=False)
    path, cnt = save_csv(no_src, "03_no_source.csv")
    w(f"  Count: {cnt}")
    w(f"  CSV:   {path}")

    # ── 5. Variant Entries ──
    section("5. Variant Entries")
    variants = df[is_variant].sort_values("frequency", ascending=False)
    path, cnt = save_csv(variants, "04_variants.csv")
    w(f"  Count: {cnt}")
    w(f"  CSV:   {path}")

    # ── 6. Frequency = 0 ──
    section("6. Entries With frequency = 0")
    freq_zero = df[df["frequency"] == 0].sort_values(["hanzi", "tl"])
    path, cnt = save_csv(freq_zero, "05_freq_zero.csv")
    w(f"  Count: {cnt}")
    w(f"  CSV:   {path}")

    # ── 7. Frequency Distribution ──
    section("7. Frequency Distribution")
    freq = df["frequency"]
    w(f"  Max: {freq.max()}, Min: {freq.min()}, Mean: {freq.mean():.1f}, Median: {freq.median():.0f}")
    w()
    buckets = [
        (0, 0, "= 0"),
        (1, 10, "1-10"),
        (11, 100, "11-100"),
        (101, 1000, "101-1K"),
        (1001, 10000, "1K-10K"),
        (10001, 100000, "10K-100K"),
        (100001, float("inf"), "> 100K"),
    ]
    for lo, hi, label in buckets:
        cnt = ((freq >= lo) & (freq <= hi)).sum()
        pct = cnt / len(df) * 100
        bar = "█" * int(pct / 2)
        w(f"    {label:>10}: {cnt:>7} ({pct:5.1f}%) {bar}")

    # ── 8. Syllable Distribution ──
    section("8. Syllable Count Distribution")
    syl_dist = df["_nsyl"].value_counts().sort_index()
    for nsyl, cnt in syl_dist.items():
        pct = cnt / len(df) * 100
        bar = "█" * int(pct / 2)
        w(f"    {nsyl} syl: {cnt:>7} ({pct:5.1f}%) {bar}")

    # ── 9. Homographs ──
    section("9. Homographs (same hanzi, multiple readings)")
    hanzi_only = df[has_hanzi]
    hanzi_groups = hanzi_only.groupby("hanzi")["tl"].nunique()
    multi_reading_hanzi = hanzi_groups[hanzi_groups > 1].sort_values(ascending=False)
    w(f"  Hanzi with multiple readings: {len(multi_reading_hanzi)}")

    # Build detailed CSV: one row per (hanzi, tl), grouped by hanzi
    homograph_rows = []
    for hanzi in multi_reading_hanzi.index:
        group = hanzi_only[hanzi_only["hanzi"] == hanzi].sort_values("frequency", ascending=False)
        for _, row in group.iterrows():
            homograph_rows.append({
                "hanzi": hanzi,
                "reading_count": len(group),
                "tl": row["tl"],
                "poj": row["poj"],
                "frequency": row["frequency"],
                "is_variant": row["is_variant"],
                "_sources": row["_sources"],
            })
    homograph_df = pd.DataFrame(homograph_rows)
    if len(homograph_df) > 0:
        path = AUDIT_DIR / "06_homographs.csv"
        homograph_df.to_csv(path, index=False)
        w(f"  CSV:   {path} ({len(homograph_df)} rows)")

    w()
    w("  Top 20:")
    for hanzi, cnt in multi_reading_hanzi.head(20).items():
        readings = hanzi_only[hanzi_only["hanzi"] == hanzi]["tl"].unique()
        w(f"    {hanzi} ({cnt}): {', '.join(readings[:5])}")

    # ── 10. Homophones ──
    section("10. Homophones (same tl, multiple hanzi)")
    tl_groups = hanzi_only.groupby("tl")["hanzi"].nunique()
    multi_hanzi_tl = tl_groups[tl_groups > 1].sort_values(ascending=False)
    w(f"  TL with multiple hanzi: {len(multi_hanzi_tl)}")

    homophone_rows = []
    for tl in multi_hanzi_tl.index:
        group = hanzi_only[hanzi_only["tl"] == tl].sort_values("frequency", ascending=False)
        for _, row in group.iterrows():
            homophone_rows.append({
                "tl": tl,
                "hanzi_count": len(group),
                "hanzi": row["hanzi"],
                "frequency": row["frequency"],
                "is_variant": row["is_variant"],
                "_sources": row["_sources"],
            })
    homophone_df = pd.DataFrame(homophone_rows)
    if len(homophone_df) > 0:
        path = AUDIT_DIR / "07_homophones.csv"
        homophone_df.to_csv(path, index=False)
        w(f"  CSV:   {path} ({len(homophone_df)} rows)")

    w()
    w("  Top 20:")
    for tl, cnt in multi_hanzi_tl.head(20).items():
        hanzis = hanzi_only[hanzi_only["tl"] == tl]["hanzi"].unique()
        w(f"    {tl} ({cnt}): {', '.join(str(h) for h in hanzis[:8])}")

    # ── 11. Source Coverage ──
    section("11. Source Coverage")
    w()
    w(f"    {'Source':<10} {'Total':>7} {'Only':>7} {'w/ hanzi':>8} {'no hanzi':>8} {'variant':>8}")
    w(f"    {'-'*10} {'-'*7} {'-'*7} {'-'*8} {'-'*8} {'-'*8}")
    for col in SOURCE_COLUMNS:
        mask = df[col].astype(bool)
        total = mask.sum()
        others = [c for c in SOURCE_COLUMNS if c != col]
        only = (mask & ~df[others].any(axis=1)).sum()
        w_hanzi = (mask & has_hanzi).sum()
        wo_hanzi = (mask & ~has_hanzi).sum()
        var = (mask & is_variant).sum()
        w(f"    {col:<10} {total:>7} {only:>7} {w_hanzi:>8} {wo_hanzi:>8} {var:>8}")

    w()
    w("  Source overlap (entries in N sources):")
    src_count = df[SOURCE_COLUMNS].sum(axis=1)
    for n in range(0, src_count.max().astype(int) + 1):
        cnt = (src_count == n).sum()
        if cnt > 0:
            w(f"    {n} sources: {cnt}")

    # ── 12. Potential Issues ──
    section("12. Potential Issues")

    # TL containing punctuation
    unusual = df[df["tl"].str.contains(r"[;；。，]", na=False, regex=True)]
    path, cnt = save_csv(unusual, "09_tl_punctuation.csv")
    w(f"  TL with punctuation: {cnt}")
    if cnt > 0:
        w(f"  CSV: {path}")

    # Hanzi containing digits
    hanzi_digit = df[has_hanzi & df["hanzi"].astype(str).str.contains(r"\d", na=False, regex=True)]
    path, cnt = save_csv(hanzi_digit, "08_hanzi_digit.csv")
    w(f"  Hanzi containing digits: {cnt}")
    if cnt > 0:
        w(f"  CSV: {path}")

    # Empty tl_num
    empty_num = df[df["tl_num"].isna() | (df["tl_num"].astype(str).str.strip() == "")]
    path, cnt = save_csv(empty_num, "11_empty_tl_num.csv")
    w(f"  Empty tl_num: {cnt}")
    if cnt > 0:
        w(f"  CSV: {path}")

    # Empty poj
    empty_poj = df[df["poj"].isna() | (df["poj"].astype(str).str.strip() == "")]
    path, cnt = save_csv(empty_poj, "10_empty_poj.csv")
    w(f"  Empty poj: {cnt}")
    if cnt > 0:
        w(f"  CSV: {path}")

    # Stale POJ — fail-fast invariant: every row's `poj` MUST equal
    # `convert_tl_to_poj(tl)`. Any divergence means the trie ships
    # `poj:<wrong key>` and POJ-mode users miss those entries.
    # Uses the STRICT converter so a Node IPC failure aborts the audit
    # rather than silently returning input (which could let stale
    # `poj == tl` rows pass — Codex pre-impl P1).
    expected_poj = df["tl"].fillna("").astype(str).map(
        lambda s: convert_tl_to_poj_strict(s) if s else ""
    )
    expected_poj_norm = expected_poj.map(_norm)
    poj_norm = df["poj"].fillna("").astype(str).map(_norm)
    stale_mask = poj_norm != expected_poj_norm
    stale = df[stale_mask].copy()
    stale["expected_poj"] = expected_poj[stale_mask].values
    path, cnt = save_csv(
        stale,
        "12_stale_poj.csv",
        extra_cols=["hanzi", "tl", "poj", "expected_poj"],
    )
    w(f"  Stale POJ (tl→poj mismatch): {cnt}")
    if cnt > 0:
        w(f"  CSV: {path}")
        fatal_errors.append(
            f"Stale POJ: {cnt} rows; poj column doesn't match "
            f"convert_tl_to_poj(tl). See {path.relative_to(BASE_DIR)}"
        )

    # Stale poj_num / poj_notone / poj_abbrev — every column the trie
    # actually indexes (see build/create_fst.py). Even with a correct
    # `poj`, a stale numtone / notone / abbrev stage could ship bad
    # `poj:` keys. Re-derive each from the canonical expected_poj and
    # compare. All three are checked under one fatal category since
    # they cascade from the same expected_poj source.
    expected_poj_num = expected_poj.map(
        lambda s: to_numeric_tone(s, ascii_only=True) if s else ""
    )
    expected_poj_notone = expected_poj_num.map(remove_tone)
    expected_poj_abbrev = expected_poj.map(extract_abbrev)

    def _stale(actual_col: str, expected_series: pd.Series) -> pd.Series:
        actual_norm = df[actual_col].fillna("").astype(str).map(_norm)
        expected_norm = expected_series.map(_norm)
        return actual_norm != expected_norm

    stale_num_mask = _stale("poj_num", expected_poj_num)
    stale_notone_mask = _stale("poj_notone", expected_poj_notone)
    stale_abbrev_mask = _stale("poj_abbrev", expected_poj_abbrev)
    derived_stale_mask = stale_num_mask | stale_notone_mask | stale_abbrev_mask
    derived_stale = df[derived_stale_mask].copy()
    derived_stale["expected_poj_num"] = expected_poj_num[derived_stale_mask].values
    derived_stale["expected_poj_notone"] = expected_poj_notone[derived_stale_mask].values
    derived_stale["expected_poj_abbrev"] = expected_poj_abbrev[derived_stale_mask].values
    path, cnt = save_csv(
        derived_stale,
        "13_stale_poj_derived.csv",
        extra_cols=[
            "hanzi", "tl", "poj",
            "poj_num", "expected_poj_num",
            "poj_notone", "expected_poj_notone",
            "poj_abbrev", "expected_poj_abbrev",
        ],
    )
    w(
        f"  Stale poj_num/notone/abbrev (derived columns ≠ expected): {cnt}"
        f" (num={stale_num_mask.sum()} notone={stale_notone_mask.sum()}"
        f" abbrev={stale_abbrev_mask.sum()})"
    )
    if cnt > 0:
        w(f"  CSV: {path}")
        fatal_errors.append(
            f"Stale poj derived columns: {cnt} rows; trie keys "
            f"`poj:<num|notone|abbrev>` would not match user input. "
            f"See {path.relative_to(BASE_DIR)}"
        )

    # KeSi cross-validation — REPORT-ONLY. Independent Python TL→POJ
    # converter (at references/KeSi). Flags rows where KeSi and the
    # taigi-converter expected_poj disagree. Catches taigi-converter
    # output bugs that 12_stale_poj alone (single-engine self-check)
    # cannot — see PR #175 + PR #184 incident where SYLLABLE_RE silently
    # bypassed `tsí` / `tíng` / `tiûnn` and shipped stale POJ. KeSi has no
    # SYLLABLE_RE tokenization layer so it doesn't share the failure mode.
    if _kesi_ku is not None:
        kesi_poj = df["tl"].fillna("").astype(str).map(_kesi_tl_to_poj)
        kesi_poj_norm = kesi_poj.map(_norm)
        # Compare against expected_poj_norm (already computed above) — this
        # isolates "KeSi vs taigi-converter" as a clean signal independent
        # of the actual `poj` column staleness.
        divergence_mask = kesi_poj_norm != expected_poj_norm
        divergence = df[divergence_mask].copy()
        divergence["expected_poj_taigi"] = expected_poj[divergence_mask].values
        divergence["kesi_poj"] = kesi_poj[divergence_mask].values
        path, cnt = save_csv(
            divergence,
            "14_kesi_divergence.csv",
            extra_cols=["hanzi", "tl", "expected_poj_taigi", "kesi_poj"],
        )
        w(f"  KeSi vs taigi-converter divergence: {cnt}  (report-only)")
        if cnt > 0:
            w(f"  CSV: {path}")
    else:
        w(f"  KeSi cross-validation: SKIPPED (vendored copy not at {_KESI_PATH.relative_to(BASE_DIR.parent)})")

    # Cleanup temp columns
    df.drop(columns=["_nsyl", "_sources"], inplace=True)

    # Write report
    report = "\n".join(lines)
    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        f.write(report)

    print(report)
    print(f"\nReport saved: {OUTPUT_FILE}")
    print(f"Detail CSVs:  {AUDIT_DIR}/")

    if fatal_errors:
        print()
        print("─" * 60)
        print("BUILD FAILED — fatal audit errors:")
        for err in fatal_errors:
            print(f"  • {err}")
        sys.exit(1)


if __name__ == "__main__":
    main()

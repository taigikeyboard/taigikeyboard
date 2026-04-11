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
    └── 11_empty_tl_num.csv       tl_num 為空
"""

import os
import re
import pandas as pd
from datetime import datetime

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
INPUT_FILE = os.path.join(BASE_DIR, "output", "dictionary.csv")
OUTPUT_FILE = os.path.join(BASE_DIR, "output", "audit_report.txt")
AUDIT_DIR = os.path.join(BASE_DIR, "output", "audit")

SOURCE_COLUMNS = ["kautian", "taigitv", "itaigi", "sitbut", "taihoa", "taijit", "kungge", "stti", "khpoo"]

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
    path = os.path.join(AUDIT_DIR, filename)
    subset[cols].to_csv(path, index=False)
    return path, len(subset)


def main():
    if not os.path.exists(INPUT_FILE):
        print(f"[ERROR] {INPUT_FILE} not found")
        return

    os.makedirs(AUDIT_DIR, exist_ok=True)
    df = pd.read_csv(INPUT_FILE)

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
        path = os.path.join(AUDIT_DIR, "06_homographs.csv")
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
        path = os.path.join(AUDIT_DIR, "07_homophones.csv")
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

    # Cleanup temp columns
    df.drop(columns=["_nsyl", "_sources"], inplace=True)

    # Write report
    report = "\n".join(lines)
    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        f.write(report)

    print(report)
    print(f"\nReport saved: {OUTPUT_FILE}")
    print(f"Detail CSVs:  {AUDIT_DIR}/")


if __name__ == "__main__":
    main()

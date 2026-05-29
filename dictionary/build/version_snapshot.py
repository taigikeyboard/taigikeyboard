#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Version snapshots + build-drop / vs-previous diff summary.

Replaces the verbose `build/audit.py` report (removed in the v3.5.9 round).
Two jobs:

1. **Drop summary** — how many word entries the build pipeline discarded.
   Read from `output/.build_stats.json` (written by build/merge_csv.py).

2. **Version diff** — which `(hanzi, tl)` entries were added / removed versus
   the previous release snapshot. The canonical word unit is the `(hanzi, tl)`
   pair (same key merge_csv dedups on).

Release mode (`--version vX.Y.Z` or `RELEASE_VERSION` env): also writes a
distilled keyset snapshot to `dictionary/snapshots/vX.Y.Z.tsv` and prunes to
the newest `MAX_SNAPSHOTS` by semantic version. Re-running the same version
overwrites its snapshot in place (does not count as a new one).

No version given (plain `make dict`): report-only — diffs current
`dictionary.csv` against the latest existing snapshot, writes nothing tracked.

This step is REPORT-ONLY: a large added/removed count never halts the build.
Release-scope is the maintainer's call. Fatal only on a malformed version, a
tab/newline in a key field, or (release mode) a version older than the latest
snapshot. Exact `(hanzi, tl)` duplicates in dictionary.csv legitimately
collapse to one entry — multiple source/variant rows can share a key.

Usage:
  python3 -m build.version_snapshot                 # report-only diff
  python3 -m build.version_snapshot --version v3.5.9 # snapshot + diff + prune
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import unicodedata
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))

from common import read_dictionary_csv  # noqa: E402

INPUT_FILE = BASE_DIR / "output" / "dictionary.csv"
STATS_FILE = BASE_DIR / "output" / ".build_stats.json"
SNAPSHOT_DIR = BASE_DIR / "snapshots"
DIFF_FILE = BASE_DIR / "output" / "version_diff.txt"

MAX_SNAPSHOTS = 3
SAMPLE_ROWS = 30
VERSION_RE = re.compile(r"^v\d+\.\d+\.\d+$")

WordKey = tuple[str, str]


def _norm(text: str) -> str:
    """NFC-normalize so snapshot/diff stays stable across NFC/NFD source drift."""
    return unicodedata.normalize("NFC", text or "")


def parse_version(filename_stem: str) -> tuple[int, ...]:
    """`v3.10.0` → (3, 10, 0) for semantic-version sorting (not string sort)."""
    return tuple(int(p) for p in filename_stem.lstrip("v").split("."))


def load_keyset(csv_path: Path) -> set[WordKey]:
    """Load the `(hanzi, tl)` keyset as stored in dictionary.csv.

    `tl` is kept verbatim — official entries retain spaces (e.g. "m̄ bat")
    while supplements use hyphens. (merge_csv dedups on a space-normalized
    key but writes back the official spaced form; the snapshot tracks what
    actually shipped, so a rare space↔hyphen spelling flip surfaces as a real
    add+remove rather than being hidden.)

    Fails fast if any key field contains a tab/newline (would corrupt the TSV
    snapshot) — keys must be clean by build-pipeline invariant. Exact duplicate
    keys collapse into one entry (multiple source/variant rows share a key).
    """
    df = read_dictionary_csv(csv_path)
    keys: set[WordKey] = set()
    for hanzi, tl in zip(df["hanzi"].fillna(""), df["tl"].fillna("")):
        hanzi_n, tl_n = _norm(str(hanzi)), _norm(str(tl))
        if any(c in hanzi_n or c in tl_n for c in ("\t", "\n", "\r")):
            sys.exit(
                f"[ERROR] tab/newline in key field: hanzi={hanzi_n!r} tl={tl_n!r}"
            )
        keys.add((hanzi_n, tl_n))
    return keys


def read_snapshot(path: Path) -> set[WordKey]:
    keys: set[WordKey] = set()
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw or raw == "hanzi\ttl":
            continue
        hanzi, _, tl = raw.partition("\t")
        keys.add((hanzi, tl))
    return keys


def write_snapshot(path: Path, keys: set[WordKey]) -> None:
    lines = ["hanzi\ttl"]
    lines.extend(f"{h}\t{t}" for h, t in sorted(keys))
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def existing_snapshots() -> list[Path]:
    """Snapshot files sorted oldest→newest by semantic version."""
    if not SNAPSHOT_DIR.is_dir():
        return []
    snaps = [p for p in SNAPSHOT_DIR.glob("v*.tsv") if VERSION_RE.match(p.stem)]
    return sorted(snaps, key=lambda p: parse_version(p.stem))


def print_drop_summary() -> None:
    print("=" * 60)
    print("Build-internal drop summary")
    print("=" * 60)
    if not STATS_FILE.exists():
        print("  (no .build_stats.json — run merge_csv first)")
        return
    stats = json.loads(STATS_FILE.read_text())
    raw = stats.get("raw_rows")
    dedup = stats.get("after_dedup")
    print(f"  raw merged input:        {raw:>8,}")
    print(f"  after cross-source dedup:{dedup:>8,}  (dropped {raw - dedup:,})")
    for key, label in (
        ("khiin_added", "khiin"),
        ("dev_added", "dev"),
        ("lkk_added", "lkk"),
    ):
        if key in stats:
            print(f"  + {label} supplement:      {stats[key]:>8,}")
    filt = stats.get("filter_drops", {})
    if filt:
        parts = ", ".join(f"{k}={v}" for k, v in filt.items())
        print(f"  supplement filter drops: {parts}")
    print(f"  final dictionary entries:{stats.get('final', 0):>8,}")


def print_version_diff(current: set[WordKey], prev_path: Path | None) -> None:
    print()
    print("=" * 60)
    print("Version diff (entries = (hanzi, tl) pairs)")
    print("=" * 60)
    if prev_path is None:
        print("  no previous snapshot — nothing to diff")
        return

    prev = read_snapshot(prev_path)
    added = sorted(current - prev)
    removed = sorted(prev - current)
    print(f"  vs {prev_path.stem}: +{len(added)} added, -{len(removed)} removed")

    def _sample(label: str, items: list[WordKey]) -> None:
        if not items:
            return
        print(f"\n  {label} (first {min(SAMPLE_ROWS, len(items))}):")
        for h, t in items[:SAMPLE_ROWS]:
            print(f"    {h}\t{t}")

    _sample("added", added)
    _sample("removed", removed)

    # Full lists to an ephemeral file (gitignored) for the maintainer to read.
    out = [f"# diff vs {prev_path.stem}", f"# added: {len(added)}  removed: {len(removed)}", ""]
    out.append("## added")
    out.extend(f"{h}\t{t}" for h, t in added)
    out.append("")
    out.append("## removed")
    out.extend(f"{h}\t{t}" for h, t in removed)
    DIFF_FILE.write_text("\n".join(out) + "\n", encoding="utf-8")
    print(f"\n  full lists: {DIFF_FILE.relative_to(BASE_DIR)}")


def prune_snapshots() -> None:
    snaps = existing_snapshots()
    for stale in snaps[:-MAX_SNAPSHOTS]:
        stale.unlink()
        print(f"  pruned old snapshot: {stale.name}")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--version",
        default=None,
        help="release version label vX.Y.Z (writes a tracked snapshot)",
    )
    args = ap.parse_args()

    if not INPUT_FILE.exists():
        print(f"[ERROR] {INPUT_FILE} not found — run merge_csv first")
        return 1

    version = args.version
    print_drop_summary()
    current = load_keyset(INPUT_FILE)

    snaps = existing_snapshots()

    if version is None:
        # Report-only: diff against the latest existing snapshot, write nothing.
        prev = snaps[-1] if snaps else None
        print_version_diff(current, prev)
        return 0

    # ── release mode ──
    if not VERSION_RE.match(version):
        print(f"[ERROR] malformed version {version!r} (want vMAJOR.MINOR.PATCH)")
        return 1

    # Refuse a version older than the latest snapshot: it would be written then
    # immediately pruned as the oldest, silently losing the file. Re-running the
    # latest version (==) overwrites in place and is allowed; the true
    # first-snapshot case (no snaps) is allowed.
    if snaps and parse_version(version) < parse_version(snaps[-1].stem):
        print(
            f"[ERROR] {version} is older than latest snapshot {snaps[-1].stem}; "
            "refusing (would be pruned immediately). Backports not supported."
        )
        return 1

    target = SNAPSHOT_DIR / f"{version}.tsv"
    # Previous = newest snapshot strictly older than target (skip same-version
    # rerun, which overwrites in place rather than diffing against itself).
    older = [p for p in snaps if parse_version(p.stem) < parse_version(version)]
    prev = older[-1] if older else None
    if prev is None and target.exists():
        print(f"  note: re-running {version} (overwriting snapshot in place)")

    print_version_diff(current, prev)

    SNAPSHOT_DIR.mkdir(parents=True, exist_ok=True)
    write_snapshot(target, current)
    action = "overwrote" if target in snaps else "wrote"
    print(f"\n  {action} snapshot: {target.relative_to(BASE_DIR)} ({len(current):,} entries)")
    prune_snapshots()
    return 0


if __name__ == "__main__":
    sys.exit(main())

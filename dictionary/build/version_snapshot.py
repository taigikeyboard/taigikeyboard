#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Build-drop summary + vs-previous-release diff (no stored snapshot files).

Replaces the verbose `build/audit.py` report (removed in the v3.5.9 round).
Two jobs at dict build time:

1. **Drop summary** — how many word entries the build pipeline discarded.
   Read from `output/.build_stats.json` (written by build/merge_csv.py).

2. **Version diff** — which `(hanzi, tl)` entries were added / removed versus
   the previous RELEASE TAG. The canonical word unit is the `(hanzi, tl)` pair
   (same key merge_csv dedups on), and the only thing the diff needs is that
   key set — so this module writes one, `dictionary/word-keys.tsv`, and reads
   the previous release's copy straight from git (`git show <tag>:…`).

   That file is the diff basis precisely so `dictionary/output/` does not have
   to be committed for the diff to work: the key set is 3.1 MB where the full
   `dictionary.csv` it is derived from is 35 MB, and every other column in that
   CSV is either a romanization this project computes or a source bit — none of
   it participates in the diff.

Previous-tag resolution (semver, **3-segment `vX.Y.Z` only** — a 4-segment tag
like `v3.4.8.1` is ignored by release policy):
  - release mode (`--version vX.Y.Z` / `RELEASE_VERSION`): previous = newest tag
    strictly older than the target (so re-running after the target is tagged
    excludes the target itself).
  - report-only (plain `make dict`): previous = newest release tag overall.

REPORT-ONLY: never halts the build. Any git / decode / parse failure degrades to
"no previous release available — nothing to diff". In release mode a missing
predecessor (no tag, or a resolved tag whose CSV can't be read) additionally
prints a loud `[WARN]` (a release expects a base; a shallow / tags-less checkout
would otherwise silently skip the review). The only
fatal cases are a malformed `--version` and a tab/newline in a current-build key
field. Exact
`(hanzi, tl)` duplicates in dictionary.csv legitimately collapse to one entry —
multiple source/variant rows can share a key.

Usage:
  python3 -m build.version_snapshot                  # report-only diff vs latest tag
  python3 -m build.version_snapshot --version v3.6.0 # diff vs newest tag < v3.6.0
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import unicodedata
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))

from common import read_dictionary_csv  # noqa: E402

INPUT_FILE = BASE_DIR / "output" / "dictionary.csv"
STATS_FILE = BASE_DIR / "output" / ".build_stats.json"
DIFF_FILE = BASE_DIR / "output" / "version_diff.txt"

# The committed `(hanzi, tl)` key set: written here on every dictionary build,
# read back out of a release tag to diff against. Lives outside `output/`
# because `output/` holds build artifacts that are not committed.
KEYS_FILE = BASE_DIR / "word-keys.tsv"
TRACKED_KEYS = "dictionary/word-keys.tsv"
KEYS_HEADER = "hanzi\ttl"

SAMPLE_ROWS = 30
# Release tags are 3-segment semver only; a 4-segment tag is ignored on purpose.
VERSION_RE = re.compile(r"^v\d+\.\d+\.\d+$")

WordKey = tuple[str, str]


def _norm(text: str) -> str:
    """NFC-normalize so the diff stays stable across NFC/NFD source drift."""
    return unicodedata.normalize("NFC", text or "")


def parse_version(tag: str) -> tuple[int, ...]:
    """`v3.10.0` → (3, 10, 0) for semantic-version sorting (not string sort)."""
    return tuple(int(p) for p in tag.lstrip("v").split("."))


def _keyset_from_df(df, *, strict: bool) -> set[WordKey]:
    """Extract the `(hanzi, tl)` keyset, NFC-normalized.

    `tl` is kept verbatim — official entries retain spaces (e.g. "m̄ bat") while
    supplements use hyphens, so a rare space↔hyphen spelling flip surfaces as a
    real add+remove rather than being hidden. Exact duplicate keys collapse into
    one entry (multiple source/variant rows share a key).

    `strict=True` (current build) fails fast on a tab/newline in a key field —
    a real data-integrity bug. `strict=False` (previous release loaded from git)
    is best-effort: historical data is only set-diffed, never re-serialized, so a
    stray control char is harmless.
    """
    keys: set[WordKey] = set()
    for hanzi, tl in zip(df["hanzi"].fillna(""), df["tl"].fillna("")):
        hanzi_n, tl_n = _norm(str(hanzi)), _norm(str(tl))
        if strict and any(c in hanzi_n or c in tl_n for c in ("\t", "\n", "\r")):
            sys.exit(
                f"[ERROR] tab/newline in key field: hanzi={hanzi_n!r} tl={tl_n!r}"
            )
        keys.add((hanzi_n, tl_n))
    return keys


def load_current_keyset() -> set[WordKey]:
    """Current build's `(hanzi, tl)` keyset from output/dictionary.csv."""
    return _keyset_from_df(read_dictionary_csv(INPUT_FILE), strict=True)


def _git_tags() -> list[str]:
    """Release tags (3-segment semver only), oldest→newest. `[]` on any failure."""
    try:
        out = subprocess.run(
            ["git", "tag"], cwd=BASE_DIR, capture_output=True, text=True, check=True
        ).stdout
    except (OSError, subprocess.SubprocessError):
        return []
    tags = [t for t in out.splitlines() if VERSION_RE.match(t)]
    return sorted(tags, key=parse_version)


def resolve_prev_tag(target: str | None) -> str | None:
    """Newest release tag to diff against, or None when none qualifies.

    Release mode (`target` given): newest tag strictly older than `target`.
    Report-only (`target` None): newest release tag overall.
    """
    tags = _git_tags()
    if target is not None:
        tags = [t for t in tags if parse_version(t) < parse_version(target)]
    return tags[-1] if tags else None


def write_keyset(keys: set[WordKey]) -> None:
    """Write the sorted key set to the committed `dictionary/word-keys.tsv`.

    Sorted so the file is a stable diff: a rebuild that changes no words
    produces a byte-identical file and no commit.
    """
    lines = [KEYS_HEADER]
    lines.extend(f"{hanzi}\t{tl}" for hanzi, tl in sorted(keys))
    KEYS_FILE.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _parse_keys_tsv(text: str) -> set[WordKey]:
    """Parse `word-keys.tsv` content. Rows that are not exactly two fields are skipped."""
    keys: set[WordKey] = set()
    for line in text.splitlines():
        if not line or line == KEYS_HEADER:
            continue
        parts = line.split("\t")
        if len(parts) != 2:
            continue
        keys.add((_norm(parts[0]), _norm(parts[1])))
    return keys


def load_keyset_from_git(tag: str) -> set[WordKey] | None:
    """`(hanzi, tl)` keyset from a release tag's committed word-keys.tsv.

    Best-effort: any git / decode / parse failure returns None so the
    report-only diff degrades to "nothing to diff" instead of failing the build.
    Tags cut before this file existed resolve that way, which is expected once
    and self-heals at the release after it.
    """
    try:
        result = subprocess.run(
            ["git", "show", f"{tag}:{TRACKED_KEYS}"],
            cwd=BASE_DIR,
            capture_output=True,
        )
        if result.returncode != 0:
            return None
        return _parse_keys_tsv(result.stdout.decode("utf-8"))
    except (OSError, subprocess.SubprocessError, UnicodeDecodeError, ValueError):
        return None


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


def print_version_diff(
    current: set[WordKey], prev: set[WordKey] | None, prev_tag: str | None
) -> None:
    print()
    print("=" * 60)
    print("Version diff (entries = (hanzi, tl) pairs)")
    print("=" * 60)
    if prev is None:
        print("  no previous release available — nothing to diff")
        return

    base_label = f"{prev_tag}:{TRACKED_KEYS}"
    added = sorted(current - prev)
    removed = sorted(prev - current)
    print(f"  vs {base_label}: +{len(added)} added, -{len(removed)} removed")

    def _sample(label: str, items: list[WordKey]) -> None:
        if not items:
            return
        print(f"\n  {label} (first {min(SAMPLE_ROWS, len(items))}):")
        for h, t in items[:SAMPLE_ROWS]:
            print(f"    {h}\t{t}")

    _sample("added", added)
    _sample("removed", removed)

    # Full lists to an ephemeral file (gitignored) for the maintainer to read.
    out = [f"# diff vs {base_label}", f"# added: {len(added)}  removed: {len(removed)}", ""]
    out.append("## added")
    out.extend(f"{h}\t{t}" for h, t in added)
    out.append("")
    out.append("## removed")
    out.extend(f"{h}\t{t}" for h, t in removed)
    DIFF_FILE.write_text("\n".join(out) + "\n", encoding="utf-8")
    print(f"\n  full lists: {DIFF_FILE.relative_to(BASE_DIR)}")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--version",
        default=None,
        help="release target vX.Y.Z; diff base = newest tag strictly older than it",
    )
    args = ap.parse_args()

    if not INPUT_FILE.exists():
        print(f"[ERROR] {INPUT_FILE} not found — run merge_csv first")
        return 1

    version = args.version
    if version is not None and not VERSION_RE.match(version):
        print(f"[ERROR] malformed version {version!r} (want vMAJOR.MINOR.PATCH)")
        return 1

    print_drop_summary()
    current = load_current_keyset()
    write_keyset(current)

    prev_tag = resolve_prev_tag(version)
    prev = load_keyset_from_git(prev_tag) if prev_tag else None
    if version is not None and prev is None:
        # Release mode expects a predecessor to review against. Warn loudly
        # (but don't halt — report-only invariant) whenever the base is missing,
        # whether no tag resolved or a resolved tag's dictionary.csv could not be
        # read (first release, shallow clone, or a tags-less checkout).
        print(
            f"[WARN] release {version}: previous release dictionary unavailable "
            "(first release, shallow clone, or missing tags/blob?) — version diff "
            "skipped. Run `git fetch --tags` for the added/removed review."
        )
    print_version_diff(current, prev, prev_tag)
    return 0


if __name__ == "__main__":
    sys.exit(main())

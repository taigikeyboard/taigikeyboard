#!/usr/bin/env python3
"""Every `INVARIANT_*` label a living doc names must appear in a test.

`.claude/rules/cross-platform-alignment.md` §3a: the test that pins a documented
cross-platform behaviour carries its label, case-insensitive — in the test's
name (Rust `fn invariant_<label>_…`, XCTest `func testINVARIANT_<label>_…`,
JUnit `fun INVARIANT_<label>_…`) or in a comment above the tests
(`// INVARIANT_<LABEL> (§35)`). Test code = a file under a test directory, or
the part of a Rust source file after its first `#[cfg(test)]`. A mention in
production code does not count. A label ending in `_` (written
`INVARIANT_foo_*` in prose) names a family: any mention starting with it counts.

Labels no test names yet sit in `tools/invariant_labels_pending.txt`, a baseline
that only shrinks: an entry whose label gained a test fails the gate until it is
deleted. Only the doc → test direction is checked. Dated snapshots (`docs/reports/`,
`docs/releases/`) and the incident log are history, not contracts.

Usage: python3 tools/invariant_labels.py   (exit 1 lists every label to fix)
"""

from __future__ import annotations

import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

DOC_GLOBS = (
    "docs/architecture/*.md",
    "docs/engine/*.md",
    "docs/ui/*.md",
    ".claude/rules/*.md",
)
DOC_EXCLUDES = {"docs/architecture/incident-log.md"}
TEST_SUFFIXES = (".rs", ".swift", ".kt")
TEST_ROOTS = ("engine", "desktop", "windows", "linux", "ios", "macos", "android")

LABEL = re.compile(r"`INVARIANT_([A-Za-z0-9_]+?)(_?\*)?`")
MENTION = re.compile(r"INVARIANT_([A-Za-z0-9_]+)", re.IGNORECASE)
TEST_DIR = re.compile(r"(^|/)(tests?|[A-Za-z]*Tests)/")
RUST_TEST_MODULE = "#[cfg(test)]"
PENDING = ROOT / "tools" / "invariant_labels_pending.txt"


@dataclass(frozen=True)
class Label:
    name: str  # lower-case, without the INVARIANT_ prefix
    family: bool
    path: str
    line: int  # of the first mention


def labels_in(rel: str, text: str) -> list[Label]:
    """Each distinct label a doc names, at its first mention."""
    found: dict[tuple[str, bool], Label] = {}
    for number, line in enumerate(text.splitlines(), 1):
        for match in LABEL.finditer(line):
            raw, star = match.group(1), match.group(2)
            family = bool(star) or raw.endswith("_")
            name = raw.rstrip("_").lower()
            found.setdefault((name, family), Label(name, family, rel, number))
    return list(found.values())


def doc_labels(root: Path) -> list[Label]:
    labels: dict[tuple[str, bool], Label] = {}
    for pattern in DOC_GLOBS:
        for path in sorted(root.glob(pattern)):
            rel = path.relative_to(root).as_posix()
            if rel in DOC_EXCLUDES:
                continue
            for label in labels_in(rel, path.read_text(encoding="utf-8")):
                labels.setdefault((label.name, label.family), label)
    return sorted(labels.values(), key=lambda label: (label.path, label.line))


def test_code(rel: str, text: str) -> str:
    """The part of a source file that is test code ("" when none)."""
    if TEST_DIR.search(rel):
        return text
    if rel.endswith(".rs") and RUST_TEST_MODULE in text:
        return text[text.index(RUST_TEST_MODULE) :]
    return ""


def test_mentions(root: Path) -> set[str]:
    listed = subprocess.run(
        ["git", "-C", str(root), "ls-files", "--", *TEST_ROOTS],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    names: set[str] = set()
    for rel in listed:
        if not rel.endswith(TEST_SUFFIXES):
            continue
        text = test_code(rel, (root / rel).read_text(encoding="utf-8", errors="ignore"))
        names.update(match.group(1).lower() for match in MENTION.finditer(text))
    return names


def is_covered(label: Label, names: set[str]) -> bool:
    if label.family:
        return any(name.startswith(label.name) for name in names)
    return any(
        name == label.name or name.startswith(label.name + "_") for name in names
    )


def unmatched(labels: list[Label], names: set[str]) -> list[Label]:
    return [label for label in labels if not is_covered(label, names)]


def pending_key(label: Label) -> str:
    """How a label is written in the pending baseline: `name`, or `name_*` for a family."""
    return f"{label.name}_*" if label.family else label.name


def read_pending(text: str) -> set[str]:
    entries = (line.split("#", 1)[0].strip().lower() for line in text.splitlines())
    return {entry for entry in entries if entry}


def check(missing: list[Label], pending: set[str]) -> tuple[list[Label], list[str]]:
    """(labels with no test and no baseline entry, baseline entries to delete)."""
    missing_keys = {pending_key(label) for label in missing}
    new = [label for label in missing if pending_key(label) not in pending]
    stale = sorted(pending - missing_keys)
    return new, stale


def main() -> int:
    labels = doc_labels(ROOT)
    missing = unmatched(labels, test_mentions(ROOT))
    new, stale = check(missing, read_pending(PENDING.read_text(encoding="utf-8")))
    for label in new:
        print(
            f"✗ {label.path}:{label.line}: INVARIANT_{pending_key(label)} is named in no test"
        )
    for key in stale:
        print(
            f"✗ {PENDING.relative_to(ROOT)}: `{key}` has a test now (or left the docs) — delete its line"
        )
    if new or stale:
        print(
            "Name the label in the test that pins it (test name or comment), or drop the label "
            "if the behaviour is gone (.claude/rules/cross-platform-alignment.md §3a)."
        )
        return 1
    print(
        f"✓ {len(labels)} INVARIANT labels: {len(labels) - len(missing)} named in a test, "
        f"{len(missing)} in the pending baseline"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

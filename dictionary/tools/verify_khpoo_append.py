#!/usr/bin/env python3
"""Check the lines a change appends to the khpoo raw CSV (齒盤補充辭典).

Contributions from the accent-variation site append `hanzi,tl` lines to
`sources/supplementary/khpoo/data/raw/khpoo.csv`. The maintainer reviews each
TL by hand (docs/architecture/khiunnkhau-contrib-roadmap.md D6), so this gate
only rejects what review should never see:

- an edit to an existing line (contributions only append);
- a malformed line: not CRLF-terminated, quotes or a byte-order mark, not
  exactly two non-empty fields, or not already in the form khpoo's
  `cleanup.minimal` stage would produce (`common.cleanup.normalize_roman`:
  NFC, lowercase, space → hyphen);
- a duplicate `(hanzi, tl)`: already in the base file, already in
  `output/dictionary.csv` (any source), or added twice.

Run from `dictionary/`:
    python3 tools/verify_khpoo_append.py --base <git-ref>
"""

from __future__ import annotations

import argparse
import csv
import subprocess
import sys
import unicodedata
from collections.abc import Iterable
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(BASE_DIR))

from common.cleanup import normalize_roman

KHPOO_RAW = BASE_DIR / "sources" / "supplementary" / "khpoo" / "data" / "raw" / "khpoo.csv"
DICTIONARY_CSV = BASE_DIR / "output" / "dictionary.csv"
REPO_ROOT = BASE_DIR.parent


def split_lines(text: str) -> list[str]:
    """Split on LF only, keeping each line's terminator, so a missing CR stays visible."""
    lines = text.split("\n")
    tail = lines.pop()
    return [line + "\n" for line in lines] + ([tail] if tail else [])


def appended_lines(base_text: str, head_text: str) -> list[tuple[int, str]] | None:
    """(1-based head line number, line) for each line after the base, or None when
    the head does not start with the base (an existing line was edited or removed)."""
    if not head_text.startswith(base_text):
        return None
    first = base_text.count("\n") + 1
    return list(enumerate(split_lines(head_text[len(base_text) :]), start=first))


def row_key(hanzi: str, tl: str) -> tuple[str, str]:
    """The (hanzi, tl) word identity. `normalize_roman` also turns spaces into hyphens, so
    `dictionary.csv`'s spaced TL (`bô kâng`, preserve_spaces sources) keys as `bô-kâng`,
    the pair `build/merge_csv.py`'s `_tl_key` merges it into."""
    return unicodedata.normalize("NFC", hanzi.strip()), normalize_roman(tl.strip())


def keys_of(pairs: Iterable[tuple[str, str]]) -> set[tuple[str, str]]:
    return {row_key(hanzi, tl) for hanzi, tl in pairs}


def check_added(
    added: list[tuple[int, str]],
    khpoo_keys: set[tuple[str, str]],
    dictionary_keys: set[tuple[str, str]],
) -> list[str]:
    errors = []
    seen: set[tuple[str, str]] = set()
    for number, line in added:
        where = f"line {number}"
        if not line.endswith("\r\n"):
            errors.append(f"{where}: must end with CRLF, as every khpoo line does")
            continue
        if '"' in line or "\ufeff" in line:
            errors.append(f"{where}: no quotes or byte-order mark; khpoo lines are plain `hanzi,tl`")
            continue
        fields = line[:-2].split(",")
        if len(fields) != 2 or not all(fields):
            errors.append(f"{where}: expected two non-empty fields `hanzi,tl`, got {line[:-2]!r}")
            continue
        hanzi, tl = fields
        key = row_key(hanzi, tl)
        if (hanzi, tl) != key:
            errors.append(f"{where}: write `{key[0]},{key[1]}` (NFC, lowercase TL, hyphens for spaces)")
            continue
        if key in khpoo_keys:
            errors.append(f"{where}: `{hanzi},{tl}` is already in khpoo")
        elif key in dictionary_keys:
            errors.append(f"{where}: `{hanzi},{tl}` is already in the dictionary")
        elif key in seen:
            errors.append(f"{where}: `{hanzi},{tl}` is added twice")
        seen.add(key)
    return errors


def git_show(ref: str, path: Path) -> str:
    spec = f"{ref}:{path.relative_to(REPO_ROOT).as_posix()}"
    result = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "show", spec], capture_output=True, check=False
    )
    if result.returncode != 0:
        sys.exit(f"cannot read {spec}: {result.stderr.decode().strip()} (is the base commit fetched?)")
    return result.stdout.decode("utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--base", required=True, help="git ref to diff the khpoo raw file against")
    args = parser.parse_args()

    label = KHPOO_RAW.relative_to(REPO_ROOT)
    base_text = git_show(args.base, KHPOO_RAW)
    added = appended_lines(base_text, KHPOO_RAW.read_bytes().decode("utf-8"))
    if added is None:
        print(
            f"{label}: existing lines changed, but contributions only append. An edit to"
            " existing rows needs its own PR, which the maintainer reviews and merges by hand.",
            file=sys.stderr,
        )
        return 1
    if not added:
        print("khpoo append check: no appended lines")
        return 0

    khpoo_keys = keys_of(row[:2] for row in csv.reader(base_text.splitlines()) if len(row) >= 2)
    with DICTIONARY_CSV.open(encoding="utf-8", newline="") as f:
        dictionary_keys = keys_of((row["hanzi"], row["tl"]) for row in csv.DictReader(f))
    errors = check_added(added, khpoo_keys, dictionary_keys)
    for error in errors:
        print(f"{label}: {error}", file=sys.stderr)
    print(f"khpoo append check: {len(added)} appended line(s), {len(errors)} error(s)")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())

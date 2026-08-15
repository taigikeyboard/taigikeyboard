#!/usr/bin/env python3
# Fails if any committed generated i18n artifact is stale vs the i18n/*.json sources.
#
# Generates outputs in memory (no worktree mutation) and compares against the on-disk tree.
# Run by the Gradle `checkI18nGenerated` task (preBuild) so Android Studio / archive / direct `assemble`
# builds cannot link against stale committed output. Run by hand with `python3 tools/i18n/check.py`.

import json
import sys
from pathlib import Path

from i18n_lib import IOS_XCSTRINGS, build_outputs

REPO_ROOT = Path(__file__).resolve().parents[2]


def _xcstrings_is_stale(expected: str, actual_bytes: bytes) -> bool:
    # The String Catalog is the one generated file Xcode itself writes to: every build re-extracts
    # string literals from Swift source into it and rewrites the whole file in Xcode's own key order.
    # A byte comparison therefore fails after any Xcode build even though no generated string changed,
    # which made the gate cry wolf and block Android builds. Compare semantics instead: every
    # generator-owned key must be present with an identical payload. Xcode's re-sort is ignored, and
    # its auto-extracted literals (`"%@"`, `"•"`, …) are tolerated as additive noise — `make i18n`
    # drops them on the next canonical regen.
    #
    # Set `SWIFT_EMIT_LOC_STRINGS = NO` in the Xcode project to stop the extraction at the source;
    # this check stays tolerant either way.
    try:
        actual = json.loads(actual_bytes)
        wanted = json.loads(expected)
    except (json.JSONDecodeError, UnicodeDecodeError):
        return True
    if actual.get("sourceLanguage") != wanted.get("sourceLanguage") or actual.get("version") != wanted.get("version"):
        return True
    actual_strings = actual.get("strings", {})
    return any(actual_strings.get(key) != payload for key, payload in wanted.get("strings", {}).items())


def main() -> int:
    try:
        outputs = build_outputs(REPO_ROOT, enforce_production_completeness=True)
    except ValueError as exc:
        print(f"i18n: {exc}", file=sys.stderr)
        return 1
    stale = []
    for rel_path, content in outputs.items():
        target = REPO_ROOT / rel_path
        if not target.exists():
            stale.append(rel_path)
            continue
        actual_bytes = target.read_bytes()
        if rel_path == IOS_XCSTRINGS:
            if _xcstrings_is_stale(content, actual_bytes):
                stale.append(rel_path)
        # Byte comparison (not read_text, which normalizes newlines) so CRLF drift is caught.
        elif actual_bytes != content.encode("utf-8"):
            stale.append(rel_path)
    if stale:
        print("i18n: generated output is stale. Run `make i18n`. Affected:", file=sys.stderr)
        for rel_path in stale:
            print(f"  - {rel_path}", file=sys.stderr)
        return 1
    print(f"i18n: {len(outputs)} generated artifacts up to date")
    return 0


if __name__ == "__main__":
    sys.exit(main())

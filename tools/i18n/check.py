#!/usr/bin/env python3
# Fails if any committed generated i18n artifact is stale vs the i18n/*.json sources.
#
# Generates outputs in memory (no worktree mutation) and byte-compares against the on-disk tree.
# Run by the Gradle `checkI18nGenerated` task (preBuild) so Android Studio / archive / direct `assemble`
# builds cannot link against stale committed output. Run by hand with `python3 tools/i18n/check.py`.

import sys
from pathlib import Path

from i18n_lib import build_outputs

REPO_ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    try:
        outputs = build_outputs(REPO_ROOT, enforce_production_completeness=True)
    except ValueError as exc:
        print(f"i18n: {exc}", file=sys.stderr)
        return 1
    stale = []
    for rel_path, content in outputs.items():
        target = REPO_ROOT / rel_path
        # Byte comparison (not read_text, which normalizes newlines) so CRLF drift is caught.
        if not target.exists() or target.read_bytes() != content.encode("utf-8"):
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

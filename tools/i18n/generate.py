#!/usr/bin/env python3
# Writes every generated i18n artifact from the canonical i18n/*.json sources. Run via `make i18n`.

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
    for rel_path, content in outputs.items():
        target = REPO_ROOT / rel_path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
        print(f"i18n: wrote {rel_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

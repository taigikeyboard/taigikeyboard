#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Post-build smoke verification: assert known prefixes hit minimum counts.

Reads `tools/known_keys_fixture.yaml` (a flat `prefix: min_hits` map),
queries each against `output/dictionary.fst` via the `fst-builder` Rust
binary, and exits non-zero if any prefix returns fewer hits than asserted.

Deliberately `output/`, not the committed `dictionaries/`: `build.sh` runs this
before `deploy`, so it has to gate the index this build just produced. Querying
the deployed copy would pass on the previous build's artifacts. That also means
a standalone run needs `make dict` first — the same requirement
`tools/compare_baseline.py` has, and for the same reason.

Defends against the PR #175 + PR #184 stale-POJ class of bug — when the
trie ships missing `poj:` keys, POJ-mode users silently lose ~5 % of
candidates and only end-user reports surface the regression. This script
is the build-time tripwire.

Wired into `dictionary/build.sh` after `build.audit` and before
`build/deploy.sh`. Run standalone:

    python3 -m tools.verify_known_keys              # uses default FST path
    python3 -m tools.verify_known_keys --fst PATH   # override
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

import yaml

BASE_DIR = Path(__file__).resolve().parent.parent
DEFAULT_FST = BASE_DIR / "output" / "dictionary.fst"
FIXTURE_FILE = Path(__file__).resolve().parent / "known_keys_fixture.yaml"
ENGINE_DIR = BASE_DIR.parent / "engine"
BUILDER_RELEASE = ENGINE_DIR / "target" / "release" / "fst-builder"
BUILDER_DEBUG = ENGINE_DIR / "target" / "debug" / "fst-builder"


def resolve_builder_bin() -> Path:
    if BUILDER_RELEASE.exists():
        return BUILDER_RELEASE
    if BUILDER_DEBUG.exists():
        return BUILDER_DEBUG
    cargo = shutil.which("cargo")
    if not cargo:
        sys.exit(
            "fst-builder binary missing and `cargo` not in PATH; build it manually "
            f"with `cargo build --release -p fst-builder` from {ENGINE_DIR}"
        )
    subprocess.run(
        [cargo, "build", "--release", "-p", "fst-builder"],
        cwd=ENGINE_DIR,
        check=True,
    )
    return BUILDER_RELEASE


def count_prefix_hits(builder_bin: Path, fst_path: Path, prefix: str) -> int:
    """Run `fst-builder query` and count result lines."""
    proc = subprocess.run(
        [str(builder_bin), "query", str(fst_path), prefix],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        stderr = proc.stderr.decode("utf-8", errors="replace")
        raise RuntimeError(f"fst-builder query failed for {prefix!r}: {stderr}")
    output = proc.stdout.decode("utf-8", errors="replace")
    return sum(1 for line in output.splitlines() if line.strip())


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Verify FST contains expected key coverage.")
    p.add_argument("--fst", type=Path, default=DEFAULT_FST, help="Path to dictionary.fst")
    p.add_argument("--fixture", type=Path, default=FIXTURE_FILE, help="Path to fixture YAML")
    return p.parse_args()


def main() -> None:
    args = parse_args()

    if not args.fst.exists():
        sys.exit(f"fst not found: {args.fst} — run dictionary/build.sh first")
    if not args.fixture.exists():
        sys.exit(f"fixture not found: {args.fixture}")

    with args.fixture.open(encoding="utf-8") as f:
        raw = yaml.safe_load(f)
    if not isinstance(raw, dict) or not raw:
        sys.exit(f"fixture must be a non-empty mapping of prefix→min_hits: {args.fixture}")
    fixture: dict[str, int] = {}
    for prefix, min_hits in raw.items():
        if not isinstance(prefix, str) or not isinstance(min_hits, int) or min_hits < 0:
            sys.exit(
                f"fixture entry must be `<str>: <non-negative int>`; got "
                f"{prefix!r}: {min_hits!r} ({type(min_hits).__name__})"
            )
        fixture[prefix] = min_hits

    builder_bin = resolve_builder_bin()
    failures: list[str] = []
    try:
        display_fst = args.fst.relative_to(BASE_DIR.parent)
    except ValueError:
        # Custom --fst path outside the repo — show absolute.
        display_fst = args.fst
    print(f"Verifying {len(fixture)} fixture entries against {display_fst}")
    print("─" * 60)

    for prefix, min_hits in fixture.items():
        actual = count_prefix_hits(builder_bin, args.fst, prefix)
        ok = actual >= min_hits
        marker = "✓" if ok else "✗"
        print(f"  {marker} {prefix:24s} {actual:>6} hits (min {min_hits})")
        if not ok:
            failures.append(f"{prefix}: got {actual}, expected ≥ {min_hits}")

    print("─" * 60)
    if failures:
        print(f"FAILED — {len(failures)} fixture entries below minimum:")
        for f in failures:
            print(f"  • {f}")
        sys.exit(1)
    print(f"OK — all {len(fixture)} fixture entries meet minimums.")


if __name__ == "__main__":
    main()

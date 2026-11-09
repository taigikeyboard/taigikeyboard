# -*- coding: utf-8 -*-
"""Run build.sh steps 1–8 in one process.

Each step used to be its own `python3 -m build.<step>` process, so pandas was
imported eight times, five steps re-parsed the same 39 MB dictionary.csv into
records, and every step started a fresh Node converter with an empty reply
cache. In one process `load_dictionary_records` parses the CSV once and the
converter cache carries from step to step. Every step module keeps its own
`main` and still runs on its own.

Usage (from dictionary/, as build.sh does): `python3 -m build.run [--version vX.Y.Z]`
"""

from __future__ import annotations

import argparse
import sys

from build import (
    create_association_bin,
    create_dictionary_bin,
    create_fst,
    create_syllables_fst,
    merge_csv,
    verify_poj_integrity,
    version_snapshot,
)
from tools import verify_known_keys

GREEN = "\033[0;32m"
NC = "\033[0m"
# build.sh runs step 9 (deploy) itself after this module returns.
TOTAL_STEPS = 9


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--version",
        default=None,
        help="release target vX.Y.Z; passed to version_snapshot",
    )
    args = ap.parse_args(argv)
    snapshot_argv = [] if args.version is None else ["--version", args.version]

    steps = [
        ("Merging dictionaries...", merge_csv.main),
        ("Creating dictionary.bin...", lambda: create_dictionary_bin.main(["--verify"])),
        ("Creating fst prefix index...", create_fst.main),
        ("Creating syllables.fst (tl/poj/tps tagged-single-FST)...", create_syllables_fst.main),
        ("Creating association.bin...", lambda: create_association_bin.main(["--verify"])),
        ("Verifying POJ integrity (fatal gate)...", verify_poj_integrity.main),
        ("Drop summary + vs-previous-release-tag diff...", lambda: version_snapshot.main(snapshot_argv)),
        ("Verifying known fst keys...", lambda: verify_known_keys.main([])),
    ]
    # Step logs go to stderr and step reports to stdout; line buffering keeps
    # them in order when the output is piped.
    sys.stdout.reconfigure(line_buffering=True)
    for number, (title, run) in enumerate(steps, start=1):
        print(f"\n{GREEN}▶ Step {number}/{TOTAL_STEPS}: {title}{NC}\n")
        status = run()
        if status:
            return status
    return 0


if __name__ == "__main__":
    sys.exit(main())

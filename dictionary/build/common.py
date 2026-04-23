# -*- coding: utf-8 -*-
"""Shared constants + helpers for dictionary build scripts.

Each build script needs the same BASE_DIR / OUTPUT_DIR / logger / build
timestamp, so centralise here. Scripts import from this module instead of
rebuilding the boilerplate prelude.
"""

from __future__ import annotations

import sys
import time
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
OUTPUT_DIR = BASE_DIR / "output"
LOG_DIR = BASE_DIR / "logs"
BUILD_TS_FILE = OUTPUT_DIR / ".build_ts"

# Build scripts live under dictionary/build/; importing common.* requires
# dictionary/ on sys.path. Each build script calls this once at import time.
if str(BASE_DIR) not in sys.path:
    sys.path.insert(0, str(BASE_DIR))


def start_new_build_timestamp() -> int:
    """Record a fresh build timestamp for this run.

    Called once per `build.sh` invocation by `create_dictionary_bin` (the
    first binary-producing step). Overwrites any stale `.build_ts` left
    behind by an earlier run so each build embeds a truthful timestamp.
    """
    ts = int(time.time())
    BUILD_TS_FILE.parent.mkdir(parents=True, exist_ok=True)
    BUILD_TS_FILE.write_text(str(ts))
    return ts


def read_shared_build_timestamp() -> int:
    """Read the timestamp recorded by `start_new_build_timestamp`.

    Used by `create_association_bin` so both binaries embed the same
    `build_ts` header value despite running seconds apart. Errors if the
    writer step did not run first.
    """
    if not BUILD_TS_FILE.exists():
        raise RuntimeError(
            f"{BUILD_TS_FILE} does not exist — "
            "create_dictionary_bin must run before create_association_bin"
        )
    return int(BUILD_TS_FILE.read_text().strip())

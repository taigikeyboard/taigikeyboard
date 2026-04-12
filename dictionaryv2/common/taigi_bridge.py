# -*- coding: utf-8 -*-
"""
taigi-converter Python bridge (via persistent Node.js subprocess)

Replaces kesi dependency. Uses line-based JSON IPC to taigi_batch.js
for TL/POJ conversion and tone number operations.
"""

import json
import os
import subprocess

_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
_BATCH_SCRIPT = os.path.join(_SCRIPT_DIR, "taigi_batch.js")


class TaigiConverter:
    """Bridge to taigi-converter via persistent Node.js subprocess."""

    def __init__(self):
        self._process = None

    def _ensure_started(self):
        if self._process is None or self._process.poll() is not None:
            self._process = subprocess.Popen(
                ["node", _BATCH_SCRIPT],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                text=True,
            )

    def _call(self, op, **kwargs):
        self._ensure_started()
        request = {"op": op, **kwargs}
        self._process.stdin.write(json.dumps(request, ensure_ascii=False) + "\n")
        self._process.stdin.flush()
        line = self._process.stdout.readline()
        if not line:
            raise RuntimeError("taigi-converter subprocess closed unexpectedly")
        return json.loads(line)

    def convert(self, text, source, target):
        """Convert between romanization systems (tl, poj)."""
        result = self._call("convert", text=text, source=source, target=target)
        return result.get("result") or text

    def to_tone_number(self, text, system="tl"):
        """Convert tone marks to tone numbers."""
        result = self._call("toToneNumber", text=text, system=system)
        return result.get("result") or text

    def close(self):
        if self._process:
            self._process.stdin.close()
            self._process.terminate()
            self._process.wait()
            self._process = None


# Module-level singleton — started on first call, reused across all calls
_converter = TaigiConverter()


def convert_tl_to_poj(tl: str) -> str:
    """Convert TL romanization to POJ."""
    try:
        return _converter.convert(tl, "tl", "poj")
    except Exception:
        return tl


def convert_poj_to_tl(poj: str) -> str:
    """Convert POJ romanization to TL."""
    try:
        return _converter.convert(poj, "poj", "tl")
    except Exception:
        return poj


def to_tone_number(text: str, system: str = "tl") -> str:
    """Convert tone marks to tone numbers (e.g. 'hó-sè' → 'ho2-se3')."""
    try:
        return _converter.to_tone_number(text, system)
    except Exception:
        return text

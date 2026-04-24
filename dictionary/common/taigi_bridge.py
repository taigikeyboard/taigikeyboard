# -*- coding: utf-8 -*-
"""taigi-converter Python bridge (via persistent Node.js subprocess).

Line-based JSON IPC to `taigi_batch.js` for TL/POJ conversion, tone-number
operations, and romanization validation.

Two tiers of caller:

- **Graceful** (`convert_tl_to_poj`, `to_tone_number`, `is_valid_romanization`):
  return the input unchanged (or `False` for validation) when the subprocess
  fails. Used by the pipeline `cleanup` stage where per-row conversion drift
  is tolerable.
- **Strict** (`*_strict`): raise `RuntimeError` on subprocess / JS failure so
  the caller can decide to skip a row. Used by `build/merge_csv.py`
  supplement loaders.

Subprocess failures are logged once per process with stderr attached so
silent drift is visible to maintainers without spamming for every row.
"""

from __future__ import annotations

import json
import logging
import subprocess
import unicodedata
from pathlib import Path
from subprocess import CalledProcessError
from typing import Any, TextIO

_SCRIPT_DIR = Path(__file__).resolve().parent
_BATCH_SCRIPT = _SCRIPT_DIR / "taigi_batch.js"

_logger = logging.getLogger(__name__)

# Suppress per-row spam; log stderr once per distinct failure reason.
_logged_failures: set[str] = set()

# Errors that legitimately mean "subprocess or IPC failed" — narrow wrap here
# so a surprise ValueError/AttributeError inside bridge methods still surfaces.
_BRIDGE_FAILURES = (
    OSError,
    json.JSONDecodeError,
    CalledProcessError,
    RuntimeError,
)

# MOE 教育部造字碼 → Unicode 正式碼位（from KeSi normalize_kautian）
_MOE_CHAR_MAP = {
    "": "\U0002A736",  # 𪜶
    "": "\U0002B74F",  # 𫝏
    "": "\U0002B75B",  # 𫝛
    "": "\U0002B77A",  # 𫝺
    "": "\U0002B77B",  # 𫝻
    "": "\U0002B7BC",  # 𫞼
    "": "\U0002B7C2",  # 𫟂
    "": "\U0002C9B0",  # 𬦰
    "": "\U000308FB",  # 𰣻
}


def _log_bridge_failure(op: str, reason: str, stderr: str | None = None) -> None:
    key = f"{op}:{reason}"
    if key in _logged_failures:
        return
    _logged_failures.add(key)
    msg = f"taigi-converter {op!r} failed: {reason}"
    if stderr:
        msg += f"\n  stderr: {stderr.strip()}"
    _logger.warning(msg)


class TaigiConverter:
    """Bridge to taigi-converter via persistent Node.js subprocess."""

    def __init__(self) -> None:
        self._process: subprocess.Popen[str] | None = None
        self._stderr_path = Path("/tmp") / f"taigi_batch.stderr.{id(self)}.log"
        self._stderr_file: TextIO | None = None

    def _ensure_started(self) -> None:
        if self._process is None or self._process.poll() is not None:
            # Capture stderr to a file so bridge failures can surface it via
            # `_log_bridge_failure(..., stderr=...)` instead of /dev/null-ing it.
            # Keep the handle as an attribute so close() can release it.
            if self._stderr_file is not None:
                self._stderr_file.close()
            self._stderr_file = open(self._stderr_path, "w", encoding="utf-8")
            self._process = subprocess.Popen(
                ["node", str(_BATCH_SCRIPT)],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=self._stderr_file,
                text=True,
            )

    def _tail_stderr(self) -> str | None:
        if not self._stderr_path.exists():
            return None
        try:
            return self._stderr_path.read_text(encoding="utf-8")[-2048:]
        except OSError:
            return None

    def _call(self, op: str, **kwargs: Any) -> dict:
        self._ensure_started()
        assert self._process is not None and self._process.stdin is not None
        assert self._process.stdout is not None
        request = {"op": op, **kwargs}
        self._process.stdin.write(json.dumps(request, ensure_ascii=False) + "\n")
        self._process.stdin.flush()
        line = self._process.stdout.readline()
        if not line:
            raise RuntimeError("taigi-converter subprocess closed unexpectedly")
        return json.loads(line)

    def convert(self, text: str, source: str, target: str) -> str:
        """Convert between romanization systems (tl, poj)."""
        result = self._call("convert", text=text, source=source, target=target)
        return result.get("result") or text

    def to_tone_number(self, text: str) -> str:
        """Convert tone marks to tone numbers (preserves POJ non-ASCII o͘/ⁿ)."""
        result = self._call("toToneNumber", text=text)
        return result.get("result") or text

    def to_tone_number_ascii(self, text: str) -> str:
        """Convert tone marks to tone numbers with POJ ASCII folding (o͘→oo, ⁿ→nn)."""
        result = self._call("toToneNumberAscii", text=text)
        return result.get("result") or text

    def close(self) -> None:
        if self._process:
            if self._process.stdin:
                self._process.stdin.close()
            self._process.terminate()
            self._process.wait()
            self._process = None
        if self._stderr_file is not None:
            self._stderr_file.close()
            self._stderr_file = None


# Module-level singleton — started on first call, reused across all calls.
_converter = TaigiConverter()


def convert_tl_to_poj(tl: str) -> str:
    """Convert TL romanization to POJ. Graceful — returns input on failure."""
    try:
        return _converter.convert(tl, "tl", "poj")
    except _BRIDGE_FAILURES as e:
        _log_bridge_failure("tl→poj", str(e), _converter._tail_stderr())
        return tl


def convert_poj_to_tl(poj: str) -> str:
    """Convert POJ romanization to TL. Graceful — returns input on failure."""
    try:
        return _converter.convert(poj, "poj", "tl")
    except _BRIDGE_FAILURES as e:
        _log_bridge_failure("poj→tl", str(e), _converter._tail_stderr())
        return poj


def _convert_strict(text: str, source: str, target: str) -> str:
    """Strict convert: raise RuntimeError on subprocess / JS failure.

    Used by callers that must skip a row when conversion cannot complete
    (e.g. `build/merge_csv.py` supplement loaders), rather than silently
    writing the untransformed input into the wrong column.
    """
    result = _converter._call("convert", text=text, source=source, target=target)
    if result.get("error"):
        raise RuntimeError(f"taigi-converter {source}→{target} failed: {result['error']}")
    converted = result.get("result")
    if converted is None:
        raise RuntimeError(f"taigi-converter {source}→{target} returned null for {text!r}")
    return converted


def convert_tl_to_poj_strict(tl: str) -> str:
    """Convert TL → POJ. Raises RuntimeError on subprocess / JS failure."""
    return _convert_strict(tl, "tl", "poj")


def convert_poj_to_tl_strict(poj: str) -> str:
    """Convert POJ → TL. Raises RuntimeError on subprocess / JS failure."""
    return _convert_strict(poj, "poj", "tl")


def to_tone_number(text: str) -> str:
    """Convert tone marks to tone numbers (e.g. 'hó-sè' → 'ho2-se3').

    POJ non-ASCII characters (o͘, ⁿ) are preserved. For ASCII-folded POJ
    output suitable for trie storage, use `to_tone_number_ascii` instead.
    """
    try:
        return _converter.to_tone_number(text)
    except _BRIDGE_FAILURES as e:
        _log_bridge_failure("toToneNumber", str(e), _converter._tail_stderr())
        return text


def to_tone_number_ascii(text: str) -> str:
    """Convert POJ tone marks to tone numbers + fold non-ASCII to ASCII.

    Applies the kesi `tsuan_sooji_tiau(ascii=True)` convention:
      - o͘ → oo, O͘ → OO
      - ⁿ → nn, ᴺ → NN

    Used by the pipeline `numtone` stage to populate `poj_num` / `poj_notone`
    columns that downstream trie lookup matches against ASCII user input.
    """
    try:
        return _converter.to_tone_number_ascii(text)
    except _BRIDGE_FAILURES as e:
        _log_bridge_failure("toToneNumberAscii", str(e), _converter._tail_stderr())
        return text


def is_valid_romanization(text: str) -> bool:
    """Check if text is valid TL/POJ romanization (phonological validation)."""
    try:
        result = _converter._call("validate", text=text)
        return result.get("result", False)
    except _BRIDGE_FAILURES as e:
        _log_bridge_failure("validate", str(e), _converter._tail_stderr())
        return False


def normalize_taibun(text: str) -> str:
    """Unicode NFC normalization + MOE 教育部造字碼 remapping."""
    for pua, uni in _MOE_CHAR_MAP.items():
        text = text.replace(pua, uni)
    return unicodedata.normalize("NFC", text)

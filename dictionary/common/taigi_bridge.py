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
import re
import subprocess
import unicodedata
from pathlib import Path
from subprocess import CalledProcessError
from typing import Any, TextIO

_SCRIPT_DIR = Path(__file__).resolve().parent
_BATCH_SCRIPT = _SCRIPT_DIR / "taigi_batch.js"
# taigi_batch.js:8-9 imports these two out of the taigi-converter submodule. A
# clone made without --recurse-submodules leaves that directory empty, node
# exits on the failed import, and every conversion in the run comes back as an
# error string — which the pipeline then treats as data. Naming the real cause
# here is the difference between "run git submodule update --init" and chasing a
# TypeError in cleanup.py 200 lines downstream.
_CONVERTER_ENTRY_POINTS = (
    _SCRIPT_DIR.parent.parent / "taigi-converter" / "src" / "converter.js",
    _SCRIPT_DIR.parent.parent / "taigi-converter" / "src" / "phonetics.js",
)


class ConverterSubmoduleMissingError(RuntimeError):
    """Raised when the taigi-converter submodule has not been checked out."""


def _require_converter_submodule() -> None:
    missing = [p for p in _CONVERTER_ENTRY_POINTS if not p.exists()]
    if not missing:
        return
    raise ConverterSubmoduleMissingError(
        "taigi-converter submodule is not checked out — "
        + ", ".join(str(p) for p in missing)
        + " missing. Run `git submodule update --init --recursive` (or "
        "`make update-submodules`) and re-run. Without it every romanization "
        "conversion fails and the pipeline silently ingests the error text."
    )

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

# MOE PUA character code -> canonical Unicode code point (from KeSi normalize_kautian)
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
            _require_converter_submodule()
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
        """Send one JSON-line request to the Node subprocess, read one line.

        Every IPC failure surfaces as `BridgeDeadError` so callers can
        catch a single class to detect "the bridge is dead, abort the
        build" — covers stdin pipe death (`OSError` / `BrokenPipeError`),
        unexpected EOF from stdout, and malformed JSON output. Codex
        PR #334 post-impl review #2 caught the original raise that
        only covered the EOF case.
        """
        self._ensure_started()
        assert self._process is not None and self._process.stdin is not None
        assert self._process.stdout is not None
        request = {"op": op, **kwargs}
        try:
            self._process.stdin.write(json.dumps(request, ensure_ascii=False) + "\n")
            self._process.stdin.flush()
            line = self._process.stdout.readline()
        except OSError as exc:
            raise BridgeDeadError(
                f"taigi-converter IPC pipe failed: {exc}"
            ) from exc
        if not line:
            raise BridgeDeadError(
                "taigi-converter subprocess closed unexpectedly"
            )
        try:
            return json.loads(line)
        except json.JSONDecodeError as exc:
            raise BridgeDeadError(
                f"taigi-converter emitted malformed JSON: {exc} (raw: {line!r})"
            ) from exc

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


# Residue after TL→TPS conversion = converter silently dropped a syllable
# boundary (e.g. fused `tai5gi2` only converts `tai5` and leaves `gi2`) OR
# the TL contained dialectal characters with no Bopomofo equivalent
# (e.g. `ṳ` U+1E73, used in some Hokkien sub-dialects). Codex pre-impl
# C-0 Q3: strict bridge must fail rather than ship mixed-shape TPS.
# Allowed code points:
#   - Bopomofo (U+3100–U+312F) + Bopomofo Extended (U+31A0–U+31BF)
#   - Tone marks: U+02C6 ˆ, U+02C7 ˇ, U+02CA ́, U+02CB ̀, U+02D9 ˙,
#     U+02EA ˪, U+02EB ˫, U+0307 combining dot
#   - Whitespace + hyphen (converter joins per-syllable with space;
#     callers strip both before storing)
_TPS_RESIDUE_RE = re.compile(
    "[^㄀-ㄯㆠ-ㆿˆˇˊˋ˙˪˫̇\\s\\-]"
)


class TpsResidueError(RuntimeError):
    """Per-row TL→TPS residue failure — recoverable by skipping the row.

    Distinguished from bare `RuntimeError` so callers can narrow their
    `except` to per-row residue without also swallowing bridge / IPC
    death (which now raises `BridgeDeadError`). Codex PR #334 review
    caught the original catch-too-wide pattern that silently let a
    mid-build bridge death ship a partially-truncated TPS index.
    """
    pass


class BridgeDeadError(RuntimeError):
    """Node subprocess closed unexpectedly mid-IPC — fail loud.

    Raised by `_call` when `stdout.readline()` returns an empty
    string (process exited). Distinguished from bare `RuntimeError`
    (per-row JS-side conversion errors) and `TpsResidueError`
    (per-row residue) so callers can `except BridgeDeadError: raise`
    before any broader `except Exception` block that would otherwise
    silently swallow it. Codex PR #334 post-impl review flagged the
    pre-existing supplement-loader broad catches as a complementary
    failure mode: bridge death raised from any `_strict` call inside
    `_assemble_supplement_row` would be swallowed by the loader's
    `except Exception: continue`, producing a partial CSV.
    """
    pass


def convert_tl_to_tps_strict(tl: str) -> str:
    """Convert hyphen/space-separated TL → per-syllable-space-separated TPS.

    Output is `taigi-converter`'s per-syllable join (one syllable per
    hyphenated TL token, separated by ASCII space). Most callers fuse
    with `.replace(" ", "")` before storing in `tps_num` / FST keys
    (FST keys + user input have no separator).

    Raises:
      - `TpsResidueError` (recoverable, per-row): non-Bopomofo residue
        in the converted output (malformed TL that broke per-syllable
        splitting in `taigi-converter/src/converter.js:20-30`, or a
        dialectal source-data character with no Bopomofo equivalent,
        e.g. `ṳ`), or JS-side `error` / null result on this specific
        TL token. Callers catch this subclass to skip the row.
      - `BridgeDeadError` (fail-loud, non-recoverable): Node
        subprocess died mid-IPC (closed stdout, broken pipe, or
        malformed JSON output). Callers MUST NOT catch — a dead
        bridge at build time must abort the whole `make dict` run.

    Callers feed the hyphenated `tl` column, NOT the fused `tl_num`.
    """
    try:
        converted = _convert_strict(tl, "tl", "zhuyin")
    except BridgeDeadError:
        # Fail loud — propagate.
        raise
    except RuntimeError as exc:
        # JS-side per-row conversion failure (`_convert_strict` raises
        # bare RuntimeError for the JS `error` field or null result).
        # Unify with residue rejection under `TpsResidueError` so
        # callers have ONE recoverable class to catch.
        raise TpsResidueError(
            f"taigi-converter tl→zhuyin failed for {tl!r}: {exc}"
        ) from exc
    if _TPS_RESIDUE_RE.search(converted):
        raise TpsResidueError(
            f"taigi-converter tl→zhuyin produced non-Bopomofo residue: {tl!r} → {converted!r} "
            f"— check for dialectal source chars or pass hyphenated TL, not fused tl_num"
        )
    return converted


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
    """Unicode NFC normalization + MOE PUA character remapping."""
    for pua, uni in _MOE_CHAR_MAP.items():
        text = text.replace(pua, uni)
    return unicodedata.normalize("NFC", text)

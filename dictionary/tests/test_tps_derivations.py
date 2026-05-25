"""Smoke tests for v3.5.9 D / C-0 TPS derivation helpers.

Covers the per-syllable bridge (`convert_tl_to_tps_strict`), the
tone-stripper (`remove_tps_tone`), and the per-syllable abbrev
helper (`extract_tps_abbrev`). The persistent `taigi-converter`
Node subprocess is started lazily on the first call, so these tests
exercise the full bridge round-trip just like the build pipeline
does.
"""

from __future__ import annotations

import pytest

from common.abbrev import extract_tps_abbrev
from common.notone import remove_tps_tone
from common.taigi_bridge import BridgeDeadError, TpsResidueError, convert_tl_to_tps_strict


def test_convert_tl_to_tps_basic_two_syllable():
    # Hyphenated TL passes; converter joins per-syllable TPS with a space.
    # `gi` (Hokkien voiced velar) maps to ㆣㄧ (U+31A3 + U+3127), NOT ㄍ.
    out = convert_tl_to_tps_strict("tai5-gi2")
    assert "ㄉ" in out  # ㄉ (initial of `tai`)
    assert "ㆣ" in out  # ㆣ (Hokkien voiced velar initial of `gi`)
    # tone marks must be preserved (5 → ́, 2 → ̀)
    assert "ˊ" in out
    assert "ˋ" in out


def test_convert_tl_to_tps_rejects_fused_tl_num():
    # `tai5gi2` is fused (no hyphen) — converter.js iterates by hyphen,
    # so the second syllable leaks as ASCII residue. Strict bridge
    # raises `TpsResidueError` (recoverable per-row, RuntimeError
    # subclass) so callers can skip the row without also swallowing
    # `BridgeDeadError` (Codex PR #334 review).
    with pytest.raises(TpsResidueError, match="non-Bopomofo residue"):
        convert_tl_to_tps_strict("tai5gi2")


def test_convert_tl_to_tps_rejects_dialectal_u_diaeresis():
    # `ṳ` (U+1E73, Latin small u with diaeresis below) appears in
    # some Hokkien sub-dialect TL rows but has no Bopomofo equivalent.
    # Pipeline must reject so `tps:` keys stay pure-Bopomofo.
    with pytest.raises(TpsResidueError, match="non-Bopomofo residue"):
        convert_tl_to_tps_strict("sṳ")


def test_tps_residue_error_is_runtime_error_subclass():
    # Codex PR #334 review: `TpsResidueError` is a `RuntimeError`
    # subclass so existing `except RuntimeError` callers stay
    # compatible; the narrower hierarchy lets new call sites
    # discriminate per-row residue from bridge / IPC death.
    assert issubclass(TpsResidueError, RuntimeError)


def test_bridge_dead_error_is_runtime_error_but_not_tps_residue():
    # Codex PR #334 post-impl review: `BridgeDeadError` is a sibling
    # of `TpsResidueError` under `RuntimeError`, NOT a subclass of
    # `TpsResidueError`. This means narrow `except TpsResidueError`
    # blocks (per-row recoverable) do not swallow bridge death;
    # callers that want to catch it must use `except BridgeDeadError`
    # explicitly and re-raise to abort the build.
    assert issubclass(BridgeDeadError, RuntimeError)
    assert not issubclass(BridgeDeadError, TpsResidueError)
    assert not issubclass(TpsResidueError, BridgeDeadError)


def test_remove_tps_tone_strips_all_tone_marks():
    # Cover every standalone tone-mark code point from `ZHUYIN_TONES`
    # plus the encode-safe variant and the combining dot for tone-8.
    assert remove_tps_tone("ㄐㄧˋ") == "ㄐㄧ"  # tone-2 ̀
    assert remove_tps_tone("ㄍㄧˊ") == "ㄍㄧ"  # tone-5 ́
    assert remove_tps_tone("ㄅㆤ˪") == "ㄅㆤ"  # tone-3 ˪
    assert remove_tps_tone("ㄅㆤ˫") == "ㄅㆤ"  # tone-7 ˫
    assert remove_tps_tone("ㄅㆤˇ") == "ㄅㆤ"  # tone-6 ˇ
    assert remove_tps_tone("ㄅㆤˆ") == "ㄅㆤ"  # tone-9 ˆ
    assert remove_tps_tone("ㄅㆤㆵ̇") == "ㄅㆤㆵ"  # tone-8 combining
    assert remove_tps_tone("ㄅㆤㆵ˙") == "ㄅㆤㆵ"  # tone-8 encode-safe


def test_remove_tps_tone_strips_hyphens_and_spaces():
    # Multi-syllable + hyphen / converter-emitted space must collapse to
    # fused toneless form for the `tps:<tps_notone>` FST key shape.
    assert remove_tps_tone("ㄐㄧˋ-ㄍㄧˊ") == "ㄐㄧㄍㄧ"
    assert remove_tps_tone("ㄐㄧˋ ㄍㄧˊ") == "ㄐㄧㄍㄧ"


def test_remove_tps_tone_empty():
    assert remove_tps_tone("") == ""


def test_extract_tps_abbrev_two_syllables():
    # `tai5-gi2` → TPS per-syllable [`ㄉㄞˊ`, `ㆣㄧˋ`] → first chars `ㄉ` + `ㆣ`.
    # Per-syllable TPS strings come from the caller (caller invoked
    # `convert_tl_to_tps_strict` per TL syllable) — this helper only
    # picks first chars.
    abbrev = extract_tps_abbrev("tai5-gi2", ["ㄉㄞˊ", "ㆣㄧˋ"])
    assert abbrev == "ㄉㆣ"


def test_extract_tps_abbrev_single_syllable_empty():
    # Single-syllable rows never emit an abbrev — mirrors `extract_abbrev`
    # (and the FST avoids polluting prefix keys with single-char entries).
    assert extract_tps_abbrev("tai5", ["ㄉㄞˊ"]) == ""


def test_extract_tps_abbrev_length_mismatch_empty():
    # Defensive: if caller-side per-syllable count diverges from TL split,
    # return "" rather than producing a half-shaped abbrev.
    assert extract_tps_abbrev("tai5-gi2", ["ㄉㄞˊ"]) == ""


def test_extract_tps_abbrev_empty_input():
    assert extract_tps_abbrev("", []) == ""


def test_extract_tps_abbrev_empty_per_syllable():
    # If any per-syllable TPS is empty (conversion failed mid-row),
    # the abbrev is unreliable — skip rather than emit garbage.
    assert extract_tps_abbrev("tai5-gi2", ["ㄉㄞˊ", ""]) == ""

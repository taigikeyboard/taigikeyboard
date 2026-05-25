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
from common.notone import apply_or_dialect_variant, remove_tps_tone
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


# ----- C-3a er↔or dialect dual-emit variant helper -----------------------


def test_apply_or_dialect_variant_swaps_all_oe_to_o():
    # Bridge collapses TL `er` and `or` to ㄜ; the `or_maps_to_er=false`
    # form renders TL `or` as ㄛ. Substitute every ㄜ in the bridge
    # output to produce the variant key.
    assert apply_or_dialect_variant("ㄍㄜ˪") == "ㄍㄛ˪"
    assert apply_or_dialect_variant("ㄜㆷ") == "ㄛㆷ"
    assert apply_or_dialect_variant("ㄒㄧㄜㆷ") == "ㄒㄧㄛㆷ"


def test_apply_or_dialect_variant_handles_multi_er_or_tokens():
    # Multi-token rows (`ere-ther` style) get the fully-swapped form;
    # partial-mix variants are intentionally out of scope (4 rows total
    # in production data).
    assert apply_or_dialect_variant("ㄜㆤㄊㄜ") == "ㄛㆤㄊㄛ"


def test_apply_or_dialect_variant_empty_when_no_oe():
    # Returns "" when input has no ㄜ so callers can short-circuit the
    # variant FST emit instead of de-duplicating an identical key.
    assert apply_or_dialect_variant("ㄍㄚˋ") == ""
    assert apply_or_dialect_variant("") == ""


def test_apply_or_dialect_variant_preserves_other_glyphs():
    # Only ㄜ (U+311C) is touched. Other Bopomofo + tone marks stay
    # exactly as the bridge emitted them.
    assert apply_or_dialect_variant("ㄗㄜㆤˊ") == "ㄗㄛㆤˊ"
    # ㆦ (U+31A6, `oo` vowel) must NOT be affected — only the er/or
    # collapse glyph U+311C swaps.
    assert apply_or_dialect_variant("ㆦ") == ""


def test_zhuyin_vowels_table_invariant_only_er_or_map_to_oe():
    """C-3a hard invariant: blanket ㄜ→ㄛ substitution in
    `apply_or_dialect_variant` is safe ONLY while `er` and `or` are
    the only TL vowels that produce ㄜ (U+311C) in bridge output.

    If a future `taigi-converter/src/tables.js` edit maps another TL
    vowel to ㄜ, the dual-emit semantics would silently broaden recall
    to that new vowel as well. This test parses the canonical vowel
    table and fails loud on that condition so the C-3a invariant gets
    revisited before shipping the change.
    """
    import re
    from pathlib import Path

    repo_root = Path(__file__).resolve().parents[2]
    tables_path = repo_root / "taigi-converter" / "src" / "tables.js"
    text = tables_path.read_text(encoding="utf-8")

    match = re.search(
        r"export const ZHUYIN_VOWELS\s*=\s*\[(.*?)\];",
        text,
        re.DOTALL,
    )
    assert match, "ZHUYIN_VOWELS export missing from tables.js"
    body = match.group(1)

    # Entries are `["tl", "\\uXXXX..."]`. Pull each (tl, bopomofo) pair.
    entries = re.findall(r'\["([^"]+)",\s*"([^"]+)"\]', body)
    assert entries, "ZHUYIN_VOWELS parsed empty — regex needs update"

    OE_GLYPH = "ㄜ"  # ㄜ
    oe_sources = [tl for tl, bopomofo in entries if OE_GLYPH in bopomofo.encode().decode("unicode_escape")]
    assert sorted(oe_sources) == ["er", "or"], (
        f"C-3a invariant violated: ZHUYIN_VOWELS maps {sorted(oe_sources)!r} to ㄜ, "
        f"expected exactly ['er', 'or']. The `apply_or_dialect_variant` blanket "
        "substitution would now silently broaden recall to additional vowels — "
        "re-examine the C-3a dual-emit design before shipping the bridge change."
    )

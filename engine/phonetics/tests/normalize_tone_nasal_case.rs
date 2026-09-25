//! `api::normalize_tone` integration tests for in-band nasal-marker case
//! agreement: the case of the POJ nasal marker (ⁿ U+207F vs ᴺ U+1D3A)
//! must agree with the case of the preceding letter on the way out of
//! `normalize_tone`. The standalone `adjust_nasal_marker_case` unit
//! tests live alongside the helper in `engine/phonetics/src/case_adjust.rs`;
//! these tests pin the rule end-to-end through the public API.

use phonetics::api::normalize_tone;
use protos::engine::AppConfig;

fn normalize(input: &str, cfg: AppConfig) -> String {
    normalize_tone(input, &cfg)
}

fn poj_doubletap() -> AppConfig {
    AppConfig {
        oo_doubletap_enabled: true,
        nn_doubletap_enabled: true,
        ..poj()
    }
}

fn poj() -> AppConfig {
    AppConfig {
        input_mode: "POJ".to_string(),
        ..Default::default()
    }
}

fn tl() -> AppConfig {
    AppConfig {
        input_mode: "TL".to_string(),
        ..Default::default()
    }
}

#[test]
fn normalize_tone_uppercase_input_promotes_nasal_marker() {
    // POJ `ANN2` with `nn_doubletap_enabled`: preprocess → `A` + `\u{207f}` +
    // `2`, then `to_tone_marks` adds tone diacritic on `A`. Final character
    // sequence has uppercase letters preceding the nasal marker, so the
    // adjustment must promote `\u{207f}` → `\u{1D3A}`.
    let out = normalize("ANN2", poj_doubletap());
    assert!(
        out.contains('\u{1D3A}'),
        "expected uppercase ᴺ in normalize-tone result for uppercase input: {out:?}"
    );
    assert!(
        !out.contains('\u{207F}'),
        "lowercase ⁿ should not appear after case adjustment: {out:?}"
    );
}

#[test]
fn normalize_tone_mixed_case_per_marker_resolution() {
    // Multi-syllable input where one syllable is uppercase and another is
    // lowercase — depends on the inline case adjustment to emit ᴺ for the
    // first marker and ⁿ for the second. `to_tone_marks` alone would emit
    // a single literal codepoint for both; only the post-process produces
    // per-marker case agreement.
    let out = normalize("ANN2-ann2", poj_doubletap());
    assert!(
        out.contains('\u{1D3A}'),
        "expected ᴺ in uppercase syllable: {out:?}"
    );
    assert!(
        out.contains('\u{207F}'),
        "expected ⁿ in lowercase syllable: {out:?}"
    );
}

// -------------------------------------------------------------------------
// Typed case survives tone placement letter by letter (Discord report
// 2026-09-14: Caps Lock `SIANN5` showed `Siâⁿ`). `convert_syllable` places
// the tone on a lowercased copy and restores case via `match_case`, so
// every typed capital comes back, not only the first one.
// -------------------------------------------------------------------------

#[test]
fn normalize_tone_poj_keeps_every_typed_capital() {
    // trace: preprocess "SIANN5" → "SIAⁿ5", base "SIAⁿ", placed on lowered
    // "siâⁿ", match_case → "SIÂⁿ", nasal adjust after capital Â → "SIÂᴺ".
    let cases = [
        ("SIANN5", "SI\u{c2}\u{1d3a}"),
        ("SIAnn5", "SI\u{c2}\u{1d3a}"),
        ("SiAnn5", "Si\u{c2}\u{1d3a}"),
        ("sIann5", "sI\u{e2}\u{207f}"),
        ("Siann5", "Si\u{e2}\u{207f}"),
        ("siann5", "si\u{e2}\u{207f}"),
        ("TAI5-OAN5", "T\u{c2}I-O\u{c2}N"),
        ("HOONN2", "H\u{d3}\u{358}\u{1d3a}"),
        ("O\u{358}2", "\u{d3}\u{358}"),
        ("GOA2", "G\u{d3}A"),
        // Decomposed placements: the combining mark is not a letter, so
        // the letter-aligned restore skips it.
        ("NG5", "N\u{302}G"),
        ("CHIAH8", "CHIA\u{30d}H"),
        ("A9", "\u{102}"),
        // Untouched early returns: tone 1 / 4 and a non-syllable.
        ("TAI1", "TAI1"),
        ("XYZ2", "XYZ2"),
    ];
    for (input, expected) in cases {
        assert_eq!(
            normalize(input, poj_doubletap()),
            expected,
            "input {input:?}"
        );
    }
}

#[test]
fn normalize_tone_poj_doubletap_off_keeps_every_typed_capital() {
    // `nn` stays two letters, each restored on its own: no marker to re-case.
    assert_eq!(normalize("SIANN5", poj()), "SI\u{c2}NN");
    assert_eq!(normalize("HOO2", poj()), "H\u{d3}O");
}

#[test]
fn normalize_tone_english_and_tps_bypass_keep_input() {
    for mode in ["english", "tps"] {
        let cfg = AppConfig {
            input_mode: mode.to_string(),
            ..Default::default()
        };
        assert_eq!(normalize("SIANN5", cfg), "SIANN5", "mode {mode}");
    }
}

#[test]
fn normalize_tone_tl_keeps_every_typed_capital() {
    let cases = [
        ("SIANN5", "SI\u{c2}NN"),
        ("SiAnn5", "Si\u{c2}nn"),
        ("HOO2", "H\u{d3}O"),
        ("A9", "A\u{30b}"),
        ("TSHIAH4", "TSHIAH4"),
    ];
    for (input, expected) in cases {
        assert_eq!(normalize(input, tl()), expected, "input {input:?}");
    }
}

// -------------------------------------------------------------------------
// ⁿ becomes ᴺ in capitals OFF (`AppConfig.force_lowercase_nasal_marker`, USER 2026-09-22, §53):
// the marker is always `ⁿ`, whatever the case of the letters before it.
// -------------------------------------------------------------------------

fn poj_doubletap_lowercase_nasal() -> AppConfig {
    AppConfig {
        force_lowercase_nasal_marker: true,
        ..poj_doubletap()
    }
}

#[test]
fn normalize_tone_force_lowercase_nasal_marker_keeps_the_marker_lowercase_after_a_capital() {
    // trace: "SIANN5" → "SIÂᴺ" by default (`match_case` writes ᴺ after Â);
    // the flag folds that one glyph back and leaves every letter's case.
    let cases = [
        ("SIANN5", "SI\u{c2}\u{207f}"),
        ("SIAnn5", "SI\u{c2}\u{207f}"),
        ("Siann5", "Si\u{e2}\u{207f}"),
        ("siann5", "si\u{e2}\u{207f}"),
        ("HOONN2", "H\u{d3}\u{358}\u{207f}"),
        ("ANN2-ann2", "\u{c1}\u{207f}-\u{e1}\u{207f}"),
        // A typed capital marker is lowered too.
        ("SIA\u{1d3a}5", "SI\u{c2}\u{207f}"),
        // No marker: identical to the default.
        ("TAI5-OAN5", "T\u{c2}I-O\u{c2}N"),
    ];
    for (input, expected) in cases {
        assert_eq!(
            normalize(input, poj_doubletap_lowercase_nasal()),
            expected,
            "input {input:?}"
        );
    }
}

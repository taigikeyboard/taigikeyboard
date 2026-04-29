//! Cross-source fixture suite. Cases are merged from:
//! - `taigi-converter/tests/{phonetics,tl,poj,zhuyin,converter}.test.js` (canonical)
//! - `ios/TaigiKeyboardTests/TaigiPhoneticsTests.swift` (production iOS)
//! - `android/.../TaigiPhoneticsTest.kt` (production Android)
//!
//! Each case is annotated with `// SOURCE:` so a future drift triage can find
//! provenance fast. Per the §4 cross-validation plan, failures here must trigger
//! a tri-state diagnosis (canonical-missing / iOS-Android-drift / intentional).

use phonetics::{normalize_to_tl, strip_tone_mark, to_poj, to_tl};

// MARK: - strip_tone_mark — covers all 7 tones + no-mark + trailing digit.
// SOURCE: phonetics.test.js, TaigiPhoneticsTests.swift, TaigiPhoneticsTest.kt — identical cases across all three.

#[test]
fn strip_tone_mark_acute_tone2() {
    assert_eq!(strip_tone_mark("\u{00e1}"), ("a".into(), "2".into()));
}

#[test]
fn strip_tone_mark_grave_tone3() {
    assert_eq!(strip_tone_mark("\u{00e0}"), ("a".into(), "3".into()));
}

#[test]
fn strip_tone_mark_circumflex_tone5() {
    assert_eq!(strip_tone_mark("\u{00e2}"), ("a".into(), "5".into()));
}

#[test]
fn strip_tone_mark_macron_tone7() {
    assert_eq!(strip_tone_mark("\u{0101}"), ("a".into(), "7".into()));
}

#[test]
fn strip_tone_mark_vertical_line_tone8() {
    assert_eq!(strip_tone_mark("a\u{030d}"), ("a".into(), "8".into()));
}

#[test]
fn strip_tone_mark_breve_tone9_poj() {
    assert_eq!(strip_tone_mark("\u{0103}"), ("a".into(), "9".into()));
}

#[test]
fn strip_tone_mark_double_acute_tone9_tl() {
    assert_eq!(strip_tone_mark("a\u{030b}"), ("a".into(), "9".into()));
}

#[test]
fn strip_tone_mark_no_mark() {
    assert_eq!(strip_tone_mark("a"), ("a".into(), "".into()));
}

#[test]
fn strip_tone_mark_trailing_digit() {
    assert_eq!(strip_tone_mark("ka2"), ("ka".into(), "2".into()));
}

#[test]
fn strip_tone_mark_multi_char_syllable() {
    assert_eq!(
        strip_tone_mark("tshi\u{016b}"),
        ("tshiu".into(), "7".into())
    );
}

// MARK: - normalize_to_tl. SOURCE: TaigiPhoneticsTests.swift testNormalizeToTL_cases.

#[test]
fn normalize_to_tl_cases() {
    let cases = [
        ("ch", "ts"),
        ("chh", "tsh"),
        ("oa", "ua"),
        ("oe", "ue"),
        ("eng", "ing"),
        ("ek", "ik"),
        ("ou", "oo"),
        ("o\u{0358}", "oo"),
        ("\u{207f}", "nn"),
        ("oonn", "onn"),
    ];
    for (input, expected) in cases {
        assert_eq!(
            normalize_to_tl(input),
            expected,
            "normalize_to_tl({input}) should be {expected}"
        );
    }
}

// `is_stop_tone`, `split_initial_final`, `parse_syllable` tests live in
// `engine/phonetics/src/syllable.rs` `#[cfg(test)] mod tests` (these are
// `pub(crate)` helpers; integration tests can't reach them).

// MARK: - to_tl. SOURCE: tl.test.js + TaigiPhoneticsTests.swift testToTL_*.

#[test]
fn to_tl_all_tones() {
    let cases = [
        ("k", "a", "1", "ka"),
        ("k", "a", "2", "k\u{00e1}"),
        ("k", "a", "3", "k\u{00e0}"),
        ("k", "ah", "4", "kah"),
        ("k", "a", "5", "k\u{00e2}"),
        ("k", "a", "7", "k\u{0101}"),
        ("k", "ah", "8", "ka\u{030d}h"),
    ];
    for (init, fin, tone, expected) in cases {
        assert_eq!(to_tl(init, fin, tone), expected);
    }
}

#[test]
fn to_tl_tone9_double_acute() {
    let result = to_tl("k", "a", "9");
    assert!(
        result.contains('\u{030b}'),
        "TL tone 9 needs U+030B, got {result}"
    );
}

#[test]
fn to_tl_vowel_priority() {
    let cases = [
        ("k", "ai", "2", "k\u{00e1}i"),
        ("k", "oo", "5", "k\u{00f4}o"),
        ("t", "e", "7", "t\u{0113}"),
        ("k", "o", "2", "k\u{00f3}"),
        ("k", "ui", "3", "ku\u{00ec}"),
        ("tsh", "iu", "7", "tshi\u{016b}"),
        ("", "ng", "5", "n\u{0302}g"),
        ("", "m", "7", "m\u{0304}"),
    ];
    for (init, fin, tone, expected) in cases {
        assert_eq!(
            to_tl(init, fin, tone),
            expected,
            "to_tl({init},{fin},{tone})"
        );
    }
}

#[test]
fn to_tl_no_initial() {
    assert_eq!(to_tl("", "a", "2"), "\u{00e1}");
}

#[test]
fn to_tl_complex_iang() {
    assert_eq!(to_tl("k", "iang", "5"), "ki\u{00e2}ng");
}

// MARK: - to_poj. SOURCE: poj.test.js + TaigiPhoneticsTests.swift testToPOJ_*.

#[test]
fn to_poj_initial_conversion() {
    assert_eq!(to_poj("ts", "u", "2"), "ch\u{00fa}");
    assert_eq!(to_poj("tsh", "iu", "7"), "chhi\u{016b}");
}

#[test]
fn to_poj_final_conversions() {
    assert!(to_poj("k", "ann", "2").contains('\u{207f}'));
    assert!(to_poj("k", "oo", "1").contains('\u{0358}'));
    assert!(to_poj("k", "ua", "1").contains("oa"));
    assert!(to_poj("k", "ue", "1").contains("oe"));
    assert_eq!(to_poj("p", "ing", "1"), "peng");
    assert!(to_poj("p", "ik", "4").contains("ek"));
}

#[test]
fn to_poj_tone_marks() {
    assert_eq!(to_poj("k", "a", "1"), "ka");
    assert_eq!(to_poj("k", "a", "2"), "k\u{00e1}");
    assert_eq!(to_poj("k", "a", "5"), "k\u{00e2}");
    assert_eq!(to_poj("k", "a", "7"), "k\u{0101}");
}

#[test]
fn to_poj_tone9_breve() {
    assert!(to_poj("k", "a", "9").contains('\u{0103}'));
}

#[test]
fn to_poj_iau_mark_on_a() {
    assert!(to_poj("", "iau", "5").contains('\u{00e2}'));
}

#[test]
fn to_poj_diphthong_ai_first() {
    assert_eq!(to_poj("k", "ai", "2"), "k\u{00e1}i");
}

#[test]
fn to_poj_non_ts_initial_unchanged() {
    assert_eq!(to_poj("k", "a", "2"), "k\u{00e1}");
    assert_eq!(to_poj("p", "a", "2"), "p\u{00e1}");
    assert_eq!(to_poj("h", "a", "2"), "h\u{00e1}");
}

// MARK: - P2 regression — Codex PR #183 discussion r3143646949.
// place_poj_tone_mark suffix lookahead must decode the next Unicode scalar,
// not cast a single byte. Previously, finals like the POJ form of TL `uannh`
// (which is `oa\u{207f}h`) hit the multi-byte ⁿ at byte position 2 and the
// `bytes[after] as char` cast yielded a UTF-8 lead byte (0xe2 → 'â'), missing
// the nasal in the suffix set and mis-placing the tone on the first vowel.
//
// Cross-validates against canonical taigi-converter:
//   node -e "import('./src/converter.js').then(m => console.log(m.convert('uannh5','tl','poj')))"
//   → "oâⁿh"

#[test]
fn to_poj_uannh_tone5_marks_second_vowel() {
    use phonetics::{convert, System};
    // tl→poj path: parse_syllable("uannh5") → ("", "uannh", "5");
    // assembler = to_poj → tl_final_to_poj("uannh") = "oa\u{207f}h" → place_poj_tone_mark.
    let result = convert("uannh5", System::Tl, System::Poj).unwrap();
    assert_eq!(result, "o\u{00e2}\u{207f}h", "expected oâⁿh, got {result}");
}

#[test]
fn to_poj_uennh_tone3_marks_second_vowel() {
    use phonetics::{convert, System};
    // Parallel case: TL "uennh" → POJ "oeⁿh"; tone 3 mark on 'e'.
    let result = convert("uennh3", System::Tl, System::Poj).unwrap();
    assert_eq!(result, "o\u{00e8}\u{207f}h", "expected oèⁿh, got {result}");
}

// MARK: - P1 regression — Codex PR #183 discussion r3143631196.
// SYLLABLE_RE must include UPPERCASE precomposed vowels, lowercase í/û,
// caron triplet ǐ/ǒ/ǔ, and POJ tone 9 breve ă (U+0103). Previously these
// inputs were silently bypassed by `convert` (regex didn't match → no
// rewrite attempted → original returned).

#[test]
fn convert_uppercase_poj_to_tl_does_not_bypass() {
    use phonetics::{convert, System};
    // "Ká" — POJ K + acute. Pre-fix: returned verbatim. Post-fix: enters the
    // conversion pipeline. K initial maps unchanged; acute stays. Output is
    // identical to input here, but the PIPELINE was reached — verified via
    // CHÂN test below where output differs.
    let result = convert("K\u{00e1}", System::Poj, System::Tl).unwrap();
    assert_eq!(result, "K\u{00e1}");
}

#[test]
fn convert_uppercase_chan_poj_to_tl_substitutes_initial() {
    use phonetics::{convert, System};
    // POJ "CHÂN" → TL "TSÂN" (single-h ch → ts; chh would have been tsh).
    // Pre-fix this returned "CHÂN" verbatim because the SYLLABLE_RE class
    // missed uppercase Â (U+00C2).
    let result = convert("CH\u{00c2}N", System::Poj, System::Tl).unwrap();
    assert_eq!(result, "TS\u{00c2}N", "expected TSÂN, got {result}");
}

#[test]
fn convert_lowercase_i_acute_round_trips() {
    use phonetics::{convert, System};
    // Pre-fix: "kí" silently returned. Post-fix: enters pipeline. í (U+00ED)
    // is now in the SYLLABLE_RE class.
    let result = convert("k\u{00ed}", System::Poj, System::Tl).unwrap();
    assert_eq!(result, "k\u{00ed}");
}

#[test]
fn convert_lowercase_u_circumflex_round_trips() {
    use phonetics::{convert, System};
    let result = convert("k\u{00fb}", System::Poj, System::Tl).unwrap();
    assert_eq!(result, "k\u{00fb}");
}

#[test]
fn convert_poj_tone9_breve_to_tl_double_acute() {
    use phonetics::{convert, System};
    // POJ "kă" (k + breve = tone 9) → TL k + U+030B (double acute, NFC).
    // No precomposed exists for TL tone 9; output is k + combining mark.
    // Pre-fix: ă (U+0103) was missing from the class; "kă" returned verbatim.
    let result = convert("k\u{0103}", System::Poj, System::Tl).unwrap();
    assert!(
        result.contains('\u{030b}'),
        "expected TL double-acute U+030B in output; got {result:?}"
    );
}

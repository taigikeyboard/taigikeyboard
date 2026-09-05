//! Golden-case integration tests for `phonetics::case_transform`.
//!
//! Ports the comprehensive table-driven cases from
//! the pre-Rust iOS `CaseTransformerTests.swift` and Android
//! `SuggestionCaseTransformerTest.kt` into Rust; the per-platform
//! algorithm tests were deleted in the Path G platform rewiring commits.
//!
//! The platform-side tests post-rewire become thin bridge round-trip
//! tests verifying FFI plumbing, NOT algorithm correctness — that
//! responsibility lives here.

// case_transform 的整合測試 (golden cases),整併 iOS/Android 兩平台的表驅動測例。

use phonetics::case_transform::{
    adjust_nasal_marker_case, capitalize_candidate, transform_input_case, transform_suggestion,
    LetterCase,
};
use phonetics::InputMode;

// =========================================================================
// CaseTransformer.transformForInput — POJ tone-letter golden table
// =========================================================================

#[test]
fn poj_uppercase_all_tone_letters() {
    let cases: &[(&str, &str)] = &[
        // a
        ("á", "Á"),
        ("à", "À"),
        ("â", "Â"),
        ("ǎ", "Ǎ"),
        ("ā", "Ā"),
        ("a̍", "A̍"),
        ("ă", "Ă"),
        // e
        ("é", "É"),
        ("è", "È"),
        ("ê", "Ê"),
        ("ě", "Ě"),
        ("ē", "Ē"),
        ("e̍", "E̍"),
        ("ĕ", "Ĕ"),
        // i
        ("í", "Í"),
        ("ì", "Ì"),
        ("î", "Î"),
        ("ǐ", "Ǐ"),
        ("ī", "Ī"),
        ("i̍", "I̍"),
        ("ĭ", "Ĭ"),
        // o
        ("ó", "Ó"),
        ("ò", "Ò"),
        ("ô", "Ô"),
        ("ǒ", "Ǒ"),
        ("ō", "Ō"),
        ("o̍", "O̍"),
        ("ŏ", "Ŏ"),
        // u
        ("ú", "Ú"),
        ("ù", "Ù"),
        ("û", "Û"),
        ("ǔ", "Ǔ"),
        ("ū", "Ū"),
        ("u̍", "U̍"),
        ("ŭ", "Ŭ"),
        // n
        ("ń", "Ń"),
        ("ǹ", "Ǹ"),
        ("n̂", "N̂"),
        ("ň", "Ň"),
        ("n̄", "N̄"),
        ("n̍", "N̍"),
        ("n̋", "N̋"),
        // m
        ("ḿ", "Ḿ"),
        ("m̀", "M̀"),
        ("m̂", "M̂"),
        ("m̌", "M̌"),
        ("m̄", "M̄"),
        ("m̍", "M̍"),
        ("m̋", "M̋"),
    ];
    for (input, expected) in cases {
        let got = transform_input_case(input, LetterCase::Uppercased, InputMode::Poj);
        assert_eq!(
            &got, expected,
            "POJ upper '{}' expected '{}', got '{}'",
            input, expected, got
        );
    }
}

#[test]
fn poj_lowercase_all_tone_letters() {
    let cases: &[(&str, &str)] = &[
        ("Á", "á"),
        ("À", "à"),
        ("Â", "â"),
        ("Ǎ", "ǎ"),
        ("Ā", "ā"),
        ("A̍", "a̍"),
        ("Ă", "ă"),
        ("É", "é"),
        ("È", "è"),
        ("Ê", "ê"),
        ("Ě", "ě"),
        ("Ē", "ē"),
        ("E̍", "e̍"),
        ("Ĕ", "ĕ"),
        ("Í", "í"),
        ("Ì", "ì"),
        ("Î", "î"),
        ("Ǐ", "ǐ"),
        ("Ī", "ī"),
        ("I̍", "i̍"),
        ("Ĭ", "ĭ"),
        ("Ó", "ó"),
        ("Ò", "ò"),
        ("Ô", "ô"),
        ("Ǒ", "ǒ"),
        ("Ō", "ō"),
        ("O̍", "o̍"),
        ("Ŏ", "ŏ"),
        ("Ú", "ú"),
        ("Ù", "ù"),
        ("Û", "û"),
        ("Ǔ", "ǔ"),
        ("Ū", "ū"),
        ("U̍", "u̍"),
        ("Ŭ", "ŭ"),
        ("Ń", "ń"),
        ("Ǹ", "ǹ"),
        ("N̂", "n̂"),
        ("Ň", "ň"),
        ("N̄", "n̄"),
        ("N̍", "n̍"),
        ("N̋", "n̋"),
        ("Ḿ", "ḿ"),
        ("M̀", "m̀"),
        ("M̂", "m̂"),
        ("M̌", "m̌"),
        ("M̄", "m̄"),
        ("M̍", "m̍"),
        ("M̋", "m̋"),
    ];
    for (input, expected) in cases {
        let got = transform_input_case(input, LetterCase::Lowercased, InputMode::Poj);
        assert_eq!(
            &got, expected,
            "POJ lower '{}' expected '{}', got '{}'",
            input, expected, got
        );
    }
}

#[test]
fn tl_uppercase_specific_tone_letters() {
    let cases: &[(&str, &str)] = &[
        // TL-specific tone-9 (˝)
        ("a̋", "A̋"),
        ("e̋", "E̋"),
        ("i̋", "I̋"),
        ("ő", "Ő"),
        ("ű", "Ű"),
        // oo (TL doubled-vowel form)
        ("óo", "Óo"),
        ("òo", "Òo"),
        ("ôo", "Ôo"),
        ("ǒo", "Ǒo"),
        ("ōo", "Ōo"),
        ("o̍o", "O̍o"),
        ("őo", "Őo"),
    ];
    for (input, expected) in cases {
        let got = transform_input_case(input, LetterCase::Uppercased, InputMode::Tl);
        assert_eq!(
            &got, expected,
            "TL upper '{}' expected '{}', got '{}'",
            input, expected, got
        );
    }
}

#[test]
fn tl_lowercase_specific_tone_letters() {
    let cases: &[(&str, &str)] = &[
        ("A̋", "a̋"),
        ("E̋", "e̋"),
        ("I̋", "i̋"),
        ("Ő", "ő"),
        ("Ű", "ű"),
        ("Óo", "óo"),
        ("Òo", "òo"),
        ("Ôo", "ôo"),
        ("Ǒo", "ǒo"),
        ("Ōo", "ōo"),
        ("O̍o", "o̍o"),
        ("Őo", "őo"),
    ];
    for (input, expected) in cases {
        let got = transform_input_case(input, LetterCase::Lowercased, InputMode::Tl);
        assert_eq!(
            &got, expected,
            "TL lower '{}' expected '{}', got '{}'",
            input, expected, got
        );
    }
}

#[test]
fn poj_o_dot_uppercase() {
    // POJ combining `o͘` (U+0358) — distinct from TL's `oo`.
    let cases: &[(&str, &str)] = &[
        ("ó͘", "Ó͘"),
        ("ò͘", "Ò͘"),
        ("ô͘", "Ô͘"),
        ("ǒ͘", "Ǒ͘"),
        ("ō͘", "Ō͘"),
        ("o̍͘", "O̍͘"),
        ("ŏ͘", "Ŏ͘"),
    ];
    for (input, expected) in cases {
        let got = transform_input_case(input, LetterCase::Uppercased, InputMode::Poj);
        assert_eq!(
            &got, expected,
            "POJ o-dot upper '{}' expected '{}', got '{}'",
            input, expected, got
        );
    }
}

#[test]
fn poj_o_dot_lowercase() {
    let cases: &[(&str, &str)] = &[
        ("Ó͘", "ó͘"),
        ("Ò͘", "ò͘"),
        ("Ô͘", "ô͘"),
        ("Ǒ͘", "ǒ͘"),
        ("Ō͘", "ō͘"),
        ("O̍͘", "o̍͘"),
        ("Ŏ͘", "ŏ͘"),
    ];
    for (input, expected) in cases {
        let got = transform_input_case(input, LetterCase::Lowercased, InputMode::Poj);
        assert_eq!(
            &got, expected,
            "POJ o-dot lower '{}' expected '{}', got '{}'",
            input, expected, got
        );
    }
}

// =========================================================================
// CaseTransformer.capitalizeCandidate
// =========================================================================

#[test]
fn capitalize_candidate_input_uppercase_capitalizes() {
    assert_eq!(
        capitalize_candidate("tâi-gí", "Tai", true, InputMode::Poj),
        "Tâi-gí"
    );
}

#[test]
fn capitalize_candidate_input_lowercase_passthrough() {
    assert_eq!(
        capitalize_candidate("tâi-gí", "tai", true, InputMode::Poj),
        "tâi-gí"
    );
}

#[test]
fn capitalize_candidate_auto_cap_off_passthrough() {
    assert_eq!(
        capitalize_candidate("tâi-gí", "Tai", false, InputMode::Poj),
        "tâi-gí"
    );
}

#[test]
fn capitalize_candidate_tone_letter_first_uses_table() {
    assert_eq!(
        capitalize_candidate("ô-pêh-sai", "O", true, InputMode::Poj),
        "Ô-pêh-sai"
    );
}

#[test]
fn capitalize_candidate_non_letter_first_passthrough() {
    assert_eq!(
        capitalize_candidate("123abc", "A", true, InputMode::Poj),
        "123abc"
    );
}

#[test]
fn capitalize_candidate_is_deterministic() {
    // INVARIANT mirror — same inputs must produce same output across calls.
    let fixtures: &[(&str, &str, bool, InputMode)] = &[
        ("台語", "T", true, InputMode::Tl),
        ("tâi-gí", "t", false, InputMode::Tl),
        ("Guá", "G", true, InputMode::Poj),
        ("hō-gē", "H", true, InputMode::Poj),
    ];
    for (text, input, auto_cap, mode) in fixtures {
        let first = capitalize_candidate(text, input, *auto_cap, *mode);
        let second = capitalize_candidate(text, input, *auto_cap, *mode);
        assert_eq!(
            first, second,
            "non-determinism on ({}, {}, {}, {:?})",
            text, input, auto_cap, mode
        );
    }
}

// =========================================================================
// SuggestionCaseTransformer.transform — Android golden cases
// =========================================================================

#[test]
fn suggestion_caps_lock_all_uppercase_poj() {
    assert_eq!(
        transform_suggestion("tâi-gí", "tai", LetterCase::CapsLocked, InputMode::Poj),
        "TÂI-GÍ"
    );
}

#[test]
fn suggestion_caps_lock_all_uppercase_tl() {
    assert_eq!(
        transform_suggestion("tâi-gí", "tai", LetterCase::CapsLocked, InputMode::Tl),
        "TÂI-GÍ"
    );
}

#[test]
fn suggestion_caps_capitalize_next_letter_after_typed() {
    // Typed "Tai" (3 letters) → first 3 of candidate "Tâi-gí" matchCase
    // → "Tâi"; remaining "-gí" with caps=Uppercased → first LETTER upper
    // → "-Gí". Final: "Tâi-Gí".
    assert_eq!(
        transform_suggestion("tâi-gí", "Tai", LetterCase::Uppercased, InputMode::Poj),
        "Tâi-Gí"
    );
}

#[test]
fn suggestion_caps_single_letter_remaining() {
    // Typed "h" (1 letter) → "h" matchCase → "h"; remaining "ó" with
    // Uppercased → "Ó" via tone table. Final: "hÓ".
    assert_eq!(
        transform_suggestion("hó", "h", LetterCase::Uppercased, InputMode::Poj),
        "hÓ"
    );
}

#[test]
fn suggestion_no_caps_lowercase_remainder() {
    // Typed "Tai" → "Tâi"; remaining "-gí" with Lowercased → "-gí".
    assert_eq!(
        transform_suggestion("tâi-gí", "Tai", LetterCase::Lowercased, InputMode::Poj),
        "Tâi-gí"
    );
}

#[test]
fn suggestion_match_case_preserves_typed_case() {
    assert_eq!(
        transform_suggestion("tâi-gí", "Tai", LetterCase::Lowercased, InputMode::Poj),
        "Tâi-gí"
    );
}

#[test]
fn suggestion_match_case_all_typed() {
    // composing "HO2" has 2 letters; candidate "hó" has 2 letters
    // (h + ó precomposed); typed >= original → matchCase whole candidate.
    assert_eq!(
        transform_suggestion("hó", "HO2", LetterCase::Lowercased, InputMode::Poj),
        "HÓ"
    );
}

#[test]
fn suggestion_empty_composing_passthrough() {
    assert_eq!(
        transform_suggestion("tâi-gí", "", LetterCase::Lowercased, InputMode::Poj),
        "tâi-gí"
    );
}

// =========================================================================
// Nasal marker case adjust — post-process verification through transform_suggestion
// =========================================================================

#[test]
fn suggestion_post_process_nasal_marker_promotes_after_uppercase() {
    // composing "AN" (2 letters upper) → candidate "an\u{207F}" 2 letters →
    // matchCase → "AN\u{207F}" → adjust_nasal_marker_case → "AN\u{1D3A}"
    assert_eq!(
        transform_suggestion("an\u{207F}", "AN", LetterCase::Lowercased, InputMode::Poj),
        "AN\u{1D3A}"
    );
}

#[test]
fn nasal_adjust_direct_call_promotes_lower_after_upper() {
    assert_eq!(adjust_nasal_marker_case("AN\u{207F}"), "AN\u{1D3A}");
}

#[test]
fn nasal_adjust_direct_call_demotes_upper_after_lower() {
    assert_eq!(adjust_nasal_marker_case("an\u{1D3A}"), "an\u{207F}");
}

// =========================================================================
// Additional Android golden cases (Codex mid-slice review gap-fill)
// =========================================================================

#[test]
fn suggestion_tl_tone_letter_capitalization() {
    // SuggestionCaseTransformerTest.kt:128-132 — TL `ôo` doubled-vowel
    // form must capitalize via TL table to "Ôo".
    assert_eq!(
        transform_suggestion("ôo-peh-sai", "O", LetterCase::Lowercased, InputMode::Tl),
        "Ôo-peh-sai"
    );
}

#[test]
fn suggestion_digits_not_counted_as_letters() {
    // SuggestionCaseTransformerTest.kt:163-170 — composing "Ka2" has 2
    // letters (k, a); digit 2 not counted. Candidate "ká" has 2 letters
    // (k + ó precomposed). typed >= original → matchCase whole.
    assert_eq!(
        transform_suggestion("ká", "Ka2", LetterCase::Lowercased, InputMode::Poj),
        "Ká"
    );
}

#[test]
fn suggestion_empty_roman_unchanged() {
    // SuggestionCaseTransformerTest.kt:172-177 — empty original returns
    // empty regardless of caps state.
    assert_eq!(
        transform_suggestion("", "tai", LetterCase::CapsLocked, InputMode::Poj),
        ""
    );
}

#[test]
fn suggestion_caps_lock_dominates_caps_flag() {
    // SuggestionCaseTransformerTest.kt:249-258 — when capsLock=true,
    // the value of caps doesn't matter (both produce identical output).
    let with_caps_off = transform_suggestion(
        "tâi-gí",
        "T",
        LetterCase::CapsLocked, // CapsLocked subsumes both caps states
        InputMode::Poj,
    );
    let with_caps_on = transform_suggestion(
        "tâi-gí",
        "T",
        LetterCase::CapsLocked, // same — there is no "caps + capsLock" combined state in our enum
        InputMode::Poj,
    );
    assert_eq!(
        with_caps_off, with_caps_on,
        "CapsLocked must produce deterministic output regardless of caps interpretation"
    );
    assert_eq!(with_caps_off, "TÂI-GÍ");
}

// =========================================================================
// Round-trip property — every table entry must invert cleanly
// =========================================================================

#[test]
fn poj_table_round_trip_uppercase_then_lowercase_is_identity() {
    // Sample of POJ table entries spanning all vowel groups + nasal +
    // combining-mark forms. For each: lower → upper via tone_char →
    // lower via tone_char must equal the original lower.
    let lowers: &[&str] = &[
        // a-group
        "á", "à", "â", "ǎ", "ā", "a̍", "ă", // e-group
        "é", "ê", "e̍", // i-group
        "í", "î", "i̍", // o-group + o͘ combining
        "ó", "ô", "o̍", "ó͘", "ô͘", "o̍͘", // u-group
        "ú", "û", "u̍", // n-group
        "ń", "n̂", "n̍", // m-group
        "ḿ", "m̂", "m̍",
    ];
    for lower in lowers {
        let upper = transform_input_case(lower, LetterCase::CapsLocked, InputMode::Poj);
        let back = transform_input_case(&upper, LetterCase::Lowercased, InputMode::Poj);
        assert_eq!(
            &back, lower,
            "POJ round-trip broke for '{}' — went '{}' → '{}'",
            lower, upper, back
        );
    }
}

#[test]
fn tl_table_round_trip_uppercase_then_lowercase_is_identity() {
    let lowers: &[&str] = &[
        // TL-specific tone-9 (˝)
        "a̋", "e̋", "i̋", "ő", "ű", // TL doubled-vowel form
        "óo", "ôo", "o̍o", "őo", // shared tone letters
        "á", "é", "í", "ó", "ú",
    ];
    for lower in lowers {
        let upper = transform_input_case(lower, LetterCase::CapsLocked, InputMode::Tl);
        let back = transform_input_case(&upper, LetterCase::Lowercased, InputMode::Tl);
        assert_eq!(
            &back, lower,
            "TL round-trip broke for '{}' — went '{}' → '{}'",
            lower, upper, back
        );
    }
}

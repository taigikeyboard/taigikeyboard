//! Golden-case integration tests for `phonetics::case_transform`.
//!
//! Ports the comprehensive table-driven cases from
//! the pre-Rust iOS case-transformer tests into Rust; the per-platform
//! algorithm tests were deleted in the Path G platform rewiring commits.
//!
//! The platform-side tests post-rewire become thin bridge round-trip
//! tests verifying FFI plumbing, NOT algorithm correctness — that
//! responsibility lives here.

use phonetics::case_transform::{adjust_nasal_marker_case, transform_input_case, LetterCase};

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
        let got = transform_input_case(input, LetterCase::Uppercased);
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
        let got = transform_input_case(input, LetterCase::Lowercased);
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
        let got = transform_input_case(input, LetterCase::Uppercased);
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
        let got = transform_input_case(input, LetterCase::Lowercased);
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
        let got = transform_input_case(input, LetterCase::Uppercased);
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
        let got = transform_input_case(input, LetterCase::Lowercased);
        assert_eq!(
            &got, expected,
            "POJ o-dot lower '{}' expected '{}', got '{}'",
            input, expected, got
        );
    }
}

// =========================================================================
// Nasal marker case adjust
// =========================================================================

#[test]
fn nasal_adjust_direct_call_promotes_lower_after_upper() {
    assert_eq!(adjust_nasal_marker_case("AN\u{207F}"), "AN\u{1D3A}");
}

#[test]
fn nasal_adjust_direct_call_demotes_upper_after_lower() {
    assert_eq!(adjust_nasal_marker_case("an\u{1D3A}"), "an\u{207F}");
}

// INVARIANT_CASE_TRANSFORMER_IS_DETERMINISTIC (behavioral-invariants.md §9)
#[test]
fn input_caps_lock_is_deterministic() {
    // trace: CapsLocked → `full_uppercase_tone_string`; `tâi-gí` is no
    // table key, so the stdlib upper — same output on every call.
    let first = transform_input_case("tâi-gí", LetterCase::CapsLocked);
    let second = transform_input_case("tâi-gí", LetterCase::CapsLocked);
    assert_eq!(first, second);
    assert_eq!(first, "TÂI-GÍ");
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
        let upper = transform_input_case(lower, LetterCase::CapsLocked);
        let back = transform_input_case(&upper, LetterCase::Lowercased);
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
        let upper = transform_input_case(lower, LetterCase::CapsLocked);
        let back = transform_input_case(&upper, LetterCase::Lowercased);
        assert_eq!(
            &back, lower,
            "TL round-trip broke for '{}' — went '{}' → '{}'",
            lower, upper, back
        );
    }
}

//! Op coverage — exercises every `PhoneticsRequest.method` variant
//! end-to-end through the dispatcher so the wire format, dispatch
//! routing, and per-op implementation stay in sync.

use phonetics::api::normalize_tone;
use phonetics::requests::handle;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{
    AppConfig, BoolResult, DoubleSpacePeriod, ExternalLookupDigitForm, GetToneVariations,
    IsAttachingPunctuation, IsJoinerPunctuation, IsSpacedPunctuation, PhoneticsRequest,
    PhoneticsResponse, StringResult, ToneVariationsResult,
};

//
// Domain-level tests: drive `phonetics::requests::handle` directly and
// assert against `Result<PhoneticsResponse>`. Envelope concerns
// (`taigi.engine.Request` decoding, `Response.id`/`error`,
// panic catching) live in `engine/dispatch/tests/` — exercising them
// from the phonetics crate would invert the production dependency
// graph.

fn run(method: Method) -> PhoneticsResponse {
    let req = PhoneticsRequest {
        method: Some(method),
    };
    handle(&req).expect("dispatch handle should succeed")
}

fn ok_phon(resp: &PhoneticsResponse) -> &PhonResult {
    resp.result
        .as_ref()
        .expect("PhoneticsResponse.result should be present")
}

fn string_result(resp: &PhoneticsResponse) -> String {
    let PhonResult::StringResult(StringResult { output }) = ok_phon(resp) else {
        panic!("expected StringResult, got {:?}", ok_phon(resp));
    };
    output.clone()
}

fn bool_result(resp: &PhoneticsResponse) -> bool {
    let PhonResult::BoolResult(BoolResult { value }) = ok_phon(resp) else {
        panic!("expected BoolResult");
    };
    *value
}

fn tone_variations_result(resp: &PhoneticsResponse) -> ToneVariationsResult {
    let PhonResult::ToneVariationsResult(t) = ok_phon(resp) else {
        panic!("expected ToneVariationsResult");
    };
    t.clone()
}

fn tl_config() -> AppConfig {
    AppConfig {
        input_mode: "tl".to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: false,
        is_hanji_first: false,
        output_both_scripts: false,
        candidate_display_mode: 0,
        syllable_separator: 0,
        force_lowercase_nasal_marker: false,
        tps_or_maps_to_er: false,
        hanji_conversion: None,
        dictionary_sources: None,
        custom_dictionary_disabled: false,
        c_types_affricate: false,
        f_types_hyphen: false,
        khinsiann_in_hanji: false,
    }
}

fn poj_config(oo: bool, nn: bool) -> AppConfig {
    AppConfig {
        input_mode: "poj".to_string(),
        oo_doubletap_enabled: oo,
        nn_doubletap_enabled: nn,
        is_hanji_first: false,
        output_both_scripts: false,
        candidate_display_mode: 0,
        syllable_separator: 0,
        force_lowercase_nasal_marker: false,
        tps_or_maps_to_er: false,
        hanji_conversion: None,
        dictionary_sources: None,
        custom_dictionary_disabled: false,
        c_types_affricate: false,
        f_types_hyphen: false,
        khinsiann_in_hanji: false,
    }
}

// `api::normalize_tone` — live via `composing::derived`; the retired
// `NormalizeTone` op no longer carries it, so these call it directly.

#[test]
fn normalize_tone_tl_basic() {
    let out = normalize_tone("ho2", &tl_config());
    // Tone digit 2 → diacritic on vowel; exact NFC form matches existing
    // to_tone_marks behavior.
    assert!(
        !out.is_empty(),
        "TL normalize should produce non-empty output"
    );
    assert!(!out.contains('2'), "tone digit should be removed");
}

#[test]
fn normalize_tone_poj_oo_doubletap_enabled() {
    // No tone digit so to_tone_marks is a no-op; only the doubletap
    // preprocessor's transformation is observable. With toggle ON, "hoo"
    // becomes "ho͘".
    let out = normalize_tone("hoo", &poj_config(true, false));
    assert!(out.contains('\u{0358}'), "expected o͘ (U+0358): {out:?}");
}

#[test]
fn normalize_tone_poj_oo_doubletap_disabled() {
    // Same input, toggle OFF — preprocessor is a no-op so "oo" remains
    // literal and U+0358 is absent.
    let out = normalize_tone("hoo", &poj_config(false, false));
    assert!(!out.contains('\u{0358}'), "no preprocess; got {out:?}");
}

#[test]
fn normalize_tone_poj_nn_doubletap_enabled() {
    let out = normalize_tone("ann2", &poj_config(false, true));
    // Vowel + nn → vowel + ⁿ (U+207F).
    assert!(out.contains('\u{207f}'), "expected ⁿ (U+207F): {out:?}");
}

#[test]
fn normalize_tone_poj_both_doubletaps_fold_independently() {
    // The two toggles are independent per-key affordances, so a buffer holding
    // BOTH `oo` and `nn` must get both rewrites. Reported on device: typing
    // `h o o n n` showed `ho͘nn` — the `oo` folded, the `nn` did not.
    //
    // Root cause was ordering. `convert_nasal_double_n` only fires when the
    // char immediately before `nn` is an ASCII vowel, and the `oo` fold puts a
    // combining mark (U+0358) exactly there. Running `oo` first stranded the
    // `nn`; `honn` (nothing to fold) converted fine, which is why the two
    // single-toggle tests above never caught it.
    for (input, expected) in [
        ("hoonn", "ho\u{0358}\u{207f}"),
        ("hoonnh", "ho\u{0358}\u{207f}h"),
        // Unchanged by the reorder — no `oo` to fold.
        ("honn", "ho\u{207f}"),
        ("ann", "a\u{207f}"),
    ] {
        let out = normalize_tone(input, &poj_config(true, true));
        assert_eq!(out, expected, "input {input:?}");
    }
}

#[test]
fn normalize_tone_poj_doubletap_folds_uppercase() {
    // Exercises the `OO` arm together with the reordered folds AND the
    // `adjust_nasal_marker_case` pass that runs after them: the marker follows
    // the case of the nearest preceding letter, so an all-caps buffer ends on
    // `ᴺ` (U+1D3A) rather than `ⁿ`.
    // trace: "HOONN" -> nn fold -> "HOOⁿ" -> oo fold -> "HO͘ⁿ" -> case -> "HO͘ᴺ"
    for (input, expected) in [
        ("HOONN", "HO\u{0358}\u{1d3a}"),
        ("Hoonn", "Ho\u{0358}\u{207f}"),
        ("hooNN", "ho\u{0358}\u{207f}"),
    ] {
        let out = normalize_tone(input, &poj_config(true, true));
        assert_eq!(out, expected, "input {input:?}");
    }
    // A shift between the two taps still folds: the pair is matched
    // case-insensitively and the FIRST tap decides the letter's case.
    // trace: "hoOnn" -> nn fold -> "hoOⁿ" -> oo fold pairs o+O, first tap
    //        lowercase -> "ho͘ⁿ" -> case pass sees `o` -> "ho͘ⁿ"
    let out = normalize_tone("hoOnn", &poj_config(true, true));
    assert_eq!(out, "ho\u{0358}\u{207f}");
}

#[test]
fn normalize_tone_poj_oo_doubletap_pairs_in_one_pass() {
    // The fold pairs taps left to right in a single scan, so it can neither
    // miss a case combination nor re-read what it just wrote.
    //
    // Both were real: the previous three sequential `String::replace` passes
    // (`oo`, `Oo`, `OO`) had no `oO` arm, and they ate each other's output —
    // `OOoo` folded to `O͘o͘`, whose `Oo` seam the second pass matched and whose
    // `OO` seam the third matched again, stacking THREE combining dots on one
    // letter.
    for (input, expected) in [
        // First tap decides the case of the letter.
        ("hoo", "ho\u{0358}"),
        ("hoO", "ho\u{0358}"),
        ("hOo", "hO\u{0358}"),
        ("hOO", "hO\u{0358}"),
        // Adjacent pairs stay independent — exactly one dot each.
        ("OOoo", "O\u{0358}o\u{0358}"),
        ("ooOO", "o\u{0358}O\u{0358}"),
        ("oooo", "o\u{0358}o\u{0358}"),
        // Odd run: the trailing unpaired `o` is left alone.
        ("hooo", "ho\u{0358}o"),
    ] {
        let out = normalize_tone(input, &poj_config(true, false));
        assert_eq!(out, expected, "input {input:?}");
    }
}

#[test]
fn normalize_tone_poj_doubletap_toggles_stay_independent() {
    // Each toggle alone still does exactly its own rewrite and nothing else —
    // the reorder must not make one imply the other.
    let out = normalize_tone("hoonn", &poj_config(false, true));
    assert_eq!(out, "hoo\u{207f}", "nn only");

    let out = normalize_tone("hoonn", &poj_config(true, false));
    assert_eq!(out, "ho\u{0358}nn", "oo only");

    let out = normalize_tone("hoonn", &poj_config(false, false));
    assert_eq!(out, "hoonn", "neither");
}

// No wire op since 2026-11-23 (tag 13 reserved); composing calls it in-process.
#[test]
fn tl_to_poj_display_round_trip() {
    let out = phonetics::api::tl_display_to_poj_display("hoo");
    assert!(
        out.contains('\u{0358}'),
        "TL oo should map to POJ o͘: {out:?}"
    );
}

#[test]
fn get_tone_variations_returns_all_three_maps() {
    let resp = run(Method::GetToneVariations(GetToneVariations {}));
    let result = tone_variations_result(&resp);
    assert!(!result.poj_variations.is_empty(), "POJ map non-empty");
    assert!(!result.tl_variations.is_empty(), "TL map non-empty");
    assert!(result.poj_variations.contains_key("a"));
    assert!(result.tl_variations.contains_key("a"));
    // TL 'oo' entry exists; POJ 'o͘' entry exists.
    assert!(result.tl_variations.contains_key("oo"));
    assert!(result.poj_variations.contains_key("o\u{0358}"));
    // Universal: both tone-9 marks on `a`, and both scripts' o-keys.
    let a = &result.universal_variations["a"].variations;
    assert!(a.contains(&"a\u{30B}".to_string()) && a.contains(&"ă".to_string()));
    assert!(result.universal_variations.contains_key("oo"));
    assert!(result.universal_variations.contains_key("o\u{0358}"));
}

#[test]
fn external_lookup_digit_form_folds_o_dot_and_nasal_marker() {
    // trace: per syllable base form (U+0358→o, U+207F→nn), first tone mark
    // stripped to a digit, tone 1 omitted; the full table is in
    // `src/external_lookup.rs`.
    for (reading, expected) in [
        ("ho\u{0358}\u{0301}", "hoo2"),
        ("sa\u{207f}", "sann"),
        ("T\u{e2}i-g\u{ed}", "tai5-gi2"),
    ] {
        let resp = run(Method::ExternalLookupDigitForm(ExternalLookupDigitForm {
            reading: reading.to_string(),
        }));
        assert_eq!(string_result(&resp), expected, "{reading:?}");
    }
}

// `api::contains_tps` — live via composing / custom_search / nextword; the
// retired `ContainsTps` op no longer carries it, so these call it directly.

#[test]
fn contains_tps_true_for_zhuyin() {
    assert!(phonetics::api::contains_tps("ㄉㄧㄠ"));
}

#[test]
fn contains_tps_false_for_latin() {
    assert!(!phonetics::api::contains_tps("tiau"));
}

// `tl_numeric_to_tps` has no wire op (tag 32 reserved 2026-10-09); it is the
// numeric half of `api::tl_display_to_tps`, covered here through the native API.
#[test]
fn tl_numeric_to_tps_basic() {
    let out = phonetics::api::tl_numeric_to_tps("tiau5", false);
    assert!(
        !out.is_empty(),
        "TL numeric → TPS should produce zhuyin: {out:?}"
    );
}

/// Default (`or_maps_to_er=false`) renders the vowel `or` as ㄛ (`\u{311b}`),
/// matching the iOS pre-D9.4 default. Mirrors `TPSConverter.toTPS` /
/// `TLToTPS.convert`.
#[test]
fn tl_numeric_to_tps_or_default_uses_o_vowel() {
    let out = phonetics::api::tl_numeric_to_tps("kor1", false);
    assert!(
        out.contains('\u{311b}'),
        "or → ㄛ when toggle off; got {out:?}"
    );
    assert!(
        !out.contains('\u{311c}'),
        "must not contain ㄜ when toggle off; got {out:?}"
    );
}

/// With the toggle ON, the same `or` syllable renders as ㄜ (`\u{311c}`),
/// matching iOS `orMapsToER=true`.
#[test]
fn tl_numeric_to_tps_or_maps_to_er_when_enabled() {
    let out = phonetics::api::tl_numeric_to_tps("kor1", true);
    assert!(
        out.contains('\u{311c}'),
        "or → ㄜ when toggle on; got {out:?}"
    );
    assert!(
        !out.contains('\u{311b}'),
        "must not contain ㄛ when toggle on; got {out:?}"
    );
}

/// Multi-syllable input must preserve the syllable boundary as a single
/// space, matching iOS `joined(separator: " ")` / Android `joinToString(" ")`.
#[test]
fn tl_numeric_to_tps_preserves_syllable_boundaries() {
    let out = phonetics::api::tl_numeric_to_tps("gua2-gua2", false);
    let space_count = out.matches(' ').count();
    assert_eq!(
        space_count, 1,
        "two syllables should be joined by exactly one space; got {out:?}"
    );
}

/// A neutral-tone `--` (or leading / trailing `-`) must NOT introduce double
/// spaces — empty tokens are filtered before joining.
#[test]
fn tl_numeric_to_tps_handles_repeated_hyphen_without_double_space() {
    let out = phonetics::api::tl_numeric_to_tps("gua2--gua2", false);
    assert!(
        !out.contains("  "),
        "no double space across `--`; got {out:?}"
    );
    assert_eq!(
        out.matches(' ').count(),
        1,
        "exactly one space; got {out:?}"
    );
}

/// A continuous pick's words are space-joined (`tâi hó-lâng`); every syllable
/// of every word converts.
#[test]
fn tl_display_to_tps_converts_every_word_of_a_multi_word_reading() {
    use phonetics::api::{tl_display_to_tps, tl_numeric_to_tps};
    // trace: tps.rs t → ㄉ, ai → ㄞ, 5 → ˊ; h → ㄏ, o → ㄛ (or_maps_to_er
    // off), 2 → ˋ; l → ㄌ, ang → ㄤ, 5 → ˊ.
    const EXPECTED: &str = "ㄉㄞˊ ㄏㄛˋ ㄌㄤˊ";
    assert_eq!(tl_display_to_tps("tâi hó-lâng", false), EXPECTED);
    // Control: the hyphen-only reading always converted.
    assert_eq!(tl_display_to_tps("tâi-hó-lâng", false), EXPECTED);
    // Tab and runs of separators collapse to one space; `--` is one space too,
    // and marks the syllable after it neutral (˙).
    assert_eq!(tl_numeric_to_tps("tai5\tho2  lang5", false), EXPECTED);
    assert_eq!(tl_numeric_to_tps(" tai5--ho2 ", false), "ㄉㄞˊ ˙ㄏㄛˋ");
    // A three-word sentence. trace: g → ㆣ, ua → ㄨㄚ, 2 → ˋ; b → ㆠ, e → ㆤ,
    // h coda → ㆷ (tone 4 unmarked); kh → ㄎ, i → ㄧ, 3 → ˪.
    assert_eq!(
        tl_display_to_tps("guá beh khì", false),
        "ㆣㄨㄚˋ ㆠㆤㆷ ㄎㄧ˪"
    );
}

/// Display-form path shares the helper, so the syllable-boundary fix must
/// also apply there.
#[test]
fn tl_display_to_tps_preserves_syllable_boundaries() {
    let out = phonetics::api::tl_display_to_tps("guá-guá", false);
    assert_eq!(
        out.matches(' ').count(),
        1,
        "display-form multi-syllable must keep one space; got {out:?}"
    );
}

#[test]
fn is_attaching_punctuation_answers_through_the_dispatcher() {
    // trace: `punctuation::ATTACHING` holds `?`, `」` and the two-cell
    // ellipsis `……` (#879), not the opener `(`; text matches an entry whole,
    // so `?!` does not attach.
    for (text, expected) in [
        ("?", true),
        ("」", true),
        ("……", true),
        ("(", false),
        ("?!", false),
        ("", false),
    ] {
        let resp = run(Method::IsAttachingPunctuation(IsAttachingPunctuation {
            text: text.to_string(),
        }));
        assert_eq!(bool_result(&resp), expected, "{text:?}");
    }
}

#[test]
fn is_joiner_punctuation_answers_through_the_dispatcher() {
    // trace: `punctuation::is_joiner_punctuation` compares to
    // `separators::KHINSIANN_DOT` (U+00B7); `?` attaches, it does not join.
    for (text, expected) in [("\u{00b7}", true), ("?", false), ("-", false), ("", false)] {
        let resp = run(Method::IsJoinerPunctuation(IsJoinerPunctuation {
            text: text.to_string(),
        }));
        assert_eq!(bool_result(&resp), expected, "{text:?}");
    }
}

#[test]
fn is_spaced_punctuation_answers_through_the_dispatcher() {
    // trace: `punctuation::is_spaced_punctuation` = attaching AND ASCII —
    // `?` and `)` yes; their full-width `？` `）`, the full-width dash `——`
    // and the opener `(` no.
    for (text, expected) in [
        ("?", true),
        (")", true),
        ("？", false),
        ("）", false),
        ("——", false),
        ("(", false),
        ("", false),
    ] {
        let resp = run(Method::IsSpacedPunctuation(IsSpacedPunctuation {
            text: text.to_string(),
        }));
        assert_eq!(bool_result(&resp), expected, "{text:?}");
    }
}

fn double_space(text: &str, millis: i64, full_width: bool) -> String {
    string_result(&run(Method::DoubleSpacePeriod(DoubleSpacePeriod {
        text_before_caret: text.to_string(),
        millis_since_plain_space: millis,
        full_width,
    })))
}

#[test]
fn double_space_period_after_a_letter_number_or_mark() {
    // trace: last code point before the space — "a" Ll, "7" Nd, "á" Ll,
    // "o͘" ends in U+0358 Mn, "台" Lo, "𠢕" U+20895 Lo (supplementary plane);
    // all in `GeneralCategoryGroup::{Letter, Number, Mark}`.
    for text in [
        "a ",
        "7 ",
        "á ",
        "o\u{0358} ",
        "台 ",
        "\u{20895} ",
        "x\u{20895} ",
    ] {
        assert_eq!(double_space(text, 0, false), ". ", "{text:?}");
        assert_eq!(double_space(text, 299, true), "。", "{text:?}");
    }
}

#[test]
fn double_space_keeps_the_space_after_punctuation_whitespace_or_emoji() {
    // trace: "😀" = U+1F600 So; "\t " ends in a tab (Cc) before the space;
    // "a" / "ab" end in no space; " " has nothing before its space.
    for text in [
        ". ",
        "。 ",
        "? ",
        "  ",
        "\t ",
        "\n ",
        "\u{1F600} ",
        " ",
        "",
        "a",
        "ab",
    ] {
        assert_eq!(double_space(text, 0, false), "", "{text:?}");
    }
}

#[test]
fn double_space_window_is_under_300_ms() {
    // trace: `DOUBLE_SPACE_WINDOW_MS` = 300, half-open `0..300`.
    assert_eq!(double_space("a ", 299, false), ". ");
    assert_eq!(double_space("a ", 300, false), "");
    assert_eq!(double_space("a ", -1, false), "");
}

/// Neutral tone (`--`): every syllable after it, up to the end of its word,
/// gets the `˙` prefix and keeps its own tone mark (reference §5.5; mark
/// U+02D9 = `tps::TPS_NEUTRAL_MARK`).
#[test]
fn tl_numeric_to_tps_marks_neutral_tone_syllables_to_word_end() {
    use phonetics::api::{tl_display_to_tps, tl_numeric_to_tps};
    // trace: th → ㄊ, iann → ㄧㆩ (tone 1 unmarked); t → ㄉ, ioh → ㄧㄛㆷ,
    // 8 → U+0307.
    assert_eq!(
        tl_display_to_tps("thiann--tio̍h", false),
        "ㄊㄧㆩ ˙ㄉㄧㄛㆷ\u{0307}"
    );
    // Every syllable to the word end: ts → ㄗ, au → ㄠ, 2 → ˋ; tsh → ㄘ,
    // ut → ㄨㆵ (tone 4); kh → ㄎ, i → ㄧ, 3 → ˪.
    assert_eq!(
        tl_numeric_to_tps("tsau2--tshut4-khi3", false),
        "ㄗㄠˋ ˙ㄘㄨㆵ ˙ㄎㄧ˪"
    );
    // Two `--` in one word; word-initial `--`; the mark resets at a space.
    assert_eq!(
        tl_numeric_to_tps("tng2--lai5--ah4", false),
        "ㄉㆭˋ ˙ㄌㄞˊ ˙ㄚㆷ"
    );
    assert_eq!(tl_numeric_to_tps("--e5", false), "˙ㆤˊ");
    assert_eq!(tl_numeric_to_tps("m7 --eh4", false), "ㆬ˫ ˙ㆤㆷ");
    assert_eq!(
        tl_numeric_to_tps("au7--jit8 ka2", false),
        "ㄠ˫ ˙ㆢㄧㆵ\u{0307} ㄍㄚˋ"
    );
    // Control: a TPS-only reading (TPS OOV synth carries Bopomofo as its
    // roman) passes through unchanged.
    assert_eq!(tl_numeric_to_tps("ㄉㄞˊ ㄏㄛˋ", false), "ㄉㄞˊ ㄏㄛˋ");
}

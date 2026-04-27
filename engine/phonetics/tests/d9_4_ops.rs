//! D9.4 op coverage — exercises every new intent introduced in commit 2/3
//! end-to-end through `process_request` so the wire format + dispatcher +
//! implementation all stay in sync. Pre-impl Codex review v3 §3 mandated
//! branch coverage for TPS adjustment and NBSP no-split for abbrev; both
//! are pinned here.

use phonetics::api::process_request;
use prost::Message;
use protos::engine::phonetics_request::Intent;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{
    request, response, AppConfig, BoolResult, ContainsTps, DeriveAbbrev, DeriveNotone, ErrorCode,
    GetToneVariations, HasToneMarks, IsTpsToneMark, NormalizeInput, NormalizeTone, NormalizeToTl,
    OptionalStringResult, PhoneticsRequest, PojToTl, Request, Response, RestoreTone, StringResult,
    StripTone, StripToneResult, TlDisplayToTps, TlNumericToTps, TlToPoj, ToneVariationsResult,
    TpsAdjustResult, TpsInputAdjust, TpsToTl,
};

// ---------------- helpers ----------------

fn run(intent: Intent, config: AppConfig) -> Response {
    let req = Request {
        id: 42,
        r#type: 0,
        config_snapshot: Some(config),
        generation: 0,
        payload: Some(request::Payload::Phonetics(PhoneticsRequest {
            intent: Some(intent),
        })),
    };
    let mut buf = Vec::with_capacity(req.encoded_len());
    req.encode(&mut buf).unwrap();
    let resp_bytes = process_request(&buf);
    Response::decode(resp_bytes.as_slice()).expect("response decodes")
}

fn ok_phon(resp: &Response) -> &PhonResult {
    assert_eq!(resp.error, ErrorCode::Ok as i32, "expected OK, got {resp:?}");
    let Some(response::Payload::Phonetics(p)) = &resp.payload else {
        panic!("expected phonetics payload");
    };
    p.result.as_ref().expect("expected result")
}

fn string_result(resp: &Response) -> String {
    let PhonResult::StringResult(StringResult { output }) = ok_phon(resp) else {
        panic!("expected StringResult, got {:?}", ok_phon(resp));
    };
    output.clone()
}

fn bool_result(resp: &Response) -> bool {
    let PhonResult::BoolResult(BoolResult { value }) = ok_phon(resp) else {
        panic!("expected BoolResult");
    };
    *value
}

fn opt_result(resp: &Response) -> Option<String> {
    let PhonResult::OptionalStringResult(OptionalStringResult { output, present }) = ok_phon(resp)
    else {
        panic!("expected OptionalStringResult");
    };
    if *present { Some(output.clone()) } else { None }
}

fn strip_result(resp: &Response) -> (String, String) {
    let PhonResult::StripToneResult(StripToneResult { bare, tone }) = ok_phon(resp) else {
        panic!("expected StripToneResult");
    };
    (bare.clone(), tone.clone())
}

fn tps_adjust_result(resp: &Response) -> (String, Option<String>) {
    let PhonResult::TpsAdjustResult(TpsAdjustResult {
        adjusted,
        replace_last,
    }) = ok_phon(resp)
    else {
        panic!("expected TpsAdjustResult");
    };
    let opt = replace_last.as_ref().and_then(|r| {
        if r.present { Some(r.output.clone()) } else { None }
    });
    (adjusted.clone(), opt)
}

fn tone_variations_result(resp: &Response) -> &ToneVariationsResult {
    let PhonResult::ToneVariationsResult(t) = ok_phon(resp) else {
        panic!("expected ToneVariationsResult");
    };
    // Static reference into the response — but Response is owned here.
    // Workaround: leak via a static slot (never freed) for read-only inspection.
    // Tests are short-lived; acceptable.
    Box::leak(Box::new(t.clone()))
}

fn tl_config() -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: "tl".to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: false,
    }
}

fn poj_config(oo: bool, nn: bool) -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: "poj".to_string(),
        oo_doubletap_enabled: oo,
        nn_doubletap_enabled: nn,
    }
}

// ============================================================
// Phonetics core
// ============================================================

#[test]
fn normalize_tone_tl_basic() {
    let resp = run(
        Intent::NormalizeTone(NormalizeTone {
            input: "ho2".to_string(),
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    // Tone digit 2 → diacritic on vowel; exact NFC form matches existing
    // to_tone_marks behavior.
    assert!(!out.is_empty(), "TL normalize should produce non-empty output");
    assert!(!out.contains('2'), "tone digit should be removed");
}

#[test]
fn normalize_tone_poj_oo_doubletap_enabled() {
    // No tone digit so to_tone_marks is a no-op; only the doubletap
    // preprocessor's transformation is observable. With toggle ON, "hoo"
    // becomes "ho͘".
    let resp = run(
        Intent::NormalizeTone(NormalizeTone {
            input: "hoo".to_string(),
        }),
        poj_config(true, false),
    );
    let out = string_result(&resp);
    assert!(out.contains('\u{0358}'), "expected o͘ (U+0358): {out:?}");
}

#[test]
fn normalize_tone_poj_oo_doubletap_disabled() {
    // Same input, toggle OFF — preprocessor is a no-op so "oo" remains
    // literal and U+0358 is absent.
    let resp = run(
        Intent::NormalizeTone(NormalizeTone {
            input: "hoo".to_string(),
        }),
        poj_config(false, false),
    );
    let out = string_result(&resp);
    assert!(!out.contains('\u{0358}'), "no preprocess; got {out:?}");
}

#[test]
fn normalize_tone_poj_nn_doubletap_enabled() {
    let resp = run(
        Intent::NormalizeTone(NormalizeTone {
            input: "ann2".to_string(),
        }),
        poj_config(false, true),
    );
    let out = string_result(&resp);
    // Vowel + nn → vowel + ⁿ (U+207F).
    assert!(out.contains('\u{207f}'), "expected ⁿ (U+207F): {out:?}");
}

#[test]
fn strip_tone_returns_bare_and_tone() {
    let resp = run(
        Intent::StripTone(StripTone {
            input: "hó".to_string(),
        }),
        tl_config(),
    );
    let (bare, tone) = strip_result(&resp);
    assert_eq!(bare, "ho");
    assert_eq!(tone, "2");
}

#[test]
fn poj_to_tl_display_round_trip() {
    let resp = run(
        Intent::PojToTl(PojToTl {
            input: "ho͘".to_string(),
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(out.contains("hoo"), "POJ o͘ should map to TL oo: {out:?}");
}

#[test]
fn tl_to_poj_display_round_trip() {
    let resp = run(
        Intent::TlToPoj(TlToPoj {
            input: "hoo".to_string(),
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(out.contains('\u{0358}'), "TL oo should map to POJ o͘: {out:?}");
}

#[test]
fn normalize_to_tl_passes_input_through_normalizer() {
    let resp = run(
        Intent::NormalizeToTl(NormalizeToTl {
            input: "hoo".to_string(),
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert_eq!(out, "hoo");
}

#[test]
fn normalize_input_extracts_tone_from_diacritic() {
    let resp = run(
        Intent::NormalizeInput(NormalizeInput {
            input: "hó".to_string(),
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert_eq!(out, "ho2");
}

#[test]
fn normalize_input_handles_hyphenated_syllables() {
    let resp = run(
        Intent::NormalizeInput(NormalizeInput {
            input: "gâu-tsá".to_string(),
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(out.contains("gau5") && out.contains("tsa2"), "got {out:?}");
}

#[test]
fn normalize_input_keeps_existing_tone_digit() {
    let resp = run(
        Intent::NormalizeInput(NormalizeInput {
            input: "ho2".to_string(),
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert_eq!(out, "ho2");
}

#[test]
fn restore_tone_returns_text_without_last_diacritic() {
    let resp = run(
        Intent::RestoreTone(RestoreTone {
            text: "hó".to_string(),
        }),
        tl_config(),
    );
    let out = opt_result(&resp);
    assert_eq!(out, Some("ho".to_string()));
}

#[test]
fn restore_tone_returns_none_when_no_tone_mark() {
    let resp = run(
        Intent::RestoreTone(RestoreTone {
            text: "ho".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(opt_result(&resp), None);
}

#[test]
fn has_tone_marks_true_for_diacritic() {
    let resp = run(
        Intent::HasToneMarks(HasToneMarks {
            text: "hó".to_string(),
        }),
        tl_config(),
    );
    assert!(bool_result(&resp));
}

#[test]
fn has_tone_marks_false_for_plain_ascii() {
    let resp = run(
        Intent::HasToneMarks(HasToneMarks {
            text: "ho".to_string(),
        }),
        tl_config(),
    );
    assert!(!bool_result(&resp));
}

#[test]
fn get_tone_variations_returns_both_modes() {
    let resp = run(Intent::GetToneVariations(GetToneVariations {}), tl_config());
    let result = tone_variations_result(&resp);
    assert!(!result.poj_variations.is_empty(), "POJ map non-empty");
    assert!(!result.tl_variations.is_empty(), "TL map non-empty");
    assert!(result.poj_variations.contains_key("a"));
    assert!(result.tl_variations.contains_key("a"));
    // TL 'oo' entry exists; POJ 'o͘' entry exists.
    assert!(result.tl_variations.contains_key("oo"));
    assert!(result.poj_variations.contains_key("o\u{0358}"));
}

// ============================================================
// Derivation
// ============================================================

#[test]
fn derive_notone_strips_diacritics_digits_hyphens_spaces() {
    let resp = run(
        Intent::DeriveNotone(DeriveNotone {
            roman: "Gâu-tsá 2".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "gautsa");
}

#[test]
fn derive_notone_converts_nasal_marker_to_nn() {
    let resp = run(
        Intent::DeriveNotone(DeriveNotone {
            roman: "siu\u{207f}".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "siunn");
}

#[test]
fn derive_abbrev_returns_first_char_per_syllable() {
    let resp = run(
        Intent::DeriveAbbrev(DeriveAbbrev {
            roman: "gâu-tsá".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "gt");
}

#[test]
fn derive_abbrev_returns_empty_on_single_syllable() {
    let resp = run(
        Intent::DeriveAbbrev(DeriveAbbrev {
            roman: "hó".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "");
}

#[test]
fn derive_abbrev_splits_on_ascii_whitespace_and_hyphen() {
    let resp = run(
        Intent::DeriveAbbrev(DeriveAbbrev {
            roman: "a\tb\nc d-e".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "abcde");
}

/// Codex v3 §1 fixture: `[ \t\n\x0B\f\r-]+` literal MUST NOT split NBSP.
/// Pinning Android JVM `Regex("[\\s-]+")` ASCII-only behavior — Rust
/// `regex` `\s` is Unicode-aware by default; using a literal char class
/// avoids splitting U+00A0.
#[test]
fn derive_abbrev_does_not_split_on_nbsp() {
    let resp = run(
        Intent::DeriveAbbrev(DeriveAbbrev {
            roman: "a\u{00A0}b".to_string(),
        }),
        tl_config(),
    );
    // NBSP is not a delimiter, so "a\u{00A0}b" is a single syllable → "" abbrev.
    assert_eq!(string_result(&resp), "");
}

// ============================================================
// TPS
// ============================================================

#[test]
fn contains_tps_true_for_zhuyin() {
    let resp = run(Intent::ContainsTps(ContainsTps { text: "ㄉㄧㄠ".to_string() }), tl_config());
    assert!(bool_result(&resp));
}

#[test]
fn contains_tps_false_for_latin() {
    let resp = run(Intent::ContainsTps(ContainsTps { text: "tiau".to_string() }), tl_config());
    assert!(!bool_result(&resp));
}

#[test]
fn tps_to_tl_basic() {
    let resp = run(Intent::TpsToTl(TpsToTl { text: "ㄉㄧㄠˊ".to_string() }), tl_config());
    let out = string_result(&resp);
    assert!(out.contains("tiau"), "got {out:?}");
}

#[test]
fn tl_numeric_to_tps_basic() {
    let resp = run(
        Intent::TlNumericToTps(TlNumericToTps {
            text: "tiau5".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(!out.is_empty(), "TL numeric → TPS should produce zhuyin: {out:?}");
}

#[test]
fn tl_display_to_tps_uses_display_form() {
    let resp = run(
        Intent::TlDisplayToTps(TlDisplayToTps {
            text: "tiâu".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(!out.is_empty(), "TL display → TPS should produce zhuyin: {out:?}");
}

/// Default (`or_maps_to_er=false`) renders the vowel `or` as ㄛ (`\u{311b}`),
/// matching the iOS pre-D9.4 default. Mirrors `TPSConverter.toTPS` /
/// `TLToTPS.convert`.
#[test]
fn tl_numeric_to_tps_or_default_uses_o_vowel() {
    let resp = run(
        Intent::TlNumericToTps(TlNumericToTps {
            text: "kor1".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(out.contains('\u{311b}'), "or → ㄛ when toggle off; got {out:?}");
    assert!(!out.contains('\u{311c}'), "must not contain ㄜ when toggle off; got {out:?}");
}

/// With the toggle ON, the same `or` syllable renders as ㄜ (`\u{311c}`),
/// matching iOS `orMapsToER=true`.
#[test]
fn tl_numeric_to_tps_or_maps_to_er_when_enabled() {
    let resp = run(
        Intent::TlNumericToTps(TlNumericToTps {
            text: "kor1".to_string(),
            or_maps_to_er: true,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(out.contains('\u{311c}'), "or → ㄜ when toggle on; got {out:?}");
    assert!(!out.contains('\u{311b}'), "must not contain ㄛ when toggle on; got {out:?}");
}

/// Multi-syllable input must preserve the syllable boundary as a single
/// space, matching iOS `joined(separator: " ")` / Android `joinToString(" ")`.
#[test]
fn tl_numeric_to_tps_preserves_syllable_boundaries() {
    let resp = run(
        Intent::TlNumericToTps(TlNumericToTps {
            text: "gua2-gua2".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    let space_count = out.matches(' ').count();
    assert_eq!(space_count, 1, "two syllables should be joined by exactly one space; got {out:?}");
}

/// Repeated hyphen `--` (or leading / trailing `-`) must NOT introduce
/// double spaces — empty tokens are filtered before joining.
#[test]
fn tl_numeric_to_tps_handles_repeated_hyphen_without_double_space() {
    let resp = run(
        Intent::TlNumericToTps(TlNumericToTps {
            text: "gua2--gua2".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(!out.contains("  "), "no double space across `--`; got {out:?}");
    assert_eq!(out.matches(' ').count(), 1, "exactly one space; got {out:?}");
}

/// Display-form path shares the helper, so the syllable-boundary fix must
/// also apply there.
#[test]
fn tl_display_to_tps_preserves_syllable_boundaries() {
    let resp = run(
        Intent::TlDisplayToTps(TlDisplayToTps {
            text: "guá-guá".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert_eq!(out.matches(' ').count(), 1, "display-form multi-syllable must keep one space; got {out:?}");
}

#[test]
fn is_tps_tone_mark_true_for_acute() {
    let resp = run(Intent::IsTpsToneMark(IsTpsToneMark { char: "\u{02ca}".to_string() }), tl_config());
    assert!(bool_result(&resp));
}

#[test]
fn is_tps_tone_mark_false_for_letter() {
    let resp = run(Intent::IsTpsToneMark(IsTpsToneMark { char: "a".to_string() }), tl_config());
    assert!(!bool_result(&resp));
}

#[test]
fn is_tps_tone_mark_false_for_empty() {
    let resp = run(Intent::IsTpsToneMark(IsTpsToneMark { char: String::new() }), tl_config());
    assert!(!bool_result(&resp));
}

// ---- TpsInputAdjust branch coverage (mirrors commit-1 platform fixtures)

fn tps_adjust(incoming: &str, raw: &str) -> (String, Option<String>) {
    let resp = run(
        Intent::TpsInputAdjust(TpsInputAdjust {
            incoming: incoming.to_string(),
            raw_input: raw.to_string(),
        }),
        tl_config(),
    );
    tps_adjust_result(&resp)
}

#[test]
fn tps_adjust_dual_form_keeps_initial_on_empty_buffer() {
    for ch in ["ㄇ", "ㄋ", "ㄫ", "ㄅ", "ㄉ", "ㄍ", "ㄏ"] {
        let (adjusted, replace) = tps_adjust(ch, "");
        assert_eq!(adjusted, ch, "{ch} on empty buffer should stay initial");
        assert_eq!(replace, None);
    }
}

#[test]
fn tps_adjust_m_after_vowel_returns_final() {
    let (adjusted, replace) = tps_adjust("ㄇ", "ㄚ");
    assert_eq!(adjusted, "ㆬ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_ng_after_i_returns_ing() {
    let (adjusted, replace) = tps_adjust("ㄫ", "ㄧ");
    assert_eq!(adjusted, "ㄥ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_ng_after_other_vowel_returns_syllabic_ng() {
    let (adjusted, replace) = tps_adjust("ㄫ", "ㄚ");
    assert_eq!(adjusted, "ㆭ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_ainn_after_i_becomes_aunn() {
    let (adjusted, replace) = tps_adjust("ㆮ", "ㄧ");
    assert_eq!(adjusted, "ㆯ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_tone_after_m_replaces_with_syllabic_m() {
    let (adjusted, replace) = tps_adjust("\u{02ca}", "ㄇ");
    assert_eq!(adjusted, "\u{02ca}");
    assert_eq!(replace, Some("ㆬ".to_string()));
}

#[test]
fn tps_adjust_tone_after_ng_replaces_with_syllabic_ng() {
    let (adjusted, replace) = tps_adjust("\u{02ca}", "ㄫ");
    assert_eq!(adjusted, "\u{02ca}");
    assert_eq!(replace, Some("ㆭ".to_string()));
}

#[test]
fn tps_adjust_tone_after_other_consonant_no_replace() {
    let (adjusted, replace) = tps_adjust("\u{02ca}", "ㄉ");
    assert_eq!(adjusted, "\u{02ca}");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_palatalization_i_after_ts_replaces() {
    let (adjusted, replace) = tps_adjust("ㄧ", "ㄗ");
    assert_eq!(adjusted, "ㄧ");
    assert_eq!(replace, Some("ㄐ".to_string()));
}

#[test]
fn tps_adjust_palatalization_inn_after_ts_replaces() {
    let (adjusted, replace) = tps_adjust("ㆪ", "ㄗ");
    assert_eq!(adjusted, "ㆪ");
    assert_eq!(replace, Some("ㄐ".to_string()));
}

#[test]
fn tps_adjust_palatalization_negative_non_trigger() {
    let (adjusted, replace) = tps_adjust("ㄚ", "ㄗ");
    assert_eq!(adjusted, "ㄚ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_palatalization_negative_non_affricate() {
    let (adjusted, replace) = tps_adjust("ㄧ", "ㄆ");
    assert_eq!(adjusted, "ㄧ");
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_dual_form_after_hyphen_returns_final() {
    // Locks the hyphen quirk — "-" is NOT in syllableBoundaryChars on iOS or
    // Android, so dual-form key after hyphen still gets final form. If
    // parity audit later flags this, fix as a separate parity: PR.
    let (adjusted, _) = tps_adjust("ㄇ", "ㄚ-");
    assert_eq!(adjusted, "ㆬ");
}

#[test]
fn tps_adjust_disjoint_trigger_invariant() {
    // Syllabic-nasal trigger set ({ㄇ, ㄫ}) is disjoint from palatalization
    // trigger set ({ㄗ, ㄘ, ㄙ, ㆡ}). Both effects can never fire on the
    // same call, so the `?:` short-circuit in dispatch is observationally
    // identical to running both checks.
    for last in ["ㄇ", "ㄫ"] {
        for incoming in ["ㄧ", "ㆪ"] {
            let (adjusted, replace) = tps_adjust(incoming, last);
            assert_eq!(adjusted, incoming);
            assert_eq!(replace, None, "{incoming} after {last} must not trigger either");
        }
    }
}

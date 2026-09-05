//! Op coverage — exercises every `PhoneticsRequest.method` variant
//! end-to-end through the dispatcher so the wire format, dispatch
//! routing, and per-op implementation stay in sync. Includes branch
//! coverage for the TPS input adjuster and NBSP-as-non-delimiter for
//! `derive_abbrev`.

// 走過每一個 `PhoneticsRequest.method` 變體的端到端測試,確保 wire 格式、分派路由、實作三者保持同步。

use phonetics::dispatch::handle;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{
    AppConfig, BoolResult, ContainsTps, DeriveAbbrev, DeriveNotone, GetToneVariations,
    IsTpsToneMark, NfdPreprocessForLookup, NormalizeInput, NormalizeToTl, NormalizeTone,
    OptionalStringResult, PhoneticsRequest, PhoneticsResponse, PojToTl, RestoreTone, StringResult,
    StripTone, StripToneResult, TlDisplayToTps, TlNumericToTps, TlToPoj, ToneVariationsResult,
    TpsAdjustResult, TpsInputAdjust,
};

// ---------------- helpers ----------------
//
// Domain-level tests: drive `phonetics::dispatch::handle` directly and
// assert against `Result<PhoneticsResponse>`. Envelope concerns
// (`taigi.engine.Request` decoding, `Response.id`/`error`/`generation`,
// panic catching) live in `engine/dispatch/tests/` — exercising them
// from the phonetics crate would invert the production dependency
// graph.

fn run(method: Method, config: AppConfig) -> PhoneticsResponse {
    let req = PhoneticsRequest {
        method: Some(method),
    };
    handle(&req, &config).expect("dispatch handle should succeed")
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

fn opt_result(resp: &PhoneticsResponse) -> Option<String> {
    let PhonResult::OptionalStringResult(OptionalStringResult { output, present }) = ok_phon(resp)
    else {
        panic!("expected OptionalStringResult");
    };
    if *present {
        Some(output.clone())
    } else {
        None
    }
}

fn strip_result(resp: &PhoneticsResponse) -> (String, String) {
    let PhonResult::StripToneResult(StripToneResult { bare, tone }) = ok_phon(resp) else {
        panic!("expected StripToneResult");
    };
    (bare.clone(), tone.clone())
}

fn tps_adjust_result(resp: &PhoneticsResponse) -> (String, Option<String>) {
    let PhonResult::TpsAdjustResult(TpsAdjustResult {
        adjusted,
        replace_last,
    }) = ok_phon(resp)
    else {
        panic!("expected TpsAdjustResult");
    };
    let opt = replace_last.as_ref().and_then(|r| {
        if r.present {
            Some(r.output.clone())
        } else {
            None
        }
    });
    (adjusted.clone(), opt)
}

fn tone_variations_result(resp: &PhoneticsResponse) -> ToneVariationsResult {
    let PhonResult::ToneVariationsResult(t) = ok_phon(resp) else {
        panic!("expected ToneVariationsResult");
    };
    t.clone()
}

fn tl_config() -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: "tl".to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: false,
        is_translate_swapped: false,
        is_association_recording_enabled: false,
        platform_id: 0,
        output_both_scripts: false,
        candidate_display_mode: 0,
    }
}

fn poj_config(oo: bool, nn: bool) -> AppConfig {
    AppConfig {
        tone_mode: String::new(),
        input_mode: "poj".to_string(),
        oo_doubletap_enabled: oo,
        nn_doubletap_enabled: nn,
        is_translate_swapped: false,
        is_association_recording_enabled: false,
        platform_id: 0,
        output_both_scripts: false,
        candidate_display_mode: 0,
    }
}

// ============================================================
// Phonetics core
// ============================================================

#[test]
fn normalize_tone_tl_basic() {
    let resp = run(
        Method::NormalizeTone(NormalizeTone {
            input: "ho2".to_string(),
        }),
        tl_config(),
    );
    let out = string_result(&resp);
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
    let resp = run(
        Method::NormalizeTone(NormalizeTone {
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
        Method::NormalizeTone(NormalizeTone {
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
        Method::NormalizeTone(NormalizeTone {
            input: "ann2".to_string(),
        }),
        poj_config(false, true),
    );
    let out = string_result(&resp);
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
    // 兩個開關是各自獨立的按鍵 affordance,同時含 oo 與 nn 的 buffer 兩邊都要生效。
    //   實機回報:打 h-o-o-n-n 顯示 ho͘nn,oo 折了、nn 沒折。根因是順序 ——
    //   nn 折疊只認「前一字元是 ASCII 母音」,而 oo 折疊正好在那裡插入 U+0358。
    //   honn(無 oo 可折)卻正常,所以上面兩個單開關測試抓不到。
    for (input, expected) in [
        ("hoonn", "ho\u{0358}\u{207f}"),
        ("hoonnh", "ho\u{0358}\u{207f}h"),
        // Unchanged by the reorder — no `oo` to fold.
        ("honn", "ho\u{207f}"),
        ("ann", "a\u{207f}"),
    ] {
        let resp = run(
            Method::NormalizeTone(NormalizeTone {
                input: input.to_string(),
            }),
            poj_config(true, true),
        );
        assert_eq!(string_result(&resp), expected, "input {input:?}");
    }
}

#[test]
fn normalize_tone_poj_doubletap_folds_uppercase() {
    // Exercises the `OO` arm together with the reordered folds AND the
    // `adjust_nasal_marker_case` pass that runs after them: the marker follows
    // the case of the nearest preceding letter, so an all-caps buffer ends on
    // `ᴺ` (U+1D3A) rather than `ⁿ`.
    // trace: "HOONN" -> nn fold -> "HOOⁿ" -> oo fold -> "HO͘ⁿ" -> case -> "HO͘ᴺ"
    // 同時驗證 OO arm、重排後的折疊順序,以及其後的 adjust_nasal_marker_case
    //   (鼻化符號跟隨前一個字母的大小寫,全大寫收在 ᴺ 而非 ⁿ)。
    for (input, expected) in [
        ("HOONN", "HO\u{0358}\u{1d3a}"),
        ("Hoonn", "Ho\u{0358}\u{207f}"),
        ("hooNN", "ho\u{0358}\u{207f}"),
    ] {
        let resp = run(
            Method::NormalizeTone(NormalizeTone {
                input: input.to_string(),
            }),
            poj_config(true, true),
        );
        assert_eq!(string_result(&resp), expected, "input {input:?}");
    }
    // A shift between the two taps still folds: the pair is matched
    // case-insensitively and the FIRST tap decides the letter's case.
    // 雙擊不分大小寫,且由「第一下」決定字母大小寫。
    // trace: "hoOnn" -> nn fold -> "hoOⁿ" -> oo fold pairs o+O, first tap
    //        lowercase -> "ho͘ⁿ" -> case pass sees `o` -> "ho͘ⁿ"
    let resp = run(
        Method::NormalizeTone(NormalizeTone {
            input: "hoOnn".to_string(),
        }),
        poj_config(true, true),
    );
    assert_eq!(string_result(&resp), "ho\u{0358}\u{207f}");
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
    // 單次左到右配對,既不會漏掉某個大小寫組合,也讀不到自己剛寫下的內容。
    //   兩者都真的發生過:舊的三次 String::replace 沒有 oO arm,而且會互相吃 ——
    //   OOoo 折成 O͘o͘ 後,第二趟match到接縫的 Oo、第三趟又match到 OO,
    //   同一個字母疊出三個結合點。
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
        let resp = run(
            Method::NormalizeTone(NormalizeTone {
                input: input.to_string(),
            }),
            poj_config(true, false),
        );
        assert_eq!(string_result(&resp), expected, "input {input:?}");
    }
}

#[test]
fn normalize_tone_poj_doubletap_toggles_stay_independent() {
    // Each toggle alone still does exactly its own rewrite and nothing else —
    // the reorder must not make one imply the other.
    // 單獨開一個開關時只做自己那一項,重排順序不可讓其中一個牽動另一個。
    let resp = run(
        Method::NormalizeTone(NormalizeTone {
            input: "hoonn".to_string(),
        }),
        poj_config(false, true),
    );
    assert_eq!(string_result(&resp), "hoo\u{207f}", "nn only");

    let resp = run(
        Method::NormalizeTone(NormalizeTone {
            input: "hoonn".to_string(),
        }),
        poj_config(true, false),
    );
    assert_eq!(string_result(&resp), "ho\u{0358}nn", "oo only");

    let resp = run(
        Method::NormalizeTone(NormalizeTone {
            input: "hoonn".to_string(),
        }),
        poj_config(false, false),
    );
    assert_eq!(string_result(&resp), "hoonn", "neither");
}

#[test]
fn strip_tone_returns_bare_and_tone() {
    let resp = run(
        Method::StripTone(StripTone {
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
        Method::PojToTl(PojToTl {
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
        Method::TlToPoj(TlToPoj {
            input: "hoo".to_string(),
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(
        out.contains('\u{0358}'),
        "TL oo should map to POJ o͘: {out:?}"
    );
}

#[test]
fn normalize_to_tl_passes_input_through_normalizer() {
    let resp = run(
        Method::NormalizeToTl(NormalizeToTl {
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
        Method::NormalizeInput(NormalizeInput {
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
        Method::NormalizeInput(NormalizeInput {
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
        Method::NormalizeInput(NormalizeInput {
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
        Method::RestoreTone(RestoreTone {
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
        Method::RestoreTone(RestoreTone {
            text: "ho".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(opt_result(&resp), None);
}

#[test]
fn get_tone_variations_returns_both_modes() {
    let resp = run(Method::GetToneVariations(GetToneVariations {}), tl_config());
    let result = tone_variations_result(&resp);
    assert!(!result.poj_variations.is_empty(), "POJ map non-empty");
    assert!(!result.tl_variations.is_empty(), "TL map non-empty");
    assert!(result.poj_variations.contains_key("a"));
    assert!(result.tl_variations.contains_key("a"));
    // TL 'oo' entry exists; POJ 'o͘' entry exists.
    assert!(result.tl_variations.contains_key("oo"));
    assert!(result.poj_variations.contains_key("o\u{0358}"));
}

#[test]
fn nfd_preprocess_for_lookup_collapses_o_dot() {
    let resp = run(
        Method::NfdPreprocessForLookup(NfdPreprocessForLookup {
            input: "ho\u{0358}".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "hoo");
}

#[test]
fn nfd_preprocess_for_lookup_substitutes_nasal_marker() {
    let resp = run(
        Method::NfdPreprocessForLookup(NfdPreprocessForLookup {
            input: "sa\u{207f}".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "sann");
}

// ============================================================
// Derivation
// ============================================================

#[test]
fn derive_notone_strips_diacritics_digits_hyphens_spaces() {
    let resp = run(
        Method::DeriveNotone(DeriveNotone {
            roman: "Gâu-tsá 2".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "gautsa");
}

#[test]
fn derive_notone_converts_nasal_marker_to_nn() {
    let resp = run(
        Method::DeriveNotone(DeriveNotone {
            roman: "siu\u{207f}".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "siunn");
}

#[test]
fn derive_abbrev_returns_first_char_per_syllable() {
    let resp = run(
        Method::DeriveAbbrev(DeriveAbbrev {
            roman: "gâu-tsá".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "gt");
}

#[test]
fn derive_abbrev_returns_empty_on_single_syllable() {
    let resp = run(
        Method::DeriveAbbrev(DeriveAbbrev {
            roman: "hó".to_string(),
        }),
        tl_config(),
    );
    assert_eq!(string_result(&resp), "");
}

#[test]
fn derive_abbrev_splits_on_ascii_whitespace_and_hyphen() {
    let resp = run(
        Method::DeriveAbbrev(DeriveAbbrev {
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
        Method::DeriveAbbrev(DeriveAbbrev {
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
    let resp = run(
        Method::ContainsTps(ContainsTps {
            text: "ㄉㄧㄠ".to_string(),
        }),
        tl_config(),
    );
    assert!(bool_result(&resp));
}

#[test]
fn contains_tps_false_for_latin() {
    let resp = run(
        Method::ContainsTps(ContainsTps {
            text: "tiau".to_string(),
        }),
        tl_config(),
    );
    assert!(!bool_result(&resp));
}

#[test]
fn tl_numeric_to_tps_basic() {
    let resp = run(
        Method::TlNumericToTps(TlNumericToTps {
            text: "tiau5".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(
        !out.is_empty(),
        "TL numeric → TPS should produce zhuyin: {out:?}"
    );
}

#[test]
fn tl_display_to_tps_uses_display_form() {
    let resp = run(
        Method::TlDisplayToTps(TlDisplayToTps {
            text: "tiâu".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert!(
        !out.is_empty(),
        "TL display → TPS should produce zhuyin: {out:?}"
    );
}

/// Default (`or_maps_to_er=false`) renders the vowel `or` as ㄛ (`\u{311b}`),
/// matching the iOS pre-D9.4 default. Mirrors `TPSConverter.toTPS` /
/// `TLToTPS.convert`.
#[test]
fn tl_numeric_to_tps_or_default_uses_o_vowel() {
    let resp = run(
        Method::TlNumericToTps(TlNumericToTps {
            text: "kor1".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
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
    let resp = run(
        Method::TlNumericToTps(TlNumericToTps {
            text: "kor1".to_string(),
            or_maps_to_er: true,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
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
    let resp = run(
        Method::TlNumericToTps(TlNumericToTps {
            text: "gua2-gua2".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    let space_count = out.matches(' ').count();
    assert_eq!(
        space_count, 1,
        "two syllables should be joined by exactly one space; got {out:?}"
    );
}

/// Repeated hyphen `--` (or leading / trailing `-`) must NOT introduce
/// double spaces — empty tokens are filtered before joining.
#[test]
fn tl_numeric_to_tps_handles_repeated_hyphen_without_double_space() {
    let resp = run(
        Method::TlNumericToTps(TlNumericToTps {
            text: "gua2--gua2".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
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

/// Display-form path shares the helper, so the syllable-boundary fix must
/// also apply there.
#[test]
fn tl_display_to_tps_preserves_syllable_boundaries() {
    let resp = run(
        Method::TlDisplayToTps(TlDisplayToTps {
            text: "guá-guá".to_string(),
            or_maps_to_er: false,
        }),
        tl_config(),
    );
    let out = string_result(&resp);
    assert_eq!(
        out.matches(' ').count(),
        1,
        "display-form multi-syllable must keep one space; got {out:?}"
    );
}

#[test]
fn is_tps_tone_mark_true_for_acute() {
    let resp = run(
        Method::IsTpsToneMark(IsTpsToneMark {
            char: "\u{02ca}".to_string(),
        }),
        tl_config(),
    );
    assert!(bool_result(&resp));
}

#[test]
fn is_tps_tone_mark_false_for_letter() {
    let resp = run(
        Method::IsTpsToneMark(IsTpsToneMark {
            char: "a".to_string(),
        }),
        tl_config(),
    );
    assert!(!bool_result(&resp));
}

#[test]
fn is_tps_tone_mark_false_for_empty() {
    let resp = run(
        Method::IsTpsToneMark(IsTpsToneMark {
            char: String::new(),
        }),
        tl_config(),
    );
    assert!(!bool_result(&resp));
}

// ---- TpsInputAdjust branch coverage (mirrors commit-1 platform fixtures)

fn tps_adjust(incoming: &str, raw: &str) -> (String, Option<String>) {
    let resp = run(
        Method::TpsInputAdjust(TpsInputAdjust {
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
            assert_eq!(
                replace, None,
                "{incoming} after {last} must not trigger either"
            );
        }
    }
}

#[test]
fn tps_adjust_reported_bug_repro_taiuan_tai() {
    // Reported 2026-05-27: TPS continuous input `ㄉㄞㄨㄢㄉㄞㆣㄧ`
    // (intended `Tâi-uân-tâi-gí` = 臺灣台語) corrupted to
    // `ㄉㄞㄨㄢㆵㄞㆣㄧ`. The second `ㄉ` lands after precomposed nasal
    // coda `ㄢ` (U+3122), which was missing from SYLLABLE_BOUNDARY_CHARS.
    let (adjusted, replace) = tps_adjust("ㄉ", "ㄉㄞㄨㄢ");
    assert_eq!(
        adjusted, "ㄉ",
        "ㄉ after precomposed nasal coda ㄢ must stay initial"
    );
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_precomposed_nasal_coda_finals_are_boundary() {
    // Every precomposed nasal-coda compound final (am/an/ang/om/ong) ends
    // the syllable for every dual-form initial. Mirrors the existing
    // syllabic-nasal coverage of `ㆬ ㄣ ㆭ ㄥ`.
    let finals = ["ㆬ", "ㄣ", "ㆭ", "ㄥ", "ㆰ", "ㄢ", "ㄤ", "ㆱ", "ㆲ"];
    let initials = ["ㄇ", "ㄋ", "ㄫ", "ㄅ", "ㄉ", "ㄍ", "ㄏ"];
    for last in finals {
        for incoming in initials {
            let raw = format!("ㄚ{last}");
            let (adjusted, _) = tps_adjust(incoming, &raw);
            assert_eq!(
                adjusted, incoming,
                "{incoming} after nasal-coda final {last} must stay initial"
            );
        }
    }
}

#[test]
fn tps_adjust_nasalized_vowel_is_boundary_for_all_initials() {
    // Nasalized vowels (ainn/aunn/ann/enn/inn/onn/unn) are complete syllables
    // and end the syllable for EVERY dual-form initial — including `ㄏ`. The
    // earlier `ㄏ`-exception (treating `<nasalized-vowel> + ㄏ` as the
    // nasalized checked final `-nnh`) was reverted because it broke common
    // continuous-input cases like `ㄏㄨㆩㄏㄧ` (歡喜 = huann-hi); see the
    // dedicated regression test `tps_adjust_continuous_input_huann_hi_keeps_h_initial`.
    // Each fixture uses a realistic raw-input prefix that produces a legal
    // standalone nasalized syllable.
    let cases: [(&str, &str); 7] = [
        ("ㆪ", "ㄒㄧㆪ"), // sinn (新)
        ("ㆥ", "ㄙㆥ"),   // senn
        ("ㆧ", "ㆧ"),     // onn standalone
        ("ㆫ", "ㄏㄧㆫ"), // hiunn
        ("ㆩ", "ㄍㄧㆩ"), // kiann (驚)
        ("ㆮ", "ㄆㆮ"),   // phainn (歹)
        ("ㆯ", "ㄎㆯ"),   // khaunn
    ];
    let all_dual_initials = ["ㄇ", "ㄋ", "ㄫ", "ㄅ", "ㄉ", "ㄍ", "ㄏ"];
    for (last, raw) in cases {
        for incoming in all_dual_initials {
            let (adjusted, _) = tps_adjust(incoming, raw);
            assert_eq!(
                adjusted, incoming,
                "{incoming} after nasalized vowel {last} (raw `{raw}`) must stay initial"
            );
        }
    }
}

#[test]
fn tps_adjust_continuous_input_huann_hi_keeps_h_initial() {
    // Regression for Codex PR #348 r3308735236 — dictionary entry `歡喜`
    // (huann-hi) has TPS key `ㄏㄨㆩㄏㄧ`. The second `ㄏ` lands after the
    // nasalized vowel `ㆩ` and MUST stay as initial; otherwise it converts
    // to `ㆷ`, producing `ㄏㄨㆩㆷㄧ` which misses the FST key.
    let (adjusted, replace) = tps_adjust("ㄏ", "ㄏㄨㆩ");
    assert_eq!(
        adjusted, "ㄏ",
        "ㄏ after nasalized vowel ㆩ in continuous input (huann-hi 歡喜) must stay initial"
    );
    assert_eq!(replace, None);
}

#[test]
fn tps_adjust_standalone_nasalized_vowel_plus_h_no_longer_synthesizes_nnh() {
    // Pin the documented trade-off (knowledge/tps-auto-correct-rules.md
    // Rule 2): TPS auto-correct deliberately does NOT synthesize the
    // nasalized checked final `-nnh` from `<nasalized-vowel> + ㄏ` because
    // it cannot disambiguate from cross-syllable `<nasalized-vowel> + ㄏ-initial`
    // (e.g. 歡喜 = ㄏㄨㆩㄏㄧ). `-nnh` dictionary entries store `ㆷ`
    // directly and surface via lookup, not via input rewriting.
    for raw in ["ㆩ", "ㆥ", "ㆪ", "ㆫ", "ㆧ", "ㆮ", "ㆯ"] {
        let (adjusted, _) = tps_adjust("ㄏ", raw);
        assert_eq!(
            adjusted, "ㄏ",
            "ㄏ after standalone nasalized vowel {raw} must stay initial (no -nnh synthesis)"
        );
    }
}

#[test]
fn tps_adjust_pure_vowel_still_triggers_entering_tone_final() {
    // Regression guard: pure-vowel finals are INTENTIONALLY excluded from
    // the boundary set so single-syllable entering-tone input still works
    // (ㄍㄚ + ㄉ → ㄍㄚㆵ for `kat`). Pinning the documented examples from
    // knowledge/tps-auto-correct-rules.md §Rule 2.
    let cases = [
        ("ㄅ", "ㄍㄚ", "ㆴ"),
        ("ㄉ", "ㄍㄚ", "ㆵ"),
        ("ㄍ", "ㄍㄚ", "ㆻ"),
        ("ㄏ", "ㄍㄚ", "ㆷ"),
        ("ㄇ", "ㄚ", "ㆬ"),
        ("ㄋ", "ㄚ", "ㄣ"),
    ];
    for (incoming, raw, expected) in cases {
        let (adjusted, _) = tps_adjust(incoming, raw);
        assert_eq!(
            adjusted, expected,
            "{incoming} after pure vowel `{raw}` must convert to {expected}"
        );
    }
}

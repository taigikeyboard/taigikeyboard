//! `NormalizeTone` integration tests for in-band nasal-marker case
//! agreement: the case of the POJ nasal marker (ⁿ U+207F vs ᴺ U+1D3A)
//! must agree with the case of the preceding letter on the way out of
//! `NormalizeTone`. The standalone `adjust_nasal_marker_case` unit
//! tests live alongside the helper in `engine/phonetics/src/case_adjust.rs`;
//! these tests pin the rule end-to-end through the dispatcher.

use phonetics::dispatch::handle;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{AppConfig, NormalizeTone, PhoneticsRequest, PhoneticsResponse, StringResult};

fn run(method: Method, config: AppConfig) -> PhoneticsResponse {
    let req = PhoneticsRequest {
        method: Some(method),
    };
    handle(&req, &config).expect("dispatch handle should succeed")
}

fn string_result(resp: &PhoneticsResponse) -> &str {
    let result = resp.result.as_ref().expect("phonetics result");
    match result {
        PhonResult::StringResult(StringResult { output }) => output.as_str(),
        other => panic!("expected StringResult, got {other:?}"),
    }
}

#[test]
fn normalize_tone_uppercase_input_promotes_nasal_marker() {
    // POJ `ANN2` with `nn_doubletap_enabled`: preprocess → `A` + `\u{207f}` +
    // `2`, then `to_tone_marks` adds tone diacritic on `A`. Final character
    // sequence has uppercase letters preceding the nasal marker, so the
    // adjustment must promote `\u{207f}` → `\u{1D3A}`.
    let cfg = AppConfig {
        input_mode: "POJ".to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: true,
        ..Default::default()
    };
    let resp = run(
        Method::NormalizeTone(NormalizeTone {
            input: "ANN2".to_string(),
        }),
        cfg,
    );
    let out = string_result(&resp);
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
    let cfg = AppConfig {
        input_mode: "POJ".to_string(),
        oo_doubletap_enabled: false,
        nn_doubletap_enabled: true,
        ..Default::default()
    };
    let resp = run(
        Method::NormalizeTone(NormalizeTone {
            input: "ANN2-ann2".to_string(),
        }),
        cfg,
    );
    let out = string_result(&resp);
    assert!(
        out.contains('\u{1D3A}'),
        "expected ᴺ in uppercase syllable: {out:?}"
    );
    assert!(
        out.contains('\u{207F}'),
        "expected ⁿ in lowercase syllable: {out:?}"
    );
}

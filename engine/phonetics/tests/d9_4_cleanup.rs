//! `NormalizeTone` integration tests pinning the in-band nasal-marker case
//! agreement that was added during D9.4 cleanup. Closes the gap identified
//! in commit `29b5a2a` and the `feedback_rust_swap_preserves_pipeline.md`
//! meta-feedback.
//!
//! Standalone `adjust_nasal_marker_case` is exercised by the in-source unit
//! tests in `engine/phonetics/src/case_adjust.rs`. No FFI-exposed op for it
//! because the only would-be caller (`SuggestionCaseTransformer`) reverted
//! to a platform-side helper for JVM unit-test compatibility — same reason
//! `TaigiUnicode.nfdPreprocessed` stayed on the platform (no Rust op for
//! that one either).

use phonetics::api::process_request;
use prost::Message;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{
    request, response, AppConfig, NormalizeTone, PhoneticsRequest, Request, Response, StringResult,
};

fn run(method: Method, config: AppConfig) -> Response {
    let req = Request {
        id: 99,
        r#type: 0,
        config_snapshot: Some(config),
        generation: 0,
        payload: Some(request::Payload::Phonetics(PhoneticsRequest {
            method: Some(method),
        })),
    };
    let bytes = req.encode_to_vec();
    let response_bytes = process_request(&bytes);
    Response::decode(response_bytes.as_slice()).expect("response decodes")
}

fn string_result(resp: &Response) -> &str {
    let Some(response::Payload::Phonetics(phon)) = resp.payload.as_ref() else {
        panic!("expected phonetics payload");
    };
    let result = phon.result.as_ref().expect("phonetics result");
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
    assert!(out.contains('\u{1D3A}'), "expected ᴺ in uppercase syllable: {out:?}");
    assert!(out.contains('\u{207F}'), "expected ⁿ in lowercase syllable: {out:?}");
}

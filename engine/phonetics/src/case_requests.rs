//! Case-transform request handler.
//!
//! Routes `taigi.engine.CaseRequest` oneof variants to the corresponding
//! `case_transform` functions, builds a `CaseResponse`. The input mode
//! does not affect casing; only `force_lowercase_nasal_marker` is read
//! from the envelope.
//!
//! Returns `None` for an empty `method` oneof or an unset `ToneCase.kind`
//! (the top-level dispatch converts that to `ErrorCode::FailInvariant`).
//! Otherwise always returns `Some(CaseResponse)` — the case ops are
//! infallible by construction (stdlib casing + the nasal-marker pair).

use crate::case_transform::{
    full_uppercase_tone_string, lowercase_tone_char, transform_input_case, uppercase_tone_char,
    LetterCase,
};
use protos::engine::case_request::Method;
use protos::engine::{AppConfig, CaseRequest, CaseResponse, CaseStringResult, ToneCaseKind};

pub fn handle(request: &CaseRequest, config: &AppConfig) -> Option<CaseResponse> {
    let method = request.method.as_ref()?;

    let output = match method {
        Method::ToneCase(req) => match req.kind() {
            ToneCaseKind::UppercaseFirst => uppercase_tone_char(&req.input),
            ToneCaseKind::UppercaseAll => full_uppercase_tone_string(&req.input),
            ToneCaseKind::Lowercase => lowercase_tone_char(&req.input),
            ToneCaseKind::Unspecified => return None,
        },
        Method::TransformInputCase(req) => {
            transform_input_case(&req.text, proto_to_letter_case(req.letter_case()))
        }
    };
    // ⁿ becomes ᴺ in capitals OFF (§53): the raise-only ops write `ᴺ` after a capital; fold
    // it back so the platform never receives the capital marker.
    let output = if config.force_lowercase_nasal_marker {
        crate::case_transform::lowercase_nasal_markers(&output)
    } else {
        output
    };

    Some(CaseResponse {
        result: Some(protos::engine::case_response::Result::StringResult(
            CaseStringResult { output },
        )),
    })
}

/// Translate proto `LetterCase` enum to the Rust `case_transform::LetterCase`.
/// Unspecified / unrecognised → `Lowercased` (safe-fallback contract; the
/// platform bridges always populate the field, an unset value indicates
/// proto schema mismatch and we return the most conservative behavior).
fn proto_to_letter_case(proto: protos::engine::LetterCase) -> LetterCase {
    use protos::engine::LetterCase as P;
    match proto {
        P::Uppercased => LetterCase::Uppercased,
        P::CapsLocked => LetterCase::CapsLocked,
        P::Lowercased | P::Unspecified => LetterCase::Lowercased,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::{ToneCase, TransformInputCase};

    fn tone_case(input: &str, kind: ToneCaseKind) -> Method {
        Method::ToneCase(ToneCase {
            input: input.to_string(),
            kind: kind as i32,
        })
    }

    fn unwrap_string(resp: CaseResponse) -> String {
        match resp.result.expect("result present") {
            protos::engine::case_response::Result::StringResult(s) => s.output,
        }
    }

    #[test]
    fn dispatch_uppercase_tone_char_raises_tone_letter() {
        let req = CaseRequest {
            method: Some(tone_case("á", ToneCaseKind::UppercaseFirst)),
        };
        let resp = handle(&req, &AppConfig::default()).expect("response present");
        assert_eq!(unwrap_string(resp), "Á");
    }

    #[test]
    fn dispatch_full_uppercase_tone_string_all_chars() {
        let req = CaseRequest {
            method: Some(tone_case("tsh", ToneCaseKind::UppercaseAll)),
        };
        let resp = handle(&req, &AppConfig::default()).expect("response present");
        assert_eq!(unwrap_string(resp), "TSH");
    }

    #[test]
    fn dispatch_lowercase_tone_char_lowers_tone_letter() {
        let req = CaseRequest {
            method: Some(tone_case("Á", ToneCaseKind::Lowercase)),
        };
        let resp = handle(&req, &AppConfig::default()).expect("response present");
        assert_eq!(unwrap_string(resp), "á");
    }

    #[test]
    fn dispatch_transform_input_case_caps_locked() {
        let req = CaseRequest {
            method: Some(Method::TransformInputCase(TransformInputCase {
                text: "tsh".to_string(),
                letter_case: protos::engine::LetterCase::CapsLocked as i32,
            })),
        };
        let resp = handle(&req, &AppConfig::default()).expect("response present");
        assert_eq!(unwrap_string(resp), "TSH");
    }

    #[test]
    fn dispatch_force_lowercase_nasal_marker_folds_the_capital_marker() {
        // ⁿ becomes ᴺ in capitals OFF (§53): the shifted nasal-marker key.
        // trace: `uppercase_tone_char` maps `ⁿ` → `ᴺ` (mode-independent
        // shortcut); the OFF fold lowers it back.
        let marker_key = CaseRequest {
            method: Some(tone_case("\u{207f}", ToneCaseKind::UppercaseFirst)),
        };
        let default = handle(&marker_key, &AppConfig::default()).expect("response present");
        assert_eq!(unwrap_string(default), "\u{1d3a}");
        let lowercase = AppConfig {
            force_lowercase_nasal_marker: true,
            ..AppConfig::default()
        };
        let resp = handle(&marker_key, &lowercase).expect("response present");
        assert_eq!(unwrap_string(resp), "\u{207f}");
    }

    /// TL tone 9 (`a̋`, combining U+030B) keeps its mark through the wire.
    #[test]
    fn dispatch_uppercase_keeps_tl_tone_nine_mark() {
        let req = CaseRequest {
            method: Some(tone_case("a̋", ToneCaseKind::UppercaseFirst)),
        };
        let resp = handle(&req, &AppConfig::default()).expect("response present");
        assert_eq!(unwrap_string(resp), "A̋");
    }

    #[test]
    fn dispatch_returns_none_for_unspecified_tone_case_kind() {
        let req = CaseRequest {
            method: Some(tone_case("a", ToneCaseKind::Unspecified)),
        };
        assert!(handle(&req, &AppConfig::default()).is_none());
    }

    #[test]
    fn dispatch_returns_none_for_missing_method() {
        let req = CaseRequest { method: None };
        assert!(handle(&req, &AppConfig::default()).is_none());
    }

    #[test]
    fn dispatch_proto_letter_case_unspecified_falls_back_lowercased() {
        // letter_case omitted → 0 (Unspecified) → Lowercased fallback.
        let req = CaseRequest {
            method: Some(Method::TransformInputCase(TransformInputCase {
                text: "Á".to_string(),
                letter_case: 0,
            })),
        };
        let resp = handle(&req, &AppConfig::default()).expect("response present");
        assert_eq!(unwrap_string(resp), "á");
    }
}

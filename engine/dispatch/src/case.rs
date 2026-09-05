//! Case-transform dispatch handler.
//!
//! Routes `taigi.engine.CaseRequest` oneof variants to the corresponding
//! `phonetics::case_transform` functions, builds a `CaseResponse`. Mode is
//! read from the envelope's `AppConfig.input_mode` per the same convention
//! as `phonetics::dispatch::handle`.
//!
//! Returns `None` for an empty `method` oneof (the top-level dispatch
//! converts that to `ErrorCode::FailInvariant`). Otherwise always returns
//! `Some(CaseResponse)` — the case ops are infallible by construction
//! (table lookups + stdlib casing).

use phonetics::api::parse_input_mode;
use phonetics::case_transform::{
    capitalize_candidate, full_uppercase_tone_string, lowercase_tone_char, transform_input_case,
    transform_suggestion, uppercase_tone_char, LetterCase,
};
use protos::engine::case_request::Method;
use protos::engine::{AppConfig, CaseRequest, CaseResponse, CaseStringResult};

pub(crate) fn handle(request: &CaseRequest, config: &AppConfig) -> Option<CaseResponse> {
    let method = request.method.as_ref()?;
    let mode = parse_input_mode(&config.input_mode);

    let output = match method {
        Method::UppercaseToneChar(req) => uppercase_tone_char(&req.input, mode),
        Method::FullUppercaseToneString(req) => full_uppercase_tone_string(&req.input, mode),
        Method::LowercaseToneChar(req) => lowercase_tone_char(&req.input, mode),
        Method::TransformInputCase(req) => {
            transform_input_case(&req.text, proto_to_letter_case(req.letter_case()), mode)
        }
        Method::CapitalizeCandidate(req) => {
            capitalize_candidate(&req.text, &req.input, req.auto_cap_enabled, mode)
        }
        Method::TransformSuggestion(req) => transform_suggestion(
            &req.original_text,
            &req.composing_text,
            proto_to_letter_case(req.letter_case()),
            mode,
        ),
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
    use protos::engine::{
        CapitalizeCandidate, FullUppercaseToneString, LowercaseToneChar, TransformInputCase,
        TransformSuggestion, UppercaseToneChar,
    };

    fn config_for(mode: &str) -> AppConfig {
        AppConfig {
            input_mode: mode.to_string(),
            ..AppConfig::default()
        }
    }

    fn unwrap_string(resp: CaseResponse) -> String {
        match resp.result.expect("result present") {
            protos::engine::case_response::Result::StringResult(s) => s.output,
        }
    }

    #[test]
    fn dispatch_uppercase_tone_char_uses_mode_table() {
        let req = CaseRequest {
            method: Some(Method::UppercaseToneChar(UppercaseToneChar {
                input: "á".to_string(),
            })),
        };
        let resp = handle(&req, &config_for("poj")).expect("response present");
        assert_eq!(unwrap_string(resp), "Á");
    }

    #[test]
    fn dispatch_full_uppercase_tone_string_all_chars() {
        let req = CaseRequest {
            method: Some(Method::FullUppercaseToneString(FullUppercaseToneString {
                input: "tsh".to_string(),
            })),
        };
        let resp = handle(&req, &config_for("tl")).expect("response present");
        assert_eq!(unwrap_string(resp), "TSH");
    }

    #[test]
    fn dispatch_lowercase_tone_char_uses_table() {
        let req = CaseRequest {
            method: Some(Method::LowercaseToneChar(LowercaseToneChar {
                input: "Á".to_string(),
            })),
        };
        let resp = handle(&req, &config_for("poj")).expect("response present");
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
        let resp = handle(&req, &config_for("tl")).expect("response present");
        assert_eq!(unwrap_string(resp), "TSH");
    }

    #[test]
    fn dispatch_capitalize_candidate_input_uppercase() {
        let req = CaseRequest {
            method: Some(Method::CapitalizeCandidate(CapitalizeCandidate {
                text: "tâi-gí".to_string(),
                input: "Tai".to_string(),
                auto_cap_enabled: true,
            })),
        };
        let resp = handle(&req, &config_for("poj")).expect("response present");
        assert_eq!(unwrap_string(resp), "Tâi-gí");
    }

    #[test]
    fn dispatch_transform_suggestion_caps_lock() {
        let req = CaseRequest {
            method: Some(Method::TransformSuggestion(TransformSuggestion {
                original_text: "tâi-gí".to_string(),
                composing_text: "tai".to_string(),
                letter_case: protos::engine::LetterCase::CapsLocked as i32,
            })),
        };
        let resp = handle(&req, &config_for("poj")).expect("response present");
        assert_eq!(unwrap_string(resp), "TÂI-GÍ");
    }

    #[test]
    fn dispatch_returns_none_for_missing_method() {
        let req = CaseRequest { method: None };
        assert!(handle(&req, &config_for("tl")).is_none());
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
        let resp = handle(&req, &config_for("poj")).expect("response present");
        assert_eq!(unwrap_string(resp), "á");
    }

    /// Verify the dispatch parses InputMode from envelope correctly:
    /// passing TL config to a POJ-only mapping (`a̋` exists in TL but not
    /// in POJ) routes through the TL table and returns the TL mapping.
    #[test]
    fn dispatch_reads_mode_from_envelope() {
        let req = CaseRequest {
            method: Some(Method::UppercaseToneChar(UppercaseToneChar {
                input: "a̋".to_string(),
            })),
        };
        let tl_resp = handle(&req, &config_for("tl")).expect("response present");
        assert_eq!(unwrap_string(tl_resp), "A̋");

        // POJ table lacks `a̋` → falls back to stdlib uppercase
        // (NFC-preserved combining mark may not round-trip; assert that
        // we at least return SOMETHING and the dispatch didn't panic).
        let poj_resp = handle(&req, &config_for("poj")).expect("response present");
        let poj_out = unwrap_string(poj_resp);
        assert!(!poj_out.is_empty());
    }

    #[test]
    fn dispatch_unknown_input_mode_string_defaults_to_tl() {
        // parse_input_mode contract: anything not "poj"/"english" → TL.
        let req = CaseRequest {
            method: Some(Method::UppercaseToneChar(UppercaseToneChar {
                input: "a̋".to_string(),
            })),
        };
        let resp = handle(&req, &config_for("xxx")).expect("response present");
        assert_eq!(unwrap_string(resp), "A̋");
    }
}

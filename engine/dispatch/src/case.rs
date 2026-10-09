//! Case-transform dispatch handler.
//!
//! Routes `taigi.engine.CaseRequest` oneof variants to the corresponding
//! `phonetics::case_transform` functions, builds a `CaseResponse`. Mode is
//! the envelope's `phonetics::api::composing_mode` — the TPS layout cases
//! with the TL tables, as it did while TPS arrived as `"tl"`.
//!
//! Returns `None` for an empty `method` oneof (the top-level dispatch
//! converts that to `ErrorCode::FailInvariant`). Otherwise always returns
//! `Some(CaseResponse)` — the case ops are infallible by construction
//! (table lookups + stdlib casing).

use phonetics::api::composing_mode;
use phonetics::case_transform::{
    full_uppercase_tone_string, lowercase_tone_char, transform_input_case, uppercase_tone_char,
    LetterCase,
};
use protos::engine::case_request::Method;
use protos::engine::{AppConfig, CaseRequest, CaseResponse, CaseStringResult};

pub(crate) fn handle(request: &CaseRequest, config: &AppConfig) -> Option<CaseResponse> {
    let method = request.method.as_ref()?;
    let mode = composing_mode(config);

    let output = match method {
        Method::UppercaseToneChar(req) => uppercase_tone_char(&req.input, mode),
        Method::FullUppercaseToneString(req) => full_uppercase_tone_string(&req.input, mode),
        Method::LowercaseToneChar(req) => lowercase_tone_char(&req.input, mode),
        Method::TransformInputCase(req) => {
            transform_input_case(&req.text, proto_to_letter_case(req.letter_case()), mode)
        }
    };
    // ⁿ becomes ᴺ in capitals OFF (§53): the raise-only ops write `ᴺ` after a capital; fold
    // it back so the platform never receives the capital marker.
    let output = if config.force_lowercase_nasal_marker {
        phonetics::case_transform::lowercase_nasal_markers(&output)
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
    use protos::engine::{
        FullUppercaseToneString, LowercaseToneChar, TransformInputCase, UppercaseToneChar,
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
    fn dispatch_force_lowercase_nasal_marker_folds_the_capital_marker() {
        // ⁿ becomes ᴺ in capitals OFF (§53): the shifted nasal-marker key.
        // trace: `uppercase_tone_char` maps `ⁿ` → `ᴺ` (mode-independent
        // shortcut); the OFF fold lowers it back.
        let marker_key = CaseRequest {
            method: Some(Method::UppercaseToneChar(UppercaseToneChar {
                input: "\u{207f}".to_string(),
            })),
        };
        let default = handle(&marker_key, &config_for("poj")).expect("response present");
        assert_eq!(unwrap_string(default), "\u{1d3a}");
        let lowercase = AppConfig {
            force_lowercase_nasal_marker: true,
            ..config_for("poj")
        };
        let resp = handle(&marker_key, &lowercase).expect("response present");
        assert_eq!(unwrap_string(resp), "\u{207f}");
    }

    /// The TPS layout (`"tps"`) cases with the TL tables, exactly as `"tl"`
    /// does — every op, including `a̋`, which only TL maps.
    #[test]
    fn dispatch_tps_layout_cases_like_tl() {
        let letter_case = protos::engine::LetterCase::CapsLocked as i32;
        let methods = [
            Method::UppercaseToneChar(UppercaseToneChar {
                input: "a̋".to_string(),
            }),
            Method::FullUppercaseToneString(FullUppercaseToneString {
                input: "tshiâⁿ".to_string(),
            }),
            Method::LowercaseToneChar(LowercaseToneChar {
                input: "Á".to_string(),
            }),
            Method::TransformInputCase(TransformInputCase {
                text: "tsh".to_string(),
                letter_case,
            }),
        ];
        for method in methods {
            let req = CaseRequest {
                method: Some(method),
            };
            let tl = handle(&req, &config_for("tl")).expect("response present");
            let tps = handle(&req, &config_for("tps")).expect("response present");
            assert_eq!(tps, tl, "{req:?}");
        }
        let req = CaseRequest {
            method: Some(Method::UppercaseToneChar(UppercaseToneChar {
                input: "a̋".to_string(),
            })),
        };
        let tps = handle(&req, &config_for("tps")).expect("response present");
        assert_eq!(unwrap_string(tps), "A̋");
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
        // composing_mode contract: anything not "poj"/"english" → TL.
        let req = CaseRequest {
            method: Some(Method::UppercaseToneChar(UppercaseToneChar {
                input: "a̋".to_string(),
            })),
        };
        let resp = handle(&req, &config_for("xxx")).expect("response present");
        assert_eq!(unwrap_string(resp), "A̋");
    }
}

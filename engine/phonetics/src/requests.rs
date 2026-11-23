//! Phonetics request dispatcher — routes `PhoneticsRequest.method` variants
//! to their implementations (`tone_variations`, the `punctuation` predicates
//! via `api`, `double_space`, `external_lookup`) behind the single FFI entry
//! (`docs/engine/rust-core-proto.md` §3). `case_requests` owns the
//! `CaseRequest` handler, re-exported here as `handle_case`.

use crate::api::PhoneticsError;
use crate::tone_variations;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{BoolResult, PhoneticsRequest, PhoneticsResponse, StringResult};

/// Dispatch a decoded `CaseRequest` — the one request family that reads the
/// `AppConfig` snapshot (input mode, capitals-OFF nasal fold).
pub use crate::case_requests::handle as handle_case;

/// Dispatch a decoded `PhoneticsRequest`. Every remaining op is a pure
/// function of its payload — none reads the `AppConfig` snapshot.
pub fn handle(req: &PhoneticsRequest) -> Result<PhoneticsResponse, PhoneticsError> {
    let Some(method) = &req.method else {
        // The most common cause is a Swift/Rust proto schema mismatch
        // (xcframework built before the proto was updated). Run
        // `make build` to regenerate.
        log::warn!(
            "PhoneticsRequest.method is None — likely Swift/Rust proto schema mismatch (rebuild xcframework with `make build`)"
        );
        return Err(PhoneticsError::UnsupportedOp);
    };

    let result = match method {
        Method::GetToneVariations(_) => PhonResult::ToneVariationsResult(tone_variations::build()),

        Method::IsAttachingPunctuation(payload) => PhonResult::BoolResult(BoolResult {
            value: crate::api::is_attaching_punctuation(&payload.text),
        }),
        Method::IsJoinerPunctuation(payload) => PhonResult::BoolResult(BoolResult {
            value: crate::api::is_joiner_punctuation(&payload.text),
        }),
        Method::IsSpacedPunctuation(payload) => PhonResult::BoolResult(BoolResult {
            value: crate::api::is_spaced_punctuation(&payload.text),
        }),
        Method::DoubleSpacePeriod(payload) => PhonResult::StringResult(StringResult {
            output: crate::double_space::double_space_period(
                &payload.text_before_caret,
                payload.millis_since_plain_space,
                payload.full_width,
            )
            .unwrap_or("")
            .to_string(),
        }),
        Method::ExternalLookupDigitForm(payload) => PhonResult::StringResult(StringResult {
            output: crate::external_lookup::digit_tone_form(&payload.reading),
        }),
    };

    Ok(PhoneticsResponse {
        result: Some(result),
    })
}

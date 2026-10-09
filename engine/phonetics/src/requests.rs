//! Phonetics request dispatcher — routes `PhoneticsRequest.method` variants
//! to the corresponding implementation. All variants live behind a single FFI
//! entry per `docs/engine/rust-core-proto.md` §3 and the architectural
//! convergence doc with khiin-rs / McBopomofo.
//!
//! Helpers are split across submodules:
//! - This file owns the dispatch + result-shape construction.
//! - `normalization` owns the InputNormalizer port (NFD / combining-mark
//!   mechanics).
//! - `punctuation` and `external_lookup` hold the two platform text helpers.
//! - `tps_adjust` owns the TPSAdjustmentBundle port (collapsed entry).
//! - `tone_variations` owns the GetToneVariations init-pull table builder.
//! - `api`, `syllable`, `tps`, `poj`, `tl`, `tables`, `case_adjust`
//!   provide the foundational helpers reused here.

use crate::api::{tl_display_to_poj_display, tl_display_to_tps, PhoneticsError};
use crate::tone_variations;
use crate::tps_adjust;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{
    BoolResult, OptionalStringResult, PhoneticsRequest, PhoneticsResponse, StringResult,
    TpsAdjustResult,
};

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
        // --- Phonetics core ---
        Method::TlToPoj(payload) => PhonResult::StringResult(StringResult {
            output: tl_display_to_poj_display(&payload.input),
        }),
        Method::GetToneVariations(_) => PhonResult::ToneVariationsResult(tone_variations::build()),

        // --- TPS ---
        Method::TlDisplayToTps(payload) => PhonResult::StringResult(StringResult {
            output: tl_display_to_tps(&payload.text, payload.or_maps_to_er),
        }),
        Method::IsTpsToneMark(payload) => PhonResult::BoolResult(BoolResult {
            value: tps_adjust::is_tps_tone_mark_str(&payload.char),
        }),
        Method::TpsInputAdjust(payload) => {
            let (adjusted, replace_last) =
                tps_adjust::adjust(&payload.incoming, &payload.raw_input);
            let replace_payload = match replace_last {
                Some(s) => OptionalStringResult {
                    output: s,
                    present: true,
                },
                None => OptionalStringResult {
                    output: String::new(),
                    present: false,
                },
            };
            PhonResult::TpsAdjustResult(TpsAdjustResult {
                adjusted,
                replace_last: Some(replace_payload),
            })
        }

        // --- Platform text helpers ---
        Method::IsAttachingPunctuation(payload) => PhonResult::BoolResult(BoolResult {
            value: crate::punctuation::is_attaching_punctuation(&payload.text),
        }),
        Method::ExternalLookupDigitForm(payload) => PhonResult::StringResult(StringResult {
            output: crate::external_lookup::digit_tone_form(&payload.reading),
        }),
    };

    Ok(PhoneticsResponse {
        result: Some(result),
    })
}

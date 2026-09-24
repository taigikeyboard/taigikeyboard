//! Phonetics request dispatcher — routes `PhoneticsRequest.method` variants
//! to the corresponding implementation. All variants live behind a single FFI
//! entry per `docs/engine/rust-core-proto.md` §3 and the architectural
//! convergence doc with khiin-rs / McBopomofo.
//!
//! Helpers are split across submodules:
//! - This file owns the dispatch + result-shape construction.
//! - `derivation` owns CustomDictionaryDerivation port (notone / abbrev).
//! - `normalization` owns the InputNormalizer port (NFD / combining-mark
//!   mechanics).
//! - `tps_adjust` owns the TPSAdjustmentBundle port (collapsed entry).
//! - `tone_variations` owns the GetToneVariations init-pull table builder.
//! - `api`, `syllable`, `tps`, `poj`, `tl`, `tables`, `case_adjust`
//!   provide the foundational helpers reused here.

use crate::api::{
    poj_display_to_tl_display, tl_display_to_poj_display, to_tone_number, PhoneticsError,
};
use crate::custom_search;
use crate::derivation;
use crate::normalization;
use crate::tone_variations;
use crate::tps;
use crate::tps_adjust;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{
    BoolResult, CustomSearchKeysResult, OptionalStringResult, PhoneticsRequest, PhoneticsResponse,
    StringResult, StripToneResult, TpsAdjustResult,
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
        Method::StripTone(payload) => {
            let (bare, tone) = crate::syllable::strip_tone_mark(&payload.input);
            PhonResult::StripToneResult(StripToneResult { bare, tone })
        }
        Method::PojToTl(payload) => PhonResult::StringResult(StringResult {
            output: poj_display_to_tl_display(&payload.input),
        }),
        Method::TlToPoj(payload) => PhonResult::StringResult(StringResult {
            output: tl_display_to_poj_display(&payload.input),
        }),
        Method::NormalizeInput(payload) => PhonResult::StringResult(StringResult {
            output: normalization::normalize_input(&payload.input),
        }),
        Method::GetToneVariations(_) => PhonResult::ToneVariationsResult(tone_variations::build()),
        Method::NfdPreprocessForLookup(payload) => PhonResult::StringResult(StringResult {
            output: normalization::taigi_unicode_base_form(&payload.input),
        }),

        // --- Derivation ---
        Method::DeriveNotone(payload) => PhonResult::StringResult(StringResult {
            output: derivation::derive_notone(&payload.roman),
        }),
        // Legacy `abbrev` column (first letter per syllable) — the index /
        // search-key face is `derive_abbrev` via `DeriveCustomSearchKeys`.
        Method::DeriveAbbrev(payload) => PhonResult::StringResult(StringResult {
            output: derivation::derive_abbrev_first_letter(&payload.roman),
        }),
        Method::DeriveCustomSearchKeys(payload) => {
            PhonResult::CustomSearchKeysResult(CustomSearchKeysResult {
                keys: custom_search::derive_custom_search_keys(&payload.roman)
                    .into_iter()
                    .map(to_proto_key)
                    .collect(),
            })
        }
        Method::DeriveCustomQueryKey(payload) => {
            PhonResult::CustomSearchKeysResult(CustomSearchKeysResult {
                keys: custom_search::derive_custom_query_key(&payload.input, &payload.input_mode)
                    .into_iter()
                    .map(to_proto_key)
                    .collect(),
            })
        }

        // --- TPS ---
        Method::TlNumericToTps(payload) => PhonResult::StringResult(StringResult {
            output: tps_to_tps_numeric(&payload.text, payload.or_maps_to_er),
        }),
        Method::TlDisplayToTps(payload) => PhonResult::StringResult(StringResult {
            output: tps_to_tps_display(&payload.text, payload.or_maps_to_er),
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
    };

    Ok(PhoneticsResponse {
        result: Some(result),
    })
}

// ---- Local helpers ------------------------------------------------------

/// Map a native custom-search key to its proto shape (`&'static str` family /
/// form tags → owned `String`).
fn to_proto_key(k: custom_search::CustomSearchKey) -> protos::engine::CustomSearchKey {
    protos::engine::CustomSearchKey {
        family: k.family.to_string(),
        form: k.form.to_string(),
        key: k.key,
    }
}

/// `Method::TlNumericToTps` — input is numeric tone form (e.g. `"hoo2"`).
/// Mirrors iOS `TLToTPS.convert` / Android `TPSConverter.toTPS`.
fn tps_to_tps_numeric(text: &str, or_maps_to_er: bool) -> String {
    convert_numeric_tl_to_tps(text, or_maps_to_er)
}

/// `Method::TlDisplayToTps` — input is display form with diacritics
/// (e.g. `"hóo"`). Strips diacritics → numeric → `to_zhuyin`. Mirrors iOS
/// `TLToTPS.convertFromDisplay` / Android `TPSConverter.toTPSFromDisplay`.
fn tps_to_tps_display(text: &str, or_maps_to_er: bool) -> String {
    let numeric = to_tone_number(text);
    convert_numeric_tl_to_tps(&numeric, or_maps_to_er)
}

/// Mirrors iOS `TLToTPS.convert` / Android `TPSConverter.toTPS`: split TL
/// on `-`, convert each non-empty syllable, join with a single space (the
/// reference platforms' `joined(separator: " ")` / `joinToString(" ")`).
/// Empty tokens (e.g. from `--` or leading/trailing `-`) are dropped to
/// avoid double-spacing.
fn convert_numeric_tl_to_tps(text: &str, or_maps_to_er: bool) -> String {
    if text.is_empty() {
        return String::new();
    }
    text.split('-')
        .filter(|tok| !tok.is_empty())
        .map(|tok| {
            tps::to_zhuyin(tok, false, or_maps_to_er)
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

//! Phonetics request dispatcher — routes `PhoneticsRequest.intent` (17 ops)
//! to the corresponding implementation. All 17 ops live behind a single FFI
//! entry per `docs/engine/rust-core-proto.md` §3 and the architectural
//! convergence doc with khiin-rs / McBopomofo.
//!
//! Helpers are split across submodules:
//! - This file owns the dispatch + result-shape construction.
//! - `derivation` owns CustomDictionaryDerivation port (notone / abbrev).
//! - `tps_adjust` owns the TPSAdjustmentBundle port (4-fn collapse).
//! - `tone_variations` owns the GetToneVariations init-pull table builder.
//! - Existing modules (`api`, `parser`, `tps`, `poj`, `tl`, `tables`)
//!   provide the foundational helpers reused here.

use crate::api::{
    poj_display_to_tl_display, tl_display_to_poj_display, to_tone_marks, to_tone_number, InputMode,
    PhoneticsError,
};
use crate::derivation;
use crate::tone_variations;
use crate::tps;
use crate::tps_adjust;
use protos::engine::phonetics_request::Intent;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{
    AppConfig, BoolResult, OptionalStringResult, PhoneticsRequest, PhoneticsResponse,
    StringResult, StripToneResult, TpsAdjustResult,
};

/// Dispatch a decoded `PhoneticsRequest` against the per-request `AppConfig`
/// snapshot (live-read settings per `behavioral-invariants.md` §11).
pub fn handle(req: &PhoneticsRequest, config: &AppConfig) -> Result<PhoneticsResponse, PhoneticsError> {
    let Some(intent) = &req.intent else {
        // The most common cause is a Swift/Rust proto schema mismatch
        // (xcframework built before the proto was updated). Run
        // `make build` to regenerate. The `UnsupportedOp` thiserror
        // message also mentions the segmenter case, but that path is
        // unreachable here because dispatch handles every op.
        log::warn!(
            "PhoneticsRequest.intent is None — likely Swift/Rust proto schema mismatch (rebuild xcframework with `make build`)"
        );
        return Err(PhoneticsError::UnsupportedOp);
    };

    let result = match intent {
        // --- Phonetics core ---
        Intent::NormalizeTone(payload) => {
            let mode = parse_input_mode(&config.input_mode);
            let preprocessed = preprocess_for_normalize_tone(&payload.input, mode, config);
            let output = to_tone_marks(&preprocessed, mode);
            PhonResult::StringResult(StringResult { output })
        }
        Intent::StripTone(payload) => {
            let (bare, tone) = crate::parser::strip_tone_mark(&payload.input);
            PhonResult::StripToneResult(StripToneResult { bare, tone })
        }
        Intent::PojToTl(payload) => PhonResult::StringResult(StringResult {
            output: poj_display_to_tl_display(&payload.input),
        }),
        Intent::TlToPoj(payload) => PhonResult::StringResult(StringResult {
            output: tl_display_to_poj_display(&payload.input),
        }),
        Intent::NormalizeToTl(payload) => PhonResult::StringResult(StringResult {
            output: crate::parser::normalize_to_tl(&payload.input),
        }),
        Intent::NormalizeInput(payload) => PhonResult::StringResult(StringResult {
            output: derivation::normalize_input(&payload.input),
        }),
        Intent::RestoreTone(payload) => match derivation::restore_tone(&payload.text) {
            Some(s) => PhonResult::OptionalStringResult(OptionalStringResult {
                output: s,
                present: true,
            }),
            None => PhonResult::OptionalStringResult(OptionalStringResult {
                output: String::new(),
                present: false,
            }),
        },
        Intent::HasToneMarks(payload) => PhonResult::BoolResult(BoolResult {
            value: derivation::has_tone_marks(&payload.text),
        }),
        Intent::GetToneVariations(_) => {
            PhonResult::ToneVariationsResult(tone_variations::build())
        }

        // --- Derivation ---
        Intent::DeriveNotone(payload) => PhonResult::StringResult(StringResult {
            output: derivation::derive_notone(&payload.roman),
        }),
        Intent::DeriveAbbrev(payload) => PhonResult::StringResult(StringResult {
            output: derivation::derive_abbrev(&payload.roman),
        }),

        // --- TPS ---
        Intent::ContainsTps(payload) => PhonResult::BoolResult(BoolResult {
            value: tps::is_zhuyin(&payload.text),
        }),
        Intent::TpsToTl(payload) => PhonResult::StringResult(StringResult {
            output: tps::from_zhuyin(&payload.text),
        }),
        Intent::TlNumericToTps(payload) => PhonResult::StringResult(StringResult {
            output: tps_to_tps_numeric(&payload.text, payload.or_maps_to_er),
        }),
        Intent::TlDisplayToTps(payload) => PhonResult::StringResult(StringResult {
            output: tps_to_tps_display(&payload.text, payload.or_maps_to_er),
        }),
        Intent::IsTpsToneMark(payload) => PhonResult::BoolResult(BoolResult {
            value: tps_adjust::is_tps_tone_mark_str(&payload.char),
        }),
        Intent::TpsInputAdjust(payload) => {
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

fn parse_input_mode(mode: &str) -> InputMode {
    match mode {
        "poj" | "POJ" => InputMode::Poj,
        "english" | "English" | "EN" => InputMode::English,
        _ => InputMode::Tl,
    }
}

fn preprocess_for_normalize_tone(input: &str, mode: InputMode, config: &AppConfig) -> String {
    if !matches!(mode, InputMode::Poj) {
        return input.to_string();
    }
    let mut s = input.to_string();
    if config.oo_doubletap_enabled {
        s = s.replace("oo", "o\u{0358}");
        s = s.replace("Oo", "O\u{0358}");
        s = s.replace("OO", "O\u{0358}");
    }
    if config.nn_doubletap_enabled {
        s = convert_nasal_double_n(&s);
    }
    s
}

/// Mirrors iOS ToneConverter `convertNasalDoubleN`: vowel + "nn" → vowel + "ⁿ".
fn convert_nasal_double_n(input: &str) -> String {
    const NASAL_VOWELS: &str = "aeiouAEIOU";
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    while i < chars.len() {
        let after_vowel = NASAL_VOWELS.contains(chars[i]);
        let next_is_n = chars
            .get(i + 1)
            .map(|c| c.eq_ignore_ascii_case(&'n'))
            .unwrap_or(false);
        let next2_is_n = chars
            .get(i + 2)
            .map(|c| c.eq_ignore_ascii_case(&'n'))
            .unwrap_or(false);
        if after_vowel && next_is_n && next2_is_n && i + 2 < chars.len() {
            result.push(chars[i]);
            result.push('\u{207f}');
            i += 3;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

/// `OP_TL_NUMERIC_TO_TPS` — input is numeric tone form (e.g. `"hoo2"`).
/// Mirrors iOS `TLToTPS.convert` / Android `TPSConverter.toTPS`.
fn tps_to_tps_numeric(text: &str, or_maps_to_er: bool) -> String {
    convert_numeric_tl_to_tps(text, or_maps_to_er)
}

/// `OP_TL_DISPLAY_TO_TPS` — input is display form with diacritics
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
        .map(|tok| tps::to_zhuyin(tok, false, or_maps_to_er).trim_end().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}


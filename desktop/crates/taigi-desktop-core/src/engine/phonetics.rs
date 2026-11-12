//! Phonetics slice of the engine bridge: the small conversions the
//! dictionary pages need (the search keys a custom entry is stored and
//! looked up under are the engine `userdata`'s own, behind the user-data
//! ops). Twin of iOS `RustEngineBridge+Phonetics.swift`.

use protos::engine::{
    phonetics_request, phonetics_response, request, response, ExternalLookupDigitForm,
    IsAttachingPunctuation, IsSpacedPunctuation, PhoneticsRequest, PhoneticsResponse,
    TlDisplayToTps, TlToPoj,
};

use super::bridge::{record_failure, roundtrip};

/// The POJ spelling of a TL reading, for rendering results while the user
/// is typing POJ.
pub fn tl_to_poj(input: &str) -> Option<String> {
    string_result(
        phonetics_request::Method::TlToPoj(TlToPoj {
            input: input.to_owned(),
        }),
        "tlToPoj",
    )
}

/// How TPS spells the vowel `or`: ㄜ, as `er` — the mobile default
/// (`ios/.../SharedSettings.swift` `tpsOrMapsToER`), and no desktop setting
/// changes it. Sent as `AppConfig.tps_or_maps_to_er` and passed to every
/// [`tl_display_to_tps`], so a cell and its commit spell `or` alike.
pub const TPS_OR_MAPS_TO_ER: bool = true;

/// The TPS spelling of a display-form TL reading — what a Hanji-less
/// candidate shows under TPS. `or_maps_to_er` is `AppConfig.tps_or_maps_to_er`,
/// so a cell and its commit spell `or` alike.
pub fn tl_display_to_tps(text: &str, or_maps_to_er: bool) -> Option<String> {
    string_result(
        phonetics_request::Method::TlDisplayToTps(TlDisplayToTps {
            text: text.to_owned(),
            or_maps_to_er,
        }),
        "tlDisplayToTps",
    )
}

/// The digit-tone spelling of a TL reading that the web dictionaries search
/// by (`tāi-tsì` → `tai7-tsi3`) — the engine's fold, the one every platform
/// asks.
pub fn external_lookup_digit_form(reading: &str) -> Option<String> {
    string_result(
        phonetics_request::Method::ExternalLookupDigitForm(ExternalLookupDigitForm {
            reading: reading.to_owned(),
        }),
        "externalLookupDigitForm",
    )
}

/// Whether `text` is a single punctuation character that attaches to the
/// preceding word under auto-space (`guá ` + `?` → `guá? `) — the engine's
/// set, the one every platform asks. False when the engine fails: no swap,
/// the space stays where it is.
pub fn is_attaching_punctuation(text: &str) -> bool {
    bool_result(
        phonetics_request::Method::IsAttachingPunctuation(IsAttachingPunctuation {
            text: text.to_owned(),
        }),
        "isAttachingPunctuation",
    )
}

/// Whether attaching `text` keeps the auto space after itself — a half-width
/// mark (`guá? `); a full-width one takes the space's place (`taigi，`).
/// False when the engine fails: the mark is written with no space after it.
pub fn is_spaced_punctuation(text: &str) -> bool {
    bool_result(
        phonetics_request::Method::IsSpacedPunctuation(IsSpacedPunctuation {
            text: text.to_owned(),
        }),
        "isSpacedPunctuation",
    )
}

fn bool_result(method: phonetics_request::Method, op: &str) -> bool {
    let Some(response) = phonetics_response(method, op) else {
        return false;
    };
    match response.result {
        Some(phonetics_response::Result::BoolResult(result)) => result.value,
        _ => {
            record_failure(op, "response carried no bool result");
            false
        }
    }
}

fn string_result(method: phonetics_request::Method, op: &str) -> Option<String> {
    let response = phonetics_response(method, op)?;
    match response.result {
        Some(phonetics_response::Result::StringResult(result)) => Some(result.output),
        _ => {
            record_failure(op, "response carried no string result");
            None
        }
    }
}

fn phonetics_response(method: phonetics_request::Method, op: &str) -> Option<PhoneticsResponse> {
    let payload = request::Payload::Phonetics(PhoneticsRequest {
        method: Some(method),
    });
    match roundtrip(payload, op, 0, None)? {
        response::Payload::Phonetics(response) => Some(response),
        other => {
            record_failure(op, &format!("expected a phonetics payload, got {other:?}"));
            None
        }
    }
}

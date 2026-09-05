//! Phonetics slice of the engine bridge: the search keys a custom-dictionary
//! entry is stored under, the one a query is looked up by, and the small
//! conversions the dictionary pages need. Port of
//! `RustEngineBridge+Phonetics.swift`.

// 音韻切片 — 自訂詞的儲存鍵/查詢鍵推導,以及 POJ↔TL 等小轉換。

use protos::engine::{
    phonetics_request, phonetics_response, request, response, DeriveCustomQueryKey,
    DeriveCustomSearchKeys, NfdPreprocessForLookup, PhoneticsRequest, PhoneticsResponse, PojToTl,
    StripTone, TlToPoj,
};

use super::bridge::{record_failure, roundtrip};
use crate::settings::InputMode;

/// One way a custom-dictionary entry can be found. `family` is the
/// romanization system (`tl` / `poj` / `tps`), `form` how much of the reading
/// it carries (`num` / `notone` / `abbrev`), `key` the fused string both
/// sides match on. The engine owns all three vocabularies — the platform
/// stores what it is given and asks for the query key through the same op.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomSearchKey {
    pub family: String,
    pub form: String,
    pub key: String,
}

/// Every key a stored entry should be findable under, so a word added while
/// typing TL is still found by someone typing POJ. `None` = round-trip
/// failed; `Some(empty)` = the roman produced no searchable key, which the
/// store treats as a refusal rather than writing an unreachable row.
pub fn derive_custom_search_keys(roman: &str) -> Option<Vec<CustomSearchKey>> {
    custom_search_keys(
        phonetics_request::Method::DeriveCustomSearchKeys(DeriveCustomSearchKeys {
            roman: roman.to_owned(),
        }),
        "deriveCustomSearchKeys",
    )
}

/// The single key the user's current input should be looked up by. The
/// family is decided by the engine, not by the mode alone. `None` = nothing
/// to look up (empty input, residue that forms no key, or a failed trip).
pub fn derive_custom_query_key(input: &str, mode: InputMode) -> Option<CustomSearchKey> {
    custom_search_keys(
        phonetics_request::Method::DeriveCustomQueryKey(DeriveCustomQueryKey {
            input: input.to_owned(),
            input_mode: mode.wire().to_owned(),
        }),
        "deriveCustomQueryKey",
    )?
    .into_iter()
    .next()
}

/// The TL spelling of a POJ reading, for the identity keys a restore writes.
pub fn poj_to_tl(input: &str) -> Option<String> {
    string_result(
        phonetics_request::Method::PojToTl(PojToTl {
            input: input.to_owned(),
        }),
        "pojToTl",
    )
}

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

/// The syllable without its tone, and the tone digit that was on it.
pub fn strip_tone(input: &str) -> Option<(String, String)> {
    let op = "stripTone";
    let response = phonetics_response(
        phonetics_request::Method::StripTone(StripTone {
            input: input.to_owned(),
        }),
        op,
    )?;
    match response.result {
        Some(phonetics_response::Result::StripToneResult(result)) => {
            Some((result.bare, result.tone))
        }
        _ => {
            record_failure(op, "response carried no strip-tone result");
            None
        }
    }
}

/// Taigi-specific Unicode preprocessing before a lookup: the nasal marker
/// and `o͘` folded to the ASCII spellings the external dictionaries index by.
pub fn nfd_preprocess_for_lookup(input: &str) -> Option<String> {
    string_result(
        phonetics_request::Method::NfdPreprocessForLookup(NfdPreprocessForLookup {
            input: input.to_owned(),
        }),
        "nfdPreprocessForLookup",
    )
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

fn custom_search_keys(method: phonetics_request::Method, op: &str) -> Option<Vec<CustomSearchKey>> {
    let response = phonetics_response(method, op)?;
    match response.result {
        Some(phonetics_response::Result::CustomSearchKeysResult(result)) => Some(
            result
                .keys
                .into_iter()
                .map(|key| CustomSearchKey {
                    family: key.family,
                    form: key.form,
                    key: key.key,
                })
                .collect(),
        ),
        _ => {
            record_failure(op, "response carried no custom-search-keys result");
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

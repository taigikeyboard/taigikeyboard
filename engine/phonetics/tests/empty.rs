//! Empty-input contract: every Phonetics op handles an empty input string
//! by returning a normal `Ok(PhoneticsResponse)` with the appropriate
//! empty / no-op output for that op's result variant. Pins the library's
//! "no special-case for empty input" guarantee — callers never need to
//! pre-filter.

use phonetics::requests::handle;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{ExternalLookupDigitForm, PhoneticsRequest, PhoneticsResponse};

fn run(method: Method) -> PhoneticsResponse {
    let req = PhoneticsRequest {
        method: Some(method),
    };
    handle(&req).expect("dispatch handle should succeed")
}

fn expect_string_output(resp: &PhoneticsResponse) -> String {
    let Some(PhonResult::StringResult(s)) = &resp.result else {
        panic!("expected StringResult, got {:?}", resp.result);
    };
    s.output.clone()
}

#[test]
fn external_lookup_digit_form_empty_input() {
    let resp = run(Method::ExternalLookupDigitForm(ExternalLookupDigitForm {
        reading: String::new(),
    }));
    assert_eq!(expect_string_output(&resp), "");
}

#[test]
fn normalize_tone_empty_input() {
    let out = phonetics::api::normalize_tone("", &protos::engine::AppConfig::default());
    assert_eq!(out, "");
}

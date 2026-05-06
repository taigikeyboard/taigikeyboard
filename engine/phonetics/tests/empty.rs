//! Empty-input contract: every Phonetics op handles an empty input string
//! by returning a normal `Ok(PhoneticsResponse)` with the appropriate
//! empty / no-op output for that op's result variant. Pins the library's
//! "no special-case for empty input" guarantee — callers never need to
//! pre-filter.

// 中文: 空字串輸入合約測試;每個 op 都應該回正常 Ok 而非錯誤,呼叫端不需要先過濾。

use phonetics::dispatch::handle;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{
    AppConfig, NormalizeTone, PhoneticsRequest, PhoneticsResponse, PojToTl, StripTone, TlToPoj,
};

fn run(method: Method) -> PhoneticsResponse {
    let req = PhoneticsRequest {
        method: Some(method),
    };
    handle(&req, &AppConfig::default()).expect("dispatch handle should succeed")
}

fn expect_string_output(resp: &PhoneticsResponse) -> String {
    let Some(PhonResult::StringResult(s)) = &resp.result else {
        panic!("expected StringResult, got {:?}", resp.result);
    };
    s.output.clone()
}

fn expect_strip_tone_output(resp: &PhoneticsResponse) -> (String, String) {
    let Some(PhonResult::StripToneResult(s)) = &resp.result else {
        panic!("expected StripToneResult, got {:?}", resp.result);
    };
    (s.bare.clone(), s.tone.clone())
}

#[test]
fn tl_to_poj_empty_input() {
    let resp = run(Method::TlToPoj(TlToPoj {
        input: String::new(),
    }));
    assert_eq!(expect_string_output(&resp), "");
}

#[test]
fn poj_to_tl_empty_input() {
    let resp = run(Method::PojToTl(PojToTl {
        input: String::new(),
    }));
    assert_eq!(expect_string_output(&resp), "");
}

#[test]
fn normalize_tone_empty_input() {
    let resp = run(Method::NormalizeTone(NormalizeTone {
        input: String::new(),
    }));
    assert_eq!(expect_string_output(&resp), "");
}

#[test]
fn strip_tone_empty_input() {
    let resp = run(Method::StripTone(StripTone {
        input: String::new(),
    }));
    let (bare, tone) = expect_strip_tone_output(&resp);
    assert_eq!(bare, "");
    assert_eq!(tone, "");
}

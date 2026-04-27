//! T7' — empty input is library-equivalent of T7 (null FFI handle in D9.2).
//! All ops on an empty string return `OK` with empty / appropriate output.

use phonetics::api::process_request;
use prost::Message;
use protos::engine::phonetics_request::Method;
use protos::engine::phonetics_response::Result as PhonResult;
use protos::engine::{
    request, response, ErrorCode, NormalizeTone, PhoneticsRequest, PojToTl, Request, Response,
    StripTone, TlToPoj,
};

fn run(method: Method) -> Response {
    let req = Request {
        id: 7,
        r#type: 0,
        config_snapshot: None,
        generation: 0,
        payload: Some(request::Payload::Phonetics(PhoneticsRequest {
            method: Some(method),
        })),
    };
    let mut buf = Vec::with_capacity(req.encoded_len());
    req.encode(&mut buf).unwrap();
    let resp_bytes = process_request(&buf);
    Response::decode(resp_bytes.as_slice()).expect("response decodes")
}

fn expect_string_output(resp: &Response) -> String {
    let Some(response::Payload::Phonetics(p)) = &resp.payload else {
        panic!("expected phonetics payload");
    };
    let Some(PhonResult::StringResult(s)) = &p.result else {
        panic!("expected StringResult, got {:?}", p.result);
    };
    s.output.clone()
}

fn expect_strip_tone_output(resp: &Response) -> (String, String) {
    let Some(response::Payload::Phonetics(p)) = &resp.payload else {
        panic!("expected phonetics payload");
    };
    let Some(PhonResult::StripToneResult(s)) = &p.result else {
        panic!("expected StripToneResult, got {:?}", p.result);
    };
    (s.bare.clone(), s.tone.clone())
}

#[test]
fn tl_to_poj_empty_input() {
    let resp = run(Method::TlToPoj(TlToPoj {
        input: String::new(),
    }));
    assert_eq!(resp.error, ErrorCode::Ok as i32);
    assert_eq!(resp.id, 7);
    assert_eq!(expect_string_output(&resp), "");
}

#[test]
fn poj_to_tl_empty_input() {
    let resp = run(Method::PojToTl(PojToTl {
        input: String::new(),
    }));
    assert_eq!(resp.error, ErrorCode::Ok as i32);
    assert_eq!(expect_string_output(&resp), "");
}

#[test]
fn normalize_tone_empty_input() {
    let resp = run(Method::NormalizeTone(NormalizeTone {
        input: String::new(),
    }));
    assert_eq!(resp.error, ErrorCode::Ok as i32);
    assert_eq!(expect_string_output(&resp), "");
}

#[test]
fn strip_tone_empty_input() {
    let resp = run(Method::StripTone(StripTone {
        input: String::new(),
    }));
    assert_eq!(resp.error, ErrorCode::Ok as i32);
    let (bare, tone) = expect_strip_tone_output(&resp);
    assert_eq!(bare, "");
    assert_eq!(tone, "");
}

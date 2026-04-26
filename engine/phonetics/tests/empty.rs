//! T7' — empty input is library-equivalent of T7 (null FFI handle in D9.2).
//! All four ops on an empty string return `OK` with empty output.

use phonetics::api::process_request;
use prost::Message;
use protos::engine::phonetics_request::Op;
use protos::engine::{request, response, ErrorCode, PhoneticsRequest, Request, Response};

fn run(op: Op) -> Response {
    let req = Request {
        id: 7,
        r#type: 0,
        config_snapshot: None,
        generation: 0,
        payload: Some(request::Payload::Phonetics(PhoneticsRequest {
            op: op as i32,
            input: String::new(),
        })),
    };
    let mut buf = Vec::with_capacity(req.encoded_len());
    req.encode(&mut buf).unwrap();
    let resp_bytes = process_request(&buf);
    Response::decode(resp_bytes.as_slice()).expect("response decodes")
}

#[test]
fn tl_to_poj_empty_input() {
    let resp = run(Op::TlToPoj);
    assert_eq!(resp.error, ErrorCode::Ok as i32);
    assert_eq!(resp.id, 7);
    if let Some(response::Payload::Phonetics(p)) = resp.payload {
        assert_eq!(p.output, "");
    } else {
        panic!("expected phonetics payload");
    }
}

#[test]
fn poj_to_tl_empty_input() {
    let resp = run(Op::PojToTl);
    assert_eq!(resp.error, ErrorCode::Ok as i32);
    if let Some(response::Payload::Phonetics(p)) = resp.payload {
        assert_eq!(p.output, "");
    }
}

#[test]
fn normalize_tone_empty_input() {
    let resp = run(Op::NormalizeTone);
    assert_eq!(resp.error, ErrorCode::Ok as i32);
}

#[test]
fn strip_tone_empty_input() {
    let resp = run(Op::StripTone);
    assert_eq!(resp.error, ErrorCode::Ok as i32);
}

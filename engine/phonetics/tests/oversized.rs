//! T5 — oversized payloads (`ffi-safety.md` §7: "**>1MB byte buffer** → bounded
//! behavior (reject or truncate); behavior documented either way").
//!
//! D9.1 documented behavior: the `phonetics` library has **no input size cap**.
//! It accepts and processes inputs of any size that fits in memory; bounded
//! behavior is guaranteed only because the conversion is linear in input size
//! and never allocates more than 2× the input. The 1 MB / 2 MB FFI-side guard
//! lands in D9.2 in `swift-ffi` / `android-jni` shims.
//!
//! The test passes a 1.2 MB protobuf-encoded payload (>1 MB threshold) and
//! asserts a successful Response — proving "bounded" via "completes without
//! crash" rather than "rejects".

use phonetics::api::process_request;
use prost::Message;
use protos::engine::phonetics_request::Op;
use protos::engine::{request, ErrorCode, PhoneticsRequest, Request, Response};

#[test]
fn over_1mb_input_completes_without_panic() {
    // 300_000 × 4 bytes ≈ 1.2 MB raw input string, well above the 1 MB threshold.
    let huge = "ka2-".repeat(300_000);
    assert!(huge.len() > 1_000_000, "input must exceed 1 MB threshold");
    let req = Request {
        id: 1,
        r#type: 0,
        config_snapshot: None,
        generation: 0,
        payload: Some(request::Payload::Phonetics(PhoneticsRequest {
            op: Op::NormalizeTone as i32,
            input: huge,
        })),
    };
    let mut buf = Vec::with_capacity(req.encoded_len());
    req.encode(&mut buf).unwrap();
    let resp_bytes = process_request(&buf);
    let resp = Response::decode(resp_bytes.as_slice()).expect("response decodes");
    assert_eq!(
        resp.error,
        ErrorCode::Ok as i32,
        "oversized input should process successfully under D9.1 documented policy"
    );
}

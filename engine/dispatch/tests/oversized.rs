//! `engine/dispatch::process_request` — oversized payloads
//! (`docs/engine/ffi-safety.md` §7 T5: ">1 MB byte buffer → bounded
//! behavior, documented either way").
//!
//! Library-level documented behavior: the dispatcher itself has **no
//! input size cap**. It accepts any input that fits in memory; bounded
//! behavior is guaranteed because every op is linear in input size and
//! never allocates more than 2× the input. The 2 MB FFI-side guard
//! lives in `swift-ffi` / `android-jni` (see `MAX_REQUEST_BYTES`).
//!
//! Pass a 1.2 MB protobuf payload (>1 MB threshold) and assert the
//! response is `Ok` — proving "bounded" via "completes without crash"
//! rather than "rejects".

// 驗證 dispatcher 對超大輸入採「完成不崩潰」的有界行為,2 MB 上限由 swift-ffi / android-jni 端把關。

use prost::Message;
use protos::engine::phonetics_request::Method;
use protos::engine::{request, ErrorCode, NormalizeTone, PhoneticsRequest, Request, Response};

#[test]
fn over_1mb_input_completes_without_panic() {
    let huge = "ka2-".repeat(300_000); // ~1.2 MB raw input
    assert!(huge.len() > 1_000_000, "input must exceed 1 MB threshold");
    let req = Request {
        id: 1,
        r#type: 0,
        config_snapshot: None,
        generation: 0,
        payload: Some(request::Payload::Phonetics(PhoneticsRequest {
            method: Some(Method::NormalizeTone(NormalizeTone { input: huge })),
        })),
    };
    let mut buf = Vec::with_capacity(req.encoded_len());
    req.encode(&mut buf).unwrap();
    let resp_bytes = dispatch::process_request(&buf);
    let resp = Response::decode(resp_bytes.as_slice()).expect("response decodes");
    assert_eq!(
        resp.error,
        ErrorCode::Ok as i32,
        "oversized input should process successfully under documented policy"
    );
}

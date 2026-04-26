//! T4 — malformed protobuf bytes must surface as `FailParse`.

use phonetics::api::process_request;
use prost::Message;
use protos::engine::{ErrorCode, Response};

#[test]
fn random_bytes_return_fail_parse() {
    // Bytes that look like a valid varint header but trail off — prost should
    // reject these without crashing.
    let cases: &[&[u8]] = &[
        &[0x08, 0x96, 0x01, 0xff], // partial varint
        &[0x12, 0xff, 0xff, 0xff], // length-delimited with bad length
        &[0xff, 0xff, 0xff, 0xff], // garbage
        b"not a real protobuf message",
    ];
    for bytes in cases {
        let out = process_request(bytes);
        let resp = Response::decode(out.as_slice()).expect("response always decodes");
        assert_eq!(
            resp.error,
            ErrorCode::FailParse as i32,
            "expected FailParse for {bytes:?}, got error={}",
            resp.error
        );
    }
}

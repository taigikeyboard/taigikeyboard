//! T1' — library-side equivalent of `docs/engine/ffi-safety.md` §7 T1.
//!
//! Three sub-tests:
//! 1. A handler that panics inside the catch-unwind boundary surfaces as
//!    `ErrorCode::FailInternal`. This is the actual panic-isolation guarantee.
//! 2. Malformed protobuf bytes do not panic; they return `FailParse`.
//! 3. Empty bytes decode to a default `Request` with no payload — they return
//!    `FailInvariant` (semantic error), not a panic.
//!
//! Real T1 (FFI-side panic across swift-bridge / jni) lands in D9.2.

use phonetics::api::{process_request, process_request_with};
use prost::Message;
use protos::engine::{ErrorCode, Response};

#[test]
fn forced_panic_is_caught_and_returns_fail_internal() {
    // Inject a dispatcher that panics. The same `catch_unwind` boundary used by
    // production must convert the panic into an encoded Response.
    let resp_bytes = process_request_with(&[1, 2, 3], |_| panic!("intentional T1' test panic"));
    let resp = Response::decode(resp_bytes.as_slice()).expect("response decodes");
    assert_eq!(
        resp.error,
        ErrorCode::FailInternal as i32,
        "panic in dispatcher must surface as FailInternal, got error={}",
        resp.error
    );
}

#[test]
fn malformed_request_does_not_panic() {
    let bytes = [0xff_u8, 0x01, 0x02, 0x03, 0x04];
    let resp_bytes = process_request(&bytes);
    let resp = Response::decode(resp_bytes.as_slice()).expect("response decodes");
    assert_eq!(resp.error, ErrorCode::FailParse as i32);
}

#[test]
fn empty_request_does_not_panic() {
    let resp_bytes = process_request(&[]);
    let resp = Response::decode(resp_bytes.as_slice()).expect("response decodes");
    // Empty bytes decode to default Request → no payload → FailInvariant.
    assert_eq!(resp.error, ErrorCode::FailInvariant as i32);
}

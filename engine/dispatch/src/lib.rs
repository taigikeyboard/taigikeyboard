//! Top-level FFI dispatch — single bytes-in / bytes-out entry point.
//!
//! Decodes a `taigi.engine.Request`, routes by its `payload` oneof variant
//! to the matching module crate (`phonetics` for phonetics ops, `ranking`
//! for lexicon ops), re-encodes the response, and returns the byte buffer.
//!
//! Wrapped in `catch_unwind` per `docs/engine/ffi-safety.md` §2 so panics
//! anywhere in the decode → dispatch → encode pipeline surface as a
//! `Response` with `ErrorCode::FailInternal` rather than aborting the host
//! process. The FFI crates (`swift-ffi`, `android-jni`) wrap this same
//! function in another `catch_unwind` for defense in depth.
//!
//! Empty / missing payload is reported as `FailInvariant` so the platform
//! side can distinguish "you forgot to fill `payload`" from "we crashed".

use std::panic::{catch_unwind, AssertUnwindSafe};

use prost::Message;
use protos::engine::{request, response, ErrorCode, Request, Response};

/// Decode `bytes` as a `Request`, dispatch by payload variant, encode the
/// resulting `Response`. Always returns a valid encoded `Response` —
/// never panics across the seam.
#[must_use]
pub fn process_request(bytes: &[u8]) -> Vec<u8> {
    let result = catch_unwind(AssertUnwindSafe(|| encode(&run(bytes))));
    result.unwrap_or_else(|_| {
        log::error!("engine dispatch panicked");
        encode(&error_response(0, ErrorCode::FailInternal, 0))
    })
}

fn run(bytes: &[u8]) -> Response {
    let request = match Request::decode(bytes) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("engine request decode failed: {e}");
            return error_response(0, ErrorCode::FailParse, 0);
        }
    };
    let id = request.id;
    let generation = request.generation;
    let config = request.config_snapshot.clone().unwrap_or_default();

    let Some(payload) = request.payload else {
        log::warn!("engine request missing payload (id={id})");
        return error_response(id, ErrorCode::FailInvariant, generation);
    };

    match payload {
        request::Payload::Phonetics(phon_req) => {
            match phonetics::dispatch::handle(&phon_req, &config) {
                Ok(phon_resp) => Response {
                    id,
                    error: ErrorCode::Ok as i32,
                    generation,
                    payload: Some(response::Payload::Phonetics(phon_resp)),
                },
                Err(err) => {
                    log::warn!("phonetics dispatch failed (id={id}): {err}");
                    error_response(id, phonetics_error_code(&err), generation)
                }
            }
        }
        request::Payload::Lexicon(lex_req) => {
            let Some(method) = lex_req.method else {
                log::warn!("lexicon request missing method (id={id})");
                return error_response(id, ErrorCode::FailInvariant, generation);
            };
            match method {
                protos::engine::lexicon_request::Method::ProcessCandidates(req) => {
                    let resp = ranking::process_candidates(req);
                    Response {
                        id,
                        error: ErrorCode::Ok as i32,
                        generation,
                        payload: Some(response::Payload::Lexicon(
                            protos::engine::LexiconResponse {
                                result: Some(
                                    protos::engine::lexicon_response::Result::ProcessCandidatesResult(
                                        resp,
                                    ),
                                ),
                            },
                        )),
                    }
                }
            }
        }
    }
}

fn phonetics_error_code(err: &phonetics::PhoneticsError) -> ErrorCode {
    match err {
        phonetics::PhoneticsError::InvalidProto(_) => ErrorCode::FailParse,
        phonetics::PhoneticsError::UnknownSystem(_)
        | phonetics::PhoneticsError::UnsupportedOp => ErrorCode::FailInvariant,
        phonetics::PhoneticsError::InternalPanic => ErrorCode::FailInternal,
    }
}

fn error_response(id: u32, code: ErrorCode, generation: u64) -> Response {
    Response {
        id,
        error: code as i32,
        generation,
        payload: None,
    }
}

fn encode(response: &Response) -> Vec<u8> {
    let mut buf = Vec::with_capacity(response.encoded_len());
    // JUSTIFICATION: encode into Vec<u8> never fails — prost::EncodeError
    // fires only when the target buffer is too small; Vec grows.
    response
        .encode(&mut buf)
        .expect("prost encode into Vec<u8> never fails");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::{
        AppConfig, CommandType, FrequencyEntry, LexiconRequest, ProcessCandidatesRequest,
        TaigiWord,
    };

    fn lexicon_request(req: ProcessCandidatesRequest) -> Request {
        Request {
            id: 42,
            r#type: CommandType::CmdLexicon as i32,
            config_snapshot: Some(AppConfig::default()),
            generation: 7,
            payload: Some(request::Payload::Lexicon(LexiconRequest {
                method: Some(
                    protos::engine::lexicon_request::Method::ProcessCandidates(req),
                ),
            })),
        }
    }

    #[test]
    fn dispatch_routes_lexicon_request_to_ranking() {
        let candidates = ProcessCandidatesRequest {
            raw: vec![
                TaigiWord {
                    id: 1,
                    roman: "gua".to_owned(),
                    hanji: Some("我".to_owned()),
                    length_score: Some(50),
                    source_bitmask: None,
                },
                TaigiWord {
                    id: 2,
                    roman: "gua".to_owned(),
                    hanji: Some("我".to_owned()),
                    length_score: Some(50),
                    source_bitmask: None,
                },
            ],
            normalized_input: "gua".to_owned(),
            tps_dedup_enabled: false,
            freq: vec![FrequencyEntry {
                display_text_key: "我".to_owned(),
                count: 5,
                last_used_ms: 0,
            }],
            now_ms: 1_000_000_000,
            include_breakdown: true,
        };
        let req = lexicon_request(candidates);
        let mut buf = Vec::with_capacity(req.encoded_len());
        req.encode(&mut buf).unwrap();

        let resp_bytes = process_request(&buf);
        let resp = Response::decode(resp_bytes.as_slice()).unwrap();

        assert_eq!(resp.id, 42);
        assert_eq!(resp.generation, 7);
        assert_eq!(resp.error, ErrorCode::Ok as i32);
        let payload = resp.payload.expect("payload present");
        let response::Payload::Lexicon(lex_resp) = payload else {
            panic!("expected Lexicon payload, got {payload:?}");
        };
        let result = lex_resp.result.expect("result present");
        let protos::engine::lexicon_response::Result::ProcessCandidatesResult(pc) = result;
        assert_eq!(pc.ranked.len(), 1, "duplicate dropped by engine dedup");
        assert_eq!(pc.breakdown.len(), 1, "breakdown requested");
    }

    #[test]
    fn dispatch_returns_fail_parse_for_garbage_bytes() {
        let resp_bytes = process_request(&[0xff, 0xff, 0xff]);
        let resp = Response::decode(resp_bytes.as_slice()).unwrap();
        assert_eq!(resp.error, ErrorCode::FailParse as i32);
    }

    #[test]
    fn dispatch_returns_fail_invariant_for_missing_payload() {
        let req = Request {
            id: 1,
            r#type: CommandType::CmdUnspecified as i32,
            config_snapshot: None,
            generation: 0,
            payload: None,
        };
        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();
        let resp_bytes = process_request(&buf);
        let resp = Response::decode(resp_bytes.as_slice()).unwrap();
        assert_eq!(resp.error, ErrorCode::FailInvariant as i32);
        assert_eq!(resp.id, 1);
    }

    #[test]
    fn dispatch_returns_fail_invariant_for_lexicon_missing_method() {
        let req = Request {
            id: 9,
            r#type: CommandType::CmdLexicon as i32,
            config_snapshot: None,
            generation: 0,
            payload: Some(request::Payload::Lexicon(LexiconRequest { method: None })),
        };
        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();
        let resp_bytes = process_request(&buf);
        let resp = Response::decode(resp_bytes.as_slice()).unwrap();
        assert_eq!(resp.error, ErrorCode::FailInvariant as i32);
        assert_eq!(resp.id, 9);
    }
}

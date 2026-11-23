//! Top-level FFI dispatch — single bytes-in / bytes-out entry point.
//!
//! Decodes a `taigi.engine.Request`, routes by its `payload` oneof variant
//! to the matching module crate (`phonetics`, `composing`, `lexicon`,
//! `nextword`), re-encodes the response, and returns the byte buffer.
//!
//! Wrapped in `catch_unwind` per `docs/contributing/rust-ffi-safety.md` §1.2 so panics
//! anywhere in the decode → dispatch → encode pipeline surface as a
//! `Response` with `ErrorCode::FailInternal` rather than aborting the host
//! process. The FFI crates (`swift-ffi`, `android-jni`) wrap this same
//! function in another `catch_unwind` for defense in depth.
//!
//! Empty / missing payload is reported as `FailInvariant` so the platform
//! side can distinguish "you forgot to fill `payload`" from "we crashed".

use std::panic::{catch_unwind, AssertUnwindSafe};

use prost::Message;
use protos::engine::{request, response, AppConfig, ErrorCode, Request, Response};

mod context;
mod fetch;
mod next_word_handshake;
mod predict;
#[cfg(feature = "e2e-trace")]
pub mod trace;
mod user_data;

/// The biggest `roman,hanji` CSV file a custom-dictionary import accepts
/// (`ImportCustomCsv`). Exported so a platform can refuse the file before
/// reading it, as the engine refuses the bytes; `user_data/with_stores.rs`
/// asserts it equals the `userdata` crate's own limit.
pub const CUSTOM_CSV_MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;

/// The letters the desktop Telex scheme types tones with — for a shell that
/// classifies a key as Telex before the engine resolves it (desktop-core).
pub use composing::telex::TELEX_KEYS;

/// The files the user's stores live in, by name — for a platform that moves
/// them itself (the web).
#[cfg(feature = "user-data")]
pub use userdata::USER_DATA_FILES;

/// Hands in the bytes of the dictionary file at `path`, for a target with no
/// file system to map (the browser); the lexicon install reads them there.
#[cfg(target_family = "wasm")]
pub fn register_dictionary_bytes(path: &std::path::Path, bytes: Vec<u8>) {
    mmap_host::register(path, bytes);
}

/// Maximum accepted size of an FFI request byte buffer. The largest request
/// is a `.taigi` restore (`ImportBackup`): every store at its cap, at
/// four-syllable rows, exports to about 20 MB — under half the cap
/// (`tests/backup_request_size.rs`); key path requests are kilobytes. Single
/// source of truth — `swift-ffi`, `android-jni` and the macOS FFI check it
/// before copying or decoding, so the cap stays in lock-step across the seams.
pub const MAX_REQUEST_BYTES: usize = 64 * 1024 * 1024;

/// Install a twelve-field comparison order for this thread. `fields` is a
/// permutation of the built-in stage names (`ranking/src/sort_key.rs`
/// `SORT_FIELDS`). `false` leaves the current order
/// in place. The web developer page is the caller; an official input method
/// never sets one, and another thread keeps the field order of
/// [`ranking::CandidateSortKey`].
pub fn set_dev_sort_order(fields: &[&str]) -> bool {
    ranking::set_dev_sort_order(fields)
}

/// Restore this thread's comparison to the built-in order
/// (`ranking/src/sort_key.rs` `SORT_FIELDS`).
pub fn reset_dev_sort_order() {
    ranking::reset_dev_sort_order();
}

/// The open composition's segmentation and candidate order, as JSON, or
/// `None` when nothing is being composed. The same walk and the same sort
/// the candidate list uses (`composing::explain::mechanism`). A shell that
/// draws the mechanism calls this beside the key path; the key path itself
/// is unchanged.
pub fn explain_composition(
    config: &AppConfig,
    generation: u64,
    now_ms: i64,
    literal_roman_candidate_disabled: bool,
) -> Option<String> {
    let view = user_data::explain_composition(
        config,
        generation,
        now_ms,
        literal_roman_candidate_disabled,
    )?;
    Some(explain_json(&view))
}

fn explain_json(view: &composing::explain::Mechanism) -> String {
    let piece = |text: &str, gap: bool| serde_json::json!({ "text": text, "gap": gap });
    serde_json::json!({
        "raw": view.raw,
        "cutBy": view.cut_by,
        "syllables": view.syllables.iter().map(|cut| piece(&cut.text, cut.gap)).collect::<Vec<_>>(),
        "words": view.words.iter().map(|word| serde_json::json!({
            "text": word.text,
            "hanji": word.hanji,
            "roman": word.roman,
            "kind": word.kind,
            "userRow": word.user_row,
            "picks": word.picks,
            "weight": word.weight,
        })).collect::<Vec<_>>(),
        "candidates": view.candidates.iter().map(|row| serde_json::json!({
            "hanji": row.hanji,
            "roman": row.roman,
            "before": row.before,
            "head": row.head,
            "rest": row.rest,
            "role": row.role,
            "placedBy": row.placed_by,
            "preference": row.preference,
            "coverage": row.coverage,
            "context": row.context,
            "picks": row.picks,
            "syllables": row.syllables,
            "weight": row.weight,
            "lastUsedMs": row.last_used_ms,
            "tier": row.tier,
            "score": row.score,
            "frequency": row.frequency,
            "walkerCost": row.walker_cost,
            "coverageBytes": row.coverage_bytes,
            "sourceRank": row.source_rank,
            "sources": row.sources,
            "rankingSource": row.ranking_source,
        })).collect::<Vec<_>>(),
    })
    .to_string()
}

/// The FFI adapters' pre-dispatch size gate: true when a request of `len`
/// bytes must be refused with `FAIL_INVARIANT`. Owning the check here keeps
/// the refusal traced (`adapter_reject`) for every adapter in test builds.
#[must_use]
pub fn is_request_too_large(len: usize) -> bool {
    let too_large = len > MAX_REQUEST_BYTES;
    #[cfg(feature = "e2e-trace")]
    if too_large {
        trace::adapter_reject("oversize", len);
    }
    too_large
}

/// Decode `bytes` as a `Request`, dispatch by payload variant, encode the
/// resulting `Response`. Always returns a valid encoded `Response` —
/// never panics across the seam.
#[must_use]
pub fn process_request(bytes: &[u8]) -> Vec<u8> {
    #[cfg(feature = "e2e-trace")]
    let started = std::time::Instant::now();
    let result = catch_unwind(AssertUnwindSafe(|| {
        let response = run(bytes);
        let encoded = encode(&response);
        #[cfg(feature = "e2e-trace")]
        trace::request(bytes, &response, encoded.len(), started);
        encoded
    }));
    result.unwrap_or_else(|_| {
        log::error!("engine dispatch panicked");
        #[cfg(feature = "e2e-trace")]
        trace::panic(bytes, started);
        encode(&error_response(0, ErrorCode::FailInternal))
    })
}

/// `#[cfg(test)]`-only panic-injection seam. The inline panic test passes
/// a closure that always panics so the catch-unwind boundary is
/// exercised against a known panic. Production code never references
/// this; it is compiled out of release builds.
#[cfg(test)]
fn process_request_with<F>(bytes: &[u8], dispatcher: F) -> Vec<u8>
where
    F: FnOnce(&[u8]) -> Response + std::panic::UnwindSafe,
{
    let result = catch_unwind(AssertUnwindSafe(|| encode(&dispatcher(bytes))));
    result.unwrap_or_else(|_| {
        log::error!("engine dispatch panicked");
        encode(&error_response(0, ErrorCode::FailInternal))
    })
}

fn run(bytes: &[u8]) -> Response {
    let request = match Request::decode(bytes) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("engine request decode failed: {e}");
            return error_response(0, ErrorCode::FailParse);
        }
    };
    let id = request.id;
    let generation = request.generation;
    let config = request.config_snapshot.clone().unwrap_or_default();

    let Some(payload) = request.payload else {
        log::warn!("engine request missing payload (id={id})");
        return error_response(id, ErrorCode::FailInvariant);
    };

    match payload {
        request::Payload::Phonetics(phon_req) => match phonetics::requests::handle(&phon_req) {
            Ok(phon_resp) => Response {
                id,
                error: ErrorCode::Ok as i32,
                payload: Some(response::Payload::Phonetics(phon_resp)),
            },
            Err(err) => {
                log::warn!("phonetics dispatch failed (id={id}): {err}");
                error_response(id, ErrorCode::FailInvariant)
            }
        },
        request::Payload::Composing(comp_req) => {
            // With the engine's own user data open, `FetchAtPos` reads it (U11).
            match user_data::handle_composing(&comp_req, &config, generation) {
                Ok(comp_resp) => Response {
                    id,
                    error: ErrorCode::Ok as i32,
                    payload: Some(response::Payload::Composing(comp_resp)),
                },
                Err(err) => {
                    log::warn!("composing dispatch failed (id={id}): {err}");
                    error_response(id, ErrorCode::FailInvariant)
                }
            }
        }
        request::Payload::Lexicon(lex_req) => {
            let Some(method) = lex_req.method else {
                log::warn!("lexicon request missing method (id={id})");
                return error_response(id, ErrorCode::FailInvariant);
            };
            match user_data::handle_lexicon(method) {
                Ok(lex_resp) => Response {
                    id,
                    error: ErrorCode::Ok as i32,
                    payload: Some(response::Payload::Lexicon(lex_resp)),
                },
                Err(err) => {
                    log::warn!("lexicon dispatch failed (id={id}): {err}");
                    error_response(id, lexicon_error_code(&err))
                }
            }
        }
        request::Payload::Nextword(nw_req) => {
            // With the engine's own user data open, predictions read it and
            // the associations it decides are written (U11).
            match user_data::handle_nextword(nw_req, &config, generation) {
                Ok(nw_resp) => Response {
                    id,
                    error: ErrorCode::Ok as i32,
                    payload: Some(response::Payload::Nextword(nw_resp)),
                },
                Err(err) => {
                    log::warn!("nextword dispatch failed (id={id}): {err}");
                    error_response(id, ErrorCode::FailInvariant)
                }
            }
        }
        request::Payload::CaseTransform(case_req) => {
            match phonetics::requests::handle_case(&case_req, &config) {
                Some(case_resp) => Response {
                    id,
                    error: ErrorCode::Ok as i32,
                    payload: Some(response::Payload::CaseTransform(case_resp)),
                },
                None => {
                    log::warn!("case request missing method (id={id})");
                    error_response(id, ErrorCode::FailInvariant)
                }
            }
        }
        request::Payload::UserData(user_data_req) => user_data::respond(id, &user_data_req),
    }
}

fn lexicon_error_code(err: &lexicon::LexiconError) -> ErrorCode {
    use lexicon::LexiconError as E;
    match err {
        E::InvalidPath(_) | E::PathNotAbsolute(_) | E::InvalidBinary(_) => ErrorCode::FailParse,
        E::Mmap { .. } => ErrorCode::FailIo,
        E::NotInitialized => ErrorCode::FailInvariant,
        E::Internal(_) => ErrorCode::FailInternal,
    }
}

/// Encode an error-only `Response` (id 0 — the request was refused before it
/// was decoded) for the FFI seams (`swift-ffi`, `android-jni`), so both crates
/// share one definition of the empty-payload error envelope.
#[must_use]
pub fn encode_error(code: ErrorCode) -> Vec<u8> {
    encode(&error_response(0, code))
}

/// Wire byte for a `log::Level` as delivered to the platform logger sinks
/// (iOS `SwiftLoggerSink`, Android `RustEngineBridge.dispatchLog`): Error=0 …
/// Trace=4. One table for both seams.
#[must_use]
pub fn log_level_to_byte(level: log::Level) -> u8 {
    match level {
        log::Level::Error => 0,
        log::Level::Warn => 1,
        log::Level::Info => 2,
        log::Level::Debug => 3,
        log::Level::Trace => 4,
    }
}

fn error_response(id: u32, code: ErrorCode) -> Response {
    Response {
        id,
        error: code as i32,
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
    use protos::engine::lexicon_request::Method as LexiconMethod;
    use protos::engine::{
        AppConfig, DictionarySearchRequest, DictionarySourceToggles, LexiconRequest,
    };

    fn lexicon_request(method: LexiconMethod) -> Request {
        Request {
            id: 42,
            config_snapshot: Some(AppConfig::default()),
            generation: 7,
            payload: Some(request::Payload::Lexicon(LexiconRequest {
                method: Some(method),
            })),
        }
    }

    // The lexicon arm's routing. No unit test in this crate installs a
    // lexicon, so this one sees the engine before install.
    #[test]
    fn dictionary_search_before_install_answers_no_rows() {
        let req = lexicon_request(LexiconMethod::DictionarySearch(DictionarySearchRequest {
            query: "tsiah".to_owned(),
            input_mode: "tl".to_owned(),
            toggles: Some(DictionarySourceToggles {
                kautian: true,
                ..DictionarySourceToggles::default()
            }),
            custom_dictionary_enabled: true,
            limit: 20,
        }));
        let resp = Response::decode(process_request(&req.encode_to_vec()).as_slice()).unwrap();

        assert_eq!(resp.id, 42);
        assert_eq!(
            resp.error,
            ErrorCode::Ok as i32,
            "not installed is no error"
        );
        let Some(response::Payload::Lexicon(lex_resp)) = resp.payload else {
            panic!("expected Lexicon payload");
        };
        let Some(protos::engine::lexicon_response::Result::DictionarySearchResult(found)) =
            lex_resp.result
        else {
            panic!("expected DictionarySearchResult");
        };
        assert!(found.rows.is_empty(), "{found:?}");
    }

    #[test]
    fn request_cap_accepts_exactly_the_cap_and_refuses_one_byte_over() {
        assert!(!is_request_too_large(MAX_REQUEST_BYTES));
        assert!(is_request_too_large(MAX_REQUEST_BYTES + 1));
    }

    #[test]
    fn dispatch_returns_fail_invariant_for_missing_payload() {
        let req = Request {
            id: 1,
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

    fn user_data_error(id: u32, request: protos::engine::UserDataRequest) -> i32 {
        let req = Request {
            id,
            config_snapshot: None,
            generation: 0,
            payload: Some(request::Payload::UserData(request)),
        };
        let mut buf = Vec::new();
        req.encode(&mut buf).unwrap();
        let resp = Response::decode(process_request(&buf).as_slice()).unwrap();
        assert_eq!(resp.id, id);
        assert!(resp.payload.is_none());
        resp.error
    }

    #[test]
    fn dispatch_returns_fail_invariant_for_user_data_missing_method() {
        let error = user_data_error(11, protos::engine::UserDataRequest { method: None });
        assert_eq!(error, ErrorCode::FailInvariant as i32);
    }

    #[test]
    fn dispatch_returns_fail_invariant_for_a_page_request_before_open() {
        // trace: lib unit tests never open the process's handle, so a page
        // request meets "user data is not open yet" (`settled_stores`).
        let error = user_data_error(
            12,
            protos::engine::UserDataRequest {
                method: Some(
                    protos::engine::user_data_request::Method::ListLearningRecords(
                        protos::engine::ListLearningRecords::default(),
                    ),
                ),
            },
        );
        assert_eq!(error, ErrorCode::FailInvariant as i32);
    }

    /// Mirrors `docs/contributing/rust-ffi-safety.md` §6 T1' (library-side panic
    /// isolation). Inject a dispatcher that panics; the same
    /// `catch_unwind` boundary used by `process_request` must convert
    /// the panic into an encoded `Response` carrying `FailInternal`.
    /// Uses the crate-private `process_request_with` seam so production
    /// code never has to expose a panic-injection hook.
    #[test]
    fn forced_dispatcher_panic_is_caught_and_returns_fail_internal() {
        let resp_bytes = process_request_with(&[1, 2, 3], |_| panic!("intentional T1' test panic"));
        let resp = Response::decode(resp_bytes.as_slice()).expect("response decodes");
        assert_eq!(
            resp.error,
            ErrorCode::FailInternal as i32,
            "panic in dispatcher must surface as FailInternal, got error={}",
            resp.error
        );
    }
}

//! Test-build-only engine trace — compiled only with the `e2e-trace`
//! feature; why release must carry none of it, and how that is proven, is
//! `docs/architecture/e2e-trace-schema.md` § Test mode only.
//!
//! JSON Lines, one object per line; the schema is
//! `docs/architecture/e2e-trace-schema.md`. Writing never panics and never
//! fails a request: every I/O error is dropped, because a trace must not
//! change the behavior it records.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use protos::engine::Response;

/// Bumped when an event's fields change meaning.
pub const SCHEMA_VERSION: u32 = 1;

/// Present in every traced artifact and absent from every release one —
/// the release build scripts grep for it. `#[used]` keeps it through LTO.
#[used]
pub static MARKER: [u8; 18] = *b"TAIGI_E2E_TRACE_V1";

struct Sink {
    file: File,
    origin: Instant,
    pid: u32,
}

static SINK: Mutex<Option<Sink>> = Mutex::new(None);
/// Set once a sink exists, so a traced build that never calls `open`
/// (unit tests) skips the per-request work without taking the lock.
static IS_OPEN: AtomicBool = AtomicBool::new(false);
static NEXT_THREAD_NUMBER: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// Process-local thread number, assigned on a thread's first event.
    static THREAD_NUMBER: u64 = NEXT_THREAD_NUMBER.fetch_add(1, Ordering::Relaxed);
}

/// Opens (append-creates) the trace file and writes the `trace_open`
/// header. A second call switches the sink to the new path. Returns false
/// when the file cannot be opened.
pub fn open(path: &str) -> bool {
    let Ok(file) = OpenOptions::new().create(true).append(true).open(path) else {
        return false;
    };
    let origin = Instant::now();
    *SINK.lock().unwrap_or_else(PoisonError::into_inner) = Some(Sink {
        file,
        origin,
        pid: std::process::id(),
    });
    IS_OPEN.store(true, Ordering::Release);
    let wall_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let marker = std::str::from_utf8(&MARKER).unwrap_or_default();
    emit(
        origin,
        format_args!(
            r#""event":"trace_open","schema_version":{SCHEMA_VERSION},"marker":"{marker}","layer":"engine","engine_version":"{}","wall_ms":{wall_ms}"#,
            env!("CARGO_PKG_VERSION"),
        ),
    );
    true
}

/// One `engine_request` event per `process_request` call. `started` is
/// taken before decoding, so `dur_us` covers decode + dispatch + encode but
/// not the trace write itself.
pub fn request(bytes: &[u8], response: &Response, resp_bytes: usize, started: Instant) {
    if !IS_OPEN.load(Ordering::Acquire) {
        return;
    }
    let dur_us = started.elapsed().as_micros();
    let (domain, method_tag) = classify(bytes);
    emit(
        started,
        format_args!(
            r#""event":"engine_request","req_id":{},"generation":{},"domain":"{domain}","method_tag":{method_tag},"error":{},"dur_us":{dur_us},"req_bytes":{},"resp_bytes":{resp_bytes}"#,
            response.id,
            response.generation,
            response.error,
            bytes.len(),
        ),
    );
}

/// A panic caught at the dispatch boundary.
pub fn panic(bytes: &[u8], started: Instant) {
    if !IS_OPEN.load(Ordering::Acquire) {
        return;
    }
    let (domain, method_tag) = classify(bytes);
    emit(
        started,
        format_args!(
            r#""event":"engine_panic","domain":"{domain}","method_tag":{method_tag},"req_bytes":{}"#,
            bytes.len(),
        ),
    );
}

/// A request an FFI adapter rejected before it reached dispatch.
pub(crate) fn adapter_reject(reason: &'static str, req_bytes: usize) {
    emit(
        Instant::now(),
        format_args!(r#""event":"adapter_reject","reason":"{reason}","req_bytes":{req_bytes}"#),
    );
}

/// Whether a trace file is open — lets a platform skip building an event
/// it would only drop.
pub fn is_open() -> bool {
    IS_OPEN.load(Ordering::Acquire)
}

/// A platform-layer event (key, preedit, candidates, commit — schema doc
/// § platform layer) in the same file and clock as the engine's. `fields`
/// is the rest of the JSON object, rendered by the caller; wrap every
/// string value in [`JsonStr`].
pub fn event(kind: &'static str, fields: std::fmt::Arguments<'_>) {
    if !IS_OPEN.load(Ordering::Acquire) {
        return;
    }
    emit(Instant::now(), format_args!(r#""event":"{kind}",{fields}"#));
}

/// A string as a quoted JSON value, escaped per RFC 8259 §7.
pub struct JsonStr<'a>(pub &'a str);

impl std::fmt::Display for JsonStr<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write as _;
        formatter.write_char('"')?;
        for ch in self.0.chars() {
            match ch {
                '"' => formatter.write_str("\\\"")?,
                '\\' => formatter.write_str("\\\\")?,
                '\n' => formatter.write_str("\\n")?,
                '\r' => formatter.write_str("\\r")?,
                '\t' => formatter.write_str("\\t")?,
                control if u32::from(control) < 0x20 => {
                    write!(formatter, "\\u{:04x}", u32::from(control))?
                }
                other => formatter.write_char(other)?,
            }
        }
        formatter.write_char('"')
    }
}

/// Writes one line: the common prefix (`t_us` since `open`, `pid`, `tid`)
/// then `fields`. No-op until `open` succeeds.
fn emit(at: Instant, fields: std::fmt::Arguments<'_>) {
    let mut guard = SINK.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(sink) = guard.as_mut() else {
        return;
    };
    let t_us = at.saturating_duration_since(sink.origin).as_micros();
    let tid = THREAD_NUMBER.with(|number| *number);
    let line = format!(
        "{{\"t_us\":{t_us},\"pid\":{},\"tid\":{tid},{fields}}}\n",
        sink.pid
    );
    let _ = sink.file.write_all(line.as_bytes());
}

/// `(domain, method_tag)` read straight from the request's wire bytes, so
/// tracing costs no second decode. The domain is the `Request.payload`
/// oneof field (`envelope.proto`, tags 10–15); every sub-request message
/// holds only its `oneof method`, so its first field number IS the method
/// tag. The analyzer maps `(domain, tag)` to a name from `protos/proto/`.
fn classify(bytes: &[u8]) -> (&'static str, u64) {
    let mut buf = bytes;
    while let Some((field, payload)) = next_field(&mut buf) {
        let domain = match field {
            10 => "phonetics",
            11 => "composing",
            12 => "lexicon",
            13 => "nextword",
            14 => "case",
            15 => "userdata",
            _ => continue,
        };
        let method_tag = payload
            .and_then(|mut inner| next_field(&mut inner))
            .map_or(0, |(tag, _)| tag);
        return (domain, method_tag);
    }
    ("none", 0)
}

/// Hand-written rather than `prost::encoding::decode_key` /
/// `decode_varint`: those are `#[doc(hidden)]`, outside prost's semver
/// promise.
///
/// Reads one protobuf field off the front of `buf`: its number, and its
/// body when length-delimited. `None` at the end or on malformed input.
fn next_field<'a>(buf: &mut &'a [u8]) -> Option<(u64, Option<&'a [u8]>)> {
    let key = read_varint(buf)?;
    let payload = match key & 7 {
        0 => {
            read_varint(buf)?;
            None
        }
        // fixed64 / fixed32
        wire_type @ (1 | 5) => {
            *buf = buf.get(if wire_type == 1 { 8 } else { 4 }..)?;
            None
        }
        2 => {
            let len = usize::try_from(read_varint(buf)?).ok()?;
            let body = buf.get(..len)?;
            *buf = &buf[len..];
            Some(body)
        }
        _ => return None,
    };
    Some((key >> 3, payload))
}

fn read_varint(buf: &mut &[u8]) -> Option<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let (&byte, rest) = buf.split_first()?;
        *buf = rest;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use protos::engine::{composing_request, request, Append, ComposingRequest, Request};

    #[test]
    fn classify_reads_domain_and_method_tag() {
        // trace: Request{id=7, generation=3, payload=composing(11){append(11)}}
        let bytes = Request {
            id: 7,
            generation: 3,
            payload: Some(request::Payload::Composing(ComposingRequest {
                method: Some(composing_request::Method::Append(Append {
                    char: "a".into(),
                })),
            })),
            ..Default::default()
        }
        .encode_to_vec();
        assert_eq!(classify(&bytes), ("composing", 11));
    }

    #[test]
    fn json_str_escapes_quotes_backslashes_and_controls() {
        let rendered = JsonStr("a\"b\\c\nd\u{1}台").to_string();
        assert_eq!(rendered, r#""a\"b\\c\nd\u0001台""#);
    }

    #[test]
    fn classify_tolerates_empty_and_malformed_bytes() {
        assert_eq!(classify(&[]), ("none", 0));
        assert_eq!(classify(&[0xff, 0xff]), ("none", 0));
    }
}

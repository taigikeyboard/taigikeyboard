//! `e2e-trace` feature: a real `process_request` call lands in the trace
//! file as a header line plus one `engine_request` line.
#![cfg(feature = "e2e-trace")]

use prost::Message;
use protos::engine::phonetics_request::Method;
use protos::engine::{request, PhoneticsRequest, Request, StripTone};

#[test]
fn process_request_writes_header_and_request_event() {
    let path = std::env::temp_dir().join(format!("e2e-trace-{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&path);
    assert!(dispatch::trace::open(path.to_str().unwrap()));

    // trace: payload = phonetics (Request field 10), method = strip_tone
    // (PhoneticsRequest field 11, phonetics.proto) → domain "phonetics", tag 11.
    let bytes = Request {
        id: 42,
        generation: 9,
        payload: Some(request::Payload::Phonetics(PhoneticsRequest {
            method: Some(Method::StripTone(StripTone {
                input: "ká".into()
            })),
        })),
        ..Default::default()
    }
    .encode_to_vec();
    let _ = dispatch::process_request(&bytes);
    let _ = dispatch::process_request(&[0xff]);

    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert_has(
        lines[0],
        &[
            r#""event":"trace_open""#,
            r#""schema_version":1"#,
            r#""marker":"TAIGI_E2E_TRACE_V1""#,
        ],
    );
    assert_has(
        lines[1],
        &[
            r#""event":"engine_request""#,
            r#""req_id":42"#,
            r#""generation":9"#,
            r#""domain":"phonetics""#,
            r#""method_tag":11"#,
            r#""error":0"#,
        ],
    );
    // Undecodable bytes still trace, with the parse error and no domain.
    assert!(lines[2].contains(r#""domain":"none""#), "{}", lines[2]);
    assert!(!lines[2].contains(r#""error":0"#), "{}", lines[2]);
    let _ = std::fs::remove_file(&path);
}

fn assert_has(line: &str, needles: &[&str]) {
    for needle in needles {
        assert!(line.contains(needle), "missing {needle} in {line}");
    }
}

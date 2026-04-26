//! T6 — Rust core uses the `log` crate. A capture logger installed by the test
//! must receive records emitted from the dispatch path. Real T6 (round-trip
//! through `OSLog` / `android.util.Log`) lives in D9.2 — D9.1 just verifies
//! that `phonetics` honors the contract by going through `log` at all.

use log::{Level, Metadata, Record};
use phonetics::api::process_request;
use std::sync::{Mutex, OnceLock};

struct CaptureLogger {
    sink: Mutex<Vec<String>>,
}

impl log::Log for CaptureLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }
    fn log(&self, record: &Record) {
        if record.level() <= Level::Warn {
            let line = format!("{}: {}", record.level(), record.args());
            self.sink.lock().unwrap().push(line);
        }
    }
    fn flush(&self) {}
}

static LOGGER: OnceLock<&'static CaptureLogger> = OnceLock::new();

fn install_logger() -> &'static CaptureLogger {
    LOGGER.get_or_init(|| {
        let leaked: &'static CaptureLogger = Box::leak(Box::new(CaptureLogger {
            sink: Mutex::new(Vec::new()),
        }));
        // set_logger is a one-shot. Tests in the same binary share the logger.
        let _ = log::set_logger(leaked);
        log::set_max_level(log::LevelFilter::Trace);
        leaked
    })
}

#[test]
fn malformed_request_emits_warning_through_log_crate() {
    let logger = install_logger();
    {
        // Drain prior messages so the assertion is stable when this test file
        // grows additional cases.
        logger.sink.lock().unwrap().clear();
    }
    let _ = process_request(&[0xff, 0x01, 0xff]);
    let captured = logger.sink.lock().unwrap().clone();
    assert!(
        captured.iter().any(|m| m.contains("phonetics request")),
        "expected a 'phonetics request' warning, captured: {captured:?}"
    );
}

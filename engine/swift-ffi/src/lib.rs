//! iOS / macOS swift-bridge entry point. Thin wrapper around
//! `dispatch::process_request` plus a logger sink registration.
//!
//! Everything that crosses the FFI seam is wrapped in `catch_unwind` per
//! `docs/engine/ffi-safety.md` §2 and the request body is size-capped per
//! plan v3 §B3 so an oversized payload returns `FAIL_INVARIANT` instead of
//! allocating without bound.

// iOS / macOS swift-bridge 入口,負責包覆 dispatch 並註冊 Swift 端的 logger sink。
// 所有跨 FFI 邊界的呼叫皆以 catch_unwind 包覆,並對請求大小設限避免無上限配置。

use dispatch::{encode_error, log_level_to_byte, MAX_REQUEST_BYTES};
use protos::engine::ErrorCode;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, Once, OnceLock};

// Bridge module. Doc comments live OUTSIDE this block — swift-bridge's parser
// rejects `///` on the items inside.
//
// `process_request_bytes`: single FFI entry point. Decodes a
// `taigi.engine.Request`, dispatches, returns a `taigi.engine.Response` byte
// buffer. Always returns a valid encoded `Response` — never panics across
// the seam.
//
// `install_logger_sink`: idempotent Swift-side logger registration. The
// underlying `log::set_logger` is one-shot via `std::sync::Once`.
//
// `panic_for_test`: T1 panic injector. Always present in the API surface
// because swift-bridge does not support `#[cfg(feature = "...")]` on bridge
// items. The symbol exists in both dev and release xcframeworks; the BODY is
// cfg-gated. Without the `panic-injector` feature, the function returns a
// benign FAIL_INVARIANT response rather than panicking. Production app code
// never references this symbol; the test target does.
//
// `SwiftLoggerSink`: Swift class registered through `install_logger_sink`
// that receives every `log::Record`. The Swift side maps `level` to the
// `LoggerBackend` protocol method (`error`, `warning`, `info`, `debug`).
//
// swift-bridge 模組,定義 Swift 與 Rust 雙向呼叫的 FFI 介面。
#[swift_bridge::bridge]
mod ffi {
    extern "Rust" {
        fn process_request_bytes(bytes: &[u8]) -> Vec<u8>;
        fn install_logger_sink(sink: SwiftLoggerSink);
        fn set_log_level(level: u8);
        fn panic_for_test() -> Vec<u8>;
    }

    extern "Swift" {
        type SwiftLoggerSink;
        fn log(&self, level: u8, category: String, message: String);
    }
}

// Swift 端唯一的請求入口,解碼 Request、分派、回傳已編碼 Response。
fn process_request_bytes(bytes: &[u8]) -> Vec<u8> {
    catch_unwind(AssertUnwindSafe(|| {
        if bytes.len() > MAX_REQUEST_BYTES {
            // JUSTIFICATION: mapped to FAIL_INVARIANT (not FAIL_PARSE) — bytes
            // may be wire-valid; the engine invariant violated is "request
            // size ≤ MAX_REQUEST_BYTES". Adding FAIL_SIZE would renumber proto
            // reserved fields.
            return encode_error(0, ErrorCode::FailInvariant, 0);
        }
        dispatch::process_request(bytes)
    }))
    .unwrap_or_else(|_| encode_error(0, ErrorCode::FailInternal, 0))
}

// 註冊 Swift 端的 logger sink,並一次性安裝 Rust log adapter,預設 Warn 等級。
fn install_logger_sink(sink: ffi::SwiftLoggerSink) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let mutex = LOGGER_SINK.get_or_init(|| Mutex::new(None));
        if let Ok(mut guard) = mutex.lock() {
            *guard = Some(SinkCell(sink));
        }
        SET_LOGGER.call_once(|| {
            // Returns Err if a logger is already installed; we treat that as
            // benign because installing a second time means a previous test
            // run installed the same global logger object.
            let _ = log::set_logger(&PLATFORM_LOGGER);
            // Default to Warn so release builds do NOT pay the cost of
            // formatting `log::debug!` / `log::info!` messages that the
            // platform side would only no-op anyway. Callers can opt in to
            // higher verbosity via `set_log_level` (e.g. iOS DEBUG calls
            // `set_log_level(4)` for `Debug`).
            log::set_max_level(log::LevelFilter::Warn);
        });
    }));
}

/// Adjust the Rust `log::max_level` at runtime. Intended for platform
/// bridges to bump verbosity in DEBUG builds without paying the format
/// cost in release.
///
/// Input mapping: 0=Off, 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace.
/// Anything outside the range is treated as `Off` (defensive — keeps an
/// integer typo from accidentally enabling trace).
///
/// NOT the same as `SwiftLoggerSink`'s callback bytes (`dispatch::log_level_to_byte` /
/// Swift constants `levelError=0..levelTrace=4`). Off is reserved here
/// because callers can disable; the callback never receives Off.
// 執行期調整 log::max_level,讓 DEBUG 版可開更詳細,釋出版不付格式化成本。
fn set_log_level(level: u8) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let filter = match level {
            1 => log::LevelFilter::Error,
            2 => log::LevelFilter::Warn,
            3 => log::LevelFilter::Info,
            4 => log::LevelFilter::Debug,
            5 => log::LevelFilter::Trace,
            _ => log::LevelFilter::Off,
        };
        log::set_max_level(filter);
        log::info!("rust log level set to {filter:?}");
    }));
}

// T1 恐慌注入點;符號永遠存在但內容受 panic-injector feature 控管,釋出版回傳良性錯誤。
fn panic_for_test() -> Vec<u8> {
    catch_unwind(AssertUnwindSafe(|| -> Vec<u8> {
        #[cfg(feature = "panic-injector")]
        {
            panic!("intentional T1 panic at swift-ffi boundary");
        }
        #[cfg(not(feature = "panic-injector"))]
        {
            // Release xcframework no-ops: returns a benign error sentinel. The
            // test target builds the dev xcframework with `--features
            // panic-injector` so T1 actually panics inside the catch boundary.
            encode_error(0, ErrorCode::FailInvariant, 0)
        }
    }))
    .unwrap_or_else(|_| encode_error(0, ErrorCode::FailInternal, 0))
}

// -- Logger glue ---------------------------------------------------------

/// Wrapper that lets a `SwiftLoggerSink` cross the `Send + Sync` boundary.
///
/// SAFETY: `SwiftLoggerSink` is a Swift class instance held by Rust through a
/// reference-counted pointer. The `log` method on the Swift side is required
/// to be thread-safe by the registration contract — the only conforming
/// implementation in this project routes through `LoggerFactory.make`, which
/// is `NSLock`-guarded.
// 把 SwiftLoggerSink 包成可跨執行緒邊界的型別;Swift 端 log 方法需自行確保執行緒安全。
struct SinkCell(ffi::SwiftLoggerSink);
unsafe impl Send for SinkCell {}
unsafe impl Sync for SinkCell {}

static LOGGER_SINK: OnceLock<Mutex<Option<SinkCell>>> = OnceLock::new();
static SET_LOGGER: Once = Once::new();
static PLATFORM_LOGGER: PlatformLogger = PlatformLogger;

struct PlatformLogger;

impl log::Log for PlatformLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let Some(mutex) = LOGGER_SINK.get() else {
            return;
        };
        // try_lock breaks any reentrant log loop (Swift sink that logs from
        // its own log handler) — a missed line is preferable to a deadlock.
        let Ok(guard) = mutex.try_lock() else {
            return;
        };
        let Some(cell) = guard.as_ref() else {
            return;
        };
        let level = log_level_to_byte(record.level());
        cell.0.log(
            level,
            record.target().to_string(),
            format!("{}", record.args()),
        );
    }

    fn flush(&self) {}
}


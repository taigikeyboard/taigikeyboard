//! Swift-side `log::Log` sink hosted in `taigi-macos-ffi`.
//!
//! `install_logger_sink` registers [`PLATFORM_LOGGER`] once per process and
//! stores the `SwiftLoggerSink` closure; `set_log_level` adjusts `log::max_level`
//! at runtime so release builds silence Rust logs at the source.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Mutex, Once, OnceLock};

use dispatch::log_level_to_byte;

use crate::ffi::SwiftLoggerSink;

/// Idempotent Swift-side logger registration. The underlying `log::set_logger`
/// is one-shot via `std::sync::Once`.
pub(crate) fn install(sink: SwiftLoggerSink) {
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
            // formatting `log::debug!` / `log::info!` messages that
            // `DebugLogger` would only discard anyway. Callers opt in to
            // higher verbosity via `set_log_level` (macOS DEBUG calls
            // `set_log_level(4)` for `Debug`).
            log::set_max_level(log::LevelFilter::Warn);
        });
    }));
}

/// Adjust the Rust `log::max_level` at runtime.
///
/// Input mapping: 0=Off, 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace. Anything
/// outside the range is treated as `Off` (defensive — keeps an integer typo
/// from accidentally enabling trace).
///
/// NOT the same as `SwiftLoggerSink`'s callback bytes
/// (`dispatch::log_level_to_byte`: Error=0 … Trace=4). Off is reserved here
/// because callers can disable logging; the callback never receives Off.
pub(crate) fn set_level(level: u8) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let filter = level_filter_from_byte(level);
        log::set_max_level(filter);
        log::info!("rust log level set to {filter:?}");
    }));
}

fn level_filter_from_byte(level: u8) -> log::LevelFilter {
    match level {
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        5 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Off,
    }
}

/// Wrapper that lets `SwiftLoggerSink` sit inside `LOGGER_SINK` (`Mutex<Option<SinkCell>>`).
struct SinkCell(SwiftLoggerSink);

// SAFETY: `SwiftLoggerSink` is a Swift class instance held by Rust through an
// atomically reference-counted pointer, and every access to the inner sink is
// serialized by `LOGGER_SINK`'s `Mutex<Option<SinkCell>>`. Dropping or moving
// `SinkCell` across threads only touches that atomic refcount, and `install` /
// `PlatformLogger::log` never expose or invoke the closure concurrently.
unsafe impl Send for SinkCell {}

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
        // try_lock breaks reentrant log loops (e.g., a Swift sink that logs
        // from its own handler) and drops concurrently contended cross-thread
        // log lines rather than blocking; missing a log line is preferable to
        // deadlocking the typing path.
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

#[cfg(test)]
mod tests {
    use super::level_filter_from_byte;

    #[test]
    fn level_filter_byte_mapping_matches_contract() {
        assert_eq!(level_filter_from_byte(0), log::LevelFilter::Off);
        assert_eq!(level_filter_from_byte(1), log::LevelFilter::Error);
        assert_eq!(level_filter_from_byte(2), log::LevelFilter::Warn);
        assert_eq!(level_filter_from_byte(3), log::LevelFilter::Info);
        assert_eq!(level_filter_from_byte(4), log::LevelFilter::Debug);
        assert_eq!(level_filter_from_byte(5), log::LevelFilter::Trace);
        assert_eq!(level_filter_from_byte(6), log::LevelFilter::Off);
        assert_eq!(level_filter_from_byte(255), log::LevelFilter::Off);
    }
}

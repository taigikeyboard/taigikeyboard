//! The panic boundary every COM entry point stands behind. The release
//! profile keeps `panic = "unwind"` (so a panic is catchable at all); this
//! is what catches it.

// 中文: COM 進入點的 catch_unwind 邊界;panic 變成 E_FAIL,絕不穿越到宿主。

use std::panic::{catch_unwind, AssertUnwindSafe};
use windows::core::HRESULT;
use windows::Win32::Foundation::E_FAIL;

/// Runs `body` and turns a panic into `E_FAIL` with one log line naming the
/// entry point.
pub fn guarded_hresult(entry: &'static str, body: impl FnOnce() -> HRESULT) -> HRESULT {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(_) => {
            log::error!("tsf.panic entry={entry}");
            E_FAIL
        }
    }
}

/// The same boundary for a method that answers a `windows::core::Result`.
pub fn guarded<T>(
    entry: &'static str,
    body: impl FnOnce() -> windows::core::Result<T>,
) -> windows::core::Result<T> {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(_) => {
            log::error!("tsf.panic entry={entry}");
            Err(windows::core::Error::from_hresult(E_FAIL))
        }
    }
}

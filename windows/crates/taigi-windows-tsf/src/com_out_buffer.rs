//! The COM methods that WRITE into a buffer we then read, called through the
//! interface's own vtable so the OS gets a pointer it may write through.
//!
//! Same defect as `taigi_windows_platform::os_out_buffer`, in the generated
//! COM wrappers rather than the free functions: a method taking the buffer as
//! `&mut [T]` forwards `core::mem::transmute(slice.as_ptr())` — read-only
//! provenance — and an optimized build is then free to read the buffer back
//! as its initializer. `os_out_buffer` carries the measurement that found it;
//! these three are the same shape, fixed before one of them starts returning
//! an empty selection on a compiler that decides to fold.
//!
//! Each helper mirrors its generated wrapper exactly (same vtable slot, same
//! argument order, and the same `Result` / raw `HRESULT` return — an
//! enumerator's `S_FALSE` must survive), with `as_mut_ptr()` in place of
//! `as_ptr()`.
//!
//! Every helper is `unsafe` for what a slice type cannot prove: the interface
//! must be live for the call, an edit cookie must belong to a live edit
//! session on that context, and the COM method must not keep the pointer
//! (none of these do — each writes and returns).

// 會寫入我們 buffer 的 COM 方法,直接走 vtable 呼叫,把可寫指標交給系統。
// 與 os_out_buffer 同一個上游缺陷,只是出現在 COM 包裝。

use std::mem::ManuallyDrop;
use windows::core::Interface;
use windows::Win32::Graphics::DirectWrite::{IDWriteTextLayout, DWRITE_LINE_METRICS};
use windows::Win32::System::Com::IEnumGUID;
use windows::Win32::UI::TextServices::{
    IEnumTfInputProcessorProfiles, ITfContext, ITfRange, TF_DEFAULT_SELECTION,
    TF_INPUTPROCESSORPROFILE, TF_SELECTION, TF_SELECTIONSTYLE,
};
use windows_core::Result;
use windows_core::{GUID, HRESULT};

/// The host's current selection as a range this code owns, or `None` when it
/// reports none. The one shape both callers want: the `ManuallyDrop` dance
/// that takes ownership of the returned interface is written once here rather
/// than at each of them.
pub unsafe fn default_selection_range(context: &ITfContext, edit_cookie: u32) -> Option<ITfRange> {
    let mut selections = [TF_SELECTION {
        range: ManuallyDrop::new(None),
        style: TF_SELECTIONSTYLE::default(),
    }];
    let mut fetched = 0u32;
    // SAFETY: `context` is live for the call and `edit_cookie` is the caller's
    // own session cookie; `selections` is a live, writable slice whose length
    // travels with it, and the host keeps neither pointer.
    unsafe {
        (Interface::vtable(context).GetSelection)(
            Interface::as_raw(context),
            edit_cookie,
            TF_DEFAULT_SELECTION,
            selections.len() as u32,
            selections.as_mut_ptr(),
            &mut fetched,
        )
        .ok()
        .ok()?;
    }
    if fetched == 0 {
        return None;
    }
    // SAFETY: a fetched entry carries an interface the host handed over; this
    // is the only take, and the returned range releases it.
    unsafe { ManuallyDrop::take(&mut selections[0].range) }
}

/// The range's plain text, and how many UTF-16 units were written
/// (`ITfRange::GetText`).
pub unsafe fn range_text(
    range: &ITfRange,
    edit_cookie: u32,
    flags: u32,
    text: &mut [u16],
    length: &mut u32,
) -> Result<()> {
    // SAFETY: as above — `range` is live, `edit_cookie` is the caller's, and
    // `text` is a live writable buffer whose length travels with it.
    unsafe {
        (Interface::vtable(range).GetText)(
            Interface::as_raw(range),
            edit_cookie,
            flags,
            windows_core::PWSTR(text.as_mut_ptr()),
            text.len() as u32,
            length,
        )
        .ok()
    }
}

/// The layout's per-line metrics, and how many lines it has
/// (`IDWriteTextLayout::GetLineMetrics`).
pub unsafe fn line_metrics(
    layout: &IDWriteTextLayout,
    lines: &mut [DWRITE_LINE_METRICS],
    count: &mut u32,
) -> Result<()> {
    // SAFETY: as above. `layout` is live and `lines` is a live writable
    // buffer — DirectWrite writes at most as many entries as its length says
    // and reports the real line count through `count`.
    unsafe {
        (Interface::vtable(layout).GetLineMetrics)(
            Interface::as_raw(layout),
            lines.as_mut_ptr(),
            lines.len() as u32,
            count,
        )
        .ok()
    }
}

/// The next batch of input-processor profiles, and how many were written
/// (`IEnumTfInputProcessorProfiles::Next`). `S_FALSE` (a short last batch) is
/// `Ok` here, as it is in the generated wrapper: only the count says whether
/// the enumeration is done.
pub unsafe fn next_input_processor_profiles(
    enumerator: &IEnumTfInputProcessorProfiles,
    batch: &mut [TF_INPUTPROCESSORPROFILE],
    fetched: &mut u32,
) -> Result<()> {
    // SAFETY: `enumerator` is live for the call; `batch` is a live writable
    // buffer whose length travels with it, and the enumerator keeps no pointer.
    unsafe {
        (Interface::vtable(enumerator).Next)(
            Interface::as_raw(enumerator),
            batch.len() as u32,
            batch.as_mut_ptr(),
            fetched,
        )
        .ok()
    }
}

/// The next batch of GUIDs, as the RAW `HRESULT` — `S_OK` and `S_FALSE` are
/// the enumerator's two ways of saying how much is left, and the caller
/// (`enumeration_continues`) reads the difference.
pub unsafe fn next_guids(enumerator: &IEnumGUID, batch: &mut [GUID], fetched: &mut u32) -> HRESULT {
    // SAFETY: as above — a live enumerator and a live writable batch whose
    // length is passed alongside it.
    unsafe {
        (Interface::vtable(enumerator).Next)(
            Interface::as_raw(enumerator),
            batch.len() as u32,
            batch.as_mut_ptr(),
            fetched,
        )
    }
}

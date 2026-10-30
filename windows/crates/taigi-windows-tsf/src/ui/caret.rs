//! Where the caret is, in PHYSICAL screen pixels: `ITfContextView::GetTextExt`
//! on the composition's end (rakukan `on_compose.rs:29-43`), then its start,
//! then the selection (khiin `composition_utils.rs:31-69`). A rectangle the
//! host reports empty or not at all is "no anchor" (roadmap W4): the window
//! is not shown, as on the Mac — a bar parked in a corner of the host window
//! is worse than none. A host that has not laid the text out yet
//! (`TS_E_NOLAYOUT`) is told apart ([`Caret::NotLaidOut`]): a window already
//! up keeps its place (`Surface::present`). A rectangle reported clipped is
//! still an anchor: PowerPoint flags every slide text box clipped. A host that is not
//! per-monitor DPI aware answers in its own virtualised coordinates; they are
//! mapped to physical pixels through the host window's awareness before use.

use crate::com_out_buffer;
use crate::edit_session::EditCookie;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::HiDpi::LogicalToPhysicalPointForPerMonitorDPI;
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfContext, ITfContextView, ITfRange, TfAnchor, TF_ANCHOR_END, TF_ANCHOR_START,
    TS_E_NOLAYOUT,
};

/// What the host says about where its caret is.
pub enum Caret {
    /// The caret line's rectangle, physical screen pixels.
    At(RECT),
    /// Some probe was refused with `TS_E_NOLAYOUT` and none answered: Chrome
    /// until its renderer reports the composition's character bounds — which
    /// Google Docs never does once a combining mark such as POJ `o͘` sits
    /// inside the composition.
    NotLaidOut,
    /// No usable rectangle.
    Unknown,
}

impl Caret {
    pub fn rect(self) -> Option<RECT> {
        match self {
            Caret::At(rect) => Some(rect),
            Caret::NotLaidOut | Caret::Unknown => None,
        }
    }
}

/// The extent of `range`, `Ok(None)` for a zero-height rectangle, or the
/// call's error (text not visible answers `{0,0,0,0}`;
/// an Office dialog answers a zero-height rect in a screen corner). A
/// zero-width caret line is kept. The clipped flag only says the box does
/// not cover the whole range — it is ignored, as mozc, khiin and rakukan do:
/// 64-bit PowerPoint reports every slide text box clipped, and refusing it
/// dropped the candidate list there.
unsafe fn text_extent(
    view: &ITfContextView,
    ec: EditCookie,
    range: &ITfRange,
) -> windows::core::Result<Option<RECT>> {
    let mut rect = RECT::default();
    let mut clipped = BOOL(0);
    view.GetTextExt(ec, range, &mut rect, &mut clipped)?;
    Ok((rect.bottom > rect.top).then_some(rect))
}

/// The extent at `range`'s `anchor` end; a `TS_E_NOLAYOUT` refusal is
/// recorded in `is_not_laid_out`.
unsafe fn probe(
    view: &ITfContextView,
    ec: EditCookie,
    range: &ITfRange,
    anchor: TfAnchor,
    is_not_laid_out: &mut bool,
) -> Option<RECT> {
    let collapsed = collapsed(range, ec, anchor)?;
    match text_extent(view, ec, &collapsed) {
        Ok(rect) => rect,
        Err(error) => {
            *is_not_laid_out |= error.code() == TS_E_NOLAYOUT;
            None
        }
    }
}

unsafe fn collapsed(range: &ITfRange, ec: EditCookie, anchor: TfAnchor) -> Option<ITfRange> {
    let clone = range.Clone().ok()?;
    clone.Collapse(ec, anchor).ok()?;
    Some(clone)
}

/// `GetTextExt` speaks the host's coordinate space. For a per-monitor-aware
/// host that IS physical pixels; a system-aware or unaware host is
/// virtualised by Windows, and `LogicalToPhysicalPointForPerMonitorDPI`
/// undoes that virtualisation using the host window's own awareness.
unsafe fn to_physical(rect: RECT, host: Option<HWND>) -> RECT {
    let Some(host) = host else {
        return rect;
    };
    let mut top_left = POINT {
        x: rect.left,
        y: rect.top,
    };
    let mut bottom_right = POINT {
        x: rect.right,
        y: rect.bottom,
    };
    if !LogicalToPhysicalPointForPerMonitorDPI(Some(host), &mut top_left).as_bool()
        || !LogicalToPhysicalPointForPerMonitorDPI(Some(host), &mut bottom_right).as_bool()
    {
        return rect;
    }
    RECT {
        left: top_left.x,
        top: top_left.y,
        right: bottom_right.x,
        bottom: bottom_right.y,
    }
}

/// Where the caret line is for the window to anchor to.
pub fn caret(context: &ITfContext, ec: EditCookie, composition: Option<&ITfComposition>) -> Caret {
    // SAFETY: every range is a clone this function owns; the view is the
    // context's; all under the session's cookie.
    unsafe {
        let Ok(view) = context.GetActiveView() else {
            return Caret::Unknown;
        };
        let host = view.GetWnd().ok().filter(|hwnd| !hwnd.is_invalid());
        let mut is_not_laid_out = false;
        if let Some(composition) = composition {
            if let Ok(range) = composition.GetRange() {
                for anchor in [TF_ANCHOR_END, TF_ANCHOR_START] {
                    if let Some(rect) = probe(&view, ec, &range, anchor, &mut is_not_laid_out) {
                        return Caret::At(to_physical(rect, host));
                    }
                }
            }
        }
        if let Some(range) = com_out_buffer::default_selection_range(context, ec) {
            if let Some(rect) = probe(&view, ec, &range, TF_ANCHOR_END, &mut is_not_laid_out) {
                return Caret::At(to_physical(rect, host));
            }
        }
        log::debug!("caret.no_anchor not_laid_out={is_not_laid_out}");
        if is_not_laid_out {
            Caret::NotLaidOut
        } else {
            Caret::Unknown
        }
    }
}

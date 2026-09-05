//! Where the caret is, in PHYSICAL screen pixels: `ITfContextView::GetTextExt`
//! on the composition's end (rakukan `on_compose.rs:29-43`), then its start,
//! then the selection (khiin `composition_utils.rs:31-69`). A rectangle the
//! host reports clipped, empty or not at all is "no anchor" (roadmap W4):
//! the window is not shown, as on the Mac — a bar parked in a corner of the
//! host window is worse than none. A host that is not per-monitor DPI aware
//! answers in its own virtualised coordinates; they are mapped to physical
//! pixels through the host window's awareness before use.

// 游標矩形(實體螢幕像素)— 組字尾→組字頭→選取;clipped/空/失敗 = 沒有錨點就不顯示;宿主非 DPI-aware 時做座標對映。

use crate::com_out_buffer;
use crate::edit_session::EditCookie;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::HiDpi::LogicalToPhysicalPointForPerMonitorDPI;
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfContext, ITfContextView, ITfRange, TfAnchor, TF_ANCHOR_END, TF_ANCHOR_START,
};

fn is_empty(rect: &RECT) -> bool {
    rect.right <= rect.left && rect.bottom <= rect.top
}

/// The extent of `range` when the host reports it whole: a clipped
/// rectangle (the text is scrolled partly out of view, or the host cannot
/// really say) is refused rather than trusted.
unsafe fn text_extent(view: &ITfContextView, ec: EditCookie, range: &ITfRange) -> Option<RECT> {
    let mut rect = RECT::default();
    let mut clipped = BOOL(0);
    view.GetTextExt(ec, range, &mut rect, &mut clipped).ok()?;
    if clipped.as_bool() || is_empty(&rect) || rect.bottom <= rect.top {
        return None;
    }
    Some(rect)
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

/// The caret line's rectangle (physical screen pixels) for the window to
/// anchor to, or `None` when the host cannot say where its caret is.
pub fn caret_rect(
    context: &ITfContext,
    ec: EditCookie,
    composition: Option<&ITfComposition>,
) -> Option<RECT> {
    // SAFETY: every range is a clone this function owns; the view is the
    // context's; all under the session's cookie.
    unsafe {
        let view = context.GetActiveView().ok()?;
        let host = view.GetWnd().ok().filter(|hwnd| !hwnd.is_invalid());
        if let Some(composition) = composition {
            if let Ok(range) = composition.GetRange() {
                for anchor in [TF_ANCHOR_END, TF_ANCHOR_START] {
                    if let Some(probe) = collapsed(&range, ec, anchor) {
                        if let Some(rect) = text_extent(&view, ec, &probe) {
                            return Some(to_physical(rect, host));
                        }
                    }
                }
            }
        }
        if let Some(range) = com_out_buffer::default_selection_range(context, ec) {
            if let Some(probe) = collapsed(&range, ec, TF_ANCHOR_END) {
                if let Some(rect) = text_extent(&view, ec, &probe) {
                    return Some(to_physical(rect, host));
                }
            }
        }
        log::debug!("caret.no_anchor");
        None
    }
}

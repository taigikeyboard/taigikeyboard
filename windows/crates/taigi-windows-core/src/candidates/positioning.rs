//! Where the candidate window goes, given a caret and a screen. Pure
//! geometry, port of `CandidatePanelPositioning.swift` (azooKey-Desktop
//! derived) — in WINDOWS coordinates: y grows DOWNWARD, `caret` spans the
//! caret's line (its `y` is the line's top), `visible` is the monitor's work
//! area. NAMED DELTA from the macOS file, which is written y-up.

// 中文: 候選窗擺放 — 游標下方,放不下翻到上方,再夾進螢幕;座標為 Windows 的 y 向下。

/// A point in screen space, in device-independent pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn right(&self) -> f32 {
        self.x + self.width
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
    /// All-zero, which is how the shell reports "the host gave no caret".
    pub fn is_zero(&self) -> bool {
        self.x == 0.0 && self.y == 0.0 && self.width == 0.0 && self.height == 0.0
    }
}

/// The gap between the window and the line of text it belongs to.
const GAP_FROM_CARET_LINE: f32 = 4.0;

/// The frame to place a panel of `panel_size` at for `caret` within
/// `visible`. Below the caret line by default; flipped above when there is
/// no room; the size clamped to the usable area FIRST so the origin clamps
/// cannot disagree about which edge wins; each axis clamped far edge first
/// then near edge, so an oversized panel ends flush with the leading edge.
pub fn panel_frame(caret: Rect, panel_size: Size, visible: Rect) -> Rect {
    let width = panel_size.width.min(visible.width);
    let height = panel_size.height.min(visible.height);
    let mut x = caret.x;
    let mut y = caret.bottom() + GAP_FROM_CARET_LINE;
    if y + height > visible.bottom() {
        y = caret.y - GAP_FROM_CARET_LINE - height;
    }
    x = clamp(x, width, visible.x, visible.right());
    y = clamp(y, height, visible.y, visible.bottom());
    Rect::new(x, y, width, height)
}

/// Slides an interval of `length` starting at `start` back inside
/// `lower..=upper`, far edge first.
fn clamp(start: f32, length: f32, lower: f32, upper: f32) -> f32 {
    let mut start = start;
    if start + length > upper {
        start = upper - length;
    }
    start.max(lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The macOS fixture, flipped to y-down: a 1000×800 display whose usable
    // area ends 50 above the bottom (`visibleFrame = (0, 50, 1000, 750)` y-up
    // ≡ (0, 0, 1000, 750) y-down), one 20-tall line in the middle.
    const VISIBLE: Rect = Rect::new(0.0, 0.0, 1000.0, 750.0);
    const PANEL: Size = Size {
        width: 300.0,
        height: 40.0,
    };

    fn caret(x: f32, y: f32, height: f32) -> Rect {
        Rect::new(x, y, 1.0, height)
    }

    #[test]
    fn sits_below_the_caret_line_at_its_leading_edge() {
        let frame = panel_frame(caret(400.0, 380.0, 20.0), PANEL, VISIBLE);
        assert_eq!(frame.x, 400.0);
        assert_eq!(frame.y, 400.0 + 4.0, "gap measured from the line's bottom");
        assert_eq!((frame.width, frame.height), (300.0, 40.0));
    }

    #[test]
    fn flips_above_when_there_is_no_room_below() {
        let near_bottom = caret(400.0, VISIBLE.bottom() - 30.0, 20.0);
        let frame = panel_frame(near_bottom, PANEL, VISIBLE);
        assert!(frame.bottom() <= near_bottom.y, "{frame:?}");
        assert_eq!(frame.bottom(), near_bottom.y - 4.0);
    }

    #[test]
    fn slides_left_rather_than_running_off_the_right_edge() {
        let frame = panel_frame(caret(VISIBLE.right() - 20.0, 380.0, 20.0), PANEL, VISIBLE);
        assert_eq!(frame.right(), VISIBLE.right());
        assert!(frame.x >= VISIBLE.x);
    }

    #[test]
    fn oversized_panels_are_cut_to_the_usable_area_and_pinned_to_the_leading_edge() {
        let wide = panel_frame(
            caret(400.0, 380.0, 20.0),
            Size {
                width: VISIBLE.width + 500.0,
                height: 40.0,
            },
            VISIBLE,
        );
        assert_eq!(
            (wide.x, wide.width, wide.right()),
            (0.0, VISIBLE.width, VISIBLE.right())
        );
        let tall = panel_frame(
            caret(400.0, 380.0, 20.0),
            Size {
                width: 300.0,
                height: VISIBLE.height + 500.0,
            },
            VISIBLE,
        );
        assert_eq!(
            (tall.y, tall.height, tall.bottom()),
            (0.0, VISIBLE.height, VISIBLE.bottom())
        );
    }

    #[test]
    fn position_follows_the_caret_line_height() {
        // A taller line pushes a flipped panel further up — a fixed cursor
        // height could not (the azooKey `cursorHeight: 16` fudge not carried).
        let tall = panel_frame(caret(400.0, VISIBLE.bottom() - 70.0, 60.0), PANEL, VISIBLE);
        let short = panel_frame(caret(400.0, VISIBLE.bottom() - 20.0, 10.0), PANEL, VISIBLE);
        assert!(tall.y < short.y, "{tall:?} vs {short:?}");
    }

    #[test]
    fn clamps_against_the_monitor_it_is_on_not_the_origin() {
        let second_display = Rect::new(-1600.0, -400.0, 1600.0, 900.0);
        let frame = panel_frame(caret(-100.0, 480.0, 20.0), PANEL, second_display);
        assert!(
            frame.x >= second_display.x && frame.right() <= second_display.right(),
            "{frame:?}"
        );
        assert!(
            frame.y >= second_display.y && frame.bottom() <= second_display.bottom(),
            "{frame:?}"
        );
    }
}

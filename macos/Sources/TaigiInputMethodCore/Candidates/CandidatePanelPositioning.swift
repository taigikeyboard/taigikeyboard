// Where the candidate bar goes, given a caret and a screen. Pure geometry.

import CoreGraphics

/// Places the candidate bar near the caret without letting it leave the screen.
///
/// Derived from azooKey-Desktop's `WindowPositioning.frameNearCursor`
/// (`references/azooKey-Desktop/Core/Sources/Core/Windows/WindowPositioning.swift:51-86`)
/// — the below-then-flip-above choice and the clamp-each-axis-twice shape are
/// its ideas. Three things are deliberately not carried over:
///
/// - its `cursorHeight: 16` fudge, which exists only because it is handed a
///   caret POINT. IMK reports a caret rect, so the line height is known and a
///   guessed one would misplace the bar at every other font size;
/// - its `currentFrame` in-parameter, which makes the result depend on where the
///   window happened to be last time;
/// - its `screenRect`-sized result for an oversized panel. Clamping the size to
///   the usable area first means the origin clamps below cannot disagree about
///   which edge wins.
///
/// Coordinates are AppKit's: y grows upward, and `caretRect` spans the caret's
/// line, so `minY` is the text baseline area's bottom and `maxY` its top.
enum CandidatePanelPositioning {
    /// The gap between the bar and the line of text it belongs to, in points.
    /// Without it the bar sits flush against the caret and reads as part of the
    /// composition rather than as a list of choices about it.
    private static let gapFromCaretLine: CGFloat = 4

    static func frame(
        anchoredTo caretRect: CGRect,
        panelSize: CGSize,
        within visibleFrame: CGRect,
    ) -> CGRect {
        let size = CGSize(
            width: min(panelSize.width, visibleFrame.width),
            height: min(panelSize.height, visibleFrame.height),
        )

        // Below the caret line by default — the bar then never covers the text
        // being composed, which is what the user is reading while they choose.
        var origin = CGPoint(
            x: caretRect.minX,
            y: caretRect.minY - gapFromCaretLine - size.height,
        )
        if origin.y < visibleFrame.minY {
            origin.y = caretRect.maxY + gapFromCaretLine
        }

        origin.x = clamp(origin.x, length: size.width, within: visibleFrame.minX ... visibleFrame.maxX)
        origin.y = clamp(origin.y, length: size.height, within: visibleFrame.minY ... visibleFrame.maxY)

        return CGRect(origin: origin, size: size)
    }

    /// Slides an interval of `length` starting at `start` back inside `bounds`.
    /// The far edge is fixed first and the near edge second, so an interval that
    /// cannot fit ends flush with the near edge rather than the far one — the
    /// bar's first candidates stay visible, which is where the highlight starts.
    private static func clamp(
        _ start: CGFloat,
        length: CGFloat,
        within bounds: ClosedRange<CGFloat>,
    ) -> CGFloat {
        var start = start
        if start + length > bounds.upperBound {
            start = bounds.upperBound - length
        }
        return max(start, bounds.lowerBound)
    }
}

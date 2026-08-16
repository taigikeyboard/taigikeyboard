// Where the candidate bar lands, in AppKit's y-up screen coordinates.

@testable import TaigiInputMethodCore
import XCTest

/// A 1000x800 display whose usable area starts 50pt above the bottom, so the
/// clamps have room to be wrong in a way an assertion can see.
private let visibleFrame = CGRect(x: 0, y: 50, width: 1000, height: 750)

/// One line of text in the middle of it.
private func caretRect(x: CGFloat = 400, y: CGFloat = 400, height: CGFloat = 20) -> CGRect {
    CGRect(x: x, y: y, width: 1, height: height)
}

private let panelSize = CGSize(width: 300, height: 40)

final class CandidatePanelPositioningTests: XCTestCase {
    func testBar_sitsBelowTheCaretLine_andStartsAtItsLeadingEdge() {
        let frame = CandidatePanelPositioning.frame(
            anchoredTo: caretRect(),
            panelSize: panelSize,
            within: visibleFrame,
        )

        XCTAssertEqual(frame.minX, 400, "the bar begins under the composition it describes")
        XCTAssertLessThan(
            frame.maxY,
            caretRect().minY,
            "below the line: a bar over the text would cover the composition being read",
        )
        XCTAssertEqual(frame.size, panelSize, "a bar that fits is shown at the size it asked for")
    }

    func testBar_flipsAboveTheCaret_whenThereIsNoRoomBelow() {
        let nearBottom = caretRect(y: visibleFrame.minY + 10)

        let frame = CandidatePanelPositioning.frame(
            anchoredTo: nearBottom,
            panelSize: panelSize,
            within: visibleFrame,
        )

        XCTAssertGreaterThanOrEqual(
            frame.minY,
            nearBottom.maxY,
            "with the caret on the bottom line the bar goes above it, not off the screen",
        )
    }

    func testBar_slidesLeft_ratherThanRunningOffTheRightEdge() {
        let nearRightEdge = caretRect(x: visibleFrame.maxX - 20)

        let frame = CandidatePanelPositioning.frame(
            anchoredTo: nearRightEdge,
            panelSize: panelSize,
            within: visibleFrame,
        )

        XCTAssertEqual(frame.maxX, visibleFrame.maxX)
        XCTAssertGreaterThanOrEqual(frame.minX, visibleFrame.minX)
    }

    /// The reason the size is clamped before the origin is: with a panel wider
    /// than the screen, "pull the right edge in" and "pull the left edge in"
    /// disagree, and whichever runs last wins. Clamping the width first leaves
    /// only one answer.
    func testBarWiderThanTheScreen_isCutToTheUsableAreaAndPinnedToItsLeadingEdge() {
        let frame = CandidatePanelPositioning.frame(
            anchoredTo: caretRect(),
            panelSize: CGSize(width: visibleFrame.width + 500, height: 40),
            within: visibleFrame,
        )

        XCTAssertEqual(frame.width, visibleFrame.width)
        XCTAssertEqual(frame.minX, visibleFrame.minX)
        XCTAssertEqual(frame.maxX, visibleFrame.maxX)
    }

    func testBarTallerThanTheScreen_staysInsideTheUsableArea() {
        let frame = CandidatePanelPositioning.frame(
            anchoredTo: caretRect(),
            panelSize: CGSize(width: 300, height: visibleFrame.height + 500),
            within: visibleFrame,
        )

        XCTAssertEqual(frame.height, visibleFrame.height)
        XCTAssertEqual(frame.minY, visibleFrame.minY)
        XCTAssertEqual(frame.maxY, visibleFrame.maxY)
    }

    /// The `cursorHeight: 16` constant azooKey-Desktop needs because it is handed
    /// a caret POINT is not carried over — a taller line must push the bar
    /// further down, which a fixed offset could not do.
    func testBarPosition_followsTheCaretLineHeight() {
        let short = CandidatePanelPositioning.frame(
            anchoredTo: caretRect(height: 10),
            panelSize: panelSize,
            within: visibleFrame,
        )
        let tall = CandidatePanelPositioning.frame(
            anchoredTo: caretRect(y: visibleFrame.minY + 10, height: 60),
            panelSize: panelSize,
            within: visibleFrame,
        )
        let shortFlipped = CandidatePanelPositioning.frame(
            anchoredTo: caretRect(y: visibleFrame.minY + 10, height: 10),
            panelSize: panelSize,
            within: visibleFrame,
        )

        XCTAssertEqual(short.maxY, caretRect(height: 10).minY - 4, "the gap is measured from the line's bottom")
        XCTAssertGreaterThan(
            tall.minY,
            shortFlipped.minY,
            "flipped above, a taller line pushes the bar further up — a fixed cursor height could not",
        )
    }

    /// A screen whose origin is not zero — a second display to the left of, or
    /// below, the main one. Clamps written against sizes rather than edges pass
    /// the cases above and place the bar on the wrong display here.
    func testBar_clampsAgainstTheScreenItIsOn_notAgainstTheOrigin() {
        let secondDisplay = CGRect(x: -1600, y: -400, width: 1600, height: 900)
        let caret = CGRect(x: -100, y: -380, width: 1, height: 20)

        let frame = CandidatePanelPositioning.frame(
            anchoredTo: caret,
            panelSize: panelSize,
            within: secondDisplay,
        )

        XCTAssertGreaterThanOrEqual(frame.minX, secondDisplay.minX)
        XCTAssertLessThanOrEqual(frame.maxX, secondDisplay.maxX)
        XCTAssertGreaterThanOrEqual(frame.minY, secondDisplay.minY)
        XCTAssertLessThanOrEqual(frame.maxY, secondDisplay.maxY)
    }
}

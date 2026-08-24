// The solo-Shift-tap state machine, walked as a table of event sequences.

@testable import TaigiInputMethodCore
import XCTest

/// The detector alone — timing, cancellation, and both Shift keys. The
/// controller's use of a completed tap is covered by
/// `ShiftAlphanumericControllerTests`.
final class ShiftTapDetectorTests: XCTestCase {
    private static let leftShift: UInt16 = 56
    private static let rightShift: UInt16 = 60

    private var detector = ShiftTapDetector()

    private func down(_ keyCode: UInt16 = leftShift, others: Bool = false, at time: TimeInterval) -> Bool {
        detector.observeFlagsChanged(
            keyCode: keyCode, shiftIsDown: true, otherModifiersDown: others, at: time,
        )
    }

    private func up(_ keyCode: UInt16 = leftShift, others: Bool = false, at time: TimeInterval) -> Bool {
        detector.observeFlagsChanged(
            keyCode: keyCode, shiftIsDown: false, otherModifiersDown: others, at: time,
        )
    }

    func testAQuickSoloTap_fires() {
        XCTAssertFalse(down(at: 0))
        XCTAssertTrue(up(at: 0.1))
    }

    func testEitherShiftKey_fires() {
        XCTAssertFalse(down(Self.rightShift, at: 0))
        XCTAssertTrue(up(Self.rightShift, at: 0.1))
    }

    func testAHoldPastTheTapWindow_neverFires() {
        _ = down(at: 0)
        XCTAssertFalse(up(at: ShiftTapDetector.maximumTapDuration + 0.01))
    }

    func testTypingACapital_neverFires() {
        // trace: Shift down primes → letter keyDown cancels → Shift up decides
        // nothing. This is every ⇧A, ⇧↩ and ⇧⇥ the user types.
        _ = down(at: 0)
        detector.noteKeyDown()
        XCTAssertFalse(up(at: 0.1))
    }

    func testAChordedShift_neverFires() {
        // ⌘ held while Shift goes down: the down never primes.
        _ = down(others: true, at: 0)
        XCTAssertFalse(up(at: 0.1))
    }

    func testAModifierJoiningMidTap_cancels() {
        // Shift primes, then ⌘ moves — its flagsChanged carries another key
        // code, which cancels — so the Shift release decides nothing.
        _ = down(at: 0)
        _ = detector.observeFlagsChanged(
            keyCode: 55, shiftIsDown: true, otherModifiersDown: true, at: 0.05,
        )
        XCTAssertFalse(up(at: 0.1))
    }

    func testMismatchedShiftKeys_neverFire() {
        // Left down, right up — two Shifts held is not a tap of either.
        _ = down(Self.leftShift, at: 0)
        _ = down(Self.rightShift, at: 0.05)
        XCTAssertFalse(up(Self.rightShift, at: 0.1))
        XCTAssertFalse(up(Self.leftShift, at: 0.15))
    }

    func testAPhantomRepeatInsideTheCooldown_firesOnce() {
        // Electron hosts can replay a release as a whole down-up cycle; the
        // cooldown makes one physical tap decide once.
        _ = down(at: 0)
        XCTAssertTrue(up(at: 0.1))
        _ = down(at: 0.15)
        XCTAssertFalse(up(at: 0.2))
    }

    func testASecondTapAfterTheCooldown_firesAgain() {
        _ = down(at: 0)
        XCTAssertTrue(up(at: 0.1))
        let later = 0.1 + ShiftTapDetector.retriggerGuardInterval + 0.01
        _ = down(at: later)
        XCTAssertTrue(up(at: later + 0.1))
    }

    func testCancelPriming_forgetsAHalfSeenTap() {
        // The session-boundary reset: a Shift pressed in one session must not
        // decide in the next.
        _ = down(at: 0)
        detector.cancelPriming()
        XCTAssertFalse(up(at: 0.1))
    }
}

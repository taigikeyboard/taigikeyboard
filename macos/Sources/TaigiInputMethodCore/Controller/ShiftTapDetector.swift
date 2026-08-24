// Recognizes a Shift key tapped on its own — the 英數 mode toggle gesture.

import AppKit

/// The solo-Shift-tap state machine, as pure decisions: primed by a Shift
/// going down alone, decided by the same Shift coming back up with nothing
/// typed in between, cancelled by anything else.
///
/// The design follows vChewing's `ShiftKeyUpChecker`
/// (`references/vChewing-macOS/Packages/vChewing_ModifierKeyHitChecker/Sources/ModifierKeyHitChecker/ShiftKeyUpChecker.swift`):
/// trust only the Shift state transitions, cancel on any real key-down, refuse
/// a hold past the tap threshold, and hold a cooldown so one physical release
/// reported twice — Electron hosts do this — cannot read as two taps.
///
/// A value type owned by the controller, with the clock passed in per call:
/// what fires and what does not is then a table a test can walk without
/// waiting on real time.
struct ShiftTapDetector {
    /// Longer than a deliberate tap needs, shorter than "held Shift while
    /// thinking": a press outliving this is a hold, not a toggle. vChewing
    /// ships tighter timing; the slack here errs toward the tap registering,
    /// because a missed toggle reads as the input method ignoring the user.
    static let maximumTapDuration: TimeInterval = 0.5

    /// One physical release must decide once, however many times the host
    /// reports it.
    static let retriggerGuardInterval: TimeInterval = 0.25

    private static let leftShiftKeyCode: UInt16 = 56
    private static let rightShiftKeyCode: UInt16 = 60

    private var primedKeyCode: UInt16?
    private var primedAt: TimeInterval?
    private var lastFiredAt: TimeInterval?

    /// A real key went down. Whatever Shift was priming, the user is typing —
    /// a capital, a chord, a candidate key — and the release to come is the
    /// end of that keystroke, not a tap.
    mutating func noteKeyDown() {
        cancelPriming()
    }

    /// Observes one `flagsChanged` event and answers whether a solo Shift tap
    /// just completed.
    ///
    /// `otherModifiersDown` is every device-independent modifier except Shift:
    /// any of them held makes this a chord in progress, which neither primes
    /// nor fires.
    mutating func observeFlagsChanged(
        keyCode: UInt16,
        shiftIsDown: Bool,
        otherModifiersDown: Bool,
        at now: TimeInterval,
    ) -> Bool {
        guard keyCode == Self.leftShiftKeyCode || keyCode == Self.rightShiftKeyCode else {
            // Some other modifier moved — ⌘ pressed or released mid-tap makes
            // this a chord, whichever order the keys go down in.
            cancelPriming()
            return false
        }
        guard !otherModifiersDown else {
            cancelPriming()
            return false
        }

        if shiftIsDown {
            // A second Shift going down while one is primed is two Shifts
            // held, not a tap of either.
            guard primedKeyCode == nil else {
                cancelPriming()
                return false
            }
            // The cooldown: a release the host reports twice arrives as a
            // whole phantom down-up cycle, so the guard has to sit on the
            // priming side.
            if let lastFiredAt, now - lastFiredAt < Self.retriggerGuardInterval {
                return false
            }
            primedKeyCode = keyCode
            primedAt = now
            return false
        }

        // Shift came up. It decides only when it is the very Shift that
        // primed, within the tap window.
        defer { cancelPriming() }
        guard primedKeyCode == keyCode,
              let primedAt,
              now - primedAt <= Self.maximumTapDuration
        else { return false }
        lastFiredAt = now
        return true
    }

    /// Forgets any half-seen tap. Called on session boundaries as well as by
    /// the rules above: a Shift pressed in one session must not decide in the
    /// next.
    mutating func cancelPriming() {
        primedKeyCode = nil
        primedAt = nil
    }
}

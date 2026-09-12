// The hairline between vertical rows, vibrancy-aware.

import AppKit

/// The one-point separator the vertical layout draws between rows, from
/// MacishType's `MacishSeparatorView` (`references/MacishType/macos/MacishType/
/// MacishCandidateWindow/MacishSeparatorView.swift`; MIT, © 2026 Luke Chang).
/// A drawn view rather than an `NSBox` so it can opt into vibrancy — a plain
/// separator colour reads as a solid stripe on the translucent backdrop.
final class CandidateSeparatorView: NSView {
    /// Tahoe pulls the hairline in from both ends so it does not touch the
    /// window's rounded edge; Sequoia runs it full width.
    var horizontalInset: CGFloat = 0 {
        didSet {
            if horizontalInset != oldValue {
                needsDisplay = true
            }
        }
    }

    override init(frame: NSRect) {
        super.init(frame: frame)
        translatesAutoresizingMaskIntoConstraints = true
    }

    @available(*, unavailable)
    required init?(coder _: NSCoder) {
        fatalError()
    }

    override var allowsVibrancy: Bool {
        true
    }

    override func draw(_: NSRect) {
        NSColor.separatorColor.setFill()
        bounds.insetBy(dx: horizontalInset, dy: 0).fill()
    }
}

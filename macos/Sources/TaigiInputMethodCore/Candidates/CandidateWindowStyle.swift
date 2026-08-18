// Which chrome generation the candidate window renders: Sequoia or Tahoe.

import Foundation

/// The two visual generations of the native macOS candidate window, as
/// MacishType names them (`references/MacishType/macos/MacishType/
/// CandidateWindow.swift:160-172`; MIT, © 2026 Luke Chang).
///
/// - `sequoia`: `NSVisualEffectView` vibrancy, 6pt corners, opaque row
///   highlight.
/// - `tahoe`: `NSGlassEffectView` glass, capsule corners, inset pill
///   highlight — the macOS 26 look.
enum CandidateWindowStyle: String, CaseIterable, Sendable {
    case sequoia
    case tahoe

    /// The style the running OS renders natively. Tahoe needs
    /// `NSGlassEffectView`, which does not exist before macOS 26, so this is
    /// also the ceiling a forced choice is clamped to.
    static var systemResolved: CandidateWindowStyle {
        if #available(macOS 26, *) {
            return .tahoe
        }
        return .sequoia
    }
}

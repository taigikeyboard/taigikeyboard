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

/// What the user ASKS for, one value wider than what can be drawn: `auto`
/// follows the OS. Stored raw like `CandidateLayout`.
enum CandidateWindowStyleChoice: String, CaseIterable, Sendable {
    case auto
    case sequoia
    case tahoe

    /// The style a window can actually build for this choice. A forced Tahoe
    /// on macOS 14–15 has no `NSGlassEffectView` to construct, so it resolves
    /// to Sequoia rather than crashing or shipping a broken half-look — and
    /// the resolution happens HERE, before a panel exists, so the backdrop,
    /// the cells and the corners can never disagree about which style they
    /// are.
    var resolved: CandidateWindowStyle {
        switch self {
        case .auto:
            return CandidateWindowStyle.systemResolved
        case .sequoia:
            return .sequoia
        case .tahoe:
            return CandidateWindowStyle.systemResolved == .tahoe ? .tahoe : .sequoia
        }
    }
}

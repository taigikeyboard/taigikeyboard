// Which chrome generation the candidate window renders: Sequoia or Tahoe.

import Foundation

/// The two visual generations of the native macOS candidate window, as
/// MacishType names them (`references/MacishType/macos/MacishType/
/// CandidateWindow.swift:160-172`; MIT, © 2026 Luke Chang). The rendering
/// subviews dispatch on this for backdrop material, corner radii, highlight
/// shape and separator geometry.
///
/// - `sequoia`: `NSVisualEffectView` vibrancy, 6pt corners, opaque row
///   highlight.
/// - `tahoe`: `NSGlassEffectView` glass, an inset concentric highlight, and
///   corners the cell arrangement picks — a capsule around one-line cells, a
///   fixed rounded rectangle around two-line ones
///   (`CandidateMetrics.tahoeContainerCornerRadius`) — the macOS 26 look.
///
/// The running OS picks it; there is no setting of our own, and this is the
/// canonical statement of why. A picker for it could not be symmetric: below
/// macOS 26 there is no `NSGlassEffectView` to raise a window to Tahoe, so the
/// only override it could honour anywhere was dropping a macOS 26 window back
/// to Sequoia. The 外觀 pane's accent-colour swatch was retired on its own
/// reasoning — see `CandidateAccentColor`.
enum CandidateWindowStyle: Sendable {
    case sequoia
    case tahoe

    /// The style the running OS renders natively: Tahoe from macOS 26, where
    /// `NSGlassEffectView` exists, and Sequoia below it.
    static var systemStyle: CandidateWindowStyle {
        if #available(macOS 26, *) {
            return .tahoe
        }
        return .sequoia
    }
}

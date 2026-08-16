// Which display a screen-space point is on. The AppKit half of positioning.

import AppKit

/// Finds the screen a caret is on.
///
/// Kept apart from `CandidatePanelPositioning` because it touches `NSScreen`,
/// which cannot be constructed in a test — the arithmetic that decides where the
/// bar lands stays pure and testable, and this is the one call that has to ask
/// the system.
enum ScreenLookup {
    /// Containment is tested against `NSScreen.frame`, the whole display, while
    /// callers clamp against `visibleFrame`. Separating them is azooKey-Desktop's
    /// fix for the stale-screen bug (`ScreenLookup.swift:35-52`): a caret sitting
    /// under the menu bar or over the Dock is on that display, but is outside its
    /// `visibleFrame`, so testing containment with the smaller rect drops the bar
    /// onto whichever screen happens to answer next.
    ///
    /// The nearest-screen fallback azooKey adds is not carried over: it ranks by
    /// distance between centres, which picks the wrong display whenever two
    /// displays differ in size. A point on no display at all falls back to the
    /// main one.
    static func screen(containing point: CGPoint) -> NSScreen? {
        NSScreen.screens.first { NSMouseInRect(point, $0.frame, false) } ?? NSScreen.main
    }
}

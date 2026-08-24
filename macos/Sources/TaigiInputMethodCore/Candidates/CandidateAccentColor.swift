// Resolves the highlight colour the candidate window paints selections with.

import AppKit

/// Where the candidate window's highlight colour comes from, and when it
/// changes.
///
/// Ported from MacishType's `ThemeManager` + the colour adjustments in its
/// `MacishBasePanel.syncTheme` (`references/MacishType/macos/MacishType/
/// ThemeManager.swift`, `MacishCandidateWindow/MacishBasePanel.swift:129-177`;
/// MIT, © 2026 Luke Chang). The native candidate window follows the accent
/// colour the user picked in System Settings — all eight of them — and, when
/// the accent is Multicolour, takes the frontmost app's own accent. Matching
/// that is what makes this window read as part of the system.
///
/// The client's appearance is NOT read: upstream does it through the
/// undocumented `windowEffectiveAppearance` selector, which the Codex pre-impl
/// rejected. The panel follows the system appearance, which AppKit gives every
/// window for free.
@MainActor
final class CandidateAccentColor {
    static let shared = CandidateAccentColor()

    /// Posted when the system appearance or the accent colour changes —
    /// panels re-resolve their highlight colour on it.
    static let didChange = Notification.Name("CandidateAccentColorDidChange")

    /// `AppleAccentColor` is absent from defaults exactly when the user picked
    /// Multicolour, which is also the system default. Present values are the
    /// eight fixed accents, which `NSColor.controlAccentColor` already
    /// resolves — the raw value only matters as presence.
    private static let accentColorKey = "AppleAccentColor"

    private(set) var isMulticolor: Bool
    /// `.some(nil)` caches "this bundle has no accent" so a host without one
    /// is not re-resolved on every keystroke. `reresolve()` empties it;
    /// otherwise an entry lives as long as the process does.
    private var bundleAccentColorCache: [String: NSColor?] = [:]
    private var appearanceObservation: NSKeyValueObservation?

    private init() {
        isMulticolor = UserDefaults.standard.object(forKey: Self.accentColorKey) == nil
        // The accent colour has no notification of its own, but changing it in
        // System Settings re-resolves every dynamic colour and touches the
        // effective appearance, so observing the appearance catches both.
        // Posted unguarded: re-resolving a handful of cell colours is cheaper
        // than being right about which of the two inputs changed.
        appearanceObservation = NSApp.observe(\.effectiveAppearance) { [weak self] _, _ in
            Task { @MainActor [weak self] in
                self?.reresolve()
            }
        }
    }

    private func reresolve() {
        isMulticolor = UserDefaults.standard.object(forKey: Self.accentColorKey) == nil
        // Flushed here rather than left to live as long as the process does.
        // A host's accent comes from its Info.plist, so it changes when that
        // app is replaced by a new version, and there is no direct signal for
        // an app being updated — launch and terminate notifications describe
        // process lifecycle, not bundle contents. Dropping the answers at the
        // one moment every colour is already being re-resolved picks an
        // updated host up at the user's next appearance or accent change
        // instead of only at the next restart of the input method, and costs
        // one re-read per host seen again after the flush.
        bundleAccentColorCache.removeAll()
        NotificationCenter.default.post(name: Self.didChange, object: nil)
    }

    /// The colour the selection highlight paints with, for a window rendered
    /// in `style` over the app identified by `hostBundleIdentifier`.
    ///
    /// The system is the only source: there is no per-app override to pin a
    /// colour, because one would be a FIXED colour and so would lose both the
    /// light/dark resolution and the frontmost-app adaptation this resolves.
    /// `appearance` is the panel's effective appearance: the Tahoe
    /// luminance clamp resolves dynamic colours under it, so a light-mode
    /// yellow and a dark-mode yellow darken to different pills, as the
    /// system's do.
    func highlightColor(
        style: CandidateWindowStyle,
        hostBundleIdentifier: String?,
        appearance: NSAppearance,
    ) -> NSColor {
        if isMulticolor,
           let bundleIdentifier = hostBundleIdentifier,
           let hostAccent = bundleAccentColor(bundleIdentifier: bundleIdentifier)
        {
            return switch style {
            case .sequoia: Self.sequoiaAdjusted(hostAccent)
            case .tahoe: Self.tahoeAdjusted(hostAccent, under: appearance)
            }
        }
        return switch style {
        // `selectedContentBackgroundColor` already carries the fixed accent
        // (and Multicolour's blue fallback) at the exact shade the system
        // candidate window uses, so no adjustment is applied.
        case .sequoia: .selectedContentBackgroundColor
        case .tahoe: Self.tahoeAdjusted(.controlAccentColor, under: appearance)
        }
    }

    /// Best-effort: an app states its accent as `NSAccentColorName` in
    /// Info.plist naming a colour asset in its own bundle. Apps without one —
    /// and fields hosted out of process — resolve to nil, and the caller falls
    /// back to the system accent.
    private func bundleAccentColor(bundleIdentifier: String) -> NSColor? {
        if let cached = bundleAccentColorCache[bundleIdentifier] {
            return cached
        }
        guard let bundleURL = NSWorkspace.shared
            .urlForApplication(withBundleIdentifier: bundleIdentifier),
            let hostBundle = Bundle(url: bundleURL),
            let colorName = hostBundle.object(forInfoDictionaryKey: "NSAccentColorName") as? String,
            let color = NSColor(named: colorName, bundle: hostBundle)
        else {
            bundleAccentColorCache[bundleIdentifier] = .some(nil)
            return nil
        }
        bundleAccentColorCache[bundleIdentifier] = color
        return color
    }

    /// Sequoia darkens a host accent slightly (upstream's measured affine fit
    /// of what the system window does, `MacishBasePanel.swift:129-137`) —
    /// `controlAccentColor` renders brighter than the highlight the native
    /// window paints with it.
    private static func sequoiaAdjusted(_ color: NSColor) -> NSColor {
        guard let srgb = color.usingColorSpace(.sRGB) else { return color }
        return NSColor(
            srgbRed: max(0, srgb.redComponent * 0.9417 - 0.0594),
            green: max(0, srgb.greenComponent * 0.9417 - 0.0594),
            blue: max(0, srgb.blueComponent * 0.9417 - 0.0594),
            alpha: srgb.alphaComponent,
        )
    }

    /// Tahoe clamps bright accents down so white selection text stays legible
    /// on the glass pill — upstream's measured curve over the colour's
    /// luminance (`MacishBasePanel.swift:139-155`). Colours already darker
    /// than the threshold pass through.
    private static func tahoeAdjusted(_ color: NSColor, under appearance: NSAppearance) -> NSColor {
        var resolved: NSColor?
        appearance.performAsCurrentDrawingAppearance {
            guard let srgb = color.usingColorSpace(.sRGB),
                  let gray = color.usingColorSpace(.genericGamma22Gray) else { return }
            let luminance = gray.whiteComponent
            guard luminance > 0.599 else {
                resolved = srgb
                return
            }
            let ratio = max(0.4726, 1 - 7.2 * pow(luminance - 0.599, 2.3))
            resolved = NSColor(
                srgbRed: srgb.redComponent * ratio,
                green: srgb.greenComponent * ratio,
                blue: srgb.blueComponent * ratio,
                alpha: srgb.alphaComponent,
            )
        }
        return resolved ?? color
    }
}

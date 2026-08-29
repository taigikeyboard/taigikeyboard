// Whether this app's windows render light, dark, or as the system does.

import AppKit

/// The app's light/dark choice, mirroring the 外觀 row of System Settings'
/// Appearance pane: 淺色, 深色, or 自動 — follow the system, which is what
/// every window did before this setting existed and what a fresh install
/// keeps.
///
/// Named for the app rather than for the candidate window, and filed beside
/// the settings that own it: it began as the candidate window's own choice and
/// now drives the settings window too (USER 2026-08-24). The persisted key
/// keeps its original spelling, `candidateAppearanceMode`, so no install has
/// to migrate.
///
/// `String` raw values persist through `UserDefaults` / `@AppStorage` like the
/// other window settings; unknown stored values read as `auto`.
enum AppearanceMode: String, CaseIterable, Sendable {
    case auto
    case light
    case dark

    /// The appearance to force onto a window, or nil for `auto` — an
    /// `NSWindow` whose `appearance` is nil resolves against the system, which
    /// is exactly the follow behaviour.
    var forcedAppearance: NSAppearance? {
        switch self {
        case .auto: nil
        case .light: NSAppearance(named: .aqua)
        case .dark: NSAppearance(named: .darkAqua)
        }
    }

    /// What the thumbnail's caption, tooltip and accessibility label say.
    var labelKey: StringKey {
        switch self {
        case .auto: .settingsDisplayLanguageAutomatic
        case .light: .desktopCandidateAppearanceLight
        case .dark: .desktopCandidateAppearanceDark
        }
    }
}

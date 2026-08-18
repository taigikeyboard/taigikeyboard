// Whether the candidate window renders light, dark, or as the system does.

import AppKit

/// The candidate window's light/dark choice, mirroring the 外觀 row of System
/// Settings' Appearance pane: 淺色, 深色, or 自動 — follow the system, which
/// is what the window did before this setting existed and what a fresh
/// install keeps.
///
/// `String` raw values persist through `UserDefaults` / `@AppStorage` like the
/// other candidate-window settings; unknown stored values read as `auto`.
enum CandidateAppearanceMode: String, CaseIterable, Sendable {
    case auto
    case light
    case dark

    /// The appearance to force onto the panel, or nil for `auto` — an
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
        case .light: .macosCandidateAppearanceLight
        case .dark: .macosCandidateAppearanceDark
        }
    }
}

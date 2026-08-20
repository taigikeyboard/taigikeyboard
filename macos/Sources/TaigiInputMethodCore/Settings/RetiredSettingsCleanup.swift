// Launch-time removal of persisted state for settings retired from the macOS UI.

import Foundation
import KeyboardShortcuts

/// Clears what older builds persisted for settings this build no longer
/// exposes (the 2026-08-21 trim: the 括號標注 and 顯示羅馬字候選 toggles, their
/// hotkeys, and the 揣辭典 pane), so an upgraded install behaves like a fresh
/// one. Idempotent, and run every launch rather than behind a version flag:
/// removing an absent key is free, and re-clearing also catches a manual
/// `defaults write` that would otherwise resurrect hidden state.
///
/// The engine still reads the two settings keys — they are cross-platform
/// contract, and the composing carrier encodes them either way — so a stored
/// `true` from a build that HAD the toggles would silently keep the feature on
/// with no UI left to turn it off. Removing the stored values returns both to
/// their `false` defaults.
@MainActor
enum RetiredSettingsCleanup {
    /// Raw `KeyboardShortcuts.Name`s of the retired hotkey actions, kept so a
    /// stored chord can be cleared: the library persists chords by raw name,
    /// an orphaned chord would spring back to life on any future action that
    /// reused the name, and `ShortcutConflicts` only sees the live roster.
    private static let retiredShortcutNames = [
        KeyboardShortcuts.Name("toggleBothScripts"),
        KeyboardShortcuts.Name("toggleLiteralRomanCandidate"),
    ]

    /// The sidebar pane removed from `SettingsPane`. Cleared explicitly rather
    /// than trusting `@AppStorage` to shrug off an unknown raw value — that
    /// fallback is framework behaviour this codebase has not pinned.
    private static let retiredPaneRawValue = "dictionarySearch"

    static func run(userDefaults: UserDefaults = .standard) {
        userDefaults.removeObject(forKey: SettingsStore.Keys.isOutputBothScripts.name)
        userDefaults.removeObject(forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)
        if userDefaults.string(forKey: SettingsStore.Keys.selectedSettingsPane.name)
            == retiredPaneRawValue {
            userDefaults.removeObject(forKey: SettingsStore.Keys.selectedSettingsPane.name)
        }
        for name in retiredShortcutNames {
            KeyboardShortcuts.setShortcut(nil, for: name)
        }
    }
}

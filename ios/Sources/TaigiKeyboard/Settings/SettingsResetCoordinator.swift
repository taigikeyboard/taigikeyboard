import Foundation
import KeyboardKit

/// Resets settings and user-owned data stores.
///
/// Two surfaces, intentionally separate:
/// - `resetAll()` — pure settings reset (SharedSettings + KeyboardKit defaults).
/// - `resetAllUserData()` — destructive: wipes user frequency and next-word DBs.
///
/// `SharedSettings.resetToDefaults()` can't touch the KeyboardKit store
/// directly without importing KeyboardKit in the (soon-to-be engine-only)
/// settings module. The coordinator is the composition point.
enum SettingsResetCoordinator {
    private static let logger = DebugLogger(category: "SettingsResetCoordinator")

    /// Reset the three KeyboardKit-owned user defaults that SharedSettings
    /// does not own. Keep these in sync with `SettingsTab` "reset all" UX.
    static func resetKeyboardKitDefaults() {
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled")
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled")
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled")
    }

    /// Reset every setting to its default — SharedSettings + KeyboardKit.
    /// Does NOT touch user-owned databases; call `resetAllUserData()` for that.
    static func resetAll() {
        SharedSettings.shared.resetToDefaults()
        resetKeyboardKitDefaults()
    }

    /// Delete user-owned databases (frequency + next-word association).
    /// Each deletion is attempted independently; failures are logged, never thrown,
    /// so a partial failure still clears what it can.
    static func resetAllUserData() {
        do {
            try UserFrequencyService.deleteUserDatabase()
        } catch {
            logger.error("Failed to delete frequency database: \(error)")
        }
        do {
            try NextWordService.deleteUserDatabase()
        } catch {
            logger.error("Failed to delete association database: \(error)")
        }
    }
}

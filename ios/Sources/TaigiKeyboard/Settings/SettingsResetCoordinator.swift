import Foundation
import KeyboardKit

/// Resets every setting to its default (Settings tab "Reset Settings").
///
/// Settings only: learning records and the custom dictionary are user data, cleared from their own
/// Dictionary-tab pages, never from here.
///
/// `SharedSettings.resetToDefaults()` can't touch the KeyboardKit store
/// directly without importing KeyboardKit in the (soon-to-be engine-only)
/// settings module. The coordinator is the composition point.
enum SettingsResetCoordinator {
    /// Reset the three KeyboardKit-owned user defaults that SharedSettings
    /// does not own. Keep these in sync with `SettingsTab` "reset all" UX.
    static func resetKeyboardKitDefaults() {
        UserDefaults.keyboardSettings.set(true, forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled")
        UserDefaults.keyboardSettings.set(true, forKey: "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled")
        UserDefaults.keyboardSettings.set(true, forKey: "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled")
    }

    /// Reset every setting to its default — SharedSettings + KeyboardKit.
    static func resetAll() {
        SharedSettings.shared.resetToDefaults()
        resetKeyboardKitDefaults()
    }
}

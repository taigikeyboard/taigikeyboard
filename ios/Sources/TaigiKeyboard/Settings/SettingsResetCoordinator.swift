import Foundation
import KeyboardKit

/// Resets both app-owned settings (`SharedSettings`) and KeyboardKit-owned
/// settings (`KeyboardSettings.store`) in a single call.
///
/// `SharedSettings.resetToDefaults()` can't touch the KeyboardKit store
/// directly without importing KeyboardKit in the (soon-to-be engine-only)
/// settings module. The coordinator is the composition point.
enum SettingsResetCoordinator {
    /// Reset the three KeyboardKit-owned user defaults that SharedSettings
    /// does not own. Keep these in sync with `SettingsTab` "reset all" UX.
    static func resetKeyboardKitDefaults() {
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled")
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled")
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled")
    }

    /// Reset every setting to its default — SharedSettings + KeyboardKit.
    static func resetAll() {
        SharedSettings.shared.resetToDefaults()
        resetKeyboardKitDefaults()
    }
}

// Declares the KeyboardApp value that drives the standard KeyboardKit setup.

import KeyboardKit

extension KeyboardApp {
    /// Standard KeyboardKit setup entry (KK ≥ 10.8.1 initialization-ordering
    /// fix, upstream issue #967): passing `appGroupId` makes
    /// `setupKeyboardKit(for:)` configure the settings store with App Group
    /// syncing BEFORE any settings access, which is what lets
    /// `isAutocapitalizationEnabled = false` reach the keyboard-case logic.
    ///
    /// `keyboardSettingsKeyPrefix` stays nil on purpose: the default prefix
    /// `com.keyboardkit.settings.` (verified against the 10.9.0 binary) is
    /// what the host app hard-codes when writing KK-owned keys
    /// (`SharedSettings`, `SettingsResetCoordinator`,
    /// `SettingsSelectionOverlay`) — a custom prefix would strand existing
    /// users' persisted settings.
    static var taigiKeyboard: KeyboardApp {
        .init(name: "Taigi Keyboard", appGroupId: SharedSettings.appGroupId)
    }
}

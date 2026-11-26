// Two-line label for settings rows, matching System Settings and ShortcutSettingsView.

import SwiftUI

/// Renders a setting row's primary title and optional muted secondary caption as sibling `Text`
/// views inside a grouped `Form` (`Toggle`, `Picker`, `LabeledContent`), matching macOS's native
/// grouped-form caption typography and `.disabled(...)` control styling.
///
/// For pickers, `selectedOptionKey` lets the currently selected option's own caption (when present
/// in `i18n/*.json`) take precedence over the row's caption.
struct SettingsRowLabel: View {
    @Environment(DisplayLanguageStore.self) private var language

    let titleKey: StringKey
    var selectedOptionKey: StringKey?

    var body: some View {
        Text(language.string(titleKey))
        if let caption = Self.caption(titleKey: titleKey, selectedOptionKey: selectedOptionKey, in: language) {
            Text(caption)
        }
    }

    /// Resolves the active caption for a row, preferring the selected option's caption when it has one.
    @MainActor
    static func caption(
        titleKey: StringKey,
        selectedOptionKey: StringKey? = nil,
        in language: DisplayLanguageStore,
    ) -> String? {
        selectedOptionKey.flatMap(language.caption(for:)) ?? language.caption(for: titleKey)
    }
}

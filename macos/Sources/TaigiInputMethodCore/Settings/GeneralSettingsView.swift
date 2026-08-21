// The 一般 pane: romanization system and display language.

import SwiftUI

/// The 一般 pane of the settings window.
///
/// Bound with `@AppStorage` rather than through `SettingsStore`, so the form
/// re-renders when a value is changed from outside it — the input-source menu's
/// TL/POJ items and the shortcut hotkeys write straight to `UserDefaults`, and
/// so does anyone running `defaults write`. The keys and the fallbacks come
/// from the store's descriptors, so the form and the engine cannot disagree
/// about either.
///
/// The two learning switches are deliberately absent. They live on the 詞頻紀錄
/// and 詞關聯紀錄 panes, next to the data each one governs — a second copy here
/// was the same `@AppStorage` key drawn twice, which is a settings window
/// disagreeing with itself.
///
/// Text comes from the injected `DisplayLanguageStore`: reading it inside `body` is what makes the
/// form re-render when the display language changes, with no window rebuild.
struct GeneralSettingsView: View {
    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.inputMode.name)
    private var inputMode = SettingsStore.Keys.inputMode.defaultValue

    var body: some View {
        Form {
            Section {
                Picker(language.string(.macosRomanizationSystem), selection: $inputMode) {
                    Text(language.string(.settingsTlMode)).tag(InputMode.tl)
                    Text(language.string(.settingsPojMode)).tag(InputMode.poj)
                }
                .pickerStyle(.radioGroup)

                Picker(language.string(.settingsDisplayLanguage), selection: displayLanguageSelection) {
                    ForEach(DisplayLanguage.selectableLanguages, id: \.self) { option in
                        // Endonyms for the authored languages, so a user can find their own language
                        // whatever the UI currently reads in; `.system` is the one translated row.
                        Text(language.selectionLabel(for: option)).tag(option)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .frame(maxWidth: SettingsPaneLayout.maximumFormWidth)
    }

    /// The picker's selection, read and written through the store rather than through `@AppStorage`
    /// like the settings around it. The store is what the rest of this view resolves its text from,
    /// so binding past it would let an injected store and the picker disagree — and `setLanguage`
    /// updates the live state synchronously, where a defaults write would arrive an actor hop later.
    private var displayLanguageSelection: Binding<DisplayLanguage> {
        Binding(
            get: { language.selected },
            set: { language.setLanguage($0) },
        )
    }
}

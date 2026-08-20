// The 一般 pane: romanization system, display language, learning toggles, and
// the shortcut recorders.

import KeyboardShortcuts
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
/// Text comes from the injected `DisplayLanguageStore`: reading it inside `body` is what makes the
/// form re-render when the display language changes, with no window rebuild.
struct GeneralSettingsView: View {
    /// The form is a column of controls that reads best bounded — stretched
    /// across a wide detail pane, every row becomes a label staring at a
    /// far-away control. Centered within whatever width the pane has.
    private static let maximumFormWidth: CGFloat = 640

    @Environment(DisplayLanguageStore.self) private var language

    @AppStorage(SettingsStore.Keys.inputMode.name)
    private var inputMode = SettingsStore.Keys.inputMode.defaultValue

    @AppStorage(SettingsStore.Keys.isFrequencyRecordingEnabled.name)
    private var isFrequencyRecordingEnabled = SettingsStore.Keys.isFrequencyRecordingEnabled.defaultValue

    @AppStorage(SettingsStore.Keys.isAssociationRecordingEnabled.name)
    private var isAssociationRecordingEnabled = SettingsStore.Keys.isAssociationRecordingEnabled.defaultValue

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

            Section {
                Toggle(language.string(.dictionaryFrequencyRecordingEnabled), isOn: $isFrequencyRecordingEnabled)
                Toggle(language.string(.dictionaryAssociationRecordingEnabled), isOn: $isAssociationRecordingEnabled)
            } header: {
                Text(language.string(.macosLearningSection))
            }

            Section {
                // One row per action, off the same list the hotkey registration
                // uses, so a new action cannot appear in one and not the other.
                ForEach(ShortcutAction.allCases, id: \.self) { action in
                    KeyboardShortcuts.Recorder(action.label(language), name: action.name) { _ in
                        ShortcutConflicts.resolve(after: action)
                    }
                }
            } header: {
                Text(language.string(.macosShortcutsSection))
            }
        }
        .formStyle(.grouped)
        .frame(maxWidth: Self.maximumFormWidth)
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

import SwiftUI

/// Picker subpage for the `SettingsTab` display-language row. Lists the user-selectable languages —
/// `.system` (Automatic) first with its localized label, then each authored language by its endonym;
/// tapping one writes through `DisplayLanguageStore`, which live-switches the whole app with no restart.
/// The store IS the source of truth — there is no local selection state.
struct DisplayLanguagePickerView: View {
    @Environment(DisplayLanguageStore.self) private var lang

    var body: some View {
        Form {
            Section {
                ForEach(DisplayLanguage.selectableLanguages, id: \.self) { language in
                    Button {
                        lang.setLanguage(language)
                    } label: {
                        HStack {
                            Text(lang.selectionLabel(for: language))
                                .foregroundColor(.primary)
                            Spacer()
                            if lang.selected == language {
                                Image(latinSystemName: "checkmark")
                                    .foregroundColor(AppStyle.accentBlue)
                                    .accessibilityHidden(true)
                            }
                        }
                    }
                    .accessibilityAddTraits(lang.selected == language ? .isSelected : [])
                }
            }
        }
        .navigationTitle(lang.string(.settingsDisplayLanguage))
        .navigationBarTitleDisplayMode(.inline)
    }
}

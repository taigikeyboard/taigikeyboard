// 中文: SettingsTab 「顯示語言」row 點進去的子頁,列出可選的 App UI 顯示語言(自動 / 漢字 / English / 日本語 / Tâi-lô / Pe̍h-ōe-jī)。除「自動」用翻譯標籤外,其餘用各自的母語名稱(endonym)顯示。

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

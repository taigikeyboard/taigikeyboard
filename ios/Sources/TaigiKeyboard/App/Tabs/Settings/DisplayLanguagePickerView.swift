// 中文: SettingsTab 「顯示語言」row 點進去的子頁,列出可選的 App UI 顯示語言(漢字 / English / 日本語),用各自的母語名稱(endonym)顯示。

import SwiftUI

/// Picker subpage for the `SettingsTab` display-language row. Lists the authored, user-selectable
/// languages by their endonym; tapping one writes through `DisplayLanguageStore`, which live-switches
/// the whole app with no restart. The store IS the source of truth — there is no local selection state.
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
                            Text(language.endonym)
                                .foregroundColor(.primary)
                            Spacer()
                            if lang.language == language {
                                Image(latinSystemName: "checkmark")
                                    .foregroundColor(AppStyle.accentBlue)
                                    .accessibilityHidden(true)
                            }
                        }
                    }
                    .accessibilityAddTraits(lang.language == language ? .isSelected : [])
                }
            }
        }
        .navigationTitle(lang.string(.settingsDisplayLanguage))
        .navigationBarTitleDisplayMode(.inline)
    }
}

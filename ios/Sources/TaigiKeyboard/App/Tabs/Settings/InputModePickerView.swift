// 中文: SettingsTab 「輸入模式」row 點進去的子頁,列出全部 InputMode 供選擇。

import SwiftUI

/// Picker subpage for `SettingsTab` input-mode row.
// 中文: 輸入模式挑選子頁。透過 binding + onChange 立刻寫回 SettingsTab 的 ViewModel。
struct InputModePickerView: View {
    @Binding var selectedMode: InputMode
    var onChange: (InputMode) -> Void

    var body: some View {
        Form {
            Section {
                ForEach(InputMode.allCases, id: \.self) { mode in
                    Button {
                        selectedMode = mode
                        onChange(mode)
                    } label: {
                        HStack {
                            Text(mode.displayName)
                                .foregroundColor(.primary)
                            Spacer()
                            if selectedMode == mode {
                                Image(latinSystemName: "checkmark")
                                    .foregroundColor(AppStyle.accentBlue)
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(SettingsTexts.inputMode)
        .navigationBarTitleDisplayMode(.inline)
    }
}

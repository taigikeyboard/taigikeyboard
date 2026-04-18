import SwiftUI

/// Picker subpage for `SettingsTab` input-mode row.
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
                                Image(systemName: "checkmark")
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

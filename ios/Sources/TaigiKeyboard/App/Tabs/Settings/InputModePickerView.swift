import SwiftUI

/// Picker subpage for `SettingsTab` input-mode row.
struct InputModePickerView: View {
    @Environment(DisplayLanguageStore.self) private var lang
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
                            Text(lang.string(mode.displayNameKey))
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
        .navigationTitle(lang.string(.settingsInputMode))
        .navigationBarTitleDisplayMode(.inline)
    }
}

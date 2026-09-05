import SwiftUI

/// Reusable info button that shows a `questionmark.circle` icon and displays
/// an alert with the given description when tapped.
///
/// Style matches the existing dictionary info buttons in DictionaryTab.
struct SettingInfoButton: View {
    let description: String

    @Environment(DisplayLanguageStore.self) private var lang
    @State private var showAlert = false

    var body: some View {
        Button { showAlert = true } label: {
            Image(latinSystemName: "questionmark.circle")
                .foregroundColor(AppStyle.accentBlue)
        }
        .buttonStyle(.plain)
        .alert("", isPresented: $showAlert) {
            Button(lang.string(.commonOk), role: .cancel) {}
        } message: {
            Text(description)
        }
    }
}

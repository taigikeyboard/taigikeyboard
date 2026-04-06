import SwiftUI

/// Reusable info button that shows a `questionmark.circle` icon and displays
/// an alert with the given description when tapped.
///
/// Style matches the existing dictionary info buttons in Tab3.
struct SettingInfoButton: View {
    let description: String

    @State private var showAlert = false

    var body: some View {
        Button { showAlert = true } label: {
            Image(systemName: "questionmark.circle")
                .foregroundColor(AppStyle.accentBlue)
        }
        .buttonStyle(.plain)
        .alert("", isPresented: $showAlert) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(description)
        }
    }
}

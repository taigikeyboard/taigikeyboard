// 可重用的設定說明按鈕(問號圖示 + 點擊彈出 alert)。

import SwiftUI

/// Reusable info button that shows a `questionmark.circle` icon and displays
/// an alert with the given description when tapped.
///
/// Style matches the existing dictionary info buttons in DictionaryTab.
// 問號 info 按鈕,點擊顯示 description 內容的 alert。樣式對齊 DictionaryTab 的 info 按鈕。
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

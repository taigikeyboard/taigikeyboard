import SwiftUI

/// 版本紀錄頁面
///
/// 顯示 App 版本更新歷史。
struct VersionHistoryDetailView: View {
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        Form {
            ForEach(Tab1Texts.versionHistoryEntries.indices, id: \.self) { index in
                let entry = Tab1Texts.versionHistoryEntries[index]
                Section {
                    VStack(alignment: .leading, spacing: 12) {
                        // 版本號與日期
                        HStack {
                            Text("v\(entry.version)")
                                .font(KeyboardModels.Fonts.appFont(.headline))
                                .foregroundColor(.accentColor)

                            Spacer()

                            Text(entry.date)
                                .font(KeyboardModels.Fonts.appFont(.caption))
                                .foregroundColor(.secondary)
                        }

                        // 變更內容
                        VStack(alignment: .leading, spacing: 8) {
                            ForEach(entry.changes.indices, id: \.self) { changeIndex in
                                HStack(alignment: .top, spacing: 8) {
                                    Text("•")
                                        .foregroundColor(.secondary)
                                    Text(languageManager.text(entry.changes[changeIndex]))
                                }
                            }
                        }
                    }
                }
            }
        }
        .navigationTitle(languageManager.text(Tab1Texts.versionHistory))
        .navigationBarTitleDisplayMode(.large)
    }
}

import SwiftUI

/// 預計功能詳細頁面
///
/// 顯示預計新功能的詳細說明。
struct UpcomingDetailView: View {
    let upcoming: UpcomingType
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        Form {
            ForEach(upcoming.detailParagraphs.indices, id: \.self) { index in
                Section {
                    Text(languageManager.text(upcoming.detailParagraphs[index]))
                        .lineSpacing(6)
                }
            }
        }
        .navigationTitle(languageManager.text(upcoming.title))
        .navigationBarTitleDisplayMode(.large)
    }
}

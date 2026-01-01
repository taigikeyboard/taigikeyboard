import SwiftUI

/// 已知問題詳細頁面
///
/// 顯示處理中問題的詳細說明。
struct IssueDetailView: View {
    let issue: IssueType
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        Form {
            ForEach(issue.detailParagraphs.indices, id: \.self) { index in
                Section {
                    Text(languageManager.text(issue.detailParagraphs[index]))
                        .lineSpacing(6)
                }
            }
        }
        .navigationTitle(languageManager.text(issue.title))
        .navigationBarTitleDisplayMode(.large)
    }
}

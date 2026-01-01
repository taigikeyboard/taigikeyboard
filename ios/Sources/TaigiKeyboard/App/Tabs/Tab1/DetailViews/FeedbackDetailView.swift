import SwiftUI

/// 問題回報頁面
///
/// 提供問題回報方式和聯繫資訊。
struct FeedbackDetailView: View {
    @StateObject private var languageManager = LanguageManager.shared

    private let googleFormURL = "https://docs.google.com/forms/d/e/1FAIpQLSd7PEppQ9MdAptvoY-PaaXDlbbL9Gq9Y4lFjgU9sLz4ENiPoA/viewform?usp=header"
    private let supportURL = "https://portaly.cc/siansiansu/support"

    var body: some View {
        Form {
            Section {
                Text(languageManager.text(Tab1Texts.feedbackDescription))
                    .lineSpacing(6)
            }

            Section {
                Link(destination: URL(string: googleFormURL)!) {
                    Label(languageManager.text(Tab1Texts.goToGoogleForm), systemImage: "doc.text.fill")
                }
            }

            Section {
                Text(languageManager.text(Tab1Texts.emailContact))
                    .lineSpacing(6)
            }

            Section {
                Link(destination: URL(string: supportURL)!) {
                    Label(languageManager.text(Tab1Texts.supportUs), systemImage: "heart.fill")
                }
            }
        }
        .navigationTitle(languageManager.text(Tab1Texts.contactUs))
        .navigationBarTitleDisplayMode(.large)
    }
}

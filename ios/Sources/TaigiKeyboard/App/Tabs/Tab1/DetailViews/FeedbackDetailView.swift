import SwiftUI

/// 寄付頁面
///
/// 提供贊助資訊。
struct FeedbackDetailView: View {
    @StateObject private var languageManager = LanguageManager.shared

    private let supportURL = "https://p.ecpay.com.tw/AA663DE"

    var body: some View {
        Form {
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

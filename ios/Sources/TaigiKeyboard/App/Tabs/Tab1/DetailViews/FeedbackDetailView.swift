import SwiftUI

/// Donation / feedback page with support link.
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
                    Label(languageManager.text(Tab1Texts.supportUs), systemImage: "arrow.up.right.square")
                }
            }

            Section {
                Text(languageManager.text(Tab1Texts.freePromise))
                    .lineSpacing(6)
            }
        }
        .navigationTitle(languageManager.text(Tab1Texts.contactUs))
        .navigationBarTitleDisplayMode(.large)
    }
}

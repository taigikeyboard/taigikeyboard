import SwiftUI

/// Donation / feedback page with support link.
struct FeedbackDetailView: View {
    private let supportURL = "https://p.ecpay.com.tw/AA663DE"

    var body: some View {
        Form {
            Section {
                Text(HomeTexts.emailContact)
                    .lineSpacing(6)
            }

            Section {
                Link(destination: URL(string: supportURL)!) {
                    Label(HomeTexts.supportUs, systemImage: "arrow.up.right.square")
                }
            }

            Section {
                Text(HomeTexts.freePromise)
                    .lineSpacing(6)
            }
        }
        .navigationTitle(HomeTexts.contactUs)
        .navigationBarTitleDisplayMode(.large)
    }
}

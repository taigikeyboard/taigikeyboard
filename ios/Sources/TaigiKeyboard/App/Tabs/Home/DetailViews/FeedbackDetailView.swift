import SwiftUI

/// Donation / feedback page with support link.
struct FeedbackDetailView: View {

    private let supportURL = "https://p.ecpay.com.tw/AA663DE"

    var body: some View {
        Form {
            Section {
                Text(Tab1Texts.emailContact)
                    .lineSpacing(6)
            }

            Section {
                Link(destination: URL(string: supportURL)!) {
                    Label(Tab1Texts.supportUs, systemImage: "arrow.up.right.square")
                }
            }

            Section {
                Text(Tab1Texts.freePromise)
                    .lineSpacing(6)
            }
        }
        .navigationTitle(Tab1Texts.contactUs)
        .navigationBarTitleDisplayMode(.large)
    }
}

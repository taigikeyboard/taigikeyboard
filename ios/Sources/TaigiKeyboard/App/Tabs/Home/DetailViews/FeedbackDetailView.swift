// 中文: 聯絡 / 贊助頁,含 ECPay 贊助連結與 email 聯絡資訊。

import SwiftUI

/// Donation / feedback page with support link.
// 中文: 聯絡與贊助頁。supportURL 指向 ECPay 贊助頁。
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

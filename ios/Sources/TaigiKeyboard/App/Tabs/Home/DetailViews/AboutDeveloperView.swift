// 中文: 關於開發者頁,簡介個人開發背景並導向官方網站。

import SwiftUI

/// About-developer page with link to the official website.
// 中文: 關於開發者。連到官方網站取得進一步資訊。
struct AboutDeveloperView: View {
    private let websiteURL = "https://www.taigikeyboard.tw/"

    var body: some View {
        Form {
            Section {
                Text(HomeTexts.freePromise)
                    .lineSpacing(6)
            }

            Section {
                Link(destination: URL(string: websiteURL)!) {
                    Label(HomeTexts.officialWebsite, systemImage: "arrow.up.right.square")
                }
            }
        }
        .navigationTitle(HomeTexts.aboutDeveloper)
        .navigationBarTitleDisplayMode(.large)
    }
}

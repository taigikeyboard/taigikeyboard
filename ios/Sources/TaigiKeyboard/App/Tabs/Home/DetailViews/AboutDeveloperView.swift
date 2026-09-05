// 關於開發者頁,簡介個人開發背景並導向官方網站。

import SwiftUI

/// About-developer page with link to the official website.
// 關於開發者。連到官方網站取得進一步資訊。
struct AboutDeveloperView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    private let websiteURL = "https://www.taigikeyboard.tw/"

    var body: some View {
        Form {
            Section {
                Text(lang.string(.homeFreePromise))
                    .lineSpacing(6)
            }

            Section {
                Link(destination: URL(string: websiteURL)!) {
                    Label(lang.string(.commonViewWebsite), systemImage: "arrow.up.right.square")
                }
            }
        }
        .navigationTitle(lang.string(.homeAboutDeveloper))
        .navigationBarTitleDisplayMode(.large)
    }
}

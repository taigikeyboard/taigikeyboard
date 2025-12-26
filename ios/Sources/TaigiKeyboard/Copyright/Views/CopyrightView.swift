import SwiftUI

/// 版權聲明視圖（單頁滾動式，風格與 Tab1 一致）
/// 每個項目為獨立區塊，參考 azooKey Acknowledgements 設計
struct CopyrightView: View {
    @Environment(\.openURL) private var openURL
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        ScrollView {
            VStack(spacing: 32) {
                // 粉圓體
                CopyrightCard(
                    title: Tab1Texts.openFontTitle,
                    description: Tab1Texts.openFontCopyright,
                    license: Tab1Texts.silOpenFontLicense,
                    licenseURL: "https://openfontlicense.org/",
                    websiteURL: "https://justfont.com/huninn/"
                )

                // 芫荽體
                CopyrightCard(
                    title: Tab1Texts.iansuiFontTitle,
                    description: Tab1Texts.iansuiFontCopyright,
                    license: Tab1Texts.silOpenFontLicense11,
                    licenseURL: "https://openfontlicense.org/",
                    websiteURL: "https://github.com/ButTaiwan/iansui"
                )

                // 教育部臺灣閩南語常用詞辭典
                CopyrightCard(
                    title: Tab1Texts.moeDict,
                    description: Tab1Texts.moeCopyright,
                    license: Tab1Texts.ccLicense,
                    licenseURL: "https://creativecommons.org/licenses/by-nd/3.0/tw/",
                    websiteURL: "https://sutian.moe.edu.tw/"
                )

                // 新詞辭典
                CopyrightCard(
                    title: Tab1Texts.newwordDict,
                    description: Tab1Texts.newwordCopyright,
                    license: Tab1Texts.ccBy4License,
                    licenseURL: "https://creativecommons.org/licenses/by/4.0/deed.zh-hant",
                    websiteURL: "https://www.taigitv.org.tw/taigi-words"
                )

                // 工藝辭典
                CopyrightCard(
                    title: Tab1Texts.kunggeDict,
                    description: Tab1Texts.kunggeCopyright,
                    license: Tab1Texts.ccByNcLicense,
                    licenseURL: "https://creativecommons.org/licenses/by-nc/4.0/deed.zh-hant",
                    websiteURL: "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite"
                )

                // iTaigi
                CopyrightCard(
                    title: Tab1Texts.iTaigiDict,
                    description: Tab1Texts.iTaigiCopyright,
                    license: Tab1Texts.cc0License,
                    licenseURL: "https://creativecommons.org/public-domain/cc0/",
                    websiteURL: "https://itaigi.tw/"
                )

                // 台日大辭典
                CopyrightCard(
                    title: Tab1Texts.taiwanJapanDict,
                    description: Tab1Texts.taiwanJapanCopyright,
                    license: Tab1Texts.ccByNcSA3License,
                    licenseURL: "https://creativecommons.org/licenses/by-nc-sa/3.0/tw/",
                    websiteURL: "http://taigi.fhl.net/dict/"
                )

                // 台華辭典
                CopyrightCard(
                    title: Tab1Texts.taiHuaDict,
                    description: Tab1Texts.taiHuaCopyright,
                    license: Tab1Texts.ccBySA4License,
                    licenseURL: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW",
                    websiteURL: nil
                )

                // 台灣植物辭典
                CopyrightCard(
                    title: Tab1Texts.taiwanPlantDict,
                    description: Tab1Texts.taiwanPlantCopyright,
                    license: Tab1Texts.ccBySA4License,
                    licenseURL: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW",
                    websiteURL: "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106"
                )
            }
            .padding(.horizontal, 20)
            .padding(.top, 24)
            .padding(.bottom, 40)
        }
        .background(Color.Theme.surfacePrimary)
        .navigationTitle(languageManager.text(Tab1Texts.copyrightNotice))
        .navigationBarTitleDisplayMode(.large)
    }
}

// MARK: - Copyright Card

private struct CopyrightCard: View {
    let title: LocalizedText
    let description: LocalizedText
    let license: LocalizedText
    let licenseURL: String
    let websiteURL: String?

    @StateObject private var languageManager = LanguageManager.shared
    @Environment(\.openURL) private var openURL

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // 標題區塊
            VStack(alignment: .leading, spacing: 8) {
                // 標題（較大字體，參考 azooKey .font(.title)）
                LocalizedTextView(title)
                    .themeFontTitle()
                    .foregroundColor(Color.Theme.textPrimary)

                // 說明
                LocalizedTextView(description)
                    .themeFontBody()
                    .foregroundColor(Color.Theme.textSecondary)

                // 授權
                LocalizedTextView(license)
                    .themeFontCaption()
                    .foregroundColor(Color.Theme.textSecondary)
                    .italic()
            }
            .padding(20)

            Divider()
                .background(Color.Theme.cardStroke)

            // 授權條款連結
            Button(action: {
                if let url = URL(string: licenseURL) {
                    openURL(url)
                }
            }) {
                HStack {
                    Text(languageManager.text(Tab1Texts.viewLicense))
                        .themeFontBody()
                        .foregroundColor(Color.Theme.textPrimary)
                    Spacer()
                    Image(systemName: "arrow.up.right")
                        .font(.system(size: 14))
                        .foregroundColor(Color.Theme.textSecondary)
                }
                .padding(.horizontal, 20)
                .padding(.vertical, 14)
            }

            // 官方網站連結（有分隔線）
            if let websiteURL = websiteURL, let url = URL(string: websiteURL) {
                Divider()
                    .background(Color.Theme.cardStroke)
                    .padding(.leading, 20)

                Button(action: {
                    openURL(url)
                }) {
                    HStack {
                        Text(languageManager.text(Tab1Texts.viewWebsite))
                            .themeFontBody()
                            .foregroundColor(Color.Theme.textPrimary)
                        Spacer()
                        Image(systemName: "arrow.up.right")
                            .font(.system(size: 14))
                            .foregroundColor(Color.Theme.textSecondary)
                    }
                    .padding(.horizontal, 20)
                    .padding(.vertical, 14)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(
            RoundedRectangle(cornerRadius: 12)
                .fill(Color.Theme.surfaceSecondary)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(Color.Theme.cardStroke, lineWidth: 1)
        )
    }
}

#Preview {
    NavigationStack {
        CopyrightView()
    }
    .withLanguageEnvironment()
}

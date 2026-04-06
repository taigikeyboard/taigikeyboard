import SwiftUI

/// Copyright notices for dictionaries and open-source projects.
struct CopyrightView: View {
    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        Form {
            // Open Huninn (粉圓體)
            CopyrightSection(
                title: Tab1Texts.openFontTitle,
                description: Tab1Texts.openFontCopyright,
                license: Tab1Texts.silOpenFontLicense,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://justfont.com/huninn/",
            )

            // Iansui (芫荽體)
            CopyrightSection(
                title: Tab1Texts.iansuiFontTitle,
                description: Tab1Texts.iansuiFontCopyright,
                license: Tab1Texts.silOpenFontLicense11,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://github.com/ButTaiwan/iansui",
            )

            // MOE Taiwanese Dictionary (教育部臺灣台語常用詞辭典)
            CopyrightSection(
                title: Tab1Texts.moeDict,
                description: Tab1Texts.moeCopyright,
                license: Tab1Texts.ccLicense,
                licenseURL: "https://creativecommons.org/licenses/by-nd/3.0/tw/",
                websiteURL: "https://sutian.moe.edu.tw/",
            )

            // New Words Dictionary (新詞新語)
            CopyrightSection(
                title: Tab1Texts.newwordDict,
                description: Tab1Texts.newwordCopyright,
                license: Tab1Texts.ccBy4License,
                licenseURL: "https://creativecommons.org/licenses/by/4.0/deed.zh-hant",
                websiteURL: "https://www.taigitv.org.tw/taigi-words",
            )

            // Craft Dictionary (工藝詞庫)
            CopyrightSection(
                title: Tab1Texts.kunggeDict,
                description: Tab1Texts.kunggeCopyright,
                license: Tab1Texts.ccByNcLicense,
                licenseURL: "https://creativecommons.org/licenses/by-nc/4.0/deed.zh-hant",
                websiteURL: "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite",
            )

            // iTaigi
            CopyrightSection(
                title: Tab1Texts.iTaigiDict,
                description: Tab1Texts.iTaigiCopyright,
                license: Tab1Texts.cc0License,
                licenseURL: "https://creativecommons.org/public-domain/cc0/",
                websiteURL: "https://itaigi.tw/",
            )

            // Taiwan-Japan Dictionary (台日大辭典)
            CopyrightSection(
                title: Tab1Texts.taiwanJapanDict,
                description: Tab1Texts.taiwanJapanCopyright,
                license: Tab1Texts.ccByNcSA3License,
                licenseURL: "https://creativecommons.org/licenses/by-nc-sa/3.0/tw/",
                websiteURL: "http://taigi.fhl.net/dict/",
            )

            // Tai-Hua Dictionary (台華線頂辭典)
            CopyrightSection(
                title: Tab1Texts.taiHuaDict,
                description: Tab1Texts.taiHuaCopyright,
                license: Tab1Texts.ccBySA4License,
                licenseURL: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW",
                websiteURL: nil,
            )

            // Taiwan Plant Dictionary (台灣植物名彙)
            CopyrightSection(
                title: Tab1Texts.taiwanPlantDict,
                description: Tab1Texts.taiwanPlantCopyright,
                license: Tab1Texts.ccBySA4License,
                licenseURL: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW",
                websiteURL: "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106",
            )

            // Subject Terminology Dictionary (學科術語)
            CopyrightSection(
                title: Tab1Texts.sttiDict,
                description: Tab1Texts.sttiCopyright,
                license: Tab1Texts.ogdlTaiwanLicense,
                licenseURL: "https://spdx.org/licenses/OGDL-Taiwan-1.0.html",
                websiteURL: "https://stti.moe.edu.tw/",
            )

            // Accent/dialect supplementary data (腔口補充)
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text(languageManager.text(Tab1Texts.accentDict))
                        .font(AppStyle.headlineFont)

                    Text(languageManager.text(Tab1Texts.accentDictCredit))
                        .foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle(languageManager.text(Tab1Texts.copyrightNotice))
        .navigationBarTitleDisplayMode(.large)
    }
}

// MARK: - Copyright Section

private struct CopyrightSection: View {
    let title: LocalizedText
    let description: LocalizedText
    let license: LocalizedText
    let licenseURL: String
    let websiteURL: String?

    @StateObject private var languageManager = LanguageManager.shared

    var body: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Text(languageManager.text(title))
                    .font(AppStyle.headlineFont)

                Text(languageManager.text(description))
                    .foregroundColor(.secondary)

                Text(languageManager.text(license))
                    .font(AppStyle.captionFont)
                    .foregroundColor(.secondary)
                    .italic()
            }

            Link(destination: URL(string: licenseURL)!) {
                Label(languageManager.text(Tab1Texts.viewLicense), systemImage: "doc.text")
            }

            if let websiteURL {
                Link(destination: URL(string: websiteURL)!) {
                    Label(languageManager.text(Tab1Texts.viewWebsite), systemImage: "globe")
                }
            }
        }
    }
}

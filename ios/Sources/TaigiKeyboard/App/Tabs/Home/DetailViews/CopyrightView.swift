// 中文: 字型 / 詞典 / 開源專案的版權聲明頁。

import SwiftUI

/// Copyright notices for dictionaries and open-source projects.
// 中文: 版權聲明頁。各區段透過 CopyrightSection 渲染:title、描述、授權、連結。
struct CopyrightView: View {
    var body: some View {
        Form {
            // Open Huninn (粉圓體)
            CopyrightSection(
                title: CommonTexts.fontOpenHuninn,
                description: HomeTexts.openFontCopyright,
                license: HomeTexts.silOpenFontLicense,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://justfont.com/huninn/",
            )

            // Iansui (芫荽體)
            CopyrightSection(
                title: CommonTexts.fontIansui,
                description: HomeTexts.iansuiFontCopyright,
                license: HomeTexts.silOpenFontLicense11,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://github.com/ButTaiwan/iansui",
            )

            // GenYoMin (源樣明體)
            CopyrightSection(
                title: CommonTexts.fontGenYoMin,
                description: HomeTexts.butTaiwanCopyright,
                license: HomeTexts.silOpenFontLicense11,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://github.com/ButTaiwan/genyo-font",
            )

            // GenYoGothic (源樣黑體)
            CopyrightSection(
                title: CommonTexts.fontGenYoGothic,
                description: HomeTexts.butTaiwanCopyright,
                license: HomeTexts.silOpenFontLicense11,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://github.com/ButTaiwan/genyog-font",
            )

            // MOE Taiwanese Dictionary (教育部臺灣台語常用詞辭典)
            CopyrightSection(
                title: CommonTexts.moeDict,
                description: HomeTexts.moeCopyright,
                license: HomeTexts.ccLicense,
                licenseURL: "https://creativecommons.org/licenses/by-nd/3.0/tw/",
                websiteURL: "https://sutian.moe.edu.tw/",
            )

            // New Words Dictionary (新詞新語)
            CopyrightSection(
                title: CommonTexts.newwordDict,
                description: HomeTexts.newwordCopyright,
                license: HomeTexts.ccBy4License,
                licenseURL: "https://creativecommons.org/licenses/by/4.0/deed.zh-hant",
                websiteURL: "https://www.taigitv.org.tw/taigi-words",
            )

            // Craft Dictionary (工藝詞庫)
            CopyrightSection(
                title: CommonTexts.kunggeDict,
                description: HomeTexts.kunggeCopyright,
                license: HomeTexts.ccByNcLicense,
                licenseURL: "https://creativecommons.org/licenses/by-nc/4.0/deed.zh-hant",
                websiteURL: "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite",
            )

            // iTaigi
            CopyrightSection(
                title: CommonTexts.iTaigiDict,
                description: HomeTexts.iTaigiCopyright,
                license: HomeTexts.cc0License,
                licenseURL: "https://creativecommons.org/public-domain/cc0/",
                websiteURL: "https://itaigi.tw/",
            )

            // Taiwan-Japan Dictionary (台日大辭典)
            CopyrightSection(
                title: CommonTexts.taiwanJapanDict,
                description: HomeTexts.taiwanJapanCopyright,
                license: HomeTexts.ccByNcSA3License,
                licenseURL: "https://creativecommons.org/licenses/by-nc-sa/3.0/tw/",
                websiteURL: "http://taigi.fhl.net/dict/",
            )

            // Tai-Hua Dictionary (台華線頂辭典)
            CopyrightSection(
                title: CommonTexts.taiHuaDict,
                description: HomeTexts.taiHuaCopyright,
                license: HomeTexts.ccBySA4License,
                licenseURL: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW",
                websiteURL: nil,
            )

            // Taiwan Plant Dictionary (台灣植物名彙)
            CopyrightSection(
                title: CommonTexts.taiwanPlantDict,
                description: HomeTexts.taiwanPlantCopyright,
                license: HomeTexts.ccBySA4License,
                licenseURL: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW",
                websiteURL: "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106",
            )

            // Subject Terminology Dictionary (學科術語)
            CopyrightSection(
                title: CommonTexts.sttiDict,
                description: HomeTexts.sttiCopyright,
                license: HomeTexts.ogdlTaiwanLicense,
                licenseURL: "https://spdx.org/licenses/OGDL-Taiwan-1.0.html",
                websiteURL: "https://stti.moe.edu.tw/",
            )

            // Accent/dialect supplementary data (腔口補充)
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text(CommonTexts.accentDict)
                        .font(AppStyle.headlineFont)

                    Text(HomeTexts.accentDictCredit)
                        .foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle(HomeTexts.copyrightNotice)
        .navigationBarTitleDisplayMode(.large)
    }
}

// MARK: - Copyright Section

// 中文: 單一版權項目的 Section 子 View。licenseURL 必填,websiteURL 可選。
private struct CopyrightSection: View {
    let title: String
    let description: String
    let license: String
    let licenseURL: String
    let websiteURL: String?

    var body: some View {
        Section {
            VStack(alignment: .leading, spacing: 8) {
                Text(title)
                    .font(AppStyle.headlineFont)

                Text(description)
                    .foregroundColor(.secondary)

                Text(license)
                    .font(AppStyle.captionFont)
                    .foregroundColor(.secondary)
                    .italic()
            }

            Link(destination: URL(string: licenseURL)!) {
                Label(HomeTexts.viewLicense, systemImage: "doc.text")
            }

            if let websiteURL {
                Link(destination: URL(string: websiteURL)!) {
                    Label(CommonTexts.viewWebsite, systemImage: "globe")
                }
            }
        }
    }
}

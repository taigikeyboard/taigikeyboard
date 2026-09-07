import SwiftUI

// Standardized short license names — language-invariant, never translated (kept as constants rather
// than i18n keys per the multi-language plan). Mirrors Android content/CopyrightData.kt.
// OGDL-Taiwan-1.0 is iOS-only (Android has no STTI copyright section).
private enum License {
    static let silOpenFont = "SIL Open Font License"
    static let silOpenFont11 = "SIL Open Font License 1.1"
    static let ccByNd3Tw = "CC BY-ND 3.0 TW"
    static let ccBy4 = "CC BY 4.0"
    // NTCRI publishes no licence for the glossary text; its only statement covers the images.
    static let ccByNcNdImages = "CC BY-NC-ND (images only)"
    static let cc0 = "CC0"
    static let ccByNcSa3Tw = "CC BY-NC-SA 3.0 TW"
    static let ccBySa4 = "CC BY-SA 4.0"
    static let ogdlTaiwan10 = "OGDL-Taiwan-1.0"
}

/// Copyright notices for fonts, dictionaries, and open-source projects.
struct CopyrightView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    var body: some View {
        Form {
            // Open Huninn (粉圓體)
            CopyrightSection(
                title: lang.string(.commonFontOpenHuninn),
                description: lang.string(.homeOpenFontCopyright),
                license: License.silOpenFont,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://justfont.com/huninn/",
            )

            // Iansui (芫荽體)
            CopyrightSection(
                title: lang.string(.commonFontIansui),
                description: lang.string(.homeButTaiwanCopyright),
                license: License.silOpenFont11,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://github.com/ButTaiwan/iansui",
            )

            // GenYoMin (源樣明體)
            CopyrightSection(
                title: lang.string(.commonFontGenYoMin),
                description: lang.string(.homeButTaiwanCopyright),
                license: License.silOpenFont11,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://github.com/ButTaiwan/genyo-font",
            )

            // GenYoGothic (源樣烏體)
            CopyrightSection(
                title: lang.string(.commonFontGenYoGothic),
                description: lang.string(.homeButTaiwanCopyright),
                license: License.silOpenFont11,
                licenseURL: "https://openfontlicense.org/",
                websiteURL: "https://github.com/ButTaiwan/genyog-font",
            )

            // MOE Taiwanese Dictionary (教育部臺灣台語常用詞辭典)
            CopyrightSection(
                title: lang.string(.commonMoeDict),
                description: lang.string(.homeMoeCopyright),
                license: License.ccByNd3Tw,
                licenseURL: "https://creativecommons.org/licenses/by-nd/3.0/tw/",
                websiteURL: "https://sutian.moe.edu.tw/",
            )

            // New Words Dictionary (新詞新語)
            CopyrightSection(
                title: lang.string(.commonNewwordDict),
                description: lang.string(.homeNewwordCopyright),
                license: License.ccBy4,
                licenseURL: "https://creativecommons.org/licenses/by/4.0/deed.zh-hant",
                websiteURL: "https://www.taigitv.org.tw/taigi-words",
            )

            // Craft Dictionary (工藝詞庫)
            CopyrightSection(
                title: lang.string(.commonKunggeDict),
                description: lang.string(.homeKunggeCopyright),
                license: License.ccByNcNdImages,
                licenseURL: "https://kanggesu.ntcri.gov.tw/NTCRI_TaigiWebSite/ImageLicense",
                websiteURL: "https://kanggesu.ntcri.gov.tw/NTCRI_TaigiWebSite",
            )

            // iTaigi
            CopyrightSection(
                title: lang.string(.commonITaigiDict),
                description: lang.string(.homeITaigiCopyright),
                license: License.cc0,
                licenseURL: "https://creativecommons.org/public-domain/cc0/",
                websiteURL: "https://itaigi.tw/",
            )

            // Taiwan-Japan Dictionary (台日大辭典)
            CopyrightSection(
                title: lang.string(.commonTaiwanJapanDict),
                description: lang.string(.homeTaiwanJapanCopyright),
                license: License.ccByNcSa3Tw,
                licenseURL: "https://creativecommons.org/licenses/by-nc-sa/3.0/tw/",
                websiteURL: "http://taigi.fhl.net/dict/",
            )

            // Tai-Hua Dictionary (台華線頂辭典)
            CopyrightSection(
                title: lang.string(.commonTaiHuaDict),
                description: lang.string(.homeTaiHuaCopyright),
                license: License.ccBySa4,
                licenseURL: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW",
                websiteURL: nil,
            )

            // Taiwan Plant Dictionary (台灣植物名彙)
            CopyrightSection(
                title: lang.string(.commonTaiwanPlantDict),
                description: lang.string(.homeTaiwanPlantCopyright),
                license: License.ccBySa4,
                licenseURL: "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW",
                websiteURL: "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106",
            )

            // Subject Terminology Dictionary (學科術語) — same rights-holder (教育部) as the MOE dictionary.
            CopyrightSection(
                title: lang.string(.commonSttiDict),
                description: lang.string(.homeMoeCopyright),
                license: License.ogdlTaiwan10,
                licenseURL: "https://spdx.org/licenses/OGDL-Taiwan-1.0.html",
                websiteURL: "https://stti.moe.edu.tw/",
            )

            // Accent/dialect supplementary data (腔口補充)
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text(lang.string(.commonAccentDict))
                        .font(AppStyle.headlineFont)

                    Text(lang.string(.homeAccentDictCredit))
                        .foregroundColor(.secondary)
                }
            }

            // Developer supplement dictionary (詞庫增補檔案)
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text(lang.string(.dictionaryDevSupplementDict))
                        .font(AppStyle.headlineFont)

                    Text(lang.string(.homeDevSupplementCredit))
                        .foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle(lang.string(.homeCopyrightNotice))
        .navigationBarTitleDisplayMode(.large)
    }
}

// MARK: - Copyright Section

private struct CopyrightSection: View {
    @Environment(DisplayLanguageStore.self) private var lang
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
                Label(lang.string(.homeViewLicense), systemImage: "doc.text")
            }

            if let websiteURL {
                Link(destination: URL(string: websiteURL)!) {
                    Label(lang.string(.commonViewWebsite), systemImage: "globe")
                }
            }
        }
    }
}

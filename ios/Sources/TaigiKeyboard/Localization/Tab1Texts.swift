import Foundation

// MARK: - Tab1 頭頁文字

// 包含：頭頁、啟用方法、新功能、已知問題、預計功能、FAQ、版本紀錄、問題回報、版權聲明
// 對應 Android Tab1Texts.kt

enum Tab1Texts {
    // MARK: - Tab 標題

    static let tabTitle = LocalizedText(hanji: "頭頁")
    static let appHeaderTitle = LocalizedText(hanji: "iOS 台語齒盤")

    // MARK: - 區塊標題

    static let setupKeyboard = LocalizedText(hanji: "齒盤愛拍開才會當使用")
    static let typingGuide = LocalizedText(hanji: "拍字說明")
    static let newFeatures = LocalizedText(hanji: "功能設定")
    static let faq = LocalizedText(hanji: "捷問 ê 問題")

    // MARK: - 啟用方法

    static let setupGuide = LocalizedText(hanji: "啟用方法")
    static let setupGuideDescription = LocalizedText(hanji: "手機仔系統規定第三方齒盤愛手動啟用才會當使用，請照下跤 ê 說明完成設定。")
    static let setupInfoMessage = LocalizedText(hanji: "「允准完整取用」意思是予齒盤會當捌你揤 ê 動作，成做你拍 ê 字。請放心，App 袂紀錄你 ê 資料。")
    static let setupBrandWarning = LocalizedText(hanji: "無仝牌子 ê 手機仔，設定 ê 方式可能會淡薄仔無仝款，毋過方式應該攏差不多。")

    // MARK: - Setup Guide 步驟（全螢幕模式使用）

    static let setupGuideCompletedMessage = LocalizedText(hanji: "完成了後，重開你目前使用 ê App，予 App 重掠新 ê 齒盤清單。紲落來，佇會當拍字 ê 所在，揤牢地球圖示切去台語齒盤")
    static let setupGuideGoToSettings = LocalizedText(hanji: "去設定頁")
    static let setupGuideCloseButton = LocalizedText(hanji: "關閉")
    static let setupGuideStartSetup = LocalizedText(hanji: "啟用齒盤")
    static let setupGuideStep1Settings = LocalizedText(hanji: "點揤「齒盤」")
    static let setupGuideStep2AddKeyboard = LocalizedText(hanji: "點揤「增加齒盤」、「允准完整取用」")

    // MARK: - 新功能 & FAQ

    // Feature and FAQ content (title, paragraphs, attachments) is now loaded from
    // content/tab1-features.json and content/tab1-faq.json via FeatureContentLoader.

    // MARK: - 資源連結

    static let userGuide = LocalizedText(hanji: "網站紹介")
    static let rateUs = LocalizedText(hanji: "為阮評分")
    static let contactUs = LocalizedText(hanji: "寄付支持")
    static let privacyPolicy = LocalizedText(hanji: "隱私權政策")

    // MARK: - 寄付

    static let emailContact = LocalizedText(hanji: "台語齒盤是我 1 个人用上班以外 ê 時間開發佮維護，開發者帳號、開發家私、網站費用攏是用家己薪水支付。若你感覺這个齒盤對你有幫助，歡迎贊助支持，予台語齒盤會當繼續運作落去，咱做伙為著台語拍拚。")
    static let freePromise = LocalizedText(hanji: "台語齒盤保證永遠免費，嘛袂做付費功能。台語是咱 ê 母語，無應該因為錢 ê 問題用袂著好 ê 家私。我向望逐家想欲學台語、寫台語 ê 人攏會當無負擔來使用，這是我做這个齒盤上重要 ê 心願。")
    static let supportUs = LocalizedText(hanji: "贊助台語齒盤")

    // MARK: - 版本資訊

    static let version = LocalizedText(hanji: "當前版本")
    static let versionHistory = LocalizedText(hanji: "版本紀錄")

    static let versionHistoryEntries: [(version: String, date: String, changes: [LocalizedText])] = [
        ("3.4.8", "2026/04/11", [
            LocalizedText(hanji: "Fixed an issue where app storage grew excessively over time."),
        ]),
        ("3.4.7", "2026/04/05", [
            LocalizedText(hanji: "Added globe key toggle to show or hide the keyboard switch key."),
            LocalizedText(hanji: "Added custom dictionary enable/disable toggle."),
            LocalizedText(hanji: "Added frequency recording toggle to enable or disable word frequency tracking."),
            LocalizedText(hanji: "Added association recording toggle to enable or disable next-word prediction learning."),
            LocalizedText(hanji: "Added CSV import/export for word frequency and association data."),
            LocalizedText(hanji: "Rewrote feature guides with detailed input mode and romanization tutorials."),
            LocalizedText(hanji: "Settings now show info buttons with feature descriptions."),
            LocalizedText(hanji: "Dictionary settings now show source descriptions and links."),
            LocalizedText(hanji: "Custom dictionary now shows CSV format example."),
            LocalizedText(hanji: "Added Samsung keyboard quick-switch FAQ."),
            LocalizedText(hanji: "Fixed TPS input incorrectly joining syllables after tone marks."),
            LocalizedText(hanji: "Fixed certain POJ words with o͘ not showing candidates."),
            LocalizedText(hanji: "Fixed duplicate candidates appearing in TPS mode."),
            LocalizedText(hanji: "Fixed symbols like arrows incorrectly entering composition mode."),
        ]),
        ("3.4.6", "2026/03/24", [
            LocalizedText(hanji: "Added in-keyboard settings panel accessible from toolbar."),
            LocalizedText(hanji: "Added toolbar auto-collapse toggle in settings."),
            LocalizedText(hanji: "TPS layout auto-corrects palatalized initials (ㄗ+ㄧ→ㄐ, ㄘ+ㄧ→ㄑ, etc.)."),
            LocalizedText(hanji: "TPS layout: digit keys accessible via long-press on row 1."),
            LocalizedText(hanji: "Improved TPS syllable boundary detection for more accurate input."),
            LocalizedText(hanji: "Fixed custom dictionary link not responding in dictionary settings."),
            LocalizedText(hanji: "Enabled 台語工藝詞庫 and 學科術語辭典 by default."),
        ]),
        ("3.4.5", "2026/03/21", [
            LocalizedText(hanji: "Added symbol selection panel for inserting special characters."),
            LocalizedText(hanji: "Added dictionary search in the dictionary settings page."),
            LocalizedText(hanji: "TPS layout auto-selects ㄇ/ㄫ initial and final forms based on context."),
            LocalizedText(hanji: "Added dismiss keyboard button in toolbar."),
            LocalizedText(hanji: "Improved number key input."),
            LocalizedText(hanji: "Redesigned dictionary settings page with descriptions and categories."),
            LocalizedText(hanji: "Improved symbol panel touch targets and fixed invisible characters."),
            LocalizedText(hanji: "Fixed an issue where some candidates were missing from search results."),
            LocalizedText(hanji: "Updated dictionary data."),
        ]),
        ("3.4.4", "2026/03/09", [
            LocalizedText(hanji: "Fixed an issue where some words could not be found when typing."),
        ]),
        ("3.4.2", "2026/03/08", [
            LocalizedText(hanji: "Added custom dictionary for adding your own words."),
            LocalizedText(hanji: "Added diagnostic info for easier bug reporting."),
            LocalizedText(hanji: "Fixed custom dictionary entries not appearing in search results."),
            LocalizedText(hanji: "Fixed custom dictionary capitalization not matching other candidates."),
            LocalizedText(hanji: "Fixed word-grouped POJ display conversion."),
        ]),
        ("3.4.1", "2026/02/26", [
            LocalizedText(hanji: "Added tone diacritic hints above number keys."),
            LocalizedText(hanji: "Added punctuation hints on MOE1/MOE2 layout keys."),
            LocalizedText(hanji: "Added MOE Layout 1 and MOE Layout 2 keyboards."),
            LocalizedText(hanji: "Added TPS (方音符號) keyboard layout."),
            LocalizedText(hanji: "Added keyboard appearance customization settings."),
            LocalizedText(hanji: "Added in-keyboard layout selection panel."),
            LocalizedText(hanji: "Added STTI (學科術語辭典) dictionary source."),
            LocalizedText(hanji: "Added phrase learning for continuous word selections."),
            LocalizedText(hanji: "Improved continuous typing with word-grouped display."),
            LocalizedText(hanji: "Fixed POJ and TL input mode separation."),
        ]),
        ("3.4.0", "2025/12/31", [
            LocalizedText(hanji: "Adjusted the four-syllable input limit."),
            LocalizedText(hanji: "Fixed an issue where o͘ did not trigger candidate search."),
        ]),
        ("3.3.9", "2025/12/30", [
            LocalizedText(hanji: "Added an English keyboard."),
            LocalizedText(hanji: "Added explanations for 'Typing History Dictionary' and 'Capitalization Toggle.'"),
            LocalizedText(hanji: "Added more symbols to the punctuation keyboard."),
            LocalizedText(hanji: "Added quick toggle shortcuts to the candidate bar."),
            LocalizedText(hanji: "Adopted Apple’s default UI components."),
            LocalizedText(hanji: "Fixed an issue where candidates did not appear when typing tone 1 or 4 directly."),
            LocalizedText(hanji: "Fixed an issue where capitalization was not working correctly."),
            LocalizedText(hanji: "Fixed inconsistent font sizing across different iPhone models."),
            LocalizedText(hanji: "Refactored the codebase for better cleanliness and maintainability."),
            LocalizedText(hanji: "Updated the app logo."),
            LocalizedText(hanji: "Upgraded KeyboardKit to v10."),
        ]),
        ("3.3.8", "2025/12/25", [
            LocalizedText(hanji: "Refreshed the app interface and improved in-app explanations for better clarity."),
        ]),
    ]

    // MARK: - 版權聲明

    static let copyrightNotice = LocalizedText(hanji: "致謝")
    static let viewLicense = LocalizedText(hanji: "授權條款")
    static let viewWebsite = LocalizedText(hanji: "官方網站")

    // 教育部臺灣台語常用詞辭典
    static let moeDict = LocalizedText(hanji: "教育部臺灣台語常用詞辭典")
    static let moeCopyright = LocalizedText(hanji: "© 教育部")
    static let ccLicense = LocalizedText(hanji: "CC BY-ND 3.0 TW")

    // iTaigi 華台辭典
    static let iTaigiDict = LocalizedText(hanji: "iTaigi愛台語")
    static let iTaigiCopyright = LocalizedText(hanji: "© iTaigi愛台語")
    static let cc0License = LocalizedText(hanji: "CC0")

    // 台語新詞辭庫
    static let newwordDict = LocalizedText(hanji: "公視台語台台語新詞辭庫")
    static let newwordCopyright = LocalizedText(hanji: "© 公視台語台")
    static let ccBy4License = LocalizedText(hanji: "CC BY 4.0")

    // 粉圓字型
    static let openFontTitle = LocalizedText(hanji: "粉圓")
    static let openFontCopyright = LocalizedText(hanji: "© justfont")
    static let silOpenFontLicense = LocalizedText(hanji: "SIL Open Font License")

    // 芫荽字型
    static let iansuiFontTitle = LocalizedText(hanji: "芫荽")
    static let iansuiFontCopyright = LocalizedText(hanji: "© ButTaiwan")
    static let silOpenFontLicense11 = LocalizedText(hanji: "SIL Open Font License 1.1")

    // 台灣植物名彙
    static let taiwanPlantDict = LocalizedText(hanji: "台灣植物名彙")
    static let taiwanPlantCopyright = LocalizedText(hanji: "© 佐佐木舜一")
    static let ccBySA4License = LocalizedText(hanji: "CC BY-SA 4.0")

    // 台華線頂對照典
    static let taiHuaDict = LocalizedText(hanji: "台華線頂對照典")
    static let taiHuaCopyright = LocalizedText(hanji: "© 鄭良偉")

    // 台日大辭典
    static let taiwanJapanDict = LocalizedText(hanji: "臺日大辭典台語譯本")
    static let taiwanJapanCopyright = LocalizedText(hanji: "© 小川尚義")
    static let ccByNcSA3License = LocalizedText(hanji: "CC BY-NC-SA 3.0 TW")

    // 台語工藝詞庫
    static let kunggeDict = LocalizedText(hanji: "工藝中心臺灣台語工藝詞庫")
    static let kunggeCopyright = LocalizedText(hanji: "© 國立臺灣工藝研究發展中心")
    static let ccByNcLicense = LocalizedText(hanji: "CC BY-NC 4.0")

    // 學科術語辭典
    static let sttiDict = LocalizedText(hanji: "教育部學科術語臺灣台語對譯")
    static let sttiCopyright = LocalizedText(hanji: "© 教育部")
    static let ogdlTaiwanLicense = LocalizedText(hanji: "OGDL-Taiwan-1.0")

    // 腔口差
    static let accentDict = LocalizedText(hanji: "腔口差")
    static let accentDictCredit = LocalizedText(hanji: "「實齋」整理、提供")
}

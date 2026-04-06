package com.siansiansu.taigikeyboard.localization

/**
 * Tab1 頭頁文字
 * 包含：頭頁、啟用方法、新功能、FAQ、版本紀錄、問題回報、版權聲明
 * 對應 iOS Tab1Texts.swift
 */
object Tab1Texts {
    // MARK: - Tab 標題

    val tabTitle = LocalizedText(hanji = "頭頁")

    // MARK: - 頁面標題

    val appHeaderTitle = LocalizedText(hanji = "Android 台語齒盤")

    // MARK: - 區塊標題

    val setupKeyboard = LocalizedText(hanji = "齒盤愛拍開才會當使用")
    val typingGuide = LocalizedText(hanji = "拍字說明")
    val newFeatures = LocalizedText(hanji = "功能設定")
    val faq = LocalizedText(hanji = "捷問 ê 問題")

    // MARK: - 啟用方法

    val setupGuide = LocalizedText(hanji = "啟用方法")
    val setupGuideDescription = LocalizedText(hanji = "手機仔系統規定第三方齒盤愛手動啟用才會當使用，請照下跤 ê 說明完成設定。")
    val setupInfoMessage = LocalizedText(hanji = "「允准完整取用」意思是予齒盤會當捌你揤 ê 動作，成做你拍 ê 字。請放心，App 袂紀錄你 ê 資料。")
    val setupBrandWarning = LocalizedText(hanji = "無仝牌子 ê 手機仔，設定 ê 方式可能會淡薄仔無仝款，毋過方式應該攏差不多。")

    val setupGuideStartSetup = LocalizedText(hanji = "啟用齒盤")

    // MARK: - 設定引導步驟（全螢幕模式使用）

    val setupGuideCompletedMessage = LocalizedText(hanji = "完成了後，重開你目前使用 ê App，予 App 重掠新 ê 齒盤清單。紲落來，佇會當拍字 ê 所在，揤牢地球圖示切去台語齒盤")
    val setupGuideGoToSettings = LocalizedText(hanji = "去設定頁")
    val setupGuideCloseButton = LocalizedText(hanji = "關閉")
    val setupGuideStep1Settings = LocalizedText(hanji = "點揤「齒盤」")
    val setupGuideStep2AddKeyboard = LocalizedText(hanji = "點揤「增加齒盤」、「允准完整取用」")

    // MARK: - 新功能 & FAQ
    // Feature and FAQ content (title, paragraphs, attachments) is now loaded from
    // assets/tab1-features.json and assets/tab1-faq.json via FeatureContentLoader.

    // MARK: - 資源連結

    val userGuide = LocalizedText(hanji = "網站紹介")
    val rateUs = LocalizedText(hanji = "為阮評分")
    val contactUs = LocalizedText(hanji = "寄付支持")
    val privacyPolicy = LocalizedText(hanji = "隱私權政策")

    // MARK: - 寄付

    val emailContact =
        LocalizedText(hanji = "台語齒盤是我 1 个人用上班以外 ê 時間開發佮維護，開發者帳號、開發家私、網站費用攏是用家己薪水支付。若你感覺這个齒盤對你有幫助，歡迎贊助支持，予台語齒盤會當繼續運作落去，咱做伙為著台語拍拚。")
    val freePromise =
        LocalizedText(hanji = "台語齒盤保證永遠免費，嘛袂做付費功能。台語是咱 ê 母語，無應該因為錢 ê 問題用袂著好 ê 家私。我向望逐家想欲學台語、寫台語 ê 人攏會當無負擔來使用，這是我做這个齒盤上重要 ê 心願。")
    val supportUs = LocalizedText(hanji = "贊助台語齒盤")

    // MARK: - 版本資訊

    val version = LocalizedText(hanji = "當前版本")
    val versionHistory = LocalizedText(hanji = "版本紀錄")

    data class VersionEntry(
        val version: String,
        val date: String,
        val changes: List<LocalizedText>,
    )

    val versionHistoryEntries =
        listOf(
            VersionEntry(
                "3.4.7",
                "2026/04/05",
                listOf(
                    LocalizedText(hanji = "Added globe key toggle to show or hide the keyboard switch key."),
                    LocalizedText(hanji = "Added custom dictionary enable/disable toggle."),
                    LocalizedText(hanji = "Added frequency recording toggle to enable or disable word frequency tracking."),
                    LocalizedText(hanji = "Added association recording toggle to enable or disable next-word prediction learning."),
                    LocalizedText(hanji = "Added CSV import/export for word frequency and association data."),
                    LocalizedText(hanji = "Rewrote feature guides with detailed input mode and romanization tutorials."),
                    LocalizedText(hanji = "Settings now show info buttons with feature descriptions."),
                    LocalizedText(hanji = "Dictionary settings now show source descriptions and links."),
                    LocalizedText(hanji = "Custom dictionary now shows CSV format example."),
                    LocalizedText(hanji = "Swipe to delete custom dictionary entries and frequency data."),
                    LocalizedText(hanji = "Added Samsung keyboard quick-switch FAQ."),
                    LocalizedText(hanji = "Fixed TPS input incorrectly joining syllables after tone marks."),
                    LocalizedText(hanji = "Fixed certain POJ words with o͘ not showing candidates."),
                    LocalizedText(hanji = "Fixed duplicate candidates appearing in TPS mode."),
                    LocalizedText(hanji = "Fixed a rare crash in English spell-check."),
                    LocalizedText(hanji = "Fixed symbols like arrows incorrectly entering composition mode."),
                    LocalizedText(hanji = "Updated keyboard layout preview images."),
                    LocalizedText(hanji = "Updated the app icon."),
                ),
            ),
            VersionEntry(
                "3.4.6",
                "2026/03/24",
                listOf(
                    LocalizedText(hanji = "Added in-keyboard settings panel accessible from toolbar."),
                    LocalizedText(hanji = "Added toolbar auto-collapse toggle in settings."),
                    LocalizedText(hanji = "TPS layout auto-corrects palatalized initials (ㄗ+ㄧ→ㄐ, ㄘ+ㄧ→ㄑ, etc.)."),
                    LocalizedText(hanji = "TPS layout: digit keys accessible via long-press on row 1."),
                    LocalizedText(hanji = "Improved TPS syllable boundary detection for more accurate input."),
                    LocalizedText(hanji = "Fixed toolbar settings toggles not taking effect until restart."),
                    LocalizedText(hanji = "Fixed backspace showing romanization after selecting a custom dictionary word."),
                    LocalizedText(hanji = "Enabled 台語工藝詞庫 and 學科術語辭典 by default."),
                ),
            ),
            VersionEntry(
                "3.4.5",
                "2026/03/21",
                listOf(
                    LocalizedText(hanji = "Added symbol selection panel for inserting special characters."),
                    LocalizedText(hanji = "Added dictionary search in the dictionary settings page."),
                    LocalizedText(hanji = "TPS (方音符號) keyboard layout is now available."),
                    LocalizedText(hanji = "TPS layout auto-selects ㄇ/ㄫ initial and final forms based on context."),
                    LocalizedText(hanji = "Added dismiss keyboard button in toolbar."),
                    LocalizedText(hanji = "Improved number key input."),
                    LocalizedText(hanji = "Redesigned dictionary settings page with descriptions and categories."),
                    LocalizedText(hanji = "Improved symbol panel touch targets."),
                    LocalizedText(hanji = "Fixed an issue where some candidates were missing from search results."),
                    LocalizedText(hanji = "Added reset all settings option."),
                    LocalizedText(hanji = "Updated dictionary data."),
                ),
            ),
            VersionEntry(
                "3.4.4",
                "2026/03/09",
                listOf(
                    LocalizedText(hanji = "Fixed an issue where some words could not be found when typing."),
                ),
            ),
            VersionEntry(
                "3.4.2",
                "2026/03/08",
                listOf(
                    LocalizedText(hanji = "Added custom dictionary for adding your own words."),
                    LocalizedText(hanji = "Added diagnostic info for easier bug reporting."),
                    LocalizedText(hanji = "Changed app font to 粉圓 (jf-openhuninn)."),
                    LocalizedText(hanji = "Redesigned candidate display with title and subtitle."),
                    LocalizedText(hanji = "Fixed custom dictionary entries not appearing in search results."),
                    LocalizedText(hanji = "Fixed custom dictionary capitalization not matching other candidates."),
                    LocalizedText(hanji = "Fixed word-grouped POJ display conversion."),
                ),
            ),
            VersionEntry(
                "3.4.1",
                "2026/02/26",
                listOf(
                    LocalizedText(hanji = "Added tone diacritic hints above number keys."),
                    LocalizedText(hanji = "Added punctuation hints on MOE1/MOE2 layout keys."),
                    LocalizedText(hanji = "Added MOE Layout 1 and MOE Layout 2 keyboards."),
                    LocalizedText(hanji = "Added keyboard appearance customization settings."),
                    LocalizedText(hanji = "Added input mode label (POJ/TL/EN) on the space bar."),
                    LocalizedText(hanji = "Added STTI (學科術語辭典) dictionary source."),
                    LocalizedText(hanji = "Added phrase learning for continuous word selections."),
                    LocalizedText(hanji = "Improved keyboard typing performance."),
                    LocalizedText(hanji = "Fixed POJ and TL input mode separation."),
                ),
            ),
            VersionEntry(
                "3.4.0",
                "2025/12/31",
                listOf(
                    LocalizedText(hanji = "Adjusted the four-syllable input limit."),
                    LocalizedText(hanji = "Fixed an issue where o͘ did not trigger candidate search."),
                ),
            ),
            VersionEntry(
                "3.3.9",
                "2025/12/30",
                listOf(
                    LocalizedText(hanji = "Added an English keyboard."),
                    LocalizedText(hanji = "Added explanations for ‘Typing History Dictionary’ and ‘Capitalization Toggle’."),
                    LocalizedText(hanji = "Added more symbols to the punctuation keyboard."),
                    LocalizedText(hanji = "Added quick toggle shortcuts to the candidate bar."),
                    LocalizedText(hanji = "Adopted Android’s default UI components."),
                    LocalizedText(hanji = "Fixed an issue where candidates did not appear when typing tone 1 or 4 directly."),
                    LocalizedText(hanji = "Fixed an issue where capitalization was not working correctly."),
                    LocalizedText(hanji = "Fixed inconsistent font sizing across different devices."),
                    LocalizedText(hanji = "Refactored the codebase for better cleanliness and maintainability."),
                    LocalizedText(hanji = "Updated the app logo."),
                ),
            ),
            VersionEntry(
                "3.3.8",
                "2025/12/25",
                listOf(
                    LocalizedText(hanji = "Refreshed the app interface and improved in-app explanations for better clarity."),
                ),
            ),
        )

    // MARK: - 版權聲明

    val copyrightNotice = LocalizedText(hanji = "致謝")
    val viewLicense = LocalizedText(hanji = "授權條款")
    val viewWebsite = LocalizedText(hanji = "官方網站")

    // 教育部臺灣台語常用詞辭典
    val moeDict = LocalizedText(hanji = "教育部臺灣台語常用詞辭典")
    val moeCopyright = LocalizedText(hanji = "© 教育部")
    val ccLicense = LocalizedText(hanji = "CC BY-ND 3.0 TW")

    // iTaigi 華台辭典
    val iTaigiDict = LocalizedText(hanji = "iTaigi愛台語")
    val iTaigiCopyright = LocalizedText(hanji = "© iTaigi愛台語")
    val cc0License = LocalizedText(hanji = "CC0")

    // 台語新詞辭庫
    val newwordDict = LocalizedText(hanji = "公視台語台台語新詞辭庫")
    val newwordCopyright = LocalizedText(hanji = "© 公視台語台")
    val ccBy4License = LocalizedText(hanji = "CC BY 4.0")

    // 粉圓字型
    val openFontTitle = LocalizedText(hanji = "粉圓")
    val openFontCopyright = LocalizedText(hanji = "© justfont")
    val silOpenFontLicense = LocalizedText(hanji = "SIL Open Font License")

    // 芫荽字型
    val iansuiFontTitle = LocalizedText(hanji = "芫荽")
    val iansuiFontCopyright = LocalizedText(hanji = "© ButTaiwan")
    val silOpenFontLicense11 = LocalizedText(hanji = "SIL Open Font License 1.1")

    // 台灣植物名彙
    val taiwanPlantDict = LocalizedText(hanji = "台灣植物名彙")
    val taiwanPlantCopyright = LocalizedText(hanji = "© 佐佐木舜一")
    val ccBySA4License = LocalizedText(hanji = "CC BY-SA 4.0")

    // 台華線頂對照典
    val taiHuaDict = LocalizedText(hanji = "台華線頂對照典")
    val taiHuaCopyright = LocalizedText(hanji = "© 鄭良偉")

    // 台日大辭典
    val taiwanJapanDict = LocalizedText(hanji = "臺日大辭典台語譯本")
    val taiwanJapanCopyright = LocalizedText(hanji = "© 小川尚義")
    val ccByNcSA3License = LocalizedText(hanji = "CC BY-NC-SA 3.0 TW")

    // 台語工藝詞庫
    val kunggeDict = LocalizedText(hanji = "工藝中心臺灣台語工藝詞庫")
    val kunggeCopyright = LocalizedText(hanji = "© 國立臺灣工藝研究發展中心")
    val ccByNcLicense = LocalizedText(hanji = "CC BY-NC 4.0")

    // 學科術語辭典
    val sttiDict = LocalizedText(hanji = "教育部學科術語臺灣台語對譯")
    val sttiCopyright = LocalizedText(hanji = "© 教育部")
    val ogdlTaiwanLicense = LocalizedText(hanji = "OGDL-Taiwan-1.0")

    // 腔口差
    val accentDict = LocalizedText(hanji = "腔口差")
    val accentDictCredit = LocalizedText(hanji = "「實齋」整理、提供")
}

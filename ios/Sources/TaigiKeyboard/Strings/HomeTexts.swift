// 中文: HomeTab(頭頁)所有 UI 文字常數。包含啟用導引、功能說明、版本歷史、
// 中文: 關於開發者、版權聲明等。對應 Android `HomeTexts.kt`。
// 中文: Feature / FAQ 內容已外移到 content/tab1-features.json + tab1-faq.json,
// 中文: 由 FeatureContentLoader 動態載入。

// MARK: - HomeTab 頭頁文字

// 包含：頭頁、啟用方法、新功能、已知問題、預計功能、FAQ、版本紀錄、關於開發者、版權聲明
// 對應 Android HomeTexts.kt

// 中文: 頭頁文字 namespace。MARK 子區塊與頁面 Section 對應。
enum HomeTexts {
    // MARK: - Tab 標題

    static let tabTitle = "頭頁"
    static let appHeaderTitle = "iOS 台語齒盤"

    // MARK: - 區塊標題

    static let setupKeyboard = "齒盤愛拍開才會當使用"
    static let typingGuide = "拍字說明"
    static let newFeatures = "功能設定"
    static let faq = "其他"

    // MARK: - 啟用方法

    static let setupGuide = "啟用方法"
    static let setupGuideDescription = "手機仔系統規定第三方齒盤愛手動啟用才會當使用，請照下跤 ê 說明完成設定。"
    static let setupInfoMessage = "「允准完整取用」意思是予齒盤會當捌你揤 ê 動作，成做你拍 ê 字。請放心，App 袂紀錄你 ê 資料。"
    static let setupBrandWarning = "無仝牌子 ê 手機仔，設定 ê 方式可能會淡薄仔無仝款，毋過方式應該攏差不多。"

    // MARK: - Setup Guide 步驟（全螢幕模式使用）

    static let setupGuideCompletedMessage = "完成了後，重開你目前使用 ê App，予 App 重掠新 ê 齒盤清單。紲落來，佇會當拍字 ê 所在，揤牢地球圖示切去台語齒盤"
    static let setupGuideGoToSettings = "去設定頁"
    static let setupGuideCloseButton = "關閉"
    static let setupGuideStep1Settings = "點揤「齒盤」"
    static let setupGuideStep2AddKeyboard = "點揤「增加齒盤」、「允准完整取用」"

    // MARK: - 新功能 & FAQ

    // Feature and FAQ content (title, paragraphs, attachments) is now loaded from
    // content/tab1-features.json and content/tab1-faq.json via FeatureContentLoader.

    // MARK: - 資源連結

    static let userGuide = "網站紹介"
    static let rateUs = "為阮評分"
    static let aboutDeveloper = "關於開發者"
    static let privacyPolicy = "隱私權政策"

    // MARK: - 關於開發者

    static let freePromise = "台語齒盤保證永遠免費，嘛袂做付費功能。台語是咱 ê 母語，無應該因為錢 ê 問題用袂著好家私。我向望逐家想欲學台語、寫台語 ê 人攏會當無負擔來使用，這是我做這个齒盤上重要 ê 心願。"
    static let officialWebsite = "官方網站"

    // MARK: - 版本資訊

    static let version = "當前版本"
    static let versionHistory = "版本紀錄"

    // 中文: 版本歷史資料來源。每筆 tuple = (版本號, 釋出日期, 變更條目)。
    // 中文: 釋出時手動更新一筆,英文 changes 由 update-changelog skill 與 changelog/<v>.md 同步。
    static let versionHistoryEntries: [(version: String, date: String, changes: [String])] = [
        ("3.6.0", "2026/05/31", [
            "New: each 教典 (MOE dictionary) subcollection is now its own toggle — turn individual 腔調 (accent) readings and the 姓名 (name) appendix on or off in the dictionary settings.",
            "New: multi-character words now carry 語音差異 (per-accent) readings, not just single characters.",
            "New: 詞庫增補檔案 — a new toggleable supplement source (~2,500 added words: 一府五院 / 菜市仔名 / 台臺 / 教典僻智識 / 數字時間日期 / 行政區).",
            "Changed: turning a dictionary source off now also removes its words from the keyboard's candidates, not just the dictionary browse tab.",
            "Fixed: typing an explicit tone in continuous input (e.g. tai5) now shows only that tone's readings; typing without a tone (tai) still shows all tones.",
            "Updated dictionary data.",
        ]),
        ("3.5.9", "2026/05/27", [
            "New: TPS (Bopomofo-Taiwanese) continuous input — type a whole TPS sentence and the keyboard segments it into candidates, same as TL / POJ.",
            "Fixed: toneless TPS strings whose syllables start with a medial (e.g. ㄉㄞㄨㄢ for 台灣) no longer return zero candidates.",
            "Fixed: TPS auto-correct no longer corrupts the next initial after a precomposed nasal coda (ㄉㄞㄨㄢㄉ … no longer becomes ㄉㄞㄨㄢㆵ …).",
            "Fixed: typing a lone Bopomofo initial like ㄉ now returns prefix-matched candidates instead of nothing.",
            "Fixed: continuous-input in POJ mode now stays POJ end-to-end — POJ phrases no longer fall back to TL display, and user-frequency learning is shared between POJ and TL.",
            "Fixed: short common words no longer dropped from the partial-prefix candidate list under load.",
            "Updated dictionary data.",
        ]),
        ("3.5.8", "2026/05/20", [
            "New: 連續輸入 — type a whole romanized phrase without committing each syllable; the keyboard segments the sentence and offers candidates per position. Tap to commit a segment, Enter to commit the raw text. Works with POJ / TL, the custom dictionary, user-frequency learning, and next-word prediction.",
            "Continuous-input candidates now show romanization and 漢字 on two lines.",
            "Replaced the 寄付支持 tab with a 關於開發者 page linking to the official website.",
            "Updated dictionary data.",
        ]),
        ("3.5.7", "2026/05/09", [
            "Fixed: LKK 漢羅文 mixed-script suggestions now correctly stay enabled after a settings reset.",
            "Renamed font display name 源樣黑體 → 源樣烏體 (matches official ButTaiwan naming).",
            "Updated dictionary data — regenerated POJ entries and corrected the Tab3 (Hanji) character range.",
            "Internal: end-to-end keystroke trace IDs for debug-build diagnostics.",
        ]),
        ("3.5.6", "2026/05/02", [
            "Internal: dictionary read path rewritten in Rust (no behavior change).",
        ]),
        ("3.5.5", "2026/05/02", [
            "Internal: next-word prediction engine rewritten in Rust (no behavior change).",
        ]),
        ("3.5.4", "2026/05/01", [
            "Internal: composing buffer engine rewritten in Rust (no behavior change).",
        ]),
        ("3.5.3", "2026/04/29", [
            "Internal: engine workspace cleanup, removed duplicate platform implementations (no behavior change).",
        ]),
        ("3.5.2", "2026/04/29", [
            "Internal: candidate ranking pipeline rewritten in Rust (no behavior change).",
        ]),
        ("3.5.1", "2026/04/28", [
            "Internal: phonetics conversion engine rewritten in Rust (no behavior change).",
        ]),
        ("3.5.0", "2026/04/25", [
            "Added 源樣明體 (serif) and 源樣烏體 (sans-serif) font options.",
            "Restored POJ candidates for words containing o͘ / ⁿ — about 21% of romanization queries had been missing matches.",
            "Fixed next-word candidate inserting raw TL form instead of POJ when typing in POJ mode.",
            "Fixed candidates occasionally remaining on screen after backspacing.",
            "Fixed emoji key not committing the active candidate before inserting the emoji.",
            "Fixed icon rendering on devices configured for non-Latin locales.",
            "Updated dictionary data.",
        ]),
        ("3.4.9", "2026/04/12", [
            "Reduced iOS app size by 73% (48 MB → 13 MB).",
            "Increased candidate display limit from 100 to 200.",
        ]),
        ("3.4.8", "2026/04/11", [
            "Fixed an issue where app storage grew excessively over time.",
        ]),
        ("3.4.7", "2026/04/05", [
            "Added globe key toggle to show or hide the keyboard switch key.",
            "Added custom dictionary enable/disable toggle.",
            "Added frequency recording toggle to enable or disable word frequency tracking.",
            "Added association recording toggle to enable or disable next-word prediction learning.",
            "Added CSV import/export for word frequency and association data.",
            "Rewrote feature guides with detailed input mode and romanization tutorials.",
            "Settings now show info buttons with feature descriptions.",
            "Dictionary settings now show source descriptions and links.",
            "Custom dictionary now shows CSV format example.",
            "Added Samsung keyboard quick-switch FAQ.",
            "Fixed TPS input incorrectly joining syllables after tone marks.",
            "Fixed certain POJ words with o͘ not showing candidates.",
            "Fixed duplicate candidates appearing in TPS mode.",
            "Fixed symbols like arrows incorrectly entering composition mode.",
        ]),
        ("3.4.6", "2026/03/24", [
            "Added in-keyboard settings panel accessible from toolbar.",
            "Added toolbar auto-collapse toggle in settings.",
            "TPS layout auto-corrects palatalized initials (ㄗ+ㄧ→ㄐ, ㄘ+ㄧ→ㄑ, etc.).",
            "TPS layout: digit keys accessible via long-press on row 1.",
            "Improved TPS syllable boundary detection for more accurate input.",
            "Fixed custom dictionary link not responding in dictionary settings.",
            "Enabled 台語工藝詞庫 and 學科術語辭典 by default.",
        ]),
        ("3.4.5", "2026/03/21", [
            "Added symbol selection panel for inserting special characters.",
            "Added dictionary search in the dictionary settings page.",
            "TPS layout auto-selects ㄇ/ㄫ initial and final forms based on context.",
            "Added dismiss keyboard button in toolbar.",
            "Improved number key input.",
            "Redesigned dictionary settings page with descriptions and categories.",
            "Improved symbol panel touch targets and fixed invisible characters.",
            "Fixed an issue where some candidates were missing from search results.",
            "Updated dictionary data.",
        ]),
        ("3.4.4", "2026/03/09", [
            "Fixed an issue where some words could not be found when typing.",
        ]),
        ("3.4.2", "2026/03/08", [
            "Added custom dictionary for adding your own words.",
            "Added diagnostic info for easier bug reporting.",
            "Fixed custom dictionary entries not appearing in search results.",
            "Fixed custom dictionary capitalization not matching other candidates.",
            "Fixed word-grouped POJ display conversion.",
        ]),
        ("3.4.1", "2026/02/26", [
            "Added tone diacritic hints above number keys.",
            "Added punctuation hints on MOE1/MOE2 layout keys.",
            "Added MOE Layout 1 and MOE Layout 2 keyboards.",
            "Added TPS (方音符號) keyboard layout.",
            "Added keyboard appearance customization settings.",
            "Added in-keyboard layout selection panel.",
            "Added STTI (學科術語辭典) dictionary source.",
            "Added phrase learning for continuous word selections.",
            "Improved continuous typing with word-grouped display.",
            "Fixed POJ and TL input mode separation.",
        ]),
        ("3.4.0", "2025/12/31", [
            "Adjusted the four-syllable input limit.",
            "Fixed an issue where o͘ did not trigger candidate search.",
        ]),
        ("3.3.9", "2025/12/30", [
            "Added an English keyboard.",
            "Added explanations for 'Typing History Dictionary' and 'Capitalization Toggle.'",
            "Added more symbols to the punctuation keyboard.",
            "Added quick toggle shortcuts to the candidate bar.",
            "Adopted Apple’s default UI components.",
            "Fixed an issue where candidates did not appear when typing tone 1 or 4 directly.",
            "Fixed an issue where capitalization was not working correctly.",
            "Fixed inconsistent font sizing across different iPhone models.",
            "Refactored the codebase for better cleanliness and maintainability.",
            "Updated the app logo.",
            "Upgraded KeyboardKit to v10.",
        ]),
        ("3.3.8", "2025/12/25", [
            "Refreshed the app interface and improved in-app explanations for better clarity.",
        ]),
    ]

    // MARK: - 版權聲明

    static let copyrightNotice = "致謝"
    static let viewLicense = "授權條款"

    // 教育部臺灣台語常用詞辭典
    static let moeCopyright = "© 教育部"
    static let ccLicense = "CC BY-ND 3.0 TW"

    // iTaigi 華台辭典
    static let iTaigiCopyright = "© iTaigi愛台語"
    static let cc0License = "CC0"

    // 台語新詞辭庫
    static let newwordCopyright = "© 公視台語台"
    static let ccBy4License = "CC BY 4.0"

    // 粉圓字型
    static let openFontCopyright = "© justfont"
    static let silOpenFontLicense = "SIL Open Font License"

    // 芫荽字型
    static let iansuiFontCopyright = "© ButTaiwan"
    static let silOpenFontLicense11 = "SIL Open Font License 1.1"

    /// ButTaiwan 字型（源樣明體、源樣烏體、源泉圓體、源石黑體、源起明體、源起黑體、源雲明體）
    static let butTaiwanCopyright = "© ButTaiwan"

    // 台灣植物名彙
    static let taiwanPlantCopyright = "© 佐佐木舜一"
    static let ccBySA4License = "CC BY-SA 4.0"

    /// 台華線頂對照典
    static let taiHuaCopyright = "© 鄭良偉"

    // 台日大辭典
    static let taiwanJapanCopyright = "© 小川尚義"
    static let ccByNcSA3License = "CC BY-NC-SA 3.0 TW"

    // 台語工藝詞庫
    static let kunggeCopyright = "© 國立臺灣工藝研究發展中心"
    static let ccByNcLicense = "CC BY-NC 4.0"

    // 學科術語辭典
    static let sttiCopyright = "© 教育部"
    static let ogdlTaiwanLicense = "OGDL-Taiwan-1.0"

    /// 腔口差
    static let accentDictCredit = "實齋整理、提供"

    /// 詞庫增補檔案
    static let devSupplementCredit = "「建中」整理、提供"
}

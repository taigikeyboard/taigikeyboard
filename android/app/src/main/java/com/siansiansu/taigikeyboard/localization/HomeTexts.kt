package com.siansiansu.taigikeyboard.localization

/**
 * 頭頁文字
 * 包含：頭頁、啟用方法、新功能、FAQ、版本紀錄、關於開發者、版權聲明
 * 對應 iOS HomeTexts.swift
 */
object HomeTexts {
    // MARK: - Tab 標題

    const val tabTitle = "頭頁"

    // MARK: - 頁面標題

    const val appHeaderTitle = "台語齒盤"

    // MARK: - 區塊標題

    const val setupKeyboard = "齒盤愛拍開才會當使用"
    const val typingGuide = "拍字說明"
    const val newFeatures = "功能設定"
    const val faq = "其他"

    // MARK: - 啟用方法

    const val setupGuide = "啟用方法"
    const val setupGuideDescription = "手機仔系統規定第三方齒盤愛手動啟用才會當使用，請照下跤 ê 說明完成設定。"
    const val setupInfoMessage = "「允准完整取用」意思是予齒盤會當捌你揤 ê 動作，成做你拍 ê 字。請放心，App 袂紀錄你 ê 資料。"
    const val setupBrandWarning = "無仝牌子 ê 手機仔，設定 ê 方式可能會淡薄仔無仝款，毋過方式應該攏差不多。"

    const val setupGuideStartSetup = "啟用齒盤"

    // MARK: - 設定引導步驟（全螢幕模式使用）

    const val setupGuideCompletedMessage = "完成了後，重開你目前使用 ê App，予 App 重掠新 ê 齒盤清單。紲落來，佇會當拍字 ê 所在，揤牢地球圖示切去台語齒盤"
    const val setupGuideGoToSettings = "去設定頁"
    const val setupGuideCloseButton = "關閉"
    const val setupGuideStep1Settings = "點揤「齒盤」"
    const val setupGuideStep2AddKeyboard = "點揤「增加齒盤」、「允准完整取用」"

    // MARK: - 新功能 & FAQ
    // Feature and FAQ content (title, paragraphs, attachments) is now loaded from
    // assets/tab1-features.json and assets/tab1-faq.json via FeatureContentLoader.

    // MARK: - 資源連結

    const val userGuide = "網站紹介"
    const val rateUs = "為阮評分"
    const val aboutDeveloper = "關於"
    const val privacyPolicy = "隱私權政策"

    // MARK: - 關於開發者

    const val freePromise =
        "台語齒盤保證永遠免費，嘛袂做付費功能。台語是咱 ê 母語，無應該因為錢 ê 問題用袂著好家私。我向望逐家想欲學台語、寫台語 ê 人攏會當無負擔來使用，這是我做這个齒盤上重要 ê 心願。"
    const val officialWebsite = "官方網站"

    // MARK: - 版本資訊

    const val version = "當前版本"
    const val versionHistory = "版本紀錄"

    data class VersionEntry(
        val version: String,
        val date: String,
        val changes: List<String>,
    )

    val versionHistoryEntries =
        listOf(
            VersionEntry(
                "3.6.3",
                "2026/06/20",
                listOf(
                    "New: typing a tone in TPS (注音) now shows only that tone's readings, matching TL / POJ; typing without a tone still shows all tones.",
                    "New: TPS ninth tone (ˆ) can now be typed via the 9 digit key.",
                    "Fixed: TPS words with the ir vowel after ts / tsh / s / j (自 / 事 / 故事) can now be typed.",
                    "Fixed: typing a single initial in TL / POJ / TPS now surfaces common single characters again, not only longer words.",
                    "Fixed: English-mode autocorrect now works on devices without a system spell checker (e.g. some Samsung phones), using a bundled word list.",
                    "Fixed: key-press vibration now follows the in-app vibration toggle even when the system touch-vibration setting is off.",
                    "Fixed: backspace now deletes text in rich web editors such as Gmail and Google Chat message boxes.",
                    "Changed: the emoji set is refreshed.",
                    "Changed: 顯示羅馬字 (show romanization) now defaults off — turn it on in settings to keep the romanization you type as the first candidate.",
                ),
            ),
            VersionEntry(
                "3.6.2",
                "2026/06/10",
                listOf(
                    "New: keyboard themes — a new 主題 tab lets you pick a look. Built-in themes come in three key styles (經典 filled / 框線 outlined / 簡潔 borderless) across several colours, including soft gradients (櫻花 / 金煌 / 海風 / 翠青 / 藤紫) and a dark 暗眠山貓 (Catppuccin) theme.",
                    "New: create your own themes — a theme editor lets you set keyboard colours, key sizes, and key shadow, and save up to 5 custom themes.",
                    "New: 顯示羅馬字 (show romanization) is now a toggle — when on, the romanization you type appears as the first candidate; turn it off to free up candidate-strip space.",
                    "Fixed: the five light gradient themes now keep their light look in system dark mode, so key glyphs and candidate text stay readable.",
                    "Changed: the global keyboard font moved to 齒盤設定 (keyboard settings), separate from themes.",
                ),
            ),
            VersionEntry(
                "3.6.1",
                "2026/06/06",
                listOf(
                    "New: frequency, next-word prediction, and the custom dictionary now work consistently across TL / POJ / TPS — learn or add a word in one input mode and it's found and ranked in the others.",
                    "New: the same 漢字 with different readings (e.g. 重 tāng / tîng) now keeps separate frequency counts and ranks independently.",
                    "New: in TL / POJ the romanization itself appears as the first candidate, so you can mix 漢羅 (Han-Lo) and commit the roman spelling in one tap — no need to switch to 文/A.",
                    "Fixed: TL and POJ composing now shows exactly what you type — no automatic spelling conversion (teng stays teng).",
                    "Fixed: TPS first-tone continuous input now segments correctly, with or without a separating space (ㄍㄠㄉㄞ → 交代), and some words hidden by auto-correct (雞胸 / 烏白 / 包袱) now appear as candidates.",
                    "Fixed: whole-word continuous input now follows the dictionary's spacing / 輕聲 form (hoogua → 予我 hōo--guá), and next-word predictions learned via continuous input recall correctly and no longer appear twice.",
                    "Fixed: a leading 輕聲 -- is now treated as normal text — only the syllable underlines while composing.",
                    "Changed: the 詞頻 (frequency) list now shows each word's romanization with the 漢字, listing different readings separately.",
                    "Changed: with auto-space on, attaching punctuation after a committed word now keeps the space after the punctuation (guá? not guá ?).",
                    "Changed: the 姓名 (name) appendix dictionary source now defaults on.",
                    "Changed: the custom dictionary now caps at 30,000 entries (matching iOS); existing entries are kept.",
                    "The first candidate again shows a filled keycap-colour background hint.",
                ),
            ),
            VersionEntry(
                "3.6.0",
                "2026/05/31",
                listOf(
                    "New: each 教典 (MOE dictionary) subcollection is now its own toggle — turn individual 腔調 (accent) readings and the 姓名 (name) appendix on or off in the dictionary settings.",
                    "New: multi-character words now carry 語音差異 (per-accent) readings, not just single characters.",
                    "New: 詞庫增補檔案 — a new toggleable supplement source (~2,500 added words: 一府五院 / 菜市仔名 / 台臺 / 教典僻智識 / 數字時間日期 / 行政區).",
                    "Changed: turning a dictionary source off now also removes its words from the keyboard's candidates, not just the dictionary browse tab.",
                    "Changed: toggle switches now use the native Material 3 style.",
                    "Fixed: typing an explicit tone in continuous input (e.g. tai5) now shows only that tone's readings; typing without a tone (tai) still shows all tones.",
                    "Fixed: the keyboard no longer occasionally opens collapsed to just the candidate bar on cold start.",
                    "Updated dictionary data.",
                ),
            ),
            VersionEntry(
                "3.5.9",
                "2026/05/27",
                listOf(
                    "New: TPS (Bopomofo-Taiwanese) continuous input — type a whole TPS sentence and the keyboard segments it into candidates, same as TL / POJ.",
                    "Fixed: toneless TPS strings whose syllables start with a medial (e.g. ㄉㄞㄨㄢ for 台灣) no longer return zero candidates.",
                    "Fixed: TPS auto-correct no longer corrupts the next initial after a precomposed nasal coda (ㄉㄞㄨㄢㄉ … no longer becomes ㄉㄞㄨㄢㆵ …).",
                    "Fixed: typing a lone Bopomofo initial like ㄉ now returns prefix-matched candidates instead of nothing.",
                    "Fixed: continuous-input in POJ mode now stays POJ end-to-end — POJ phrases no longer fall back to TL display, and user-frequency learning is shared between POJ and TL.",
                    "Fixed: short common words no longer dropped from the partial-prefix candidate list under load.",
                    "Fixed: cold-start preference reads no longer risk an ANR.",
                    "Updated dictionary data.",
                ),
            ),
            VersionEntry(
                "3.5.8",
                "2026/05/20",
                listOf(
                    "New: 連續輸入 — type a whole romanized phrase without committing each syllable; the keyboard segments the sentence and offers candidates per position. Tap to commit a segment, Enter to commit the raw text. Works with POJ / TL, the custom dictionary, user-frequency learning, and next-word prediction.",
                    "Continuous-input candidates now show romanization and 漢字 on two lines.",
                    "Replaced the 寄付支持 tab with a 關於開發者 page linking to the official website.",
                    "Fixed: feature/FAQ icons and slideshow images were missing on release builds.",
                    "Fixed: the keyboard preview in Settings now renders the key shapes.",
                    "Updated dictionary data.",
                ),
            ),
            VersionEntry(
                "3.5.7",
                "2026/05/09",
                listOf(
                    "Restored support for older 32-bit ARM devices (armeabi-v7a).",
                    "Fixed: keyboard crash on release builds caused by ProGuard/R8 stripping Rust engine classes.",
                    "Renamed font display name 源樣黑體 → 源樣烏體 (matches official ButTaiwan naming).",
                    "Updated dictionary data — regenerated POJ entries and corrected the Tab3 (Hanji) character range.",
                    "Internal: keyboard UI rewritten in Jetpack Compose (no behavior change).",
                ),
            ),
            VersionEntry(
                "3.5.6",
                "2026/05/02",
                listOf(
                    "Internal: dictionary read path rewritten in Rust (no behavior change).",
                ),
            ),
            VersionEntry(
                "3.5.5",
                "2026/05/02",
                listOf(
                    "Internal: next-word prediction engine rewritten in Rust (no behavior change).",
                ),
            ),
            VersionEntry(
                "3.5.4",
                "2026/05/01",
                listOf(
                    "Internal: composing buffer engine rewritten in Rust (no behavior change).",
                ),
            ),
            VersionEntry(
                "3.5.3",
                "2026/04/29",
                listOf(
                    "Internal: engine workspace cleanup, removed duplicate platform implementations (no behavior change).",
                ),
            ),
            VersionEntry(
                "3.5.2",
                "2026/04/29",
                listOf(
                    "Internal: candidate ranking pipeline rewritten in Rust (no behavior change).",
                ),
            ),
            VersionEntry(
                "3.5.1",
                "2026/04/28",
                listOf(
                    "Internal: phonetics conversion engine rewritten in Rust (no behavior change).",
                ),
            ),
            VersionEntry(
                "3.5.0",
                "2026/04/25",
                listOf(
                    "Added 源樣明體 (serif) and 源樣烏體 (sans-serif) font options.",
                    "Restored POJ candidates for words containing o͘ / ⁿ — about 21% of romanization queries had been missing matches.",
                    "Fixed candidates occasionally remaining on screen after backspacing.",
                    "Fixed emoji key not committing the active candidate before inserting the emoji.",
                    "Updated dictionary data.",
                ),
            ),
            VersionEntry(
                "3.4.9",
                "2026/04/12",
                listOf(
                    "Reduced Android app size by 75% (48 MB → 13 MB).",
                    "Increased candidate display limit from 100 to 200.",
                ),
            ),
            VersionEntry(
                "3.4.8",
                "2026/04/11",
                listOf(
                    "Fixed an issue where app storage grew excessively over time.",
                ),
            ),
            VersionEntry(
                "3.4.7",
                "2026/04/05",
                listOf(
                    "Added globe key toggle to show or hide the keyboard switch key.",
                    "Added custom dictionary enable/disable toggle.",
                    "Added frequency recording toggle to enable or disable word frequency tracking.",
                    "Added association recording toggle to enable or disable next-word prediction learning.",
                    "Added CSV import/export for word frequency and association data.",
                    "Rewrote feature guides with detailed input mode and romanization tutorials.",
                    "Settings now show info buttons with feature descriptions.",
                    "Dictionary settings now show source descriptions and links.",
                    "Custom dictionary now shows CSV format example.",
                    "Swipe to delete custom dictionary entries and frequency data.",
                    "Added Samsung keyboard quick-switch FAQ.",
                    "Fixed TPS input incorrectly joining syllables after tone marks.",
                    "Fixed certain POJ words with o͘ not showing candidates.",
                    "Fixed duplicate candidates appearing in TPS mode.",
                    "Fixed a rare crash in English spell-check.",
                    "Fixed symbols like arrows incorrectly entering composition mode.",
                    "Updated keyboard layout preview images.",
                    "Updated the app icon.",
                ),
            ),
            VersionEntry(
                "3.4.6",
                "2026/03/24",
                listOf(
                    "Added in-keyboard settings panel accessible from toolbar.",
                    "Added toolbar auto-collapse toggle in settings.",
                    "TPS layout auto-corrects palatalized initials (ㄗ+ㄧ→ㄐ, ㄘ+ㄧ→ㄑ, etc.).",
                    "TPS layout: digit keys accessible via long-press on row 1.",
                    "Improved TPS syllable boundary detection for more accurate input.",
                    "Fixed toolbar settings toggles not taking effect until restart.",
                    "Fixed backspace showing romanization after selecting a custom dictionary word.",
                    "Enabled 台語工藝詞庫 and 學科術語辭典 by default.",
                ),
            ),
            VersionEntry(
                "3.4.5",
                "2026/03/21",
                listOf(
                    "Added symbol selection panel for inserting special characters.",
                    "Added dictionary search in the dictionary settings page.",
                    "TPS (方音符號) keyboard layout is now available.",
                    "TPS layout auto-selects ㄇ/ㄫ initial and final forms based on context.",
                    "Added dismiss keyboard button in toolbar.",
                    "Improved number key input.",
                    "Redesigned dictionary settings page with descriptions and categories.",
                    "Improved symbol panel touch targets.",
                    "Fixed an issue where some candidates were missing from search results.",
                    "Added reset all settings option.",
                    "Updated dictionary data.",
                ),
            ),
            VersionEntry(
                "3.4.4",
                "2026/03/09",
                listOf(
                    "Fixed an issue where some words could not be found when typing.",
                ),
            ),
            VersionEntry(
                "3.4.2",
                "2026/03/08",
                listOf(
                    "Added custom dictionary for adding your own words.",
                    "Added diagnostic info for easier bug reporting.",
                    "Changed app font to 粉圓 (jf-openhuninn).",
                    "Redesigned candidate display with title and subtitle.",
                    "Fixed custom dictionary entries not appearing in search results.",
                    "Fixed custom dictionary capitalization not matching other candidates.",
                    "Fixed word-grouped POJ display conversion.",
                ),
            ),
            VersionEntry(
                "3.4.1",
                "2026/02/26",
                listOf(
                    "Added tone diacritic hints above number keys.",
                    "Added punctuation hints on MOE1/MOE2 layout keys.",
                    "Added MOE Layout 1 and MOE Layout 2 keyboards.",
                    "Added keyboard appearance customization settings.",
                    "Added input mode label (POJ/TL/EN) on the space bar.",
                    "Added STTI (學科術語辭典) dictionary source.",
                    "Added phrase learning for continuous word selections.",
                    "Improved keyboard typing performance.",
                    "Fixed POJ and TL input mode separation.",
                ),
            ),
            VersionEntry(
                "3.4.0",
                "2025/12/31",
                listOf(
                    "Adjusted the four-syllable input limit.",
                    "Fixed an issue where o͘ did not trigger candidate search.",
                ),
            ),
            VersionEntry(
                "3.3.9",
                "2025/12/30",
                listOf(
                    "Added an English keyboard.",
                    "Added explanations for 'Typing History Dictionary' and 'Capitalization Toggle'.",
                    "Added more symbols to the punctuation keyboard.",
                    "Added quick toggle shortcuts to the candidate bar.",
                    "Adopted Android's default UI components.",
                    "Fixed an issue where candidates did not appear when typing tone 1 or 4 directly.",
                    "Fixed an issue where capitalization was not working correctly.",
                    "Fixed inconsistent font sizing across different devices.",
                    "Refactored the codebase for better cleanliness and maintainability.",
                    "Updated the app logo.",
                ),
            ),
            VersionEntry(
                "3.3.8",
                "2025/12/25",
                listOf(
                    "Refreshed the app interface and improved in-app explanations for better clarity.",
                ),
            ),
        )

    // MARK: - 版權聲明

    const val copyrightNotice = "致謝"
    const val viewLicense = "授權條款"

    // 教育部臺灣台語常用詞辭典
    const val moeCopyright = "© 教育部"
    const val ccLicense = "CC BY-ND 3.0 TW"

    // iTaigi 華台辭典
    const val iTaigiCopyright = "© iTaigi愛台語"
    const val cc0License = "CC0"

    // 台語新詞辭庫
    const val newwordCopyright = "© 公視台語台"
    const val ccBy4License = "CC BY 4.0"

    // 粉圓字型
    const val openFontTitle = "粉圓"
    const val openFontCopyright = "© justfont"
    const val silOpenFontLicense = "SIL Open Font License"

    // 芫荽字型
    const val iansuiFontTitle = "芫荽"
    const val iansuiFontCopyright = "© ButTaiwan"
    const val silOpenFontLicense11 = "SIL Open Font License 1.1"

    // ButTaiwan 字型（源樣明體、源樣烏體、源泉圓體、源石黑體、源起明體、源起黑體、源雲明體）
    const val butTaiwanCopyright = "© ButTaiwan"
    const val genYoMinFontTitle = "源樣明體"
    const val genYoGothicFontTitle = "源樣烏體"

    // 台灣植物名彙
    const val taiwanPlantCopyright = "© 佐佐木舜一"
    const val ccBySA4License = "CC BY-SA 4.0"

    // 台華線頂對照典
    const val taiHuaCopyright = "© 鄭良偉"

    // 台日大辭典
    const val taiwanJapanCopyright = "© 小川尚義"
    const val ccByNcSA3License = "CC BY-NC-SA 3.0 TW"

    // 台語工藝詞庫
    const val kunggeCopyright = "© 國立臺灣工藝研究發展中心"
    const val ccByNcLicense = "CC BY-NC 4.0"

    // 學科術語辭典
    const val sttiCopyright = "© 教育部"
    const val ogdlTaiwanLicense = "OGDL-Taiwan-1.0"

    // 腔口差
    const val accentDictCredit = "實齋整理、提供"

    // 詞庫增補檔案
    const val devSupplementCredit = "建中整理、提供"
}

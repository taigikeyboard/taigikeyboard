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
    val newFeatures = LocalizedText(hanji = "功能解說")
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

    // MARK: - 新功能

    val featureNextWord = LocalizedText(hanji = "連紲建議詞")
    val featureVariant = LocalizedText(hanji = "異用字開關")
    val featureCustomFont = LocalizedText(hanji = "詞庫管理")

    val featureNextWordParagraphs = listOf(
        LocalizedText(hanji = "拍字會連紲建議，譬如講拍「天」這个字，齒盤會出現連紲適合 ê 詞：「天」 ⭢ 「烏 」 ⭢ 「烏」 ⭢ 「欲」⭢ 「落」 ⭢ 「雨」，毋免去想 2 个以上 ê 音節按怎拍。"),
        LocalizedText(hanji = "詞庫無 ê 字，若拍過 1 改，後擺著會自動出現佇「連紲建議詞」，譬如拍「我想欲食飯」，以後著會記起來。"),
        LocalizedText(hanji = "拍字時空格縫佮連劃 '-' 愛家己揤，若「連紲建議」揤傷緊，袂記得揤連劃，羅馬字著會黏做伙，「自動空白」開關若有切開，羅馬字著會使連紲拍，毋免家己加空格縫。")
    )

    val featureVariantParagraphs = listOf(
        LocalizedText(hanji = "依據教典 ê 資料標示台語異用字，譬如：「人」->「儂」、「生」->「青」，這个開關預設是關起來。"),
        LocalizedText(hanji = "因為教典異用字 ê 資料有欠，所以可能會落勾無標示著，若拄著這个情形著愛家己主動加字入去，請回報問題予我知。")
    )
    val featureVariantDictLink = LocalizedText(hanji = "揤遮到教典網站掠辭典資料")

    val featureCustomFontParagraphs = listOf(
        LocalizedText(hanji = "佇「詞庫」頁面會使選拍字使用 ê 詞庫，無仝詞庫收錄 ê 字有依家己 ê 特色，愛會記得調整。"),
        LocalizedText(hanji = "「台語常用詞辭典」、「台語新詞題庫」、「台語工藝詞庫」較倚教典標準，若欲比賽建議開這 3 个著好，「iTaigi 愛台語」內底 ê 詞較有爭議，預設關起來。"),
        LocalizedText(hanji = "辭典 ê 詞是半自動、半人工校對誠厚工，若有問題請回報問題予我知。")
    )

    val featureUserDict = LocalizedText(hanji = "拍字記持詞庫")
    val featureUserDictParagraphs = listOf(
        LocalizedText(hanji = "台語齒盤有特別設計 1 个拍字記持詞庫，本身無分白話字佮台羅，拍過 ê 字攏會記起來，若較捷拍著會出現佇頭前，若愈久無拍，著會沓沓仔排佇後壁。"),
        LocalizedText(hanji = "「拍字記持詞庫」是專門予「連紲建議詞」使用。佇白話字模式，台羅辭典會自動關起來，「連紲建議詞」就袂出現台羅辭典 ê 詞。但是「拍字記持詞庫」白話字佮台羅攏會顯示。"),
        LocalizedText(hanji = "捷用詞出現頻率是用「拍過幾改」和「偌久無拍」決定 ê，若感覺字攏無出現，咱會當討論看算式 ê 權重按怎調整。")
    )

    val featureCaseSwitch = LocalizedText(hanji = "3段式大小寫切換")
    val featureCaseSwitchParagraphs = listOf(
        LocalizedText(hanji = "一般 ê 情況第 1 个字會自動大本字，shift 揤鈕會反烏，但是若「自動大本字」有關起來，著愛家己揤 shift 揤鈕，第 1 個字才會變大本字。"),
        LocalizedText(hanji = "「自動大本字」開關關起來是小寫模式，拍出來 ê 字攏是小寫。"),
        LocalizedText(hanji = "連紲揤 Shift 鍵 2 改是 Caps Lock 模式，shift 揤鈕是烏色，圖示嘛無仝款，這時陣拍出來 ê 字攏會變大本字，閣揤 1 改才會改轉來小寫。"),
        LocalizedText(hanji = "若連紲切換符號齒盤，大小寫有時陣會 sio͘h-to͘h，咱先試驗看覓，若問題誠嚴重，閣來排時間修理。")
    )

    // MARK: - FAQ

    val faq1Question = LocalizedText(hanji = "齒盤裝好了後無出現")
    val faq1Paragraphs = listOf(
        LocalizedText(hanji = "1. 檢查齒盤敢有照「啟用方法」ê 方式拍開。"),
        LocalizedText(hanji = "2. 紲落來請重開你目前使用 ê App，予 App 重掠新 ê 齒盤清單。"),
        LocalizedText(hanji = "3. 重開了後，齒盤應該會出現，佇會當拍字 ê 所在，揤牢地球圖示切去台語齒盤。")
    )

    val faq2Question = LocalizedText(hanji = "回報 ê 問題無消息")
    val faq2Paragraphs = listOf(
        LocalizedText(hanji = "可能無小心會落勾，koh 回報 1 遍，抑是直接聯絡我問無要緊。")
    )

    val faq3Question = LocalizedText(hanji = "按怎拍聲調 1、4")
    val faq3Paragraphs = listOf(
        LocalizedText(hanji = "拍聲調 1，會正確出現無聲調符號 ê 字，袂和其他 ê 字濫做伙，拍尾溜是 -p, -t, -k, -h ê 字加聲調 4，會正確出現無聲調符號 ê 字。"),
        LocalizedText(hanji = "雖然是無聲調標號，但是佇拍字 ê 所在有數字 1、4 點注，若欲直接拍無聲調無欲選字，毋免加數字，拍了後揤 Enter 著會使。")
    )

    val goToSetupGuide = LocalizedText(hanji = "揤遮去看「啟用方法」")
    val goToFeedback = LocalizedText(hanji = "揤遮去看「問題回報」")

    // MARK: - 資源連結

    val userGuide = LocalizedText(hanji = "網站紹介")
    val rateUs = LocalizedText(hanji = "為阮評分")
    val contactUs = LocalizedText(hanji = "問題回報")
    val privacyPolicy = LocalizedText(hanji = "隱私權政策")

    // MARK: - 問題回報

    val feedbackDescription = LocalizedText(hanji = "無論是使用拄著 ê 問題、感覺好用 ê 所在，抑是會當改進 ê 建議，攏歡迎寫落來！影片、圖會使直接寄批去 info@taigikeyboard.tw")
    val goToGoogleForm = LocalizedText(hanji = "揤遮去 Google 表單")
    val emailContact = LocalizedText(hanji = "台語齒盤是 1 人團隊，目前由我 1 个人塌錢開發佮維護，因為有你 ê 贊助，予我有氣力繼續行落去，咱做伙為著台語打拼。")
    val supportUs = LocalizedText(hanji = "支持台語齒盤")

    // MARK: - 版本資訊

    val version = LocalizedText(hanji = "當前版本")
    val versionHistory = LocalizedText(hanji = "版本紀錄")

    data class VersionEntry(
        val version: String,
        val date: String,
        val changes: List<LocalizedText>
    )

    val versionHistoryEntries = listOf(
        VersionEntry("3.4.1", "2026/02/26", listOf(
            LocalizedText(hanji = "Added tone diacritic hints above number keys."),
            LocalizedText(hanji = "Added punctuation hints on MOE1/MOE2 layout keys."),
            LocalizedText(hanji = "Added MOE Layout 1 and MOE Layout 2 keyboards."),
            LocalizedText(hanji = "Added keyboard appearance customization settings."),
            LocalizedText(hanji = "Added input mode label (POJ/TL/EN) on the space bar."),
            LocalizedText(hanji = "Added STTI (學科術語辭典) dictionary source."),
            LocalizedText(hanji = "Added phrase learning for continuous word selections."),
            LocalizedText(hanji = "Improved keyboard typing performance."),
            LocalizedText(hanji = "Fixed POJ and TL input mode separation."),
        )),
        VersionEntry("3.4.0", "2025/12/31", listOf(
            LocalizedText(hanji = "Adjusted the four-syllable input limit."),
            LocalizedText(hanji = "Fixed an issue where o͘ did not trigger candidate search."),
        )),
        VersionEntry("3.3.9", "2025/12/30", listOf(
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
        )),
        VersionEntry("3.3.8", "2025/12/25", listOf(
            LocalizedText(hanji = "Refreshed the app interface and improved in-app explanations for better clarity."),
        ))
    )

    // MARK: - 版權聲明

    val copyrightNotice = LocalizedText(hanji = "致謝")
    val viewLicense = LocalizedText(hanji = "授權條款")
    val viewWebsite = LocalizedText(hanji = "官方網站")

    // 教育部臺灣台語常用詞辭典
    val moeDict = LocalizedText(hanji = "台語常用詞辭典 - 教育部")
    val moeCopyright = LocalizedText(hanji = "© 教育部")
    val ccLicense = LocalizedText(hanji = "CC BY-ND 3.0 TW")

    // iTaigi 華台辭典
    val iTaigiDict = LocalizedText(hanji = "iTaigi愛台語 - 群眾台語辭典")
    val iTaigiCopyright = LocalizedText(hanji = "© iTaigi愛台語")
    val cc0License = LocalizedText(hanji = "CC0")

    // 台語新詞辭庫
    val newwordDict = LocalizedText(hanji = "台語新詞辭庫 - 公視台語台")
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
    val taiwanJapanDict = LocalizedText(hanji = "台日大辭典")
    val taiwanJapanCopyright = LocalizedText(hanji = "© 小川尚義")
    val ccByNcSA3License = LocalizedText(hanji = "CC BY-NC-SA 3.0 TW")

    // 台語工藝詞庫
    val kunggeDict = LocalizedText(hanji = "台語工藝詞庫 - 工藝中心")
    val kunggeCopyright = LocalizedText(hanji = "© 國立臺灣工藝研究發展中心")
    val ccByNcLicense = LocalizedText(hanji = "CC BY-NC 4.0")

    // 學科術語辭典
    val sttiDict = LocalizedText(hanji = "學科術語辭典 - 教育部")
    val sttiCopyright = LocalizedText(hanji = "© 教育部")
    val ogdlTaiwanLicense = LocalizedText(hanji = "OGDL-Taiwan-1.0")

    // 腔口補充辭典
    val accentDict = LocalizedText(hanji = "腔口補充辭典")
    val accentDictCredit = LocalizedText(hanji = "「實齋」整理、提供")
}

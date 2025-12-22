package com.siansiansu.taigikeyboard.settings

data class LocalizedText(
    val hanji: String,
    val poj: String,
    val tl: String
) {
    fun text(language: DisplayLanguage): String {
        return when (language) {
            DisplayLanguage.HANJI -> hanji
            DisplayLanguage.POJ -> poj
            DisplayLanguage.TL -> tl
        }
    }
}

object AppTexts {
    val keyboardSettings = LocalizedText(
        hanji = "齒盤設定",
        poj = "Khí-pôaⁿ siat-tēng",
        tl = "Khí-puânn siat-tīng"
    )

    val inputMode = LocalizedText(
        hanji = "輸入模式",
        poj = "Su-ji̍p bô͘-sek",
        tl = "Su-ji̍p bôo-sik",
    )

    val pojMode = LocalizedText(
        hanji = "白話字",
        poj = "Pe̍h-ōe-jī",
        tl = "Pe̍h-uē-jī"
    )

    val tlMode = LocalizedText(
        hanji = "台羅",
        poj = "Tâi-lô",
        tl = "Tâi-lô"
    )

    val showHanji = LocalizedText(
        hanji = "使用漢字",
        poj = "Sú-iōng hàn-jī",
        tl = "Sú-iōng hàn-jī"
    )

    val outputBothScripts = LocalizedText(
        hanji = "括號標註",
        poj = "Koat-hō phiau-chù",
        tl = "Kuat-hō phiau-tsù"
    )

    val autoCapitalization = LocalizedText(
        hanji = "自動大寫",
        poj = "Chū-tōng tōa-siá",
        tl = "Tsū-tōng tuā-siá"
    )

    val autoSpace = LocalizedText(
        hanji = "自動空白",
        poj = "Chū-tōng khang-pe̍h",
        tl = "Tsū-tōng khang-pe̍h"
    )

    val inputSettings = LocalizedText(
        hanji = "拍字設定",
        poj = "Phah-jī siat-tēng",
        tl = "Phah-jī siat-tīng"
    )

    val layoutSettings = LocalizedText(
        hanji = "佈局設定",
        poj = "Pò͘-kio̍k siat-tēng",
        tl = "Pòo-kio̍k siat-tīng"
    )

    val doubleTapCombination = LocalizedText(
        hanji = "連紲拍 (限白話字)",
        poj = "Liân-sòa phah (hān Pe̍h-ōe-jī)",
        tl = "Liân-suà phah (hān Pe̍h-uē-jī)",
    )

    val doubleTapOO = LocalizedText(
        hanji = "連紲拍 oo → o͘",
        poj = "Liân-sòa phah oo → o͘",
        tl = "Liân-suà phah oo → o͘"
    )

    val doubleTapNN = LocalizedText(
        hanji = "連紲拍 nn → ⁿ",
        poj = "Liân-sòa phah nn → ⁿ",
        tl = "Liân-suà phah nn → ⁿ"
    )

    val customFont = LocalizedText(
        hanji = "字骨設定",
        poj = "Jī-hêng siat-tēng",
        tl = "Jī-hîng siat-tīng"
    )

    val fontSystemDefault = LocalizedText(
        hanji = "系統",
        poj = "Hē-thóng ī-siat",
        tl = "Hē-thóng ī-siat"
    )

    val fontOpenHuninn = LocalizedText(
        hanji = "粉圓",
        poj = "Open Hún-îⁿ",
        tl = "Open Hún-înn"
    )

    val fontIansui = LocalizedText(
        hanji = "芫荽",
        poj = "Iân-sui",
        tl = "Iân-sui"
    )

    val fontSystemWarning = LocalizedText(
        hanji = "使用系統字型可能會有豆腐字",
        poj = "Sú-iōng hē-thóng jī-hêng khó-lêng ē ū tāu-hū-jī",
        tl = "Sú-iōng hē-thóng jī-hîng khó-lîng ē ū tāu-hū-jī"
    )

    val phahTaigiLayout = LocalizedText(
        hanji = "PhahTaigi 齒盤",
        poj = "PhahTaigi khí-pôaⁿ",
        tl = "PhahTaigi khí-puânn"
    )

    val clearCache = LocalizedText(
        hanji = "挕掉捷用詞紀錄",
        poj = "Chheng-tû chu-liāu",
        tl = "Tshing-tû tsu-liāu"
    )

    val clearCacheMessage = LocalizedText(
        hanji = "這个動作會挕掉所有捷用詞 ê 記錄。敢欲繼續？",
        poj = "Che ē tōng-chok ē chheng-tû só͘-ū chia̍p-iōng-sû ê kì-lo̍k. Kám beh kè-sio̍k?",
        tl = "Tse ē tōng-tsok ē tshing-tû sóo-ū tsia̍p-iōng-sû ê kì-lo̍k. Kám beh kè-sio̍k?"
    )

    val clear = LocalizedText(
        hanji = "清除",
        poj = "Chheng-tû",
        tl = "Tshing-tû"
    )

    val cancel = LocalizedText(
        hanji = "取消",
        poj = "Chhú-siau",
        tl = "Tshú-siau"
    )

    val resetSettings = LocalizedText(
        hanji = "恢復設定",
        poj = "Khoe-ho̍k siat-tēng",
        tl = "Khue-ho̍k siat-tīng"
    )

    val resetSettingsMessage = LocalizedText(
        hanji = "這个動作會恢復所有設定，閣會清除所有記錄。敢欲繼續？",
        poj = "Che ê tōng-chok ē khoe-ho̍k só͘-ū siat-tēng, koh ē chheng-tû só͘-ū kì-lo̍k. Kám beh kè-sio̍k?",
        tl = "Tse ê tōng-tsok ē khue-ho̍k sóo-ū siat-tīng, koh ē tshing-tû sóo-ū kì-lo̍k. Kám beh kè-sio̍k?"
    )

    val reset = LocalizedText(
        hanji = "恢復",
        poj = "Khoe-ho̍k",
        tl = "Khue-ho̍k"
    )

    val confirm = LocalizedText(
        hanji = "選",
        poj = "Soán",
        tl = "Suán"
    )

    // Home Screen texts
    val appTitle = LocalizedText(
        hanji = "台語齒盤",
        poj = "Tâi-gí Khí-pôaⁿ",
        tl = "Tâi-gí Khí-puânn"
    )

    val setupGuide = LocalizedText(
        hanji = "啟用方法",
        poj = "Khé-iōng hong-hoat",
        tl = "Khé-iōng hong-huat"
    )

    val copyrightNotice = LocalizedText(
        hanji = "版權聲明",
        poj = "Pán-khoân seng-bêng",
        tl = "Pán-khuân sing-bîng"
    )

    // 詞庫管理
    val dictionarySettings = LocalizedText(
        hanji = "詞庫管理",
        poj = "Sû-khò͘ siat-tēng",
        tl = "Sû-khòo siat-tīng"
    )

    val customDictionary = LocalizedText(
        hanji = "自訂詞庫",
        poj = "Chū-tēng sû-khò͘",
        tl = "Tsū-tīng sû-khòo"
    )

    val addWord = LocalizedText(
        hanji = "新增詞條",
        poj = "Sin-cheng sû-tiâu",
        tl = "Sin-tsing sû-tiâu"
    )

    val editWord = LocalizedText(
        hanji = "編輯詞條",
        poj = "Pian-chi̍p sû-tiâu",
        tl = "Pian-tsi̍p sû-tiâu"
    )

    val deleteWord = LocalizedText(
        hanji = "刪除詞條",
        poj = "Thâi-tû sû-tiâu",
        tl = "Thâi-tû sû-tiâu"
    )

    val inputKey = LocalizedText(
        hanji = "Lô-má-jī",
        poj = "Su-ji̍p",
        tl = "Su-ji̍p"
    )

    val outputValue = LocalizedText(
        hanji = "漢字/漢羅",
        poj = "Su-chhut",
        tl = "Su-tshut"
    )

    val duplicateError = LocalizedText(
        hanji = "這組詞條已經存在",
        poj = "Chit cho͘ sû-tiâu í-keng chûn-chāi",
        tl = "Tsit tsoo sû-tiâu í-king tsûn-tsāi"
    )

    val maxLimitError = LocalizedText(
        hanji = "已達上限 200 筆",
        poj = "Í ta̍t siōng-hān 200 pit",
        tl = "Í ta̍t siōng-hān 200 pit"
    )

    val emptyFieldError = LocalizedText(
        hanji = "請輸入內容",
        poj = "Chhiáⁿ su-ji̍p lōe-iông",
        tl = "Tshiánn su-ji̍p luē-iông"
    )

    val deleteConfirmMessage = LocalizedText(
        hanji = "敢欲刪除這筆詞條？",
        poj = "Kám beh thâi-tû chit pit sû-tiâu?",
        tl = "Kám beh thâi-tû tsit pit sû-tiâu?"
    )

    val delete = LocalizedText(
        hanji = "刪除",
        poj = "Thâi-tû",
        tl = "Thâi-tû"
    )

    val save = LocalizedText(
        hanji = "儲存",
        poj = "Thú-chûn",
        tl = "Thú-tsûn"
    )

    val noCustomWords = LocalizedText(
        hanji = "無自訂詞條",
        poj = "Bô chū-tēng sû-tiâu",
        tl = "Bô tsū-tīng sû-tiâu"
    )

    val comingSoon = LocalizedText(
        hanji = "當 leh 開發",
        poj = "Kèng-chhiáⁿ kî-thāi",
        tl = "Kìng-tshiánn kî-thāi"
    )

    // 異用字開關
    val variantDictionary = LocalizedText(
        hanji = "異用字",
        poj = "Ī-iōng-jī",
        tl = "Ī-iōng-jī"
    )

    val contactUs = LocalizedText(
        hanji = "意見回饋",
        poj = "Ì-kiàn hôe-kūi",
        tl = "Ì-kiàn huê-kuī"
    )

    val websiteIntro = LocalizedText(
        hanji = "網站紹介",
        poj = "Bāng-chām siāu-kài",
        tl = "Bāng-tsām siāu-kài"
    )

    val rateUs = LocalizedText(
        hanji = "為阮評分",
        poj = "Uī goán phêng-hun",
        tl = "Uī guán phîng-hun"
    )

    val shareToFriends = LocalizedText(
        hanji = "分享予朋友",
        poj = "Hun-hióng hō͘ pêng-iú",
        tl = "Hun-hióng hōo pîng-iú"
    )

    // Onboarding texts
    val onboardingWelcomeTitle = LocalizedText(
        hanji = "歡迎使用台語齒盤",
        poj = "Hoan-gêng sú-iōng Tâi-gí Khí-pôaⁿ",
        tl = "Huan-gîng sú-iōng Tâi-gí Khí-puânn"
    )

    val onboardingWelcomeMessage = LocalizedText(
        hanji = "台語齒盤是專為台語使用者設計 ê 拍字家私，幫助咱用台語表達思想，傳承台語文化",
        poj = "Tâi-gí Khí-pôaⁿ sī choan ūi Tâi-gí sú-iōng-chiá siat-kè ê phah-jī ke-si. Pang-chō͘ lán iōng Tâi-gí piáu-ta̍t su-sióng thoân-sêng Tâi-gí bûn-hòa",
        tl = "Tâi-gí Khí-puânn sī tsuan uī Tâi-gí sú-iōng-tsiá siat-kè ê phah-jī ke-si. Pang-tsōo lán iōng Tâi-gí piáu-ta̍t su-sióng thuân-sîng Tâi-gí bûn-huà"
    )

    val onboardingStartSetup = LocalizedText(
        hanji = "啟用齒盤",
        poj = "Khé-iōng Khí-pôaⁿ",
        tl = "Khé-iōng Khí-puânn"
    )

    val onboardingAddKeyboardTitle = LocalizedText(
        hanji = "加添台語齒盤",
        poj = "Cheng-ka Tâi-gí Khí-pôaⁿ",
        tl = "Tsing-ka Tâi-gí Khí-puânn"
    )

    val setupInfoMessage = LocalizedText(
        hanji = "「允准完整取用」意思是予齒盤會當捌你揤 ê 動作，成做你拍 ê 字。請放心，App 袂紀錄你 ê 資料。",
        poj = "'Ín-chún oân-chéng chhú-iōng' sī hō͘ khí-pôaⁿ ē-tàng bat lí chhi̍h ê tōng-chok, chiâⁿ-chò lí phah ê jī. Chhiáⁿ hòng-sim, APP bōe siu-chi̍p lí ê chu-liāu.",
        tl = "'Ín-tsún uân-tsíng tshú-iōng' sī hōo khí-puânn ē-tàng bat lí tshi̍h ê tōng-tsok, tsiânn-tsò lí phah ê jī. Tshián hòng-sim, APP buē siu-tsi̍p lí ê tsu-liāu."
    )

    val onboardingKeyboardNotEnabled = LocalizedText(
        hanji = "增加齒盤",
        poj = "Cheng-ka khí-pôaⁿ",
        tl = "Tsing-ka Khí-puânn"
    )

    val onboardingKeyboardEnabled = LocalizedText(
        hanji = "增加齒盤 已完成",
        poj = "Cheng-ka khí-pôaⁿ í oân-sêng",
        tl = "Tsing-ka Khí-puânn í uân-sîng"
    )

    val onboardingGoToSettings = LocalizedText(
        hanji = "去設定頁",
        poj = "Khì Siat-tēng ia̍h",
        tl = "Khì Siat-tīng ia̍h"
    )

    val onboardingCompletedTitle = LocalizedText(
        hanji = "設定完成！",
        poj = "Siat-tēng oân-sêng!",
        tl = "Siat-tīng uân-sîng!"
    )

    val onboardingCompletedMessage = LocalizedText(
        hanji = "重開你目前使用 ê APP，予 APP 載入新 ê 齒盤清單。閣來，佇會當拍字 ê 所在，揤牢地球圖示切去台語齒盤",
        poj = "Tī ē-tàng phah-jī ê só͘-chāi, chhi̍h-tiâu tē-kiû tô͘-sī chhiat khì Tâi-gí khí-pôaⁿ",
        tl = "Tī ē-tàng phah-jī ê sóo-tsāi, tshi̍h-tiâu tē-kiû tôo-sī tshiat khì Tâi-gí Khí-puânn"
    )

    val onboardingGetStarted = LocalizedText(
        hanji = "開始使用",
        poj = "Khai-sí Sú-iōng",
        tl = "Khai-sí Sú-iōng"
    )

    // Copyright texts
    val copyrightTitle = LocalizedText(
        hanji = "版權聲明",
        poj = "Pán-khoân seng-bêng",
        tl = "Pán-khuân sing-bîng"
    )

    val guidePreviousPage = LocalizedText(
        hanji = "頂一頁",
        poj = "Téng chi̍t ia̍h",
        tl = "Tíng tsi̍t ia̍h"
    )

    val guideNextPage = LocalizedText(
        hanji = "後一頁",
        poj = "Āu chi̍t ia̍h",
        tl = "Āu tsi̍t ia̍h"
    )

    val done = LocalizedText(
        hanji = "完成",
        poj = "Oân-sêng",
        tl = "Uân-sîng"
    )

    val viewLicense = LocalizedText(
        hanji = "授權條款",
        poj = "Chhâ-khòaⁿ siū-khoân tiâu-khoán",
        tl = "Tshâ-khuànn siū-khuân tiâu-khuán"
    )

    val viewWebsite = LocalizedText(
        hanji = "官方網站",
        poj = "Koaⁿ-hong bāng-chām",
        tl = "Kuann-hong bāng-tsām"
    )

    // Open Font
    val openFontTitle = LocalizedText(
        hanji = "粉圓",
        poj = "Open Hún-îⁿ",
        tl = "Open Hún-înn"
    )

    val openFontCopyright = LocalizedText(
        hanji = "© justfont",
        poj = "© justfont",
        tl = "© justfont"
    )

    val silOpenFontLicense = LocalizedText(
        hanji = "SIL Open Font License",
        poj = "SIL Open Font License",
        tl = "SIL Open Font License"
    )

    // Iansui Font (芫荽)
    val iansuiFontTitle = LocalizedText(
        hanji = "芫荽",
        poj = "Iân-sui",
        tl = "Iân-sui"
    )

    val iansuiFontCopyright = LocalizedText(
        hanji = "© ButTaiwan",
        poj = "© ButTaiwan",
        tl = "© ButTaiwan"
    )

    val silOpenFontLicense11 = LocalizedText(
        hanji = "SIL Open Font License 1.1",
        poj = "SIL Open Font License 1.1",
        tl = "SIL Open Font License 1.1"
    )

    // MOE Dictionary
    val moeDict = LocalizedText(
        hanji = "台語常用詞辭典 - 教育部",
        poj = "Kàu-io̍k-pō͘ Tâi-oân Tâi-gí Siông-iōng-sû Sû-tián",
        tl = "Kàu-io̍k-pōo Tâi-uân Tâi-gí Siông-iōng-sû Sû-tián"
    )

    val moeCopyright = LocalizedText(
        hanji = "© 教育部",
        poj = "© Kàu-io̍k-pō͘",
        tl = "© Kàu-io̍k-pōo"
    )

    val ccLicense = LocalizedText(
        hanji = "CC BY-ND 3.0 TW",
        poj = "CC BY-ND 3.0 TW",
        tl = "CC BY-ND 3.0 TW"
    )

    // iTaigi Dictionary
    val iTaigiDict = LocalizedText(
        hanji = "iTaigi愛台語 - 群眾台語辭典",
        poj = "iTaigi Huâ-tâi tùi-chiàu-tián",
        tl = "iTaigi Huâ-tâi tuì-tsiàu-tián"
    )

    val iTaigiCopyright = LocalizedText(
        hanji = "© iTaigi愛台語",
        poj = "© iTaigi",
        tl = "© iTaigi"
    )

    val cc0License = LocalizedText(
        hanji = "CC0",
        poj = "CC0",
        tl = "CC0"
    )

    // Newword Dictionary
    val newwordDict = LocalizedText(
        hanji = "台語新詞辭庫 - 公視台語台",
        poj = "Tâi-gí Sin-sû Sû-khò͘",
        tl = "Tâi-gí Sin-sû Sû-khòo"
    )

    val newwordCopyright = LocalizedText(
        hanji = "© 公視台語台",
        poj = "© Kong-sī Tâi-gí-tâi",
        tl = "© Kong-sī Tâi-gí-tâi"
    )

    val ccBy4License = LocalizedText(
        hanji = "CC BY 4.0",
        poj = "CC BY 4.0",
        tl = "CC BY 4.0"
    )

    // Taiwan Plant Dictionary
    val taiwanPlantDict = LocalizedText(
        hanji = "台灣植物名彙",
        poj = "Tâi-oân Si̍t-bu̍t Miâ-hūi",
        tl = "Tâi-uân Si̍t-bu̍t Miâ-huī"
    )

    val taiwanPlantCopyright = LocalizedText(
        hanji = "© 佐佐木舜一",
        poj = "© Sasaki Syuniti",
        tl = "© Sasaki Syuniti"
    )

    val ccBySA4License = LocalizedText(
        hanji = "CC BY-SA 4.0",
        poj = "CC BY-SA 4.0",
        tl = "CC BY-SA 4.0"
    )

    // Tai-Hua Dictionary
    val taiHuaDict = LocalizedText(
        hanji = "台華線頂對照典",
        poj = "Tâi-hôa Sòaⁿ-téng Tùi-chiàu-tián",
        tl = "Tâi-huâ Suànn-tíng Tuì-tsiàu-tián"
    )

    val taiHuaCopyright = LocalizedText(
        hanji = "© 鄭良偉",
        poj = "© Tēⁿ Liông-úi",
        tl = "© Tēnn Liông-uí"
    )

    // Taiwan-Japan Dictionary
    val taiwanJapanDict = LocalizedText(
        hanji = "台日大辭典",
        poj = "Tâi-ji̍t Tāi-sû-tián (Tâi-e̍k-pán)",
        tl = "Tâi-ji̍t Tāi-sû-tián (Tâi-i̍k-pán)"
    )

    val taiwanJapanCopyright = LocalizedText(
        hanji = "© 小川尚義",
        poj = "© Ogawa Naoyosi",
        tl = "© Ogawa Naoyosi"
    )

    val ccByNcSA3License = LocalizedText(
        hanji = "CC BY-NC-SA 3.0 TW",
        poj = "CC BY-NC-SA 3.0 TW",
        tl = "CC BY-NC-SA 3.0 TW"
    )

    // Kungge Dictionary (台語工藝詞庫)
    val kunggeDict = LocalizedText(
        hanji = "台語工藝詞庫 - 工藝中心",
        poj = "2019 Tâi-gí Kang-gē Sû-khò͘",
        tl = "2019 Tâi-gí Kang-gē Sû-khòo"
    )

    val kunggeCopyright = LocalizedText(
        hanji = "© 國立臺灣工藝研究發展中心",
        poj = "© Kok-li̍p Tâi-oân Kang-gē Gián-kiù Hoat-tián Tiong-sim",
        tl = "© Kok-li̍p Tâi-uân Kang-gē Gián-kiù Huat-tián Tiong-sim"
    )

    val ccByNcLicense = LocalizedText(
        hanji = "CC BY-NC",
        poj = "CC BY-NC",
        tl = "CC BY-NC"
    )

}

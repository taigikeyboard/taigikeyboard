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

    val showHyphenKey = LocalizedText(
        hanji = "連字符揤鈕",
        poj = "Liân-jī-hû chhi̍h-liú",
        tl = "Liân-jī-hû tshi̍h-liú"
    )

    val customFont = LocalizedText(
        hanji = "Open 粉圓字骨",
        poj = "Open hún-îⁿ jī-kut",
        tl = "Open hún-înn jī-kut"
    )

    val phahTaigiLayout = LocalizedText(
        hanji = "PhahTaigi 齒盤",
        poj = "PhahTaigi khí-pôaⁿ",
        tl = "PhahTaigi khí-puânn"
    )

    val clearCache = LocalizedText(
        hanji = "清除資料",
        poj = "Chheng-tû chu-liāu",
        tl = "Tshing-tû tsu-liāu"
    )

    val clearCacheMessage = LocalizedText(
        hanji = "這个動作會清除所有捷用詞 ê 記錄。敢欲繼續？",
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

    val sponsorship = LocalizedText(
        hanji = "贊助支持",
        poj = "Chàn-chō͘ chi-chhî",
        tl = "Tsàn-tsōo tsi-tshî"
    )

    val contactUs = LocalizedText(
        hanji = "意見回饋",
        poj = "Ì-kiàn hôe-kūi",
        tl = "Ì-kiàn huê-kuī"
    )

    val websiteIntro = LocalizedText(
        hanji = "網站介紹",
        poj = "Bāng-chām kài-siāu",
        tl = "Bāng-tsām kài-siāu"
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
        hanji = "增加台語齒盤",
        poj = "Cheng-ka Tâi-gí Khí-pôaⁿ",
        tl = "Tsing-ka Tâi-gí Khí-puânn"
    )

    val setupInfoMessage = LocalizedText(
        hanji = "「允准完整取用」予齒盤會當捌你揤 ê 動作，成做你拍 ê 字。請放心，App 袂收集你 ê 資料。",
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
        hanji = "佇會當拍字 ê 所在，揤牢地球圖示切去台語齒盤",
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
        hanji = "查看授權條款",
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
        hanji = "Open 粉圓",
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

    // MOE Dictionary
    val moeDict = LocalizedText(
        hanji = "教育部臺灣台語常用詞辭典",
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
        hanji = "2016+ iTaigi華台對照典",
        poj = "2016+ iTaigi Huâ-tâi tùi-chiàu-tián",
        tl = "2016+ iTaigi Huâ-tâi tuì-tsiàu-tián"
    )

    val iTaigiCopyright = LocalizedText(
        hanji = "© iTaigi",
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
        hanji = "台語新詞辭庫",
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
        hanji = "1928 台灣植物名彙",
        poj = "1928 Tâi-oân Si̍t-bu̍t Miâ-hūi",
        tl = "1928 Tâi-uân Si̍t-bu̍t Miâ-huī"
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
        hanji = "2002+ 台華線頂對照典",
        poj = "2002+ Tâi-hôa Sòaⁿ-téng Tùi-chiàu-tián",
        tl = "2002+ Tâi-huâ Suànn-tíng Tuì-tsiàu-tián"
    )

    val taiHuaCopyright = LocalizedText(
        hanji = "© 鄭良偉",
        poj = "© Tēⁿ Liông-úi",
        tl = "© Tēnn Liông-uí"
    )

    // Taiwan-Japan Dictionary
    val taiwanJapanDict = LocalizedText(
        hanji = "1932 台日大辭典(台譯版)",
        poj = "1932 Tâi-ji̍t Tāi-sû-tián (Tâi-e̍k-pán)",
        tl = "1932 Tâi-ji̍t Tāi-sû-tián (Tâi-i̍k-pán)"
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

    // Sponsorship 贊助頁面文字
    val sponsorTitle = LocalizedText(
        hanji = "支持台語齒盤的運作",
        poj = "Chi-chhî Tâi-gí khí-pôaⁿ ê ūn-chok",
        tl = "Tsi-tshî Tâi-gí Khí-puânn ê ūn-tsok"
    )

    val sponsorSubtitle = LocalizedText(
        hanji = "每一份支持，攏是台語傳承 ê 力量",
        poj = "Muí chi̍t hūn chi-chhî, lóng sī Tâi-gí thoân-sêng ê le̍k-liōng",
        tl = "Muí tsi̍t hūn tsi-tshî, lóng sī Tâi-gí thuân-sîng ê li̍k-liōng"
    )

    val aboutApp = LocalizedText(
        hanji = "贊助承諾",
        poj = "Chàn-chō͘ sêng-lo̍k",
        tl = "Tsàn-tsōo sîng-lo̍k"
    )

    val oneTimeSponsor = LocalizedText(
        hanji = "贊助選項",
        poj = "Chàn-chō͘ Soán-hāng",
        tl = "Tsàn-tsōo Suán-hāng"
    )

    val coffeeTier = LocalizedText(
        hanji = "請一杯咖啡",
        poj = "Chhiáⁿ chi̍t poe ka-pi",
        tl = "Tshiánn tsi̍t pue ka-pi"
    )

    val mealTier = LocalizedText(
        hanji = "請一頓飯",
        poj = "Chhiáⁿ chi̍t tǹg pn̄ng",
        tl = "Tshiánn tsi̍t tǹg pn̄ng"
    )

    val premiumTier = LocalizedText(
        hanji = "台語英雄贊助！",
        poj = "Tâi-gí eng-hiông chàn-chō͘!",
        tl = "Tâi-gí ing-hiông tsàn-tsōo!"
    )

    val futurePlan1 = LocalizedText(
        hanji = "按你 ê 意見，予輸入法愈來愈好用",
        poj = "Àn lí ê ì-kiàn, hō͘ su-ji̍p-hoat lú-lâi-lú hó-iōng",
        tl = "Àn lí ê ì-kiàn, hōo su-ji̍p-huat lú-lâi-lú hó-iōng"
    )

    val futurePlan2 = LocalizedText(
        hanji = "定期升級維護 APP，修理資安破空",
        poj = "Tēng-kî seng-kip ûi-hō͘ APP, siu-lí chu-an phòa-khang",
        tl = "Tīng-kî sing-kip uî-hōo APP, siu-lí tsu-an phuà-khang"
    )

    val futurePlan3 = LocalizedText(
        hanji = "定期增加詞庫量，維護詞庫品質",
        poj = "Tēng-kî cheng-ka sû-khò͘ liōng, ûi-hō͘ sû-khò͘ phín-chit",
        tl = "Tīng-kî tsing-ka sû-khòo liōng, uî-hōo sû-khòo phín-tsit"
    )

    val futurePlan4 = LocalizedText(
        hanji = "支持台語教育、公益使用，袂收費、袂做付費功能、嘛袂囥廣告",
        poj = "Chi-chhî Tâi-gí kàu-io̍k, kong-ek sú-iōng. Bōe siu-huì, bōe chò hù-huì kong-lêng, mā bōe khǹg kóng-kò",
        tl = "Tsi-tshî Tâi-gí kàu-io̍k, kong-ik sú-iōng. Buē siu-huì, buē tsò hù-huì kong-lîng, mā buē khǹg kóng-kò"
    )

    val thankYouMessage = LocalizedText(
        hanji = "感謝您 ê 支持！",
        poj = "Kám-siā lín ê chi-chhî!",
        tl = "Kám-siā lín ê tsi-tshî!"
    )

    val purchaseFailed = LocalizedText(
        hanji = "購買失敗",
        poj = "Bé sit-pāi",
        tl = "Bé sit-pāi"
    )

    val alreadySponsored = LocalizedText(
        hanji = "已贊助",
        poj = "Í chàn-chō͘",
        tl = "Í tsàn-tsōo"
    )

    // Clipboard timestamp texts
    val justNow = LocalizedText(
        hanji = "拄仔",
        poj = "tú-á",
        tl = "tú-á"
    )

    val minutesAgo = LocalizedText(
        hanji = "分鐘前",
        poj = "hun-cheng chêng",
        tl = "hun-tsing tsîng"
    )

    val hoursAgo = LocalizedText(
        hanji = "點鐘前",
        poj = "tiám-cheng chêng",
        tl = "tiám-tsing tsîng"
    )

    val daysAgo = LocalizedText(
        hanji = "幾工前",
        poj = "kuí kang chêng",
        tl = "kuí kang tsîng"
    )

}

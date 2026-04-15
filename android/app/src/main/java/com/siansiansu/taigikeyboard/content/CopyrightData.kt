package com.siansiansu.taigikeyboard.content

import com.siansiansu.taigikeyboard.localization.CommonTexts
import com.siansiansu.taigikeyboard.localization.Tab1Texts

// Static copyright data for fonts, dictionaries, and data sources displayed in Tab1

private const val SIL_OFL_LICENSE_URL = "https://openfontlicense.org/"
private const val CC_BY_SA_4_LICENSE_URL =
    "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW"

data class CopyrightPage(
    val id: Int,
    val title: String,
    val description: String,
    val license: String,
    val buttons: List<CopyrightButton>,
)

data class CopyrightButton(
    val text: String,
    val url: String,
)

object CopyrightDataSource {
    val copyrightPages =
        listOf(
            // 字體
            CopyrightPage(
                id = 0,
                title = Tab1Texts.openFontTitle,
                description = Tab1Texts.openFontCopyright,
                license = Tab1Texts.silOpenFontLicense,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = Tab1Texts.viewLicense,
                            url = SIL_OFL_LICENSE_URL,
                        ),
                        CopyrightButton(
                            text = CommonTexts.viewWebsite,
                            url = "https://justfont.com/huninn/",
                        ),
                    ),
            ),
            CopyrightPage(
                id = 1,
                title = Tab1Texts.iansuiFontTitle,
                description = Tab1Texts.iansuiFontCopyright,
                license = Tab1Texts.silOpenFontLicense11,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = Tab1Texts.viewLicense,
                            url = SIL_OFL_LICENSE_URL,
                        ),
                        CopyrightButton(
                            text = CommonTexts.viewWebsite,
                            url = "https://github.com/ButTaiwan/iansui",
                        ),
                    ),
            ),
            // 辭典
            // 1. 教育部臺灣台語常用詞辭典
            CopyrightPage(
                id = 2,
                title = CommonTexts.moeDict,
                description = Tab1Texts.moeCopyright,
                license = Tab1Texts.ccLicense,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = Tab1Texts.viewLicense,
                            url = "https://creativecommons.org/licenses/by-nd/3.0/tw/",
                        ),
                        CopyrightButton(
                            text = CommonTexts.viewWebsite,
                            url = "https://sutian.moe.edu.tw/",
                        ),
                    ),
            ),
            // 2. 台語新詞辭庫
            CopyrightPage(
                id = 3,
                title = CommonTexts.newwordDict,
                description = Tab1Texts.newwordCopyright,
                license = Tab1Texts.ccBy4License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = Tab1Texts.viewLicense,
                            url = "https://creativecommons.org/licenses/by/4.0/deed.zh-hant",
                        ),
                        CopyrightButton(
                            text = CommonTexts.viewWebsite,
                            url = "https://www.taigitv.org.tw/taigi-words",
                        ),
                    ),
            ),
            // 3. 台語工藝詞庫
            CopyrightPage(
                id = 4,
                title = CommonTexts.kunggeDict,
                description = Tab1Texts.kunggeCopyright,
                license = Tab1Texts.ccByNcLicense,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = Tab1Texts.viewLicense,
                            url = "https://creativecommons.org/licenses/by-nc/4.0/deed.zh-hant",
                        ),
                        CopyrightButton(
                            text = CommonTexts.viewWebsite,
                            url = "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite",
                        ),
                    ),
            ),
            // 4. iTaigi 華台對照典
            CopyrightPage(
                id = 5,
                title = CommonTexts.iTaigiDict,
                description = Tab1Texts.iTaigiCopyright,
                license = Tab1Texts.cc0License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = Tab1Texts.viewLicense,
                            url = "https://creativecommons.org/public-domain/cc0/",
                        ),
                        CopyrightButton(
                            text = CommonTexts.viewWebsite,
                            url = "https://itaigi.tw/",
                        ),
                    ),
            ),
            // 5. 台日大辭典
            CopyrightPage(
                id = 6,
                title = CommonTexts.taiwanJapanDict,
                description = Tab1Texts.taiwanJapanCopyright,
                license = Tab1Texts.ccByNcSA3License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = Tab1Texts.viewLicense,
                            url = "https://creativecommons.org/licenses/by-nc-sa/3.0/tw/",
                        ),
                        CopyrightButton(
                            text = CommonTexts.viewWebsite,
                            url = "http://taigi.fhl.net/dict/",
                        ),
                    ),
            ),
            // 6. 台華線頂對照典
            CopyrightPage(
                id = 7,
                title = CommonTexts.taiHuaDict,
                description = Tab1Texts.taiHuaCopyright,
                license = Tab1Texts.ccBySA4License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = Tab1Texts.viewLicense,
                            url = CC_BY_SA_4_LICENSE_URL,
                        ),
                    ),
            ),
            // 7. 台灣植物名彙
            CopyrightPage(
                id = 8,
                title = CommonTexts.taiwanPlantDict,
                description = Tab1Texts.taiwanPlantCopyright,
                license = Tab1Texts.ccBySA4License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = Tab1Texts.viewLicense,
                            url = CC_BY_SA_4_LICENSE_URL,
                        ),
                        CopyrightButton(
                            text = CommonTexts.viewWebsite,
                            url = "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106",
                        ),
                    ),
            ),
            // 8. 腔口補充資料
            CopyrightPage(
                id = 9,
                title = CommonTexts.khpooDict,
                description = Tab1Texts.accentDictCredit,
                license = "",
                buttons = emptyList(),
            ),
        )
}

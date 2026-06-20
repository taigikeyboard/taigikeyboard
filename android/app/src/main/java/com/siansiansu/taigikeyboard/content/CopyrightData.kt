package com.siansiansu.taigikeyboard.content

import com.siansiansu.taigikeyboard.i18n.StringResolver
import com.siansiansu.taigikeyboard.i18n.generated.StringKey
import com.siansiansu.taigikeyboard.localization.DictionaryTexts
import com.siansiansu.taigikeyboard.localization.HomeTexts

// Copyright data for fonts, dictionaries, and data sources displayed in the Home tab

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
    // Resolver-driven so the common-namespace strings re-resolve under the active display language.
    fun copyrightPages(resolver: StringResolver): List<CopyrightPage> =
        listOf(
            // 字體
            CopyrightPage(
                id = 0,
                title = HomeTexts.openFontTitle,
                description = HomeTexts.openFontCopyright,
                license = HomeTexts.silOpenFontLicense,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = SIL_OFL_LICENSE_URL,
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://justfont.com/huninn/",
                        ),
                    ),
            ),
            CopyrightPage(
                id = 1,
                title = HomeTexts.iansuiFontTitle,
                description = HomeTexts.iansuiFontCopyright,
                license = HomeTexts.silOpenFontLicense11,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = SIL_OFL_LICENSE_URL,
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://github.com/ButTaiwan/iansui",
                        ),
                    ),
            ),
            // 源樣明體
            CopyrightPage(
                id = 2,
                title = HomeTexts.genYoMinFontTitle,
                description = HomeTexts.butTaiwanCopyright,
                license = HomeTexts.silOpenFontLicense11,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = SIL_OFL_LICENSE_URL,
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://github.com/ButTaiwan/genyo-font",
                        ),
                    ),
            ),
            // 源樣烏體
            CopyrightPage(
                id = 3,
                title = HomeTexts.genYoGothicFontTitle,
                description = HomeTexts.butTaiwanCopyright,
                license = HomeTexts.silOpenFontLicense11,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = SIL_OFL_LICENSE_URL,
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://github.com/ButTaiwan/genyog-font",
                        ),
                    ),
            ),
            // 辭典
            // 1. 教育部臺灣台語常用詞辭典
            CopyrightPage(
                id = 4,
                title = resolver.resolve(StringKey.COMMON_MOE_DICT),
                description = HomeTexts.moeCopyright,
                license = HomeTexts.ccLicense,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = "https://creativecommons.org/licenses/by-nd/3.0/tw/",
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://sutian.moe.edu.tw/",
                        ),
                    ),
            ),
            // 2. 台語新詞辭庫
            CopyrightPage(
                id = 5,
                title = resolver.resolve(StringKey.COMMON_NEWWORD_DICT),
                description = HomeTexts.newwordCopyright,
                license = HomeTexts.ccBy4License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = "https://creativecommons.org/licenses/by/4.0/deed.zh-hant",
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://www.taigitv.org.tw/taigi-words",
                        ),
                    ),
            ),
            // 3. 台語工藝詞庫
            CopyrightPage(
                id = 6,
                title = resolver.resolve(StringKey.COMMON_KUNGGE_DICT),
                description = HomeTexts.kunggeCopyright,
                license = HomeTexts.ccByNcLicense,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = "https://creativecommons.org/licenses/by-nc/4.0/deed.zh-hant",
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite",
                        ),
                    ),
            ),
            // 4. iTaigi 華台對照典
            CopyrightPage(
                id = 7,
                title = resolver.resolve(StringKey.COMMON_I_TAIGI_DICT),
                description = HomeTexts.iTaigiCopyright,
                license = HomeTexts.cc0License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = "https://creativecommons.org/public-domain/cc0/",
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://itaigi.tw/",
                        ),
                    ),
            ),
            // 5. 台日大辭典
            CopyrightPage(
                id = 8,
                title = resolver.resolve(StringKey.COMMON_TAIWAN_JAPAN_DICT),
                description = HomeTexts.taiwanJapanCopyright,
                license = HomeTexts.ccByNcSA3License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = "https://creativecommons.org/licenses/by-nc-sa/3.0/tw/",
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "http://taigi.fhl.net/dict/",
                        ),
                    ),
            ),
            // 6. 台華線頂對照典
            CopyrightPage(
                id = 9,
                title = resolver.resolve(StringKey.COMMON_TAI_HUA_DICT),
                description = HomeTexts.taiHuaCopyright,
                license = HomeTexts.ccBySA4License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = CC_BY_SA_4_LICENSE_URL,
                        ),
                    ),
            ),
            // 7. 台灣植物名彙
            CopyrightPage(
                id = 10,
                title = resolver.resolve(StringKey.COMMON_TAIWAN_PLANT_DICT),
                description = HomeTexts.taiwanPlantCopyright,
                license = HomeTexts.ccBySA4License,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = HomeTexts.viewLicense,
                            url = CC_BY_SA_4_LICENSE_URL,
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106",
                        ),
                    ),
            ),
            // 8. 腔口補充資料
            CopyrightPage(
                id = 11,
                title = resolver.resolve(StringKey.COMMON_ACCENT_DICT),
                description = HomeTexts.accentDictCredit,
                license = "",
                buttons = emptyList(),
            ),
            // 9. 詞庫增補檔案
            CopyrightPage(
                id = 12,
                title = DictionaryTexts.devSupplementDict,
                description = HomeTexts.devSupplementCredit,
                license = "",
                buttons = emptyList(),
            ),
        )
}

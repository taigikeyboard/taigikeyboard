package com.siansiansu.taigikeyboard.content

import com.siansiansu.taigikeyboard.i18n.StringResolver
import com.siansiansu.taigikeyboard.i18n.generated.StringKey

// Copyright data for fonts, dictionaries, and data sources displayed in the Home tab

private const val SIL_OFL_LICENSE_URL = "https://openfontlicense.org/"
private const val CC_BY_SA_4_LICENSE_URL =
    "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW"

// License identifiers are standardized short codes — language-invariant, never translated (kept as
// constants rather than i18n keys per the multi-language plan; SPDX-aligned).
private const val SIL_OPEN_FONT_LICENSE = "SIL Open Font License"
private const val SIL_OPEN_FONT_LICENSE_11 = "SIL Open Font License 1.1"
private const val CC_BY_ND_3_TW_LICENSE = "CC BY-ND 3.0 TW"
private const val CC_BY_4_LICENSE = "CC BY 4.0"
// NTCRI publishes no licence for the glossary text; its only statement covers the images.
private const val CC_BY_NC_ND_IMAGES_LICENSE = "CC BY-NC-ND (images only)"
private const val CC0_LICENSE = "CC0"
private const val CC_BY_NC_SA_3_TW_LICENSE = "CC BY-NC-SA 3.0 TW"
private const val CC_BY_SA_4_LICENSE = "CC BY-SA 4.0"

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
    // Resolver-driven so the i18n strings re-resolve under the active display language.
    fun copyrightPages(resolver: StringResolver): List<CopyrightPage> =
        listOf(
            // 字體
            CopyrightPage(
                id = 0,
                title = resolver.resolve(StringKey.COMMON_FONT_OPEN_HUNINN),
                description = resolver.resolve(StringKey.HOME_OPEN_FONT_COPYRIGHT),
                license = SIL_OPEN_FONT_LICENSE,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
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
                title = resolver.resolve(StringKey.COMMON_FONT_IANSUI),
                description = resolver.resolve(StringKey.HOME_BUT_TAIWAN_COPYRIGHT),
                license = SIL_OPEN_FONT_LICENSE_11,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
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
                title = resolver.resolve(StringKey.COMMON_FONT_GEN_YO_MIN),
                description = resolver.resolve(StringKey.HOME_BUT_TAIWAN_COPYRIGHT),
                license = SIL_OPEN_FONT_LICENSE_11,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
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
                title = resolver.resolve(StringKey.COMMON_FONT_GEN_YO_GOTHIC),
                description = resolver.resolve(StringKey.HOME_BUT_TAIWAN_COPYRIGHT),
                license = SIL_OPEN_FONT_LICENSE_11,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
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
                description = resolver.resolve(StringKey.HOME_MOE_COPYRIGHT),
                license = CC_BY_ND_3_TW_LICENSE,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
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
                description = resolver.resolve(StringKey.HOME_NEWWORD_COPYRIGHT),
                license = CC_BY_4_LICENSE,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
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
                description = resolver.resolve(StringKey.HOME_KUNGGE_COPYRIGHT),
                license = CC_BY_NC_ND_IMAGES_LICENSE,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
                            url = "https://kanggesu.ntcri.gov.tw/NTCRI_TaigiWebSite/ImageLicense",
                        ),
                        CopyrightButton(
                            text = resolver.resolve(StringKey.COMMON_VIEW_WEBSITE),
                            url = "https://kanggesu.ntcri.gov.tw/NTCRI_TaigiWebSite",
                        ),
                    ),
            ),
            // 4. iTaigi 華台對照典
            CopyrightPage(
                id = 7,
                title = resolver.resolve(StringKey.COMMON_I_TAIGI_DICT),
                description = resolver.resolve(StringKey.HOME_I_TAIGI_COPYRIGHT),
                license = CC0_LICENSE,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
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
                description = resolver.resolve(StringKey.HOME_TAIWAN_JAPAN_COPYRIGHT),
                license = CC_BY_NC_SA_3_TW_LICENSE,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
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
                description = resolver.resolve(StringKey.HOME_TAI_HUA_COPYRIGHT),
                license = CC_BY_SA_4_LICENSE,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
                            url = CC_BY_SA_4_LICENSE_URL,
                        ),
                    ),
            ),
            // 7. 台灣植物名彙
            CopyrightPage(
                id = 10,
                title = resolver.resolve(StringKey.COMMON_TAIWAN_PLANT_DICT),
                description = resolver.resolve(StringKey.HOME_TAIWAN_PLANT_COPYRIGHT),
                license = CC_BY_SA_4_LICENSE,
                buttons =
                    listOf(
                        CopyrightButton(
                            text = resolver.resolve(StringKey.HOME_VIEW_LICENSE),
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
                description = resolver.resolve(StringKey.HOME_ACCENT_DICT_CREDIT),
                license = "",
                buttons = emptyList(),
            ),
            // 9. 詞庫增補檔案
            CopyrightPage(
                id = 12,
                title = resolver.resolve(StringKey.DICTIONARY_DEV_SUPPLEMENT_DICT),
                description = resolver.resolve(StringKey.HOME_DEV_SUPPLEMENT_CREDIT),
                license = "",
                buttons = emptyList(),
            ),
        )
}

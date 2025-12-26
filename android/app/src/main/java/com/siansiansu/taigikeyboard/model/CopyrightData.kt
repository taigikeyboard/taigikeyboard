package com.siansiansu.taigikeyboard.model

import androidx.annotation.ColorRes
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.localization.LocalizedText
import com.siansiansu.taigikeyboard.localization.Tab1Texts

data class CopyrightPage(
    val id: Int,
    val title: LocalizedText,
    val description: LocalizedText,
    val iconResId: Int,
    @ColorRes val accentColorResId: Int,
    val license: LocalizedText,
    val buttons: List<CopyrightButton>
)

data class CopyrightButton(
    val text: LocalizedText,
    val url: String
)

object CopyrightDataSource {
    val copyrightPages = listOf(
        // 字體
        CopyrightPage(
            id = 0,
            title = Tab1Texts.openFontTitle,
            description = Tab1Texts.openFontCopyright,
            iconResId = android.R.drawable.ic_menu_edit,
            accentColorResId = R.color.copyright_accent_green,
            license = Tab1Texts.silOpenFontLicense,
            buttons = listOf(
                CopyrightButton(
                    text = Tab1Texts.viewLicense,
                    url = "https://openfontlicense.org/"
                ),
                CopyrightButton(
                    text = Tab1Texts.viewWebsite,
                    url = "https://justfont.com/huninn/"
                )
            )
        ),
        CopyrightPage(
            id = 1,
            title = Tab1Texts.iansuiFontTitle,
            description = Tab1Texts.iansuiFontCopyright,
            iconResId = android.R.drawable.ic_menu_edit,
            accentColorResId = R.color.copyright_accent_green,
            license = Tab1Texts.silOpenFontLicense11,
            buttons = listOf(
                CopyrightButton(
                    text = Tab1Texts.viewLicense,
                    url = "https://openfontlicense.org/"
                ),
                CopyrightButton(
                    text = Tab1Texts.viewWebsite,
                    url = "https://github.com/ButTaiwan/iansui"
                )
            )
        ),
        // 辭典
        // 1. 教育部臺灣台語常用詞辭典
        CopyrightPage(
            id = 2,
            title = Tab1Texts.moeDict,
            description = Tab1Texts.moeCopyright,
            iconResId = android.R.drawable.ic_menu_info_details,
            accentColorResId = R.color.copyright_accent_blue,
            license = Tab1Texts.ccLicense,
            buttons = listOf(
                CopyrightButton(
                    text = Tab1Texts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-nd/3.0/tw/"
                ),
                CopyrightButton(
                    text = Tab1Texts.viewWebsite,
                    url = "https://sutian.moe.edu.tw/"
                )
            )
        ),
        // 2. 台語新詞辭庫
        CopyrightPage(
            id = 3,
            title = Tab1Texts.newwordDict,
            description = Tab1Texts.newwordCopyright,
            iconResId = android.R.drawable.ic_menu_add,
            accentColorResId = R.color.copyright_accent_purple,
            license = Tab1Texts.ccBy4License,
            buttons = listOf(
                CopyrightButton(
                    text = Tab1Texts.viewLicense,
                    url = "https://creativecommons.org/licenses/by/4.0/deed.zh-hant"
                ),
                CopyrightButton(
                    text = Tab1Texts.viewWebsite,
                    url = "https://www.taigitv.org.tw/taigi-words"
                )
            )
        ),
        // 3. 台語工藝詞庫
        CopyrightPage(
            id = 4,
            title = Tab1Texts.kunggeDict,
            description = Tab1Texts.kunggeCopyright,
            iconResId = android.R.drawable.ic_menu_compass,
            accentColorResId = R.color.copyright_accent_orange,
            license = Tab1Texts.ccByNcLicense,
            buttons = listOf(
                CopyrightButton(
                    text = Tab1Texts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-nc/4.0/deed.zh-hant"
                ),
                CopyrightButton(
                    text = Tab1Texts.viewWebsite,
                    url = "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite"
                )
            )
        ),
        // 4. iTaigi 華台對照典
        CopyrightPage(
            id = 5,
            title = Tab1Texts.iTaigiDict,
            description = Tab1Texts.iTaigiCopyright,
            iconResId = android.R.drawable.ic_menu_mapmode,
            accentColorResId = R.color.copyright_accent_orange,
            license = Tab1Texts.cc0License,
            buttons = listOf(
                CopyrightButton(
                    text = Tab1Texts.viewLicense,
                    url = "https://creativecommons.org/public-domain/cc0/"
                ),
                CopyrightButton(
                    text = Tab1Texts.viewWebsite,
                    url = "https://itaigi.tw/"
                )
            )
        ),
        // 5. 台日大辭典
        CopyrightPage(
            id = 6,
            title = Tab1Texts.taiwanJapanDict,
            description = Tab1Texts.taiwanJapanCopyright,
            iconResId = android.R.drawable.ic_menu_my_calendar,
            accentColorResId = R.color.copyright_accent_purple,
            license = Tab1Texts.ccByNcSA3License,
            buttons = listOf(
                CopyrightButton(
                    text = Tab1Texts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-nc-sa/3.0/tw/"
                ),
                CopyrightButton(
                    text = Tab1Texts.viewWebsite,
                    url = "http://taigi.fhl.net/dict/"
                )
            )
        ),
        // 6. 台華線頂對照典
        CopyrightPage(
            id = 7,
            title = Tab1Texts.taiHuaDict,
            description = Tab1Texts.taiHuaCopyright,
            iconResId = android.R.drawable.ic_menu_sort_alphabetically,
            accentColorResId = R.color.copyright_accent_blue,
            license = Tab1Texts.ccBySA4License,
            buttons = listOf(
                CopyrightButton(
                    text = Tab1Texts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW"
                )
            )
        ),
        // 7. 台灣植物名彙
        CopyrightPage(
            id = 8,
            title = Tab1Texts.taiwanPlantDict,
            description = Tab1Texts.taiwanPlantCopyright,
            iconResId = android.R.drawable.ic_menu_sort_by_size,
            accentColorResId = R.color.copyright_accent_green,
            license = Tab1Texts.ccBySA4License,
            buttons = listOf(
                CopyrightButton(
                    text = Tab1Texts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW"
                ),
                CopyrightButton(
                    text = Tab1Texts.viewWebsite,
                    url = "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106"
                )
            )
        )
    )
}

package com.siansiansu.taigikeyboard.model

import androidx.annotation.ColorRes
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.settings.LocalizedText

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
    val iconResId: Int,
    val text: LocalizedText,
    val url: String
)

object CopyrightDataSource {
    val copyrightPages = listOf(
        // 字體
        CopyrightPage(
            id = 0,
            title = com.siansiansu.taigikeyboard.settings.AppTexts.openFontTitle,
            description = com.siansiansu.taigikeyboard.settings.AppTexts.openFontCopyright,
            iconResId = android.R.drawable.ic_menu_edit,
            accentColorResId = R.color.copyright_accent_green,
            license = com.siansiansu.taigikeyboard.settings.AppTexts.silOpenFontLicense,
            buttons = listOf(
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_info_details,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewLicense,
                    url = "https://openfontlicense.org/"
                ),
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_view,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewWebsite,
                    url = "https://justfont.com/huninn/"
                )
            )
        ),
        CopyrightPage(
            id = 1,
            title = com.siansiansu.taigikeyboard.settings.AppTexts.iansuiFontTitle,
            description = com.siansiansu.taigikeyboard.settings.AppTexts.iansuiFontCopyright,
            iconResId = android.R.drawable.ic_menu_edit,
            accentColorResId = R.color.copyright_accent_green,
            license = com.siansiansu.taigikeyboard.settings.AppTexts.silOpenFontLicense11,
            buttons = listOf(
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_info_details,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewLicense,
                    url = "https://openfontlicense.org/"
                ),
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_view,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewWebsite,
                    url = "https://github.com/ButTaiwan/iansui"
                )
            )
        ),
        // 辭典
        // 1. 教育部臺灣台語常用詞辭典
        CopyrightPage(
            id = 2,
            title = com.siansiansu.taigikeyboard.settings.AppTexts.moeDict,
            description = com.siansiansu.taigikeyboard.settings.AppTexts.moeCopyright,
            iconResId = android.R.drawable.ic_menu_info_details,
            accentColorResId = R.color.copyright_accent_blue,
            license = com.siansiansu.taigikeyboard.settings.AppTexts.ccLicense,
            buttons = listOf(
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_info_details,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-nd/3.0/tw/"
                ),
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_view,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewWebsite,
                    url = "https://sutian.moe.edu.tw/"
                )
            )
        ),
        // 2. 台語新詞辭庫
        CopyrightPage(
            id = 3,
            title = com.siansiansu.taigikeyboard.settings.AppTexts.newwordDict,
            description = com.siansiansu.taigikeyboard.settings.AppTexts.newwordCopyright,
            iconResId = android.R.drawable.ic_menu_add,
            accentColorResId = R.color.copyright_accent_purple,
            license = com.siansiansu.taigikeyboard.settings.AppTexts.ccBy4License,
            buttons = listOf(
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_info_details,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewLicense,
                    url = "https://creativecommons.org/licenses/by/4.0/deed.zh-hant"
                ),
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_view,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewWebsite,
                    url = "https://www.taigitv.org.tw/taigi-words"
                )
            )
        ),
        // 3. 台語工藝詞庫
        CopyrightPage(
            id = 4,
            title = com.siansiansu.taigikeyboard.settings.AppTexts.kunggeDict,
            description = com.siansiansu.taigikeyboard.settings.AppTexts.kunggeCopyright,
            iconResId = android.R.drawable.ic_menu_compass,
            accentColorResId = R.color.copyright_accent_orange,
            license = com.siansiansu.taigikeyboard.settings.AppTexts.ccByNcLicense,
            buttons = listOf(
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_info_details,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-nc/4.0/deed.zh-hant"
                ),
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_view,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewWebsite,
                    url = "https://kanggesu.ntcri.org.tw/NTCRI_TaigiWebSite"
                )
            )
        ),
        // 4. iTaigi 華台對照典
        CopyrightPage(
            id = 5,
            title = com.siansiansu.taigikeyboard.settings.AppTexts.iTaigiDict,
            description = com.siansiansu.taigikeyboard.settings.AppTexts.iTaigiCopyright,
            iconResId = android.R.drawable.ic_menu_mapmode,
            accentColorResId = R.color.copyright_accent_orange,
            license = com.siansiansu.taigikeyboard.settings.AppTexts.cc0License,
            buttons = listOf(
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_info_details,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewLicense,
                    url = "https://creativecommons.org/public-domain/cc0/"
                ),
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_view,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewWebsite,
                    url = "https://itaigi.tw/"
                )
            )
        ),
        // 5. 台日大辭典
        CopyrightPage(
            id = 6,
            title = com.siansiansu.taigikeyboard.settings.AppTexts.taiwanJapanDict,
            description = com.siansiansu.taigikeyboard.settings.AppTexts.taiwanJapanCopyright,
            iconResId = android.R.drawable.ic_menu_my_calendar,
            accentColorResId = R.color.copyright_accent_purple,
            license = com.siansiansu.taigikeyboard.settings.AppTexts.ccByNcSA3License,
            buttons = listOf(
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_info_details,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-nc-sa/3.0/tw/"
                ),
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_view,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewWebsite,
                    url = "http://taigi.fhl.net/dict/"
                )
            )
        ),
        // 6. 台華線頂對照典
        CopyrightPage(
            id = 7,
            title = com.siansiansu.taigikeyboard.settings.AppTexts.taiHuaDict,
            description = com.siansiansu.taigikeyboard.settings.AppTexts.taiHuaCopyright,
            iconResId = android.R.drawable.ic_menu_sort_alphabetically,
            accentColorResId = R.color.copyright_accent_blue,
            license = com.siansiansu.taigikeyboard.settings.AppTexts.ccBySA4License,
            buttons = listOf(
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_info_details,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW"
                )
            )
        ),
        // 7. 台灣植物名彙
        CopyrightPage(
            id = 8,
            title = com.siansiansu.taigikeyboard.settings.AppTexts.taiwanPlantDict,
            description = com.siansiansu.taigikeyboard.settings.AppTexts.taiwanPlantCopyright,
            iconResId = android.R.drawable.ic_menu_sort_by_size,
            accentColorResId = R.color.copyright_accent_green,
            license = com.siansiansu.taigikeyboard.settings.AppTexts.ccBySA4License,
            buttons = listOf(
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_info_details,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewLicense,
                    url = "https://creativecommons.org/licenses/by-sa/4.0/deed.zh_TW"
                ),
                CopyrightButton(
                    iconResId = android.R.drawable.ic_menu_view,
                    text = com.siansiansu.taigikeyboard.settings.AppTexts.viewWebsite,
                    url = "https://tai2.ntu.edu.tw/ebooks/ListPlFormosSasaki/0/106"
                )
            )
        )
    )
}

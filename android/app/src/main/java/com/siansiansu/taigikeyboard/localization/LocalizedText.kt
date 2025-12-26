package com.siansiansu.taigikeyboard.localization

/**
 * 本地化文字核心結構
 * 支援漢字、白話字（POJ）、台羅（TL）三種顯示方式
 */
data class LocalizedText(
    val hanji: String,
    val poj: String = hanji,
    val tl: String = hanji
) {
    fun text(language: DisplayLanguage): String {
        return when (language) {
            DisplayLanguage.HANJI -> hanji
            DisplayLanguage.POJ -> poj
            DisplayLanguage.TL -> tl
        }
    }
}

// Subtype 用 Locale ↔ 字串轉換 — 處理底線/連字號兩種 locale 字串格式。

package com.siansiansu.taigikeyboard.ime.core

import java.util.Locale

// Converts locale strings (both underscore and hyphen formats) to Locale objects
object SubtypeLocaleAdapter {
    fun stringToLocale(string: String): Locale = Locale.forLanguageTag(string.replace('_', '-'))
}

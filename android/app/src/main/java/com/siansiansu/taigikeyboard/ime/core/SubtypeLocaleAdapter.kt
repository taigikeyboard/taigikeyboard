// 中文: Subtype 用 Locale ↔ 字串的 Moshi 轉接器 — 處理底線/連字號兩種 locale 字串格式。

package com.siansiansu.taigikeyboard.ime.core

import com.squareup.moshi.FromJson
import com.squareup.moshi.ToJson
import java.util.Locale

// Converts locale strings (both underscore and hyphen formats) to Locale objects
object SubtypeLocaleAdapter {
    fun stringToLocale(string: String): Locale = Locale.forLanguageTag(string.replace('_', '-'))

    // Moshi needs a custom adapter because Locale has no built-in JSON mapping
    class JsonAdapter {
        @FromJson
        fun fromJson(raw: String): Locale = stringToLocale(raw)

        @ToJson
        fun toJson(raw: Locale): String = raw.toString()
    }
}

package com.siansiansu.taigikeyboard.util

import com.squareup.moshi.FromJson
import com.squareup.moshi.ToJson
import java.util.Locale

// Converts locale strings (both underscore and hyphen formats) to Locale objects
object LocaleUtils {
    fun stringToLocale(string: String): Locale = Locale.forLanguageTag(string.replace('_', '-'))

    // Moshi needs a custom adapter because Locale has no built-in JSON mapping
    class JsonAdapter {
        @FromJson
        fun fromJson(raw: String): Locale = stringToLocale(raw)

        @ToJson
        fun toJson(raw: Locale): String = raw.toString()
    }
}

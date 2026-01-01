
package com.siansiansu.taigikeyboard.util

import com.squareup.moshi.FromJson
import com.squareup.moshi.ToJson
import java.util.*

object LocaleUtils {
    private val DELIMITER = """[_-]""".toRegex()

    fun stringToLocale(string: String): Locale {
        return when {
            string.contains(DELIMITER) -> {
                val lc = string.split(DELIMITER)
                Locale.Builder()
                    .setLanguage(lc[0])
                    .setRegion(lc[1])
                    .build()
            }
            else -> {
                Locale.Builder()
                    .setLanguage(string)
                    .build()
            }
        }
    }

    class JsonAdapter() {
        @FromJson
        fun fromJson(raw: String): Locale {
            return stringToLocale(raw)
        }
        @ToJson
        fun toJson(raw: Locale): String {
            return raw.toString()
        }
    }
}


package com.siansiansu.taigikeyboard.ime.core

import com.squareup.moshi.Json
import com.siansiansu.taigikeyboard.util.LocaleUtils
import java.util.*

data class Subtype(
    var id: Int,
    var locale: Locale,
    var layout: String
) {
    companion object {
        /**
         * Subtype to use when prefs do not contain any valid subtypes.
         */
        val DEFAULT = Subtype(-1, Locale.ENGLISH, "qwerty_poj")

        /**
         * Converts the string representation of this object to a [Subtype]. Must be in the
         * following format:
         *  <id>/<language_code>/<layout_name>
         * or
         *  <id>/<language_tag>/<layout_name>
         * Eg: 101/en_US/qwerty
         *     201/de-DE/qwertz
         * If the given [string] does not match this format an [InvalidPropertiesFormatException]
         * will be thrown.
         */
        fun fromString(string: String): Subtype {
            val data = string.split("/")
            if (data.size != 3) {
                throw InvalidPropertiesFormatException(
                    "Given string contains more or less than 3 properties..."
                )
            } else {
                val locale = LocaleUtils.stringToLocale(data[1])
                return Subtype(
                    data[0].toInt(),
                    locale,
                    data[2]
                )
            }
        }
    }

    /**
     * Converts this object into its string representation. Format:
     *  <id>/<language_tag>/<layout_name>
     */
    override fun toString(): String {
        val languageTag = locale.toLanguageTag()
        return "$id/$languageTag/$layout"
    }
}

data class DefaultSubtype(
    var id: Int,
    @param:Json(name = "languageTag")
    var locale: Locale,
    var preferredLayout: String
)

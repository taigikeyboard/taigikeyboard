package com.siansiansu.taigikeyboard.ime.core

import java.util.Locale

// Converts locale strings (both underscore and hyphen formats) to Locale objects
object SubtypeLocaleAdapter {
    fun stringToLocale(string: String): Locale = Locale.forLanguageTag(string.replace('_', '-'))
}

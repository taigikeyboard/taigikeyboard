// Maps 1:1 to layout-JSON subdirectory names under assets/ime/text/; LayoutTypeAdapter
// converts enum <-> string at Moshi parse time, mapping "/" to "_" in the enum name.

package com.siansiansu.taigikeyboard.ime.text.layout

import android.annotation.SuppressLint
import com.squareup.moshi.FromJson

enum class LayoutType {
    CHARACTERS,
    CHARACTERS_MOD,
    EXTENSION,
    NUMERIC,
    NUMERIC_ADVANCED,
    PHONE,
    PHONE2,
    SYMBOLS,
    SYMBOLS_MOD,
    SYMBOLS2,
    SYMBOLS2_MOD,
    ;

    @SuppressLint("DefaultLocale")
    override fun toString(): String = super.toString().replace("_", "/").lowercase()

    companion object {
        @SuppressLint("DefaultLocale")
        fun fromString(string: String): LayoutType = valueOf(string.replace("/", "_").uppercase())
    }
}

class LayoutTypeAdapter {
    @FromJson
    fun fromJson(raw: String): LayoutType = LayoutType.fromString(raw)
}

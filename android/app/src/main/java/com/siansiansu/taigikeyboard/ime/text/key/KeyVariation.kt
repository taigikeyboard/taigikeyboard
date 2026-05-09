package com.siansiansu.taigikeyboard.ime.text.key

import android.annotation.SuppressLint
import com.squareup.moshi.FromJson

enum class KeyVariation {
    ALL,
    EMAIL_ADDRESS,
    NORMAL,
    PASSWORD,
    URI,
    ;

    companion object {
        @SuppressLint("DefaultLocale")
        fun fromString(string: String): KeyVariation = valueOf(string.uppercase())
    }
}

class KeyVariationAdapter {
    @FromJson
    fun fromJson(raw: String): KeyVariation = KeyVariation.fromString(raw)
}

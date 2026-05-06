package com.siansiansu.taigikeyboard.ime.text.key

import android.annotation.SuppressLint
import com.squareup.moshi.FromJson

enum class KeyType {
    CHARACTER,
    MODIFIER,
    ENTER_EDITING,
    SYSTEM_GUI,
    NAVIGATION,
    FUNCTION,
    NUMERIC,
    LOCK,
    ;

    companion object {
        @SuppressLint("DefaultLocale")
        fun fromString(string: String): KeyType {
            return valueOf(string.uppercase())
        }
    }
}

class KeyTypeAdapter {
    @FromJson
    fun fromJson(raw: String): KeyType {
        return KeyType.fromString(raw)
    }
}

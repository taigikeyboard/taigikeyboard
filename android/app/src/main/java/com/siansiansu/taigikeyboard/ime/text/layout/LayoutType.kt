// Layout JSON 的型別列舉 — 對應 assets/ime/text/ 下的子目錄名(characters / numeric / phone / symbols ...);
// 透過 LayoutTypeAdapter 在 Moshi 解析時做 enum ↔ 字串雙向轉換,字串中的 "/" 對應底線。

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

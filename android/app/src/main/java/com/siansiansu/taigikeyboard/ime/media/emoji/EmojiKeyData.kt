
package com.siansiansu.taigikeyboard.ime.media.emoji

import androidx.compose.runtime.Immutable

@Immutable
data class EmojiKeyData(
    val codePoints: List<Int>,
    val label: String = "",
    val name: String = "",
    val keywords: List<String> = emptyList()
) {
    // 快取字串轉換結果，避免重複計算
    private val cachedString: String by lazy {
        buildString {
            for (codePoint in codePoints) {
                append(String(Character.toChars(codePoint)))
            }
        }
    }

    /**
     * Returns an encoded String based on this emoji's [codePoints].
     *
     * @return The encoded String.
     */
    fun getCodePointsAsString(): String = cachedString
}

package com.siansiansu.taigikeyboard.ime.keyboard

/**
 * Emoji 膚色調整符號定義
 * 符合 Unicode Emoji 規範的膚色修飾符
 *
 * 參考：https://unicode.org/reports/tr51/#Emoji_Modifiers
 */
enum class EmojiSkinTone(val codePoint: Int) {
    /**
     * 預設（無膚色修飾符）
     */
    DEFAULT(0x0),

    /**
     * 淺膚色 🏻
     * U+1F3FB EMOJI MODIFIER FITZPATRICK TYPE-1-2
     */
    LIGHT(0x1F3FB),

    /**
     * 中淺膚色 🏼
     * U+1F3FC EMOJI MODIFIER FITZPATRICK TYPE-3
     */
    MEDIUM_LIGHT(0x1F3FC),

    /**
     * 中等膚色 🏽
     * U+1F3FD EMOJI MODIFIER FITZPATRICK TYPE-4
     */
    MEDIUM(0x1F3FD),

    /**
     * 中深膚色 🏾
     * U+1F3FE EMOJI MODIFIER FITZPATRICK TYPE-5
     */
    MEDIUM_DARK(0x1F3FE),

    /**
     * 深膚色 🏿
     * U+1F3FF EMOJI MODIFIER FITZPATRICK TYPE-6
     */
    DARK(0x1F3FF),
    ;

    companion object {
        /**
         * 從 codePoint 取得對應的 EmojiSkinTone
         * 如果找不到對應的值，回傳 DEFAULT
         */
        fun fromCodePoint(codePoint: Int): EmojiSkinTone {
            return entries.find { it.codePoint == codePoint } ?: DEFAULT
        }

        /**
         * 取得所有可用的膚色選項（不含 DEFAULT）
         */
        fun availableTones(): List<EmojiSkinTone> {
            return entries.filter { it != DEFAULT }
        }
    }
}

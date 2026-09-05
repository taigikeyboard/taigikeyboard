package com.siansiansu.taigikeyboard.ime.keyboard

/**
 * Unicode emoji skin-tone modifiers: https://unicode.org/reports/tr51/#Emoji_Modifiers
 */
enum class EmojiSkinTone(
    val codePoint: Int,
) {
    /** No modifier applied. */
    DEFAULT(0x0),

    /** U+1F3FB EMOJI MODIFIER FITZPATRICK TYPE-1-2 */
    LIGHT(0x1F3FB),

    /** U+1F3FC EMOJI MODIFIER FITZPATRICK TYPE-3 */
    MEDIUM_LIGHT(0x1F3FC),

    /** U+1F3FD EMOJI MODIFIER FITZPATRICK TYPE-4 */
    MEDIUM(0x1F3FD),

    /** U+1F3FE EMOJI MODIFIER FITZPATRICK TYPE-5 */
    MEDIUM_DARK(0x1F3FE),

    /** U+1F3FF EMOJI MODIFIER FITZPATRICK TYPE-6 */
    DARK(0x1F3FF),
    ;

    companion object {
        fun fromCodePoint(codePoint: Int): EmojiSkinTone = entries.find { it.codePoint == codePoint } ?: DEFAULT

        fun availableTones(): List<EmojiSkinTone> = entries.filter { it != DEFAULT }
    }
}

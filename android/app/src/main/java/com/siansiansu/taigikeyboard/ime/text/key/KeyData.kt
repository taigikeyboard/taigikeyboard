package com.siansiansu.taigikeyboard.ime.text.key

data class KeyData(
    var code: Int,
    var label: String = "",
    var popup: MutableList<KeyData> = mutableListOf(),
    var type: KeyType = KeyType.CHARACTER,
    var variation: KeyVariation = KeyVariation.ALL,
) {
    companion object {
        /** A long-press entry as a key: one code point types itself; longer text has code 0 and types its label. */
        fun ofEntry(entry: String): KeyData = KeyData(if (entry.codePointCount(0, entry.length) == 1) entry.codePointAt(0) else 0, entry)
    }
}

/**
 * A label-driven TPS glyph key carrying long-press variants (code 0 excludes
 * the `，` punctuation key, 65292). Shared by the keycap hint renderer
 * (KeyContent) and the popup base-glyph prepend (`tpsPopupWithBaseGlyph`, PopupCells.kt) so both
 * classify "TPS glyph key" identically; each site supplies its own
 * TPS-active check.
 */
internal fun KeyData.isTpsGlyphWithPopup(): Boolean = type == KeyType.CHARACTER && code == 0 && popup.isNotEmpty()

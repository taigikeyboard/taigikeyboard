package com.siansiansu.taigikeyboard.ime.text.key

data class KeyData(
    var code: Int,
    var label: String = "",
    var popup: MutableList<KeyData> = mutableListOf(),
    var type: KeyType = KeyType.CHARACTER,
    var variation: KeyVariation = KeyVariation.ALL,
)

/**
 * A label-driven TPS glyph key carrying long-press variants (code 0 excludes
 * the `，` punctuation key, 65292). Shared by the keycap hint renderer
 * (KeyContent) and the popup base-glyph prepend (PopupCellResolver) so both
 * classify "TPS glyph key" identically; each site supplies its own
 * TPS-active check.
 */
internal fun KeyData.isTpsGlyphWithPopup(): Boolean =
    type == KeyType.CHARACTER && code == 0 && popup.isNotEmpty()

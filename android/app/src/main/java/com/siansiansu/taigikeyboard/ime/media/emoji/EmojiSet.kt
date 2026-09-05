
package com.siansiansu.taigikeyboard.ime.media.emoji

import androidx.compose.runtime.Stable

/**
 * A base emoji plus its variations (e.g. skin tones); `emojis[0]` is the base, the rest variations.
 */
@Stable
data class EmojiSet(
    val emojis: List<EmojiKeyData>,
) {
    companion object {
        val Unspecified = EmojiSet(listOf(EmojiKeyData(emptyList(), "", "", emptyList())))
    }

    init {
        require(emojis.isNotEmpty()) { "Cannot create an EmojiSet with no emojis specified." }
    }

    private val baseEmoji: EmojiKeyData = emojis[0]

    private val allVariations: List<EmojiKeyData> = if (emojis.size > 1) emojis.drop(1) else emptyList()

    private val skinToneMap: Map<Int, EmojiKeyData> by lazy {
        emojis
            .asSequence()
            .flatMap { emoji -> emoji.codePoints.map { it to emoji } }
            .toMap()
    }

    /** Returns the variation for [withSkinTone] (0 = no preference), falling back to the base emoji. */
    fun base(withSkinTone: Int = 0): EmojiKeyData {
        if (withSkinTone == 0 || emojis.size == 1) return baseEmoji
        return skinToneMap[withSkinTone] ?: baseEmoji
    }

    /** Returns all variations, excluding any carrying the [withoutSkinTone] code point. */
    fun variations(withoutSkinTone: Int = 0): List<EmojiKeyData> {
        if (allVariations.isEmpty()) return emptyList()

        if (withoutSkinTone == 0) return allVariations

        return emojis.filterNot { it.codePoints.contains(withoutSkinTone) }
    }

    override fun hashCode(): Int = baseEmoji.hashCode()

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is EmojiSet) return false
        return baseEmoji == other.baseEmoji
    }
}

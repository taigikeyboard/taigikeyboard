
package com.siansiansu.taigikeyboard.ime.media.emoji

import androidx.compose.runtime.Stable

/**
 * Emoji 集合，包含一個基礎 emoji 及其變體（例如不同膚色）
 *
 * @property emojis 此集合中的所有 emoji（第一個為基礎 emoji，其餘為變體）
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

    // 快取基礎 emoji
    private val baseEmoji: EmojiKeyData = emojis[0]

    // 快取所有變體（不含基礎 emoji）
    private val allVariations: List<EmojiKeyData> = if (emojis.size > 1) emojis.drop(1) else emptyList()

    // 快取膚色到 emoji 的映射
    private val skinToneMap: Map<Int, EmojiKeyData> by lazy {
        emojis
            .asSequence()
            .flatMap { emoji -> emoji.codePoints.map { it to emoji } }
            .toMap()
    }

    /**
     * 取得基礎 emoji 或指定膚色的變體
     *
     * @param withSkinTone 期望的膚色 code point（預設為 0 代表無膚色偏好）
     * @return 符合條件的 emoji，若無則返回第一個 emoji
     */
    fun base(withSkinTone: Int = 0): EmojiKeyData {
        if (withSkinTone == 0 || emojis.size == 1) return baseEmoji
        return skinToneMap[withSkinTone] ?: baseEmoji
    }

    /**
     * 取得所有變體（不包含指定膚色的變體）
     *
     * @param withoutSkinTone 要排除的膚色 code point
     * @return 變體列表（若無變體則返回空列表）
     */
    fun variations(withoutSkinTone: Int = 0): List<EmojiKeyData> {
        if (allVariations.isEmpty()) return emptyList()

        if (withoutSkinTone == 0) return allVariations

        // 過濾掉包含指定膚色的 emoji
        return emojis.filterNot { it.codePoints.contains(withoutSkinTone) }
    }

    override fun hashCode(): Int = baseEmoji.hashCode()

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is EmojiSet) return false
        return baseEmoji == other.baseEmoji
    }
}

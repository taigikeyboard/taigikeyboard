package com.siansiansu.taigikeyboard.ime.media.emoji

import kotlinx.serialization.Serializable

/**
 * Emoji 歷史記錄資料模型
 *
 * @property pinned 釘選的 emoji 列表（使用 Unicode 字串儲存）
 * @property recent 最近使用的 emoji 列表（最多 30 個）
 * @property lastUpdated 最後更新時間戳
 */
@Serializable
data class EmojiHistory(
    val pinned: List<String> = emptyList(),
    val recent: List<String> = emptyList(),
    val lastUpdated: Long = System.currentTimeMillis()
) {
    companion object {
        /**
         * 最近使用列表的最大長度
         */
        const val MAX_RECENT_SIZE = 30
    }

    /**
     * 取得所有歷史記錄（釘選 + 最近使用）
     * 用於 RECENTLY_USED 分類顯示
     */
    fun getAllEmojis(): List<String> = pinned + recent

    /**
     * 檢查 emoji 是否已被釘選
     */
    fun isPinned(emoji: String): Boolean = pinned.contains(emoji)
}

package com.siansiansu.taigikeyboard.ime.media.emoji

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

/**
 * Emoji 歷史記錄管理器
 * 負責儲存和管理使用者的 emoji 使用記錄
 */
class EmojiHistoryManager(private val context: Context) {

    companion object {
        private val Context.emojiHistoryDataStore: DataStore<Preferences> by preferencesDataStore(
            name = "emoji_history"
        )
        private val HISTORY_KEY = stringPreferencesKey("emoji_history_json")

        private val json = Json {
            encodeDefaults = true
            ignoreUnknownKeys = true
        }
    }

    /**
     * 觀察 emoji 歷史記錄的 Flow
     */
    fun getHistory(): Flow<EmojiHistory> {
        return context.emojiHistoryDataStore.data.map { preferences ->
            val jsonString = preferences[HISTORY_KEY] ?: ""
            if (jsonString.isEmpty()) {
                EmojiHistory()
            } else {
                try {
                    json.decodeFromString<EmojiHistory>(jsonString)
                } catch (e: Exception) {
                    // 解析失敗時返回空歷史記錄
                    EmojiHistory()
                }
            }
        }
    }

    /**
     * 記錄 emoji 使用
     * 將 emoji 移到 recent 列表最前方，如果超過上限則移除最舊的
     *
     * @param emoji 使用的 emoji unicode 字串
     */
    suspend fun markEmojiUsed(emoji: String) {
        context.emojiHistoryDataStore.edit { preferences ->
            val currentHistory = getCurrentHistory(preferences)

            // 如果已經在 pinned 中，不加入 recent
            if (currentHistory.isPinned(emoji)) {
                return@edit
            }

            // 從 recent 中移除該 emoji（如果存在）
            val updatedRecent = currentHistory.recent
                .filter { it != emoji }
                .toMutableList()

            // 加到最前方
            updatedRecent.add(0, emoji)

            // 限制列表長度
            val trimmedRecent = if (updatedRecent.size > EmojiHistory.MAX_RECENT_SIZE) {
                updatedRecent.take(EmojiHistory.MAX_RECENT_SIZE)
            } else {
                updatedRecent
            }

            val newHistory = currentHistory.copy(
                recent = trimmedRecent,
                lastUpdated = System.currentTimeMillis()
            )

            preferences[HISTORY_KEY] = json.encodeToString(newHistory)
        }
    }

    /**
     * 釘選 emoji
     * 將 emoji 加到 pinned 列表最後方，並從 recent 中移除
     *
     * @param emoji 要釘選的 emoji unicode 字串
     */
    suspend fun pinEmoji(emoji: String) {
        context.emojiHistoryDataStore.edit { preferences ->
            val currentHistory = getCurrentHistory(preferences)

            // 如果已經釘選，不重複操作
            if (currentHistory.isPinned(emoji)) {
                return@edit
            }

            val updatedPinned = currentHistory.pinned + emoji
            val updatedRecent = currentHistory.recent.filter { it != emoji }

            val newHistory = currentHistory.copy(
                pinned = updatedPinned,
                recent = updatedRecent,
                lastUpdated = System.currentTimeMillis()
            )

            preferences[HISTORY_KEY] = json.encodeToString(newHistory)
        }
    }

    /**
     * 取消釘選 emoji
     * 從 pinned 列表中移除
     *
     * @param emoji 要取消釘選的 emoji unicode 字串
     */
    suspend fun unpinEmoji(emoji: String) {
        context.emojiHistoryDataStore.edit { preferences ->
            val currentHistory = getCurrentHistory(preferences)

            val updatedPinned = currentHistory.pinned.filter { it != emoji }

            val newHistory = currentHistory.copy(
                pinned = updatedPinned,
                lastUpdated = System.currentTimeMillis()
            )

            preferences[HISTORY_KEY] = json.encodeToString(newHistory)
        }
    }

    /**
     * 從 recent 列表中移除 emoji
     *
     * @param emoji 要移除的 emoji unicode 字串
     */
    suspend fun removeFromRecent(emoji: String) {
        context.emojiHistoryDataStore.edit { preferences ->
            val currentHistory = getCurrentHistory(preferences)

            val updatedRecent = currentHistory.recent.filter { it != emoji }

            val newHistory = currentHistory.copy(
                recent = updatedRecent,
                lastUpdated = System.currentTimeMillis()
            )

            preferences[HISTORY_KEY] = json.encodeToString(newHistory)
        }
    }

    /**
     * 清空所有歷史記錄
     */
    suspend fun clearHistory() {
        context.emojiHistoryDataStore.edit { preferences ->
            preferences[HISTORY_KEY] = json.encodeToString(EmojiHistory())
        }
    }

    /**
     * 從 Preferences 中讀取當前歷史記錄
     */
    private fun getCurrentHistory(preferences: Preferences): EmojiHistory {
        val jsonString = preferences[HISTORY_KEY] ?: ""
        return if (jsonString.isEmpty()) {
            EmojiHistory()
        } else {
            try {
                json.decodeFromString<EmojiHistory>(jsonString)
            } catch (e: Exception) {
                EmojiHistory()
            }
        }
    }
}

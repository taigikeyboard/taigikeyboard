package com.siansiansu.taigikeyboard.ime.media.emoji

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import com.siansiansu.taigikeyboard.ime.keyboard.EmojiSkinTone
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

/**
 * Emoji 偏好設定管理器
 * 負責儲存和管理使用者的 emoji 相關偏好設定
 */
class EmojiPreferences(
    private val context: Context,
) {
    companion object {
        private val Context.emojiPreferencesDataStore: DataStore<Preferences> by preferencesDataStore(
            name = "emoji_preferences",
        )
        private val PREFERRED_SKIN_TONE_KEY = intPreferencesKey("preferred_skin_tone")
    }

    /**
     * 觀察使用者偏好的膚色設定
     * 返回 EmojiSkinTone 的 Flow
     */
    fun getPreferredSkinTone(): Flow<EmojiSkinTone> =
        context.emojiPreferencesDataStore.data.map { preferences ->
            val codePoint = preferences[PREFERRED_SKIN_TONE_KEY] ?: EmojiSkinTone.DEFAULT.codePoint
            EmojiSkinTone.fromCodePoint(codePoint)
        }

    /**
     * 儲存使用者偏好的膚色設定
     *
     * @param skinTone 要儲存的膚色設定
     */
    suspend fun setPreferredSkinTone(skinTone: EmojiSkinTone) {
        context.emojiPreferencesDataStore.edit { preferences ->
            preferences[PREFERRED_SKIN_TONE_KEY] = skinTone.codePoint
        }
    }
}

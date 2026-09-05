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

/** Manages storage of the user's emoji-related preferences. */
class EmojiPreferences(
    private val context: Context,
) {
    companion object {
        private val Context.emojiPreferencesDataStore: DataStore<Preferences> by preferencesDataStore(
            name = "emoji_preferences",
        )
        private val PREFERRED_SKIN_TONE_KEY = intPreferencesKey("preferred_skin_tone")
    }

    /** Observes the user's preferred skin tone. */
    fun getPreferredSkinTone(): Flow<EmojiSkinTone> =
        context.emojiPreferencesDataStore.data.map { preferences ->
            val codePoint = preferences[PREFERRED_SKIN_TONE_KEY] ?: EmojiSkinTone.DEFAULT.codePoint
            EmojiSkinTone.fromCodePoint(codePoint)
        }

    /** Persists the user's preferred skin tone. */
    suspend fun setPreferredSkinTone(skinTone: EmojiSkinTone) {
        context.emojiPreferencesDataStore.edit { preferences ->
            preferences[PREFERRED_SKIN_TONE_KEY] = skinTone.codePoint
        }
    }
}

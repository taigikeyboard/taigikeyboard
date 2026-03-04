package com.siansiansu.taigikeyboard.ime.text.composing

import android.content.Context
import android.os.Bundle
import android.util.Log
import android.view.textservice.SentenceSuggestionsInfo
import android.view.textservice.SpellCheckerSession
import android.view.textservice.SuggestionsInfo
import android.view.textservice.TextInfo
import android.view.textservice.TextServicesManager
import com.siansiansu.taigikeyboard.BuildConfig
import java.util.Locale
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

/**
 * 英文自動補全服務
 *
 * 使用 Android 系統的 SpellCheckerSession 提供英文拼字建議。
 * 類似 iOS 的 UITextChecker 功能。
 *
 * 功能：
 * - 拼字校正：偵測拼錯的單字並提供建議
 * - 自動補全：根據部分輸入預測完整單字（受系統限制）
 */
class EnglishAutocompleteService(
    private val context: Context
) {
    companion object {
        private const val MAX_SUGGESTIONS = 3
    }

    // SpellChecker Session（懶載入）
    private var spellCheckerSession: SpellCheckerSession? = null
    private var pendingSuggestions: ((List<String>) -> Unit)? = null

    // SpellCheckerSessionListener 實作
    private val spellCheckerListener = object : SpellCheckerSession.SpellCheckerSessionListener {
        override fun onGetSuggestions(results: Array<out SuggestionsInfo>?) {
            if (BuildConfig.DEBUG) {
                Log.d("ENSPELL", "[CALLBACK] onGetSuggestions: ${results?.size} results")
            }

            val suggestions = mutableListOf<String>()
            results?.forEachIndexed { idx, info ->
                val attrs = info.suggestionsAttributes
                if (BuildConfig.DEBUG) {
                    Log.d("ENSPELL", "[CALLBACK] result[$idx] attrs=$attrs, count=${info.suggestionsCount}")
                }
                for (i in 0 until info.suggestionsCount) {
                    suggestions.add(info.getSuggestionAt(i))
                }
            }

            if (BuildConfig.DEBUG) {
                Log.d("ENSPELL", "[CALLBACK] Parsed ${suggestions.size} suggestions: $suggestions")
            }

            pendingSuggestions?.invoke(suggestions.take(MAX_SUGGESTIONS))
            pendingSuggestions = null
        }

        override fun onGetSentenceSuggestions(results: Array<out SentenceSuggestionsInfo>?) {
            if (BuildConfig.DEBUG) {
                Log.d("ENSPELL", "[CALLBACK] onGetSentenceSuggestions: ${results?.size} results")
            }

            val suggestions = mutableListOf<String>()
            results?.forEachIndexed { resultIdx, sentenceInfo ->
                if (BuildConfig.DEBUG) {
                    Log.d("ENSPELL", "[CALLBACK] result[$resultIdx] suggestionsCount=${sentenceInfo.suggestionsCount}")
                }
                for (i in 0 until sentenceInfo.suggestionsCount) {
                    val suggestionsInfo = sentenceInfo.getSuggestionsInfoAt(i)
                    val attrs = suggestionsInfo.suggestionsAttributes
                    if (BuildConfig.DEBUG) {
                        Log.d("ENSPELL", "[CALLBACK]   [$i] attrs=$attrs, count=${suggestionsInfo.suggestionsCount}")
                    }
                    for (j in 0 until suggestionsInfo.suggestionsCount) {
                        suggestions.add(suggestionsInfo.getSuggestionAt(j))
                    }
                }
            }

            if (BuildConfig.DEBUG) {
                Log.d("ENSPELL", "[CALLBACK] Parsed ${suggestions.size} suggestions: $suggestions")
            }

            pendingSuggestions?.invoke(suggestions.take(MAX_SUGGESTIONS))
            pendingSuggestions = null
        }
    }

    /**
     * 初始化 SpellChecker Session
     */
    private fun initSpellChecker() {
        if (spellCheckerSession != null) {
            if (BuildConfig.DEBUG) Log.d("ENSPELL", "[7] SpellCheckerSession already exists")
            return
        }

        if (BuildConfig.DEBUG) Log.d("ENSPELL", "[7] Initializing SpellCheckerSession...")

        try {
            val tsm = context.getSystemService(Context.TEXT_SERVICES_MANAGER_SERVICE) as? TextServicesManager
            if (tsm == null) {
                if (BuildConfig.DEBUG) {
                    Log.w("ENSPELL", "[7] TextServicesManager not available")
                }
                return
            }

            if (BuildConfig.DEBUG) Log.d("ENSPELL", "[7] TextServicesManager available, creating session...")

            spellCheckerSession = tsm.newSpellCheckerSession(
                Bundle(),
                Locale.ENGLISH,
                spellCheckerListener,
                false  // referToSpellCheckerLanguageSettings
            )

            if (spellCheckerSession != null) {
                if (BuildConfig.DEBUG) {
                    Log.d("ENSPELL", "[7] SpellCheckerSession created successfully")
                }
            } else {
                if (BuildConfig.DEBUG) {
                    Log.w("ENSPELL", "[7] newSpellCheckerSession returned null - no spell checker available on device?")
                }
            }
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                Log.e("ENSPELL", "[7] Failed to init SpellCheckerSession", e)
            }
        }
    }

    /**
     * 取得英文建議
     *
     * @param text 目前輸入的文字（游標前的完整文字）
     * @return 建議列表
     */
    suspend fun getSuggestions(text: String): List<EnglishSuggestion> {
        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[GET] getSuggestions() called with text='$text'")
        }

        val currentWord = extractCurrentWord(text)
        if (currentWord.isEmpty()) {
            if (BuildConfig.DEBUG) Log.d("ENSPELL", "[GET] currentWord is empty, returning")
            return emptyList()
        }

        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[GET] currentWord='$currentWord'")
        }

        // 嘗試使用系統 SpellChecker
        if (BuildConfig.DEBUG) Log.d("ENSPELL", "[GET] Calling getSpellCheckerSuggestions()...")
        val spellSuggestions = getSpellCheckerSuggestions(currentWord)
        if (BuildConfig.DEBUG) Log.d("ENSPELL", "[GET] getSpellCheckerSuggestions() returned ${spellSuggestions.size} suggestions")

        return if (spellSuggestions.isNotEmpty()) {
            spellSuggestions.map { EnglishSuggestion(text = it) }
        } else {
            // 無建議時，至少回傳當前輸入
            if (BuildConfig.DEBUG) Log.d("ENSPELL", "[GET] No suggestions, returning empty list")
            emptyList()
        }
    }

    /**
     * 從系統 SpellChecker 取得建議
     */
    private suspend fun getSpellCheckerSuggestions(word: String): List<String> {
        if (BuildConfig.DEBUG) Log.d("ENSPELL", "[SPELL-GET] getSpellCheckerSuggestions('$word') called")

        initSpellChecker()

        val session = spellCheckerSession
        if (session == null) {
            if (BuildConfig.DEBUG) {
                Log.w("ENSPELL", "[SPELL-GET] SpellCheckerSession is null, returning empty")
            }
            return emptyList()
        }

        if (BuildConfig.DEBUG) Log.d("ENSPELL", "[SPELL-GET] Session available, calling suspendCoroutine...")

        return try {
            kotlinx.coroutines.withTimeout(2000L) { // 2秒超時
                suspendCoroutine { continuation ->
                    pendingSuggestions = { suggestions ->
                        if (BuildConfig.DEBUG) Log.d("ENSPELL", "[SPELL-CALLBACK] Received ${suggestions.size} suggestions")
                        continuation.resume(suggestions)
                    }

                    try {
                        // 使用 getSentenceSuggestions 檢查單一詞彙
                        val textInfo = TextInfo(word)
                        if (BuildConfig.DEBUG) Log.d("ENSPELL", "[SPELL-GET] Calling session.getSentenceSuggestions()...")
                        session.getSentenceSuggestions(arrayOf(textInfo), MAX_SUGGESTIONS)

                        if (BuildConfig.DEBUG) {
                            Log.d("ENSPELL", "[SPELL-GET] getSuggestions() called, waiting for callback...")
                        }
                    } catch (e: Exception) {
                        if (BuildConfig.DEBUG) {
                            Log.e("ENSPELL", "[SPELL-GET] Failed to call getSuggestions", e)
                        }
                        continuation.resume(emptyList())
                    }
                }
            }
        } catch (e: kotlinx.coroutines.TimeoutCancellationException) {
            if (BuildConfig.DEBUG) {
                Log.w("ENSPELL", "[SPELL-GET] Timeout waiting for spell checker callback")
            }
            emptyList()
        }
    }

    /**
     * 從輸入文字中提取當前單字（最後一個空白後的文字）
     */
    private fun extractCurrentWord(text: String): String {
        val trimmed = text.trimEnd()
        if (trimmed.isEmpty()) return ""

        // 找到最後一個空白或標點
        val lastSeparatorIndex = trimmed.indexOfLast { it.isWhitespace() || it in ".,!?;:" }

        return if (lastSeparatorIndex >= 0) {
            trimmed.substring(lastSeparatorIndex + 1)
        } else {
            trimmed
        }
    }

    /**
     * 關閉 SpellChecker Session
     */
    fun close() {
        spellCheckerSession?.close()
        spellCheckerSession = null
        pendingSuggestions = null

        if (BuildConfig.DEBUG) {
            Log.d("ENSPELL", "[SPELL] SpellCheckerSession closed")
        }
    }
}

/**
 * 英文建議資料類別
 */
data class EnglishSuggestion(
    val text: String
)

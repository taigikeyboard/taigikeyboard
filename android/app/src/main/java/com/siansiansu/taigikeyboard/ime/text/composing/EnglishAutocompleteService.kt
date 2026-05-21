// 中文: 英文輸入模式的拼字建議服務 — 使用 Android SpellCheckerSession,
// 中文: 對應 iOS UITextChecker。提供拼字校正 + 受限的自動補全(系統限制)。
// 中文: 僅 English InputMode 使用,Taigi 路徑走 TaigiAutocompleteService + Rust lexicon。

package com.siansiansu.taigikeyboard.ime.text.composing

import android.content.Context
import android.os.Bundle
import android.view.textservice.SentenceSuggestionsInfo
import android.view.textservice.SpellCheckerSession
import android.view.textservice.SuggestionsInfo
import android.view.textservice.TextInfo
import android.view.textservice.TextServicesManager
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import java.util.Locale
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

private const val TAG = "ENSPELL"

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
    private val context: Context,
) {
    companion object {
        private const val MAX_SUGGESTIONS = 3
    }

    private val logger = CompositionRoot.shared(context).logger

    // SpellChecker Session（懶載入）
    private var spellCheckerSession: SpellCheckerSession? = null
    private var pendingSuggestions: ((List<String>) -> Unit)? = null

    // SpellCheckerSessionListener 實作
    private val spellCheckerListener = object : SpellCheckerSession.SpellCheckerSessionListener {
        override fun onGetSuggestions(results: Array<out SuggestionsInfo>?) {
            logger.debug(TAG) { "[CALLBACK] onGetSuggestions: ${results?.size} results" }

            val suggestions = mutableListOf<String>()
            results?.forEachIndexed { idx, info ->
                if (info == null) return@forEachIndexed
                val attrs = info.suggestionsAttributes
                logger.debug(TAG) { "[CALLBACK] result[$idx] attrs=$attrs, count=${info.suggestionsCount}" }
                for (i in 0 until info.suggestionsCount) {
                    suggestions.add(info.getSuggestionAt(i))
                }
            }

            logger.debug(TAG) { "[CALLBACK] Parsed ${suggestions.size} suggestions: $suggestions" }

            pendingSuggestions?.invoke(suggestions.take(MAX_SUGGESTIONS))
            pendingSuggestions = null
        }

        override fun onGetSentenceSuggestions(results: Array<out SentenceSuggestionsInfo>?) {
            logger.debug(TAG) { "[CALLBACK] onGetSentenceSuggestions: ${results?.size} results" }

            val suggestions = mutableListOf<String>()
            results?.forEachIndexed { resultIdx, sentenceInfo ->
                if (sentenceInfo == null) return@forEachIndexed
                logger.debug(TAG) {
                    "[CALLBACK] result[$resultIdx] suggestionsCount=${sentenceInfo.suggestionsCount}"
                }
                for (i in 0 until sentenceInfo.suggestionsCount) {
                    val suggestionsInfo = sentenceInfo.getSuggestionsInfoAt(i)
                        ?: continue
                    val attrs = suggestionsInfo.suggestionsAttributes
                    logger.debug(TAG) {
                        "[CALLBACK]   [$i] attrs=$attrs, count=${suggestionsInfo.suggestionsCount}"
                    }
                    for (j in 0 until suggestionsInfo.suggestionsCount) {
                        suggestions.add(suggestionsInfo.getSuggestionAt(j))
                    }
                }
            }

            logger.debug(TAG) { "[CALLBACK] Parsed ${suggestions.size} suggestions: $suggestions" }

            pendingSuggestions?.invoke(suggestions.take(MAX_SUGGESTIONS))
            pendingSuggestions = null
        }
    }

    /**
     * 初始化 SpellChecker Session
     */
    private fun initSpellChecker() {
        if (spellCheckerSession != null) {
            logger.debug(TAG) { "[7] SpellCheckerSession already exists" }
            return
        }

        logger.debug(TAG) { "[7] Initializing SpellCheckerSession..." }

        try {
            val tsm = context.getSystemService(Context.TEXT_SERVICES_MANAGER_SERVICE) as? TextServicesManager
            if (tsm == null) {
                logger.w(TAG, "[7] TextServicesManager not available")
                return
            }

            logger.debug(TAG) { "[7] TextServicesManager available, creating session..." }

            spellCheckerSession = tsm.newSpellCheckerSession(
                Bundle(),
                Locale.ENGLISH,
                spellCheckerListener,
                false, // referToSpellCheckerLanguageSettings
            )

            if (spellCheckerSession != null) {
                logger.debug(TAG) { "[7] SpellCheckerSession created successfully" }
            } else {
                logger.w(TAG, "[7] newSpellCheckerSession returned null - no spell checker available on device?")
            }
        } catch (e: Exception) {
            logger.e(TAG, "[7] Failed to init SpellCheckerSession", e)
        }
    }

    /**
     * 取得英文建議
     *
     * @param text 目前輸入的文字（游標前的完整文字）
     * @return 建議列表
     */
    suspend fun getSuggestions(text: String): List<EnglishSuggestion> {
        logger.debug(TAG) { "[GET] getSuggestions() called with text='$text'" }

        val currentWord = extractCurrentWord(text)
        if (currentWord.isEmpty()) {
            logger.debug(TAG) { "[GET] currentWord is empty, returning" }
            return emptyList()
        }

        logger.debug(TAG) { "[GET] currentWord='$currentWord'" }

        // 嘗試使用系統 SpellChecker
        logger.debug(TAG) { "[GET] Calling getSpellCheckerSuggestions()..." }
        val spellSuggestions = getSpellCheckerSuggestions(currentWord)
        logger.debug(TAG) {
            "[GET] getSpellCheckerSuggestions() returned ${spellSuggestions.size} suggestions"
        }

        return if (spellSuggestions.isNotEmpty()) {
            spellSuggestions.map { EnglishSuggestion(text = it) }
        } else {
            // 無建議時，至少回傳當前輸入
            logger.debug(TAG) { "[GET] No suggestions, returning empty list" }
            emptyList()
        }
    }

    /**
     * 從系統 SpellChecker 取得建議
     */
    private suspend fun getSpellCheckerSuggestions(word: String): List<String> {
        logger.debug(TAG) { "[SPELL-GET] getSpellCheckerSuggestions('$word') called" }

        initSpellChecker()

        val session = spellCheckerSession
        if (session == null) {
            logger.w(TAG, "[SPELL-GET] SpellCheckerSession is null, returning empty")
            return emptyList()
        }

        logger.debug(TAG) { "[SPELL-GET] Session available, calling suspendCoroutine..." }

        return try {
            kotlinx.coroutines.withTimeout(2000L) {
                // 2秒超時
                suspendCoroutine { continuation ->
                    pendingSuggestions = { suggestions ->
                        logger.debug(TAG) {
                            "[SPELL-CALLBACK] Received ${suggestions.size} suggestions"
                        }
                        continuation.resume(suggestions)
                    }

                    try {
                        // 使用 getSentenceSuggestions 檢查單一詞彙
                        val textInfo = TextInfo(word)
                        logger.debug(TAG) {
                            "[SPELL-GET] Calling session.getSentenceSuggestions()..."
                        }
                        session.getSentenceSuggestions(arrayOf(textInfo), MAX_SUGGESTIONS)

                        logger.debug(TAG) {
                            "[SPELL-GET] getSuggestions() called, waiting for callback..."
                        }
                    } catch (e: Exception) {
                        logger.e(TAG, "[SPELL-GET] Failed to call getSuggestions", e)
                        continuation.resume(emptyList())
                    }
                }
            }
        } catch (e: kotlinx.coroutines.TimeoutCancellationException) {
            logger.w(TAG, "[SPELL-GET] Timeout waiting for spell checker callback")
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

        logger.debug(TAG) { "[SPELL] SpellCheckerSession closed" }
    }
}

/**
 * 英文建議資料類別
 */
data class EnglishSuggestion(
    val text: String,
)

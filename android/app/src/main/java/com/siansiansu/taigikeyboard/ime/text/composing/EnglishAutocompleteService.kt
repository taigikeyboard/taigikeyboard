// 中文: 英文輸入模式的拼字建議服務 — 使用自帶英文頻率詞表 (EnglishWordMatcher)。
// 中文: 對應 iOS UITextChecker,但 iOS 留用系統 UITextChecker (deliberate cross-platform
// 中文: divergence) — Android 改自帶詞表,擺脫裝置系統 SpellCheckerSession 的相依性。
// 中文: 僅 English InputMode 使用,Taigi 路徑走 TaigiAutocompleteService + Rust lexicon。

package com.siansiansu.taigikeyboard.ime.text.composing

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

private const val TAG = "ENSPELL"

/**
 * 英文自動補全服務
 *
 * 使用自帶的英文頻率詞表 (assets/english_freq.txt) 提供拼字補全 + 校正,
 * 不依賴裝置的系統 SpellCheckerSession (部分機型未內建,候選會永遠空白)。
 *
 * 功能:
 * - 拼字校正:Damerau-Levenshtein (OSA) 距離 <= 2 的相近詞
 * - 自動補全:依輸入前綴預測完整單字
 *
 * 詞表載入失敗時 fail-closed (回空候選,不 crash IME)。比對邏輯在純 Kotlin 的
 * [EnglishWordMatcher];本類別只負責 asset I/O 與生命週期。
 */
class EnglishAutocompleteService(
    private val context: Context,
) {
    companion object {
        private const val MAX_SUGGESTIONS = 3
        private const val ASSET_NAME = "english_freq.txt"
    }

    private val logger = CompositionRoot.shared(context).logger

    private val matcherLock = Any()

    @Volatile
    private var matcher: EnglishWordMatcher? = null

    @Volatile
    private var loadFailed = false

    /**
     * 取得英文建議。
     *
     * @param text 目前輸入的文字(游標前的完整文字)
     * @return 建議列表(re-cased 對齊輸入大小寫),無 match 或載入失敗時為空
     */
    suspend fun getSuggestions(text: String): List<EnglishSuggestion> {
        val currentWord = extractCurrentWord(text)
        if (currentWord.isEmpty()) return emptyList()

        val activeMatcher = ensureMatcher() ?: return emptyList()

        val suggestions = withContext(Dispatchers.Default) {
            activeMatcher.suggest(currentWord, MAX_SUGGESTIONS)
        }
        logger.debug(TAG) { "[GET] '$currentWord' → ${suggestions.size} suggestions" }
        return suggestions.map { EnglishSuggestion(text = it) }
    }

    // Lazily builds the matcher from the bundled asset on first use. Fail-closed:
    // a load failure is logged once and cached so we never re-attempt or crash.
    private suspend fun ensureMatcher(): EnglishWordMatcher? {
        matcher?.let { return it }
        if (loadFailed) return null
        return withContext(Dispatchers.IO) {
            synchronized(matcherLock) {
                // Re-check both under the lock so a concurrent first caller neither
                // reloads a matcher already built nor re-attempts a load that failed.
                matcher ?: if (loadFailed) null else loadMatcher()?.also { matcher = it }
            }
        }
    }

    private fun loadMatcher(): EnglishWordMatcher? = try {
        val entries = context.assets.open(ASSET_NAME).bufferedReader().use { reader ->
            EnglishWordMatcher.parseEntries(reader.lineSequence())
        }
        logger.debug(TAG) { "[LOAD] english_freq.txt → ${entries.size} entries" }
        EnglishWordMatcher(entries)
    } catch (e: Exception) {
        loadFailed = true
        logger.e(TAG, "[LOAD] failed to load $ASSET_NAME — English suggestions disabled", e)
        null
    }

    /**
     * 從輸入文字中提取當前單字(最後一個分隔符後的文字)。
     * 分隔符 = 空白 + `.,!?;:`;撇號/連字號不分隔(`don't` 視為單一 token)。
     */
    private fun extractCurrentWord(text: String): String {
        val trimmed = text.trimEnd()
        if (trimmed.isEmpty()) return ""

        val lastSeparatorIndex = trimmed.indexOfLast { it.isWhitespace() || it in ".,!?;:" }

        return if (lastSeparatorIndex >= 0) {
            trimmed.substring(lastSeparatorIndex + 1)
        } else {
            trimmed
        }
    }

    /**
     * 釋放詞表記憶體(在 onDestroy 呼叫)。
     */
    fun close() {
        synchronized(matcherLock) {
            matcher = null
        }
        logger.debug(TAG) { "[CLOSE] matcher released" }
    }
}

/**
 * 英文建議資料類別
 */
data class EnglishSuggestion(
    val text: String,
)

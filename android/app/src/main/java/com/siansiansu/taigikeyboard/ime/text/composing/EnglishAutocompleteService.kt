// Spell suggestions for English InputMode, backed by the bundled frequency list (EnglishWordMatcher).
// Deliberate cross-platform divergence: iOS keeps the system UITextChecker, Android ships its own list
// so it does not depend on the device SpellCheckerSession. Taigi input uses TaigiAutocompleteService.

package com.siansiansu.taigikeyboard.ime.text.composing

import android.content.Context
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

private const val TAG = "ENSPELL"

/**
 * Completes and corrects English words from the bundled frequency list (assets/english_freq.txt).
 *
 * Correction accepts Damerau-Levenshtein (OSA) distance <= 2; completion predicts from the typed prefix.
 * Fail-closed on load failure (empty candidates, never crashes the IME). Matching lives in
 * [EnglishWordMatcher]; this class owns only asset I/O and lifecycle.
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

    /** Suggestions for the current word in [text] (full text before the cursor), re-cased to match it. */
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

    // Separators are whitespace and `.,!?;:` — apostrophe and hyphen do not split, so `don't` stays one token.
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

    /** Releases the word list; call from onDestroy. */
    fun close() {
        synchronized(matcherLock) {
            matcher = null
        }
        logger.debug(TAG) { "[CLOSE] matcher released" }
    }
}

data class EnglishSuggestion(
    val text: String,
)

package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.util.Log
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiPhonetics
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/**
 * Handles NextWord prediction logic extracted from SmartbarManager.
 *
 * Manages context tracking (last selected word, timing) and NextWord predictions
 * including compound word splitting and association recording.
 */
class NextWordHandler(
    private val scope: CoroutineScope,
    private val prefs: PrefHelper,
    private val taigikeyboard: TaigiKeyboard,
    private val isTranslateSwapped: () -> Boolean,
    private val onUpdateCandidates: (List<TaigiWord>) -> Unit,
    private val onClearCandidates: () -> Unit
) {
    private var lastSelectedWord: String? = null
    private var lastSelectionTime: Long = 0
    var isShowingNextWord: Boolean = false
        private set

    fun getLastSelectedWord(): String? = lastSelectedWord

    fun isShowingNextWordCandidates(): Boolean = isShowingNextWord

    fun resetContext() {
        lastSelectedWord = null
        lastSelectionTime = 0
    }

    fun clearNextWordState() {
        isShowingNextWord = false
    }

    fun setShowingNextWord(showing: Boolean) {
        isShowingNextWord = showing
    }

    /**
     * Handle NextWord prediction after a word is selected.
     */
    fun handleNextWordPrediction(displayText: String, committedText: String, roman: String, hanzi: String? = null, rawInput: String = "") {
        val currentTime = System.currentTimeMillis()

        // Check if context should be reset
        val shouldReset = when {
            committedText.lastOrNull() in SENTENCE_END_PUNCTUATION -> true
            lastSelectionTime > 0 && (currentTime - lastSelectionTime) > CONTEXT_TIMEOUT_MS -> true
            else -> false
        }

        if (shouldReset) {
            lastSelectedWord = null
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[NEXTWORD] Context reset")
            }
        }

        val shouldRecordAssociation = lastSelectedWord != null &&
            (currentTime - lastSelectionTime) < ASSOCIATION_TIMEOUT_MS

        val parts = splitCompoundWord(displayText)
        val romanParts = splitCompoundWord(roman)
        val prevWord = lastSelectedWord

        scope.launch {
            val useTl = (prefs.inputMode == "tl")

            if (shouldRecordAssociation && prevWord != null) {
                if (!isNoise(displayText)) {
                    val nextTl = if (useTl) roman else ""
                    NextWordService.recordAssociation(
                        prev = prevWord,
                        nextHanzi = displayText,
                        nextTl = nextTl,
                        context = taigikeyboard.context
                    )
                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[NEXTWORD] Record: '$prevWord' → '$displayText'")
                    }
                } else if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD] Skip noise: '$displayText'")
                }
            }

            // Record compound word internal associations
            for (i in 0 until parts.size - 1) {
                val prevPart = parts[i]
                val nextPart = parts[i + 1]
                val nextRoman = romanParts.getOrNull(i + 1) ?: ""
                val nextTl = if (useTl) nextRoman else ""
                NextWordService.recordAssociation(
                    prev = prevPart,
                    nextHanzi = nextPart,
                    nextTl = nextTl,
                    context = taigikeyboard.context
                )
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD] Record compound: '$prevPart' → '$nextPart'")
                }
            }

            val predictions = NextWordService.predict(
                word = displayText,
                context = taigikeyboard.context,
                prefs = taigikeyboard.prefs
            )

            kotlinx.coroutines.withContext(Dispatchers.Main) {
                if (predictions.isNotEmpty()) {
                    updateCandidatesWithPredictions(predictions)
                } else {
                    onClearCandidates()
                }
            }
        }

        // Update context (noise doesn't update lastSelectedWord)
        if (!isNoise(displayText)) {
            lastSelectedWord = displayText
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[NEXTWORD] lastSelectedWord updated: '$displayText'")
            }
        } else if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] Skip updating lastSelectedWord for noise: '$displayText'")
        }
        lastSelectionTime = currentTime
    }

    /**
     * Update lastSelectedWord without triggering NextWord prediction.
     * Used when space key confirms composing text.
     */
    fun updateLastSelectedWord(word: String) {
        if (word.isEmpty()) return

        val parts = splitCompoundWord(word)

        if (parts.size > 1) {
            val useTl = (prefs.inputMode == "tl")
            scope.launch {
                for (i in 0 until parts.size - 1) {
                    val prevPart = parts[i]
                    val nextPart = parts[i + 1]
                    val nextTl = if (useTl) nextPart else ""
                    NextWordService.recordAssociation(
                        prev = prevPart,
                        nextHanzi = nextPart,
                        nextTl = nextTl,
                        context = taigikeyboard.context
                    )
                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[NEXTWORD] Record compound (space): '$prevPart' → '$nextPart'")
                    }
                }
            }
        }

        if (!isNoise(word)) {
            lastSelectedWord = word
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[NEXTWORD] updateLastSelectedWord: '$word'")
            }
        } else if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] updateLastSelectedWord: skip noise '$word'")
        }
        lastSelectionTime = System.currentTimeMillis()
    }

    /**
     * Handle backspace: re-predict from remaining text.
     */
    fun handleBackspaceForNextWord(textBeforeCursor: String) {
        val trimmedText = textBeforeCursor.trimEnd()

        if (trimmedText.isEmpty()) {
            onClearCandidates()
            lastSelectedWord = null
            lastSelectionTime = 0
            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[NEXTWORD] Backspace: text empty, cleared predictions")
            }
            return
        }

        val lastChar = trimmedText.last().toString()

        scope.launch {
            val predictions = NextWordService.predict(
                word = lastChar,
                context = taigikeyboard.context,
                prefs = taigikeyboard.prefs
            )
            kotlinx.coroutines.withContext(Dispatchers.Main) {
                if (predictions.isNotEmpty()) {
                    updateCandidatesWithPredictions(predictions)
                } else {
                    onClearCandidates()
                }
            }
        }

        lastSelectedWord = lastChar
        lastSelectionTime = System.currentTimeMillis()

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] Backspace: re-predict from '$lastChar'")
        }
    }

    /**
     * Convert predictions to TaigiWord list and update candidates.
     */
    private fun updateCandidatesWithPredictions(predictions: List<NextWordService.Prediction>) {
        val useTl = (prefs.inputMode == "tl")
        val cachedIsTranslateSwapped = isTranslateSwapped()

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] updateCandidatesWithPredictions: ${predictions.size} predictions, useTl=$useTl, isTranslateSwapped=$cachedIsTranslateSwapped")
        }

        val words = predictions.mapIndexedNotNull { index, prediction ->
            val roman = if (useTl) prediction.tl else TaigiPhonetics.tlDisplayToPOJDisplay(prediction.tl)

            if (!cachedIsTranslateSwapped && roman.isEmpty()) {
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[NEXTWORD] Filtered out '${prediction.hanzi}' (no roman, isTranslateSwapped=$cachedIsTranslateSwapped)")
                }
                return@mapIndexedNotNull null
            }

            TaigiWord(
                id = -index - 1,
                roman = roman,
                hanzi = prediction.hanzi,
                lengthScore = prediction.score.toInt()
            )
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[NEXTWORD] After filter: ${words.size} words")
            words.forEachIndexed { index, word ->
                Log.d(TAG, "[NEXTWORD] Word[$index]: hanzi='${word.hanzi}', roman='${word.roman}', score=${word.lengthScore}")
            }
        }

        if (words.isNotEmpty()) {
            onUpdateCandidates(words)
        } else {
            onClearCandidates()
        }
    }

    companion object {
        private const val TAG = "NextWordHandler"
        private const val ASSOCIATION_TIMEOUT_MS = 10000L
        private const val CONTEXT_TIMEOUT_MS = 30_000L
        private val SENTENCE_END_PUNCTUATION = setOf('。', '！', '？', '.', '!', '?')

        private val NOISE_CHARS = setOf(
            '。', '！', '？', '.', '!', '?',
            '，', ',', '、', '；', ';', '：', ':',
            '「', '」', '『', '』', '"', '"', '\'',
            '（', '）', '(', ')', '【', '】', '[', ']', '{', '}',
            '—', '–', '-', '～', '~', '…', '·',
            ' ', '　',
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'
        )

        fun isNoise(word: String): Boolean {
            if (word.isEmpty()) return true
            return word.all { it in NOISE_CHARS }
        }

        fun splitCompoundWord(word: String): List<String> {
            if (word.isEmpty()) return emptyList()
            return word.split("-", " ").filter { it.isNotEmpty() }
        }

        fun extractCurrentWord(text: String): String {
            val trimmed = text.trimEnd()
            if (trimmed.isEmpty()) return ""
            val lastSeparatorIndex = trimmed.indexOfLast { it.isWhitespace() || it in ".,!?;:" }
            return if (lastSeparatorIndex >= 0) {
                trimmed.substring(lastSeparatorIndex + 1)
            } else {
                trimmed
            }
        }
    }
}

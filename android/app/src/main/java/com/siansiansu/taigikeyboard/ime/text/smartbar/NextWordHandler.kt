package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiPhonetics
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/**
 * NextWord prediction controller.
 *
 * Extracted from `SmartbarManager`. Owns context tracking (last selected
 * word + selection timing) and drives the NextWord prediction / recording
 * pipeline, including compound-word splitting. Depends on an injected
 * [NextWordService] so there are no global service reach-ins.
 */
class NextWordHandler(
    private val scope: CoroutineScope,
    private val prefs: PrefHelper,
    private val taigikeyboard: TaigiKeyboard,
    private val nextWord: NextWordService,
    private val logger: LoggerBackend,
    private val isTranslateSwapped: () -> Boolean,
    private val onUpdateCandidates: (List<TaigiWord>) -> Unit,
    private val onClearCandidates: () -> Unit,
) {
    private var lastSelectedWord: String? = null
    private var lastSelectedRoman: String? = null
    private var lastSelectionTime: Long = 0
    var isShowingNextWord: Boolean = false
        private set

    fun getLastSelectedWord(): String? = lastSelectedWord

    fun isShowingNextWordCandidates(): Boolean = isShowingNextWord

    fun resetContext() {
        lastSelectedWord = null
        lastSelectedRoman = null
        lastSelectionTime = 0
    }

    fun clearNextWordState() {
        isShowingNextWord = false
    }

    fun setShowingNextWord(showing: Boolean) {
        isShowingNextWord = showing
    }

    /** Handle NextWord prediction after a word is selected. */
    fun handleNextWordPrediction(
        displayText: String,
        committedText: String,
        roman: String,
        hanzi: String? = null,
        rawInput: String = "",
    ) {
        val currentTime = System.currentTimeMillis()

        val shouldReset =
            when {
                committedText.lastOrNull() in SENTENCE_END_PUNCTUATION -> true
                lastSelectionTime > 0 && (currentTime - lastSelectionTime) > CONTEXT_TIMEOUT_MS -> true
                else -> false
            }

        if (shouldReset) {
            lastSelectedWord = null
            lastSelectedRoman = null
            logger.d(TAG, "[NEXTWORD] Context reset")
        }

        val shouldRecordAssociation =
            lastSelectedWord != null &&
                (currentTime - lastSelectionTime) < ASSOCIATION_TIMEOUT_MS

        val parts = splitCompoundWord(displayText)
        // Normalize romanization to TL for consistent storage and query;
        // `pojDisplayToTLDisplay` is idempotent on TL input.
        val romanTl = TaigiPhonetics.pojDisplayToTLDisplay(roman)
        val romanTlParts = splitCompoundWord(romanTl)
        val prevWord = lastSelectedWord
        val prevTl = TaigiPhonetics.pojDisplayToTLDisplay(lastSelectedRoman ?: "")

        scope.launch {
            if (prefs.associationRecordingEnabled) {
                if (shouldRecordAssociation && prevWord != null) {
                    if (!isNoise(displayText)) {
                        nextWord.recordAssociation(
                            prev = prevWord,
                            prevTl = prevTl,
                            nextHanzi = displayText,
                            nextTl = romanTl,
                        )
                        logger.debug(TAG) { "[NEXTWORD] Record: '$prevWord($prevTl)' → '$displayText'" }
                    } else {
                        logger.debug(TAG) { "[NEXTWORD] Skip noise: '$displayText'" }
                    }
                }

                for (i in 0 until parts.size - 1) {
                    val prevPart = parts[i]
                    val prevPartTl = romanTlParts.getOrNull(i) ?: ""
                    val nextPart = parts[i + 1]
                    val nextPartTl = romanTlParts.getOrNull(i + 1) ?: ""
                    nextWord.recordAssociation(
                        prev = prevPart,
                        prevTl = prevPartTl,
                        nextHanzi = nextPart,
                        nextTl = nextPartTl,
                    )
                    logger.debug(TAG) { "[NEXTWORD] Record compound: '$prevPart' → '$nextPart'" }
                }
            }

            val predictions =
                nextWord.predict(
                    word = displayText,
                    roman = romanTl,
                    prefs = taigikeyboard.prefs,
                )

            kotlinx.coroutines.withContext(Dispatchers.Main) {
                if (predictions.isNotEmpty()) {
                    updateCandidatesWithPredictions(predictions)
                } else {
                    onClearCandidates()
                }
            }
        }

        if (!isNoise(displayText)) {
            lastSelectedWord = displayText
            lastSelectedRoman = romanTl
            logger.debug(TAG) { "[NEXTWORD] lastSelectedWord updated: '$displayText' roman='$romanTl'" }
        } else {
            logger.debug(TAG) { "[NEXTWORD] Skip updating lastSelectedWord for noise: '$displayText'" }
        }
        lastSelectionTime = currentTime
    }

    /**
     * Update `lastSelectedWord` without triggering NextWord prediction.
     * Invoked when the space key confirms composing text.
     */
    fun updateLastSelectedWord(
        word: String,
        roman: String? = null,
    ) {
        if (word.isEmpty()) return

        val parts = splitCompoundWord(word)
        val romanTl = TaigiPhonetics.pojDisplayToTLDisplay(roman ?: word)

        if (parts.size > 1 && prefs.associationRecordingEnabled) {
            val romanTlParts = splitCompoundWord(romanTl)
            scope.launch {
                for (i in 0 until parts.size - 1) {
                    val prevPart = parts[i]
                    val prevPartTl = romanTlParts.getOrNull(i) ?: ""
                    val nextPart = parts[i + 1]
                    val nextPartTl = romanTlParts.getOrNull(i + 1) ?: ""
                    nextWord.recordAssociation(
                        prev = prevPart,
                        prevTl = prevPartTl,
                        nextHanzi = nextPart,
                        nextTl = nextPartTl,
                    )
                    logger.debug(TAG) { "[NEXTWORD] Record compound (space): '$prevPart' → '$nextPart'" }
                }
            }
        }

        if (!isNoise(word)) {
            lastSelectedWord = word
            lastSelectedRoman = romanTl
            logger.debug(TAG) { "[NEXTWORD] updateLastSelectedWord: '$word' roman='$romanTl'" }
        } else {
            logger.debug(TAG) { "[NEXTWORD] updateLastSelectedWord: skip noise '$word'" }
        }
        lastSelectionTime = System.currentTimeMillis()
    }

    /** Backspace handler: re-predict from the trailing character of remaining text. */
    fun handleBackspaceForNextWord(textBeforeCursor: String) {
        val trimmedText = textBeforeCursor.trimEnd()

        if (trimmedText.isEmpty()) {
            onClearCandidates()
            lastSelectedWord = null
            lastSelectedRoman = null
            lastSelectionTime = 0
            logger.d(TAG, "[NEXTWORD] Backspace: text empty, cleared predictions")
            return
        }

        val lastChar = trimmedText.last().toString()

        scope.launch {
            val predictions =
                nextWord.predict(
                    word = lastChar,
                    prefs = taigikeyboard.prefs,
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
        lastSelectedRoman = null
        lastSelectionTime = System.currentTimeMillis()

        logger.debug(TAG) { "[NEXTWORD] Backspace: re-predict from '$lastChar'" }
    }

    private fun updateCandidatesWithPredictions(predictions: List<NextWordService.Prediction>) {
        val useTl = (prefs.inputMode == "tl")
        val cachedIsTranslateSwapped = isTranslateSwapped()

        logger.debug(TAG) {
            "[NEXTWORD] updateCandidatesWithPredictions: ${predictions.size} predictions, useTl=$useTl, isTranslateSwapped=$cachedIsTranslateSwapped"
        }

        val words =
            predictions.mapIndexedNotNull { index, prediction ->
                val roman = if (useTl) prediction.tl else TaigiPhonetics.tlDisplayToPOJDisplay(prediction.tl)

                if (!cachedIsTranslateSwapped && roman.isEmpty()) {
                    logger.debug(TAG) {
                        "[NEXTWORD] Filtered out '${prediction.hanzi}' (no roman, isTranslateSwapped=$cachedIsTranslateSwapped)"
                    }
                    return@mapIndexedNotNull null
                }

                TaigiWord(
                    id = -index - 1,
                    roman = roman,
                    hanzi = prediction.hanzi,
                    lengthScore = prediction.score.toInt(),
                )
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

        private val NOISE_CHARS =
            setOf(
                '。',
                '！',
                '？',
                '.',
                '!',
                '?',
                '，',
                ',',
                '、',
                '；',
                ';',
                '：',
                ':',
                '「',
                '」',
                '『',
                '』',
                '"',
                '"',
                '\'',
                '（',
                '）',
                '(',
                ')',
                '【',
                '】',
                '[',
                ']',
                '{',
                '}',
                '—',
                '–',
                '-',
                '～',
                '~',
                '…',
                '·',
                ' ',
                '　',
                '0',
                '1',
                '2',
                '3',
                '4',
                '5',
                '6',
                '7',
                '8',
                '9',
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

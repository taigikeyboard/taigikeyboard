package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.dictionary.InputNormalizer
import com.siansiansu.taigikeyboard.ime.dictionary.InputType
import com.siansiansu.taigikeyboard.ime.dictionary.LexiconService
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
import kotlinx.coroutines.CancellationException

/**
 * Taigi autocomplete service.
 *
 * Turns a raw composing string into a ranked list of candidates by
 * delegating to [LexiconService]. Applies context boosting — candidates
 * whose first hanzi matches the previous bigram prediction from
 * [NextWordService] float to the top. The 0-th slot is always the current
 * composing text (reference: iOS `AutocompleteService.swift:69-101`).
 *
 * Collaborators are injected via ctor; the service is recreated whenever
 * the input mode flips.
 */
class TaigiAutocompleteService(
    private val inputMode: ToneConverterModels.InputMode,
    private val prefs: PrefHelper,
    private val lexicon: LexiconService,
    private val nextWord: NextWordService,
    private val logger: LoggerBackend,
) {
    companion object {
        private const val TAG = "TaigiAutocompleteService"
    }

    suspend fun autocomplete(
        rawInput: String,
        displayText: String,
        lastSelectedWord: String? = null,
    ): List<TaigiWord> {
        if (rawInput.isEmpty() || displayText.isEmpty()) {
            return emptyList()
        }

        logger.debug(TAG) { "[INPUT] rawInput='$rawInput', displayText='$displayText', mode=$inputMode" }

        return try {
            val determineStart = System.currentTimeMillis()
            val inputType = determineInputType(rawInput)

            if (BuildConfig.DEBUG) {
                logger.d("PERF", "[3-a] determineInputType: ${System.currentTimeMillis() - determineStart}ms")
                logger.debug(TAG) { "[INPUT] inputType=$inputType" }
            }

            val searchStart = System.currentTimeMillis()
            val words =
                lexicon.search(
                    input = rawInput,
                    inputType = inputType,
                    inputMode = inputMode,
                    prefs = prefs,
                )
            if (BuildConfig.DEBUG) {
                logger.d("PERF", "[3-b] LexiconService.search call: ${System.currentTimeMillis() - searchStart}ms")
                logger.debug(TAG) { "[RESULT] LexiconService returned ${words.size} words" }
            }

            val contextBoostedWords = applyContextBoost(words, lastSelectedWord)

            val buildStart = System.currentTimeMillis()
            val composingTextWord = createComposingTextWord(displayText)

            val result =
                buildList {
                    add(composingTextWord)
                    addAll(contextBoostedWords)
                }
            if (BuildConfig.DEBUG) {
                logger.d("PERF", "[3-c] buildList: ${System.currentTimeMillis() - buildStart}ms")
            }
            result
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            logger.e(TAG, "[ERROR] autocomplete failed for: $rawInput", e)
            emptyList()
        }
    }

    /**
     * Build the slot-0 candidate that displays the user's current composing
     * text. Reference: iOS `AutocompleteService.swift:113-124`.
     */
    private fun createComposingTextWord(composingText: String): TaigiWord =
        TaigiWord(
            id = 0,
            roman = composingText,
            hanzi = null,
            lengthScore = null,
        )

    /**
     * Classify [text] into [InputType]. Uses shared helpers:
     * - [ToneConverterModels.isHanzi] for the broader CJK range.
     * - [InputNormalizer.hasToneMarks] for NFD-based diacritic detection.
     */
    private fun determineInputType(text: String): InputType =
        when {
            ToneConverterModels.isHanzi(text) -> InputType.Hanzi
            InputNormalizer.hasToneMarks(text) -> InputType.RomanWithTone
            containsNumericTone(text) -> InputType.RomanWithTone
            else -> InputType.RomanWithoutTone
        }

    /**
     * Float candidates whose display-text begins with a bigram-predicted
     * character to the front. Preserves original order within each
     * partition. Mirrors iOS `AutocompleteService.applyContextBoost`.
     */
    private suspend fun applyContextBoost(
        words: List<TaigiWord>,
        lastSelectedWord: String?,
    ): List<TaigiWord> {
        if (lastSelectedWord.isNullOrEmpty()) return words

        val predictions =
            nextWord.predict(
                word = lastSelectedWord,
                limit = 30,
                prefs = prefs,
            )
        if (predictions.isEmpty()) return words

        val contextSet = predictions.map { it.hanzi }.toSet()

        val boosted = mutableListOf<TaigiWord>()
        val rest = mutableListOf<TaigiWord>()

        for (word in words) {
            val display = word.displayText
            val firstChar = display.firstOrNull()?.toString()
            if (firstChar != null && firstChar in contextSet) {
                boosted.add(word)
            } else {
                rest.add(word)
            }
        }

        return boosted + rest
    }

    /**
     * Tones 1 and 4 are unmarked in Taiwanese, so their digit form alone
     * does not indicate a toned input. 0 is reserved for the current slot.
     */
    private fun containsNumericTone(text: String): Boolean =
        text.any { char ->
            char.isDigit() && char != '1' && char != '4' && char != '0'
        }
}

// 中文: 台語 autocomplete 服務:把原始 composing 字串轉成排序後的候選清單。
// 中文: 實際 lookup 走 LexiconService(Rust lexicon crate),context boost 取自
// 中文: NextWordService 的 bigram 預測。第 0 位永遠是當前 composing 字串(對齊 iOS)。
// 中文: 每次 input mode 切換會重建此 service。

package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.Outcome
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.dictionary.LexiconService
import com.siansiansu.taigikeyboard.ime.dictionary.NextWordService
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import kotlinx.coroutines.CancellationException

/**
 * Taigi autocomplete service.
 *
 * Turns a raw composing string into a ranked list of candidates by
 * delegating to [LexiconService]. Applies context boosting — candidates
 * whose first hanzi matches the previous bigram prediction from
 * [NextWordService] float to the top. The 0-th slot is always the current
 * composing text.
 *
 * Continuous-input branch: if [continuousFetcher] returns non-empty,
 * the lexicon path is skipped and the engine's span-local candidates fill
 * the strip. Empty result (not in Continuous, no syllable inventory, no
 * FST hits) falls through.
 *
 * Collaborators are injected via ctor; the service is recreated whenever
 * the input mode flips.
 */
class TaigiAutocompleteService(
    private val inputMode: InputMode,
    private val settings: EngineSettings,
    private val lexicon: LexiconService,
    private val nextWord: NextWordService,
    private val logger: LoggerBackend,
    /**
     * Continuous-input candidate fetcher. Caller MUST hop to the IME main
     * thread before invoking [com.siansiansu.taigikeyboard.ime.text.composing
     * .ComposingManager.fetchContinuousCandidates] — the fetch's
     * `applyTransition` writes to InputConnection.
     *
     * Default returns `emptyList()` so non-IME callers (tests, prior
     * call-sites) compile unchanged and the lexicon path runs verbatim.
     */
    private val continuousFetcher: suspend () -> List<RustEngineBridge.ContinuousCandidate> = { emptyList() },
) {
    companion object {
        private const val TAG = "TaigiAutocompleteService"
    }

    suspend fun autocomplete(
        rawInput: String,
        displayText: String,
        lastSelectedWord: String? = null,
        nextwordEnvelopeGeneration: Long = 0L,
    ): List<TaigiWord> {
        if (rawInput.isEmpty() || displayText.isEmpty()) {
            return emptyList()
        }

        logger.debug(TAG) { "[INPUT] rawInput='$rawInput', displayText='$displayText', mode=$inputMode" }

        return try {
            val continuousCandidates = continuousFetcher()
            if (continuousCandidates.isNotEmpty()) {
                if (BuildConfig.DEBUG) {
                    logger.d(TAG, "[CONTINUOUS] returning ${continuousCandidates.size} span-local candidates")
                }
                return buildContinuousSuggestionsForCandidates(continuousCandidates, displayText)
            }

            val determineStart = System.currentTimeMillis()
            val inputType = AutocompleteInputClassifier.determineInputType(rawInput)

            if (BuildConfig.DEBUG) {
                logger.d("PERF", "[3-a] determineInputType: ${System.currentTimeMillis() - determineStart}ms")
                logger.debug(TAG) { "[INPUT] inputType=$inputType" }
            }

            val searchStart = System.currentTimeMillis()
            val searchOutcome =
                lexicon.search(
                    input = rawInput,
                    inputType = inputType,
                    inputMode = inputMode,
                    settings = settings,
                )
            val words =
                when (searchOutcome) {
                    is Outcome.Success -> {
                        searchOutcome.value
                    }

                    is Outcome.Failure -> {
                        logger.e(TAG, "[ERROR] autocomplete search failed: ${searchOutcome.error}")
                        return emptyList()
                    }
                }
            if (BuildConfig.DEBUG) {
                logger.d("PERF", "[3-b] LexiconService.search call: ${System.currentTimeMillis() - searchStart}ms")
                logger.debug(TAG) { "[RESULT] LexiconService returned ${words.size} words" }
            }

            val contextBoostedWords = applyContextBoost(words, lastSelectedWord, nextwordEnvelopeGeneration)

            val buildStart = System.currentTimeMillis()
            val composingTextWord = createComposingTextCell(displayText)

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
     * Float candidates whose display-text begins with a bigram-predicted
     * character to the front. Routes the partition through
     * [RustEngineBridge.nextwordBoostCandidates] — the Rust crate owns the
     * canonical first-char partition; the platform `[TaigiWord] ↔ [String]`
     * round-trip preserves intra-partition order so original `TaigiWord`
     * identity is recovered post-bridge.
     *
     * [nextwordEnvelopeGeneration] is owned by [NextWordHandler] so all
     * bridge calls within the IME session share state in the singleton
     * `EngineHandle`.
     */
    private suspend fun applyContextBoost(
        words: List<TaigiWord>,
        lastSelectedWord: String?,
        nextwordEnvelopeGeneration: Long,
    ): List<TaigiWord> {
        if (lastSelectedWord.isNullOrEmpty()) return words

        val predictions =
            nextWord.predict(
                word = lastSelectedWord,
                limit = 30,
                settings = settings,
                nowMs = System.currentTimeMillis(),
            )
        if (predictions.isEmpty()) return words

        val contextSet = predictions.map { it.hanzi }.toSet()
        val displayTexts = words.map { it.displayText }
        val reordered = RustEngineBridge.nextwordBoostCandidates(
            words = displayTexts,
            predictedFirstChars = contextSet,
            mode = inputMode,
            translateSwapped = settings.isTranslateSwapped,
            associationRecordingEnabled = settings.isAssociationRecordingEnabled,
            generation = nextwordEnvelopeGeneration,
        )
        return remapBoostedWords(words, reordered)
    }

    /**
     * Map the bridge's `[String]` partition reorder back to `[TaigiWord]`,
     * preserving original word identity. Walks `displayOrder` and pulls the
     * next `TaigiWord` from a per-display-text FIFO. Any size mismatch falls
     * back to original order so a bridge failure cannot drop candidates.
     */
    private fun remapBoostedWords(
        original: List<TaigiWord>,
        displayOrder: List<String>,
    ): List<TaigiWord> {
        if (displayOrder.size != original.size) return original
        val queues = HashMap<String, ArrayDeque<Int>>(original.size)
        for ((i, w) in original.withIndex()) {
            queues.getOrPut(w.displayText) { ArrayDeque() }.addLast(i)
        }
        val result = ArrayList<TaigiWord>(original.size)
        for (d in displayOrder) {
            val q = queues[d] ?: return original
            val head = q.removeFirstOrNull() ?: return original
            result.add(original[head])
        }
        return result
    }
}

/**
 * Slot-0 cell that displays the user's current composing buffer. The
 * [TaigiWord.MetadataKeys.IS_COMPOSING_TEXT] flag routes tap-to-commit-raw
 * via [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler].
 * Per `docs/engine/continuous-input-ranking.md` §10, slot 0 has no visual
 * distinction from slots 1..n — the metadata is click-routing only.
 *
 * Top-level so both the lexicon path ([TaigiAutocompleteService.autocomplete])
 * and the continuous path ([buildContinuousSuggestionsForCandidates]) share
 * one constructor and the slot-0 contract stays single-sourced.
 */
internal fun createComposingTextCell(composingText: String): TaigiWord =
    TaigiWord(
        id = 0,
        roman = composingText,
        hanzi = null,
        lengthScore = null,
        additionalInfo = mapOf(TaigiWord.MetadataKeys.IS_COMPOSING_TEXT to "true"),
    )

/**
 * Wrap a list of engine [RustEngineBridge.ContinuousCandidate] into the
 * platform `[TaigiWord]` shape with metadata sidechannel pre-populated for
 * [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
 * tap routing. Slot-0 stays the composing-text cell; Continuous candidates
 * fill slots 1..n.
 *
 * Top-level so the contract is unit-testable without instantiating
 * [LexiconService] / [NextWordService].
 */
internal fun buildContinuousSuggestionsForCandidates(
    candidates: List<RustEngineBridge.ContinuousCandidate>,
    composingText: String,
): List<TaigiWord> {
    val cells = candidates.mapIndexed { index, candidate ->
        TaigiWord(
            // Synthetic id ≥ 1 keeps Continuous candidates outside English
            // (id ≤ -100) and NextWord (-99..-1) sentinel ranges. Routing
            // keys off additionalInfo — id is defense-in-depth.
            id = index + 1,
            roman = candidate.displayText,
            hanzi = null,
            lengthScore = null,
            additionalInfo = mapOf(
                TaigiWord.MetadataKeys.IS_CONTINUOUS to "true",
                TaigiWord.MetadataKeys.CONSUMED_BYTES to candidate.consumedSpanEnd.toString(),
                TaigiWord.MetadataKeys.SYLLABLE_COUNT to candidate.syllableCount.toString(),
                TaigiWord.MetadataKeys.DISPLAY_TEXT to candidate.displayText,
            ),
        )
    }
    return buildList {
        add(createComposingTextCell(composingText))
        addAll(cells)
    }
}

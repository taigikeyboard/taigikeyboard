// NextWord ops — extensions on RustEngineBridge (decide intents + predict),
// mirroring iOS RustEngineBridge+NextWord.swift over a JNI roundtrip; its DTOs are top-level types here.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.CellScript
import com.siansiansu.taigikeyboard.engine.proto.DecideResult
import com.siansiansu.taigikeyboard.engine.proto.DecisionInput
import com.siansiansu.taigikeyboard.engine.proto.NextWordRequest
import com.siansiansu.taigikeyboard.engine.proto.NextWordResponse
import com.siansiansu.taigikeyboard.ime.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.settings.SyllableSeparator

// region NextWord DTOs

/**
 * Bridge-synthesized companion to the proto `DecideResult`. Consumed
 * by the Android NextWord platform executor (`NextWordController`);
 * effect list executes in order.
 */
data class NextWordDecideResult(
    val effects: List<Effect>,
    val predictionsVisible: Boolean,
) {
    sealed class Effect {
        data class RescheduleContextTimeout(
            val afterMs: Long,
        ) : Effect()

        object CancelContextTimeout : Effect()

        /**
         * `nowMs` is reused by the platform predict() call so the
         * association-window clock and the user-row decay scoring see
         * ONE consistent "now" per intent. Per
         * `nextword-engine-boundary.md` §13.3.
         */
        data class QueryPredictions(
            val word: String,
            val roman: String,
            val generation: Long,
            val nowMs: Long,
        ) : Effect()

        object ClearPredictionsUI : Effect()
    }

    companion object {
        val NOOP = NextWordDecideResult(
            effects = emptyList(),
            predictionsVisible = false,
        )
    }
}

/**
 * UI-ready prediction value. `subtitle` is `null` when the wire string
 * is empty (filter contract — happens iff roman is empty).
 */
data class NextWordEnginePrediction(
    val text: String,
    val subtitle: String?,
    /** The Hanji as shown (POJ mode renders a Han-Lo word's romanization in POJ). */
    val hanji: String,
    /**
     * The stored Hanji spelling (`EnginePrediction.display_text`): the key a tap
     * is counted under and NextWord learns.
     */
    val displayText: String,
    val tl: String,
    /** The §42 split cell this row is (`EnginePrediction.cell_script`). */
    val cellScript: CellScript = CellScript.CELL_SCRIPT_PAIRED,
)

/**
 * Filter+merge+sort+limit result. `wasStale=true` indicates the
 * platform-supplied `queryGeneration` did not match the engine's
 * current generation — late async result; predictions are empty.
 */
data class NextWordFilterResult(
    val predictions: List<NextWordEnginePrediction>,
    val wasStale: Boolean,
)

// endregion

// region Decide intents (6)

// Records the association, starts the context timer, and optionally queries the next-word prediction.
fun RustEngineBridge.nextwordWordSelected(
    text: String,
    roman: String,
    requireRomanMode: Boolean,
    triggerPrediction: Boolean,
    nowMs: Long,
    inputMode: String,
    hanjiFirst: Boolean,
    generation: Long,
): NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.WordSelected
        .newBuilder()
        .setText(text)
        .setRoman(roman)
        .setRequireRomanMode(requireRomanMode)
        .setTriggerPrediction(triggerPrediction)
        .setInput(decisionInput(nowMs))
        .build()
    return decideDispatch(
        methodSetter = { it.wordSelected = payload },
        op = "nextwordWordSelected",
        generation = generation,
        config = appConfig(inputMode, isHanjiFirst = hanjiFirst),
    )
}

// Whether lastChar is a boundary character decides if the NextWord display clears and the timer reschedules.
fun RustEngineBridge.nextwordBackspace(
    lastChar: String,
    nowMs: Long,
    inputMode: String,
    hanjiFirst: Boolean,
    generation: Long,
): NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.Backspace
        .newBuilder()
        .setLastChar(lastChar)
        .setInput(decisionInput(nowMs))
        .build()
    return decideDispatch(
        methodSetter = { it.backspace = payload },
        op = "nextwordBackspace",
        generation = generation,
        config = appConfig(inputMode, isHanjiFirst = hanjiFirst),
    )
}

fun RustEngineBridge.nextwordContextTimeoutFired(
    nowMs: Long,
    inputMode: String,
    hanjiFirst: Boolean,
    generation: Long,
): NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.ContextTimeoutFired
        .newBuilder()
        .setInput(decisionInput(nowMs))
        .build()
    return decideDispatch(
        methodSetter = { it.contextTimeoutFired = payload },
        op = "nextwordContextTimeoutFired",
        generation = generation,
        config = appConfig(inputMode, isHanjiFirst = hanjiFirst),
    )
}

// Clears the NextWord display but keeps lastSelectedWord for the next selection.
fun RustEngineBridge.nextwordClearForNewComposing(
    nowMs: Long,
    inputMode: String,
    hanjiFirst: Boolean,
    generation: Long,
): NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.ClearForNewComposing
        .newBuilder()
        .setInput(decisionInput(nowMs))
        .build()
    return decideDispatch(
        methodSetter = { it.clearForNewComposing = payload },
        op = "nextwordClearForNewComposing",
        generation = generation,
        config = appConfig(inputMode, isHanjiFirst = hanjiFirst),
    )
}

// Full reset of lastSelectedWord / lastSelectionTimeMs / predictions visibility (focus change, input-mode switch).
fun RustEngineBridge.nextwordResetAll(
    nowMs: Long,
    inputMode: String,
    hanjiFirst: Boolean,
    generation: Long,
): NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.ResetAll
        .newBuilder()
        .setInput(decisionInput(nowMs))
        .build()
    return decideDispatch(
        methodSetter = { it.resetAll = payload },
        op = "nextwordResetAll",
        generation = generation,
        config = appConfig(inputMode, isHanjiFirst = hanjiFirst),
    )
}

/**
 * Platform → engine UI visibility sync. Call after rendering an async
 * predict() result (or clearing it on empty result) so the engine's
 * `state.predictions_visible` stays accurate. Downstream
 * `nextwordClearForNewComposing` / sentence-end / context timeout /
 * `nextwordResetAll` paths gate `ClearPredictionsUI` emission on it.
 * No effects, no `current_generation` bump.
 */
fun RustEngineBridge.nextwordSetPredictionsVisible(
    visible: Boolean,
    inputMode: String,
    hanjiFirst: Boolean,
    generation: Long,
): NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.SetPredictionsVisible
        .newBuilder()
        .setVisible(visible)
        .build()
    return decideDispatch(
        methodSetter = { it.setPredictionsVisible = payload },
        op = "nextwordSetPredictionsVisible",
        generation = generation,
        config = appConfig(inputMode, isHanjiFirst = hanjiFirst),
    )
}

// endregion
// region Predict

// The engine reads the learned rows for [word] / [roman] from its own `user_association.db`
// (§24 tiers, roadmap P8b), adds the bundled rows for the last character of [word], then
// scores, merges, sorts, limits and shapes.
// A queryGeneration that no longer matches currentGeneration returns wasStale=true — caller drops the result.
fun RustEngineBridge.nextwordPredictNext(
    word: String,
    roman: String,
    toggles: DictionaryToggles,
    queryGeneration: Long,
    nowMs: Long,
    limit: Int,
    inputMode: String,
    hanjiFirst: Boolean,
    generation: Long,
    candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    syllableSeparator: SyllableSeparator = SyllableSeparator.HYPHEN,
    isKhinsiannInHanjiEnabled: Boolean = false,
): NextWordFilterResult {
    val builder = com.siansiansu.taigikeyboard.engine.proto.PredictNext
        .newBuilder()
        .setWord(word)
        .setRoman(roman)
        .setToggles(dictionaryTogglesProto(toggles))
        .setQueryGeneration(queryGeneration)
        .setNowMs(nowMs)
        .setLimit(limit)
    val resp = nextwordDispatch(
        methodSetter = { it.predictNext = builder.build() },
        op = "nextwordPredictNext",
        generation = generation,
        // Fields 9 / 14 ride only the predict request — the sole nextword reader
        // (`nextword/src/filter.rs` collapses same-roman predictions under ROMAN_ONLY
        // and separates `text`'s syllables under the Syllable Separator).
        config =
            appConfig(
                inputMode,
                isHanjiFirst = hanjiFirst,
                candidateDisplayMode = candidateDisplayMode,
                syllableSeparator = syllableSeparator,
                isKhinsiannInHanjiEnabled = isKhinsiannInHanjiEnabled,
            ),
    ) ?: return NextWordFilterResult(emptyList(), wasStale = false)
    if (!resp.hasFilter()) {
        RustEngineBridge.recordFailure("nextwordPredictNext", "missing filter result")
        return NextWordFilterResult(emptyList(), wasStale = false)
    }
    val filter = resp.filter
    val predictions = filter.predictionsList.map { p ->
        NextWordEnginePrediction(
            text = p.text,
            subtitle = if (p.subtitle.isEmpty()) null else p.subtitle,
            hanji = p.hanji,
            displayText = p.displayText,
            tl = p.tl,
            cellScript = p.cellScript,
        )
    }
    return NextWordFilterResult(predictions = predictions, wasStale = filter.wasStale)
}

// endregion
// region Private dispatch + helpers

private fun decisionInput(nowMs: Long): DecisionInput =
    DecisionInput
        .newBuilder()
        .setNowMs(nowMs)
        .build()

private inline fun nextwordDispatch(
    methodSetter: (NextWordRequest.Builder) -> Unit,
    op: String,
    generation: Long,
    config: AppConfig,
): NextWordResponse? {
    val nextwordBuilder = NextWordRequest.newBuilder()
    methodSetter(nextwordBuilder)
    val nextwordRequest = nextwordBuilder.build()
    val response = RustEngineBridge.dispatch(op) {
        setGeneration(generation)
        setConfigSnapshot(config)
        setNextword(nextwordRequest)
    } ?: return null
    if (!response.hasNextword()) {
        RustEngineBridge.recordFailure(op, "missing nextword payload")
        return null
    }
    return response.nextword
}

private inline fun decideDispatch(
    methodSetter: (NextWordRequest.Builder) -> Unit,
    op: String,
    generation: Long,
    config: AppConfig,
): NextWordDecideResult {
    val resp = nextwordDispatch(methodSetter, op, generation, config)
        ?: return NextWordDecideResult.NOOP
    if (!resp.hasDecide()) {
        RustEngineBridge.recordFailure(op, "missing decide result")
        return NextWordDecideResult.NOOP
    }
    return synthDecideResult(resp.decide)
}

// Also reads the decision a composing answer carries (`Effect.next_word_decided`).
internal fun synthDecideResult(proto: DecideResult): NextWordDecideResult {
    val effects: List<NextWordDecideResult.Effect> = proto.effectsList.mapNotNull { eff ->
        when {
            eff.hasRescheduleContextTimeout() -> {
                NextWordDecideResult.Effect.RescheduleContextTimeout(
                    eff.rescheduleContextTimeout.afterMs,
                )
            }

            eff.hasCancelContextTimeout() -> {
                NextWordDecideResult.Effect.CancelContextTimeout
            }

            eff.hasQueryPredictions() -> {
                NextWordDecideResult.Effect.QueryPredictions(
                    word = eff.queryPredictions.word,
                    roman = eff.queryPredictions.roman,
                    generation = eff.queryPredictions.generation,
                    nowMs = eff.queryPredictions.nowMs,
                )
            }

            eff.hasClearPredictionsUi() -> {
                NextWordDecideResult.Effect.ClearPredictionsUI
            }

            else -> {
                null
            }
        }
    }
    return NextWordDecideResult(
        effects = effects,
        predictionsVisible = proto.predictionsVisible,
    )
}

// endregion

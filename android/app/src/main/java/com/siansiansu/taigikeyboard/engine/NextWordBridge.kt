// NextWord ops — extensions on RustEngineBridge (decide intents + predict),
// mirroring iOS RustEngineBridge+NextWord.swift over a JNI roundtrip; nested types stay in RustEngineBridge.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.DecideResult
import com.siansiansu.taigikeyboard.engine.proto.DecisionInput
import com.siansiansu.taigikeyboard.engine.proto.NextWordRequest
import com.siansiansu.taigikeyboard.engine.proto.NextWordResponse
import com.siansiansu.taigikeyboard.ime.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.settings.SyllableSeparator

// region Decide intents (6)
// UpdateLastSelectedWord was originally Android-only (Space-path); v3.5.8
// Phase 4 brought iOS into the call site through a continuous-input
// mid-commit handshake. iOS bridge wraps it in
// RustEngineBridge+NextWord.swift::nextwordUpdateLastSelectedWord.

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
    preceding: List<com.siansiansu.taigikeyboard.engine.proto.CommittedWord> = emptyList(),
): RustEngineBridge.NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.WordSelected
        .newBuilder()
        .setText(text)
        .setRoman(roman)
        .setRequireRomanMode(requireRomanMode)
        .setTriggerPrediction(triggerPrediction)
        .setInput(decisionInput(nowMs))
        .addAllPreceding(preceding)
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
): RustEngineBridge.NextWordDecideResult {
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
): RustEngineBridge.NextWordDecideResult {
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
): RustEngineBridge.NextWordDecideResult {
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
): RustEngineBridge.NextWordDecideResult {
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
): RustEngineBridge.NextWordDecideResult {
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

/**
 * Continuous-input nail / unnail handshake. The engine learns nothing from it
 * and changes no state; the final commit's `preceding` carries the nailed
 * segments (behavioral-invariants §40).
 */
fun RustEngineBridge.nextwordUpdateLastSelectedWord(
    text: String,
    roman: String,
    nowMs: Long,
    inputMode: String,
    hanjiFirst: Boolean,
    generation: Long,
): RustEngineBridge.NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.UpdateLastSelectedWord
        .newBuilder()
        .setText(text)
        .setRoman(roman)
        .setInput(decisionInput(nowMs))
        .build()
    return decideDispatch(
        methodSetter = { it.updateLastSelectedWord = payload },
        op = "nextwordUpdateLastSelectedWord",
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
    toggles: RustEngineBridge.DictionaryToggles,
    queryGeneration: Long,
    nowMs: Long,
    limit: Int,
    inputMode: String,
    hanjiFirst: Boolean,
    generation: Long,
    candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    syllableSeparator: SyllableSeparator = SyllableSeparator.HYPHEN,
): RustEngineBridge.NextWordFilterResult {
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
            ),
    ) ?: return RustEngineBridge.NextWordFilterResult(emptyList(), wasStale = false)
    if (!resp.hasFilter()) {
        RustEngineBridge.recordFailure("nextwordPredictNext", "missing filter result")
        return RustEngineBridge.NextWordFilterResult(emptyList(), wasStale = false)
    }
    val filter = resp.filter
    val predictions = filter.predictionsList.map { p ->
        RustEngineBridge.NextWordEnginePrediction(
            text = p.text,
            subtitle = if (p.subtitle.isEmpty()) null else p.subtitle,
            hanji = p.hanji,
            tl = p.tl,
        )
    }
    return RustEngineBridge.NextWordFilterResult(predictions = predictions, wasStale = filter.wasStale)
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
): RustEngineBridge.NextWordDecideResult {
    val resp = nextwordDispatch(methodSetter, op, generation, config)
        ?: return RustEngineBridge.NextWordDecideResult.NOOP
    if (!resp.hasDecide()) {
        RustEngineBridge.recordFailure(op, "missing decide result")
        return RustEngineBridge.NextWordDecideResult.NOOP
    }
    return synthDecideResult(resp.decide)
}

private fun synthDecideResult(proto: DecideResult): RustEngineBridge.NextWordDecideResult {
    val effects: List<RustEngineBridge.NextWordDecideResult.Effect> = proto.effectsList.mapNotNull { eff ->
        when {
            eff.hasRescheduleContextTimeout() -> {
                RustEngineBridge.NextWordDecideResult.Effect.RescheduleContextTimeout(
                    eff.rescheduleContextTimeout.afterMs,
                )
            }

            eff.hasCancelContextTimeout() -> {
                RustEngineBridge.NextWordDecideResult.Effect.CancelContextTimeout
            }

            eff.hasQueryPredictions() -> {
                RustEngineBridge.NextWordDecideResult.Effect.QueryPredictions(
                    word = eff.queryPredictions.word,
                    roman = eff.queryPredictions.roman,
                    generation = eff.queryPredictions.generation,
                    nowMs = eff.queryPredictions.nowMs,
                )
            }

            eff.hasClearPredictionsUi() -> {
                RustEngineBridge.NextWordDecideResult.Effect.ClearPredictionsUI(eff.clearPredictionsUi.generation)
            }

            else -> {
                null
            }
        }
    }
    return RustEngineBridge.NextWordDecideResult(
        effects = effects,
        currentGeneration = proto.currentGeneration,
        predictionsVisible = proto.predictionsVisible,
        lastSelectedWord = if (proto.lastSelectedWord.isEmpty()) null else proto.lastSelectedWord,
    )
}

// endregion

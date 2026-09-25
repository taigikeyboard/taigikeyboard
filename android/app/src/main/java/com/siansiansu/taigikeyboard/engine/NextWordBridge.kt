// NextWord ops — extensions on RustEngineBridge (decide intents + predict),
// mirroring iOS RustEngineBridge+NextWord.swift over a JNI roundtrip; nested types stay in RustEngineBridge.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.AssociationPair
import com.siansiansu.taigikeyboard.engine.proto.DecideResult
import com.siansiansu.taigikeyboard.engine.proto.DecisionInput
import com.siansiansu.taigikeyboard.engine.proto.NextWordRequest
import com.siansiansu.taigikeyboard.engine.proto.NextWordResponse
import com.siansiansu.taigikeyboard.engine.proto.Platform
import com.siansiansu.taigikeyboard.engine.proto.Source
import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode

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
    mode: InputMode,
    translateSwapped: Boolean,
    generation: Long,
): RustEngineBridge.NextWordDecideResult {
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
        config = nextwordConfig(mode, translateSwapped),
    )
}

// Whether lastChar is a boundary character decides if the NextWord display clears and the timer reschedules.
fun RustEngineBridge.nextwordBackspace(
    lastChar: String,
    nowMs: Long,
    mode: InputMode,
    translateSwapped: Boolean,
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
        config = nextwordConfig(mode, translateSwapped),
    )
}

fun RustEngineBridge.nextwordContextTimeoutFired(
    nowMs: Long,
    mode: InputMode,
    translateSwapped: Boolean,
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
        config = nextwordConfig(mode, translateSwapped),
    )
}

// Clears the NextWord display but keeps lastSelectedWord for the next selection.
fun RustEngineBridge.nextwordClearForNewComposing(
    nowMs: Long,
    mode: InputMode,
    translateSwapped: Boolean,
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
        config = nextwordConfig(mode, translateSwapped),
    )
}

// Full reset of lastSelectedWord / lastSelectionTimeMs / isShowing (focus change, input-mode switch).
fun RustEngineBridge.nextwordResetFull(
    nowMs: Long,
    mode: InputMode,
    translateSwapped: Boolean,
    generation: Long,
): RustEngineBridge.NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.ResetFull
        .newBuilder()
        .setInput(decisionInput(nowMs))
        .build()
    return decideDispatch(
        methodSetter = { it.resetFull = payload },
        op = "nextwordResetFull",
        generation = generation,
        config = nextwordConfig(mode, translateSwapped),
    )
}

/**
 * Platform → engine UI visibility sync. Call after rendering an async
 * predict() result (or clearing it on empty result) so the engine's
 * `state.is_showing` stays accurate. Downstream
 * `nextwordClearForNewComposing` / sentence-end / context timeout /
 * `nextwordResetFull` paths gate `ClearPredictionsUI` emission on it.
 * No effects, no `current_generation` bump.
 */
fun RustEngineBridge.nextwordSetIsShowing(
    isShowing: Boolean,
    mode: InputMode,
    translateSwapped: Boolean,
    generation: Long,
): RustEngineBridge.NextWordDecideResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.SetIsShowing
        .newBuilder()
        .setIsShowing(isShowing)
        .build()
    return decideDispatch(
        methodSetter = { it.setIsShowing = payload },
        op = "nextwordSetIsShowing",
        generation = generation,
        config = nextwordConfig(mode, translateSwapped),
    )
}

/**
 * Android-only Space-path intent. Codex v1 P1: preserves the
 * "compound-only / no timer reschedule / no generation bump"
 * semantics of the legacy `NextWordHandler.updateLastSelectedWord`.
 * iOS reaches it through the continuous-input mid-commit handshake (region header above).
 */
fun RustEngineBridge.nextwordUpdateLastSelectedWord(
    text: String,
    roman: String,
    nowMs: Long,
    mode: InputMode,
    translateSwapped: Boolean,
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
        config = nextwordConfig(mode, translateSwapped),
    )
}

// endregion
// region Predict

// Platform SQL supplies the learned rows (best evidence first); the engine adds the bundled rows
// for the last character of [word], then scores, merges, sorts, limits and shapes.
// A queryGeneration that no longer matches currentGeneration returns wasStale=true — caller drops the result.
fun RustEngineBridge.nextwordPredictNext(
    word: String,
    userRows: List<RustEngineBridge.NextWordRawRow>,
    toggles: RustEngineBridge.DictionaryToggles,
    queryGeneration: Long,
    nowMs: Long,
    limit: Int,
    mode: InputMode,
    translateSwapped: Boolean,
    generation: Long,
    candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    hyphenlessRoman: Boolean = false,
): RustEngineBridge.NextWordFilterResult {
    val builder = com.siansiansu.taigikeyboard.engine.proto.PredictNext
        .newBuilder()
        .setWord(word)
        .setToggles(dictionaryTogglesProto(toggles))
        .setQueryGeneration(queryGeneration)
        .setNowMs(nowMs)
        .setLimit(limit)
    for (row in userRows) {
        builder.addUserRows(
            com.siansiansu.taigikeyboard.engine.proto.RawNextWordPrediction
                .newBuilder()
                .setHanzi(row.hanzi)
                .setTl(row.tl)
                .setCount(row.count)
                .setLastUsedMs(row.lastUsedMs)
                .setSource(
                    when (row.source) {
                        RustEngineBridge.NextWordRawRow.Source.DICT -> Source.SOURCE_DICT
                        RustEngineBridge.NextWordRawRow.Source.USER -> Source.SOURCE_USER
                    },
                ).build(),
        )
    }
    val resp = nextwordDispatch(
        methodSetter = { it.predictNext = builder.build() },
        op = "nextwordPredictNext",
        generation = generation,
        // Fields 9 / 10 ride only the predict request — the sole nextword reader
        // (`nextword/src/filter.rs` collapses same-roman predictions under ROMAN_ONLY
        // and shapes `text` hyphenless under 無連字符).
        config = nextwordConfig(mode, translateSwapped, candidateDisplayMode, hyphenlessRoman),
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
            hanzi = p.hanzi,
            tl = p.tl,
            score = p.score,
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

private fun nextwordConfig(
    mode: InputMode,
    translateSwapped: Boolean,
    candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
    hyphenlessRoman: Boolean = false,
): AppConfig =
    AppConfig
        .newBuilder()
        .setInputMode(
            when (mode) {
                InputMode.POJ -> "poj"
                InputMode.TL -> "tl"
                InputMode.ENGLISH -> "english"
            },
        ).setOoDoubletapEnabled(false)
        .setNnDoubletapEnabled(false)
        .setCandidateDisplayMode(candidateDisplayMode.toProto())
        .setHyphenlessRoman(hyphenlessRoman)
        .setIsTranslateSwapped(translateSwapped)
        .setPlatformId(Platform.PLATFORM_ANDROID)
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

            eff.hasRecordAssociation() -> {
                RustEngineBridge.NextWordDecideResult.Effect.RecordAssociation(
                    synthAssociationPair(eff.recordAssociation.pair),
                )
            }

            eff.hasRecordCompoundAssociations() -> {
                RustEngineBridge.NextWordDecideResult.Effect.RecordCompoundAssociations(
                    eff.recordCompoundAssociations.pairsList.map(::synthAssociationPair),
                )
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
        isShowing = proto.isShowing,
        lastSelectedWord = if (proto.lastSelectedWord.isEmpty()) null else proto.lastSelectedWord,
    )
}

private fun synthAssociationPair(proto: AssociationPair): RustEngineBridge.NextWordAssociationPair =
    RustEngineBridge.NextWordAssociationPair(
        prev = proto.prev,
        prevTl = proto.prevTl,
        next = proto.next,
        nextTl = proto.nextTl,
    )

// endregion

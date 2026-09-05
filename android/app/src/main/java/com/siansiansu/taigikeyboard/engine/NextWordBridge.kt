// NextWord 橋 — 9 ops(6 decide intents + filter + boost + queryState)。
// 對應 iOS RustEngineBridge+NextWord.swift。共用 RustEngineBridge.sendRawBytes 做 JNI roundtrip。
// 巢狀型別(NextWordDecideResult / NextWordRawRow / NextWordStateSnapshot 等)留在 RustEngineBridge。

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.AssociationPair
import com.siansiansu.taigikeyboard.engine.proto.DecideResult
import com.siansiansu.taigikeyboard.engine.proto.DecisionInput
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.NextWordRequest
import com.siansiansu.taigikeyboard.engine.proto.NextWordResponse
import com.siansiansu.taigikeyboard.engine.proto.Platform
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.engine.proto.Source
import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode

/**
 * Impl object backing `RustEngineBridge` NextWord facade methods. NOT a
 * public API — callers stay on `RustEngineBridge.*` per F1=A facade
 * contract. Module-internal visibility keeps the surface honest.
 */
internal object NextWordBridge {
    // region Decide intents (6)

    fun wordSelected(
        text: String,
        roman: String,
        requireRomanMode: Boolean,
        triggerPrediction: Boolean,
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
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
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    fun backspace(
        lastChar: String,
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
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
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    fun contextTimeoutFired(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
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
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    fun clearForNewComposing(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
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
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    fun resetFull(
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
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
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    fun setIsShowing(
        isShowing: Boolean,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
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
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    fun updateLastSelectedWord(
        text: String,
        roman: String,
        nowMs: Long,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
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
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        )
    }

    // endregion
    // region Filter / Boost / QueryState

    fun filter(
        raw: List<RustEngineBridge.NextWordRawRow>,
        queryGeneration: Long,
        nowMs: Long,
        limit: Int,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
        candidateDisplayMode: CandidateDisplayMode,
    ): RustEngineBridge.NextWordFilterResult {
        val builder = com.siansiansu.taigikeyboard.engine.proto.FilterPredictions
            .newBuilder()
            .setQueryGeneration(queryGeneration)
            .setNowMs(nowMs)
            .setLimit(limit)
        for (row in raw) {
            builder.addRaw(
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
        val resp = dispatch(
            methodSetter = { it.filterPredictions = builder.build() },
            op = "nextwordFilter",
            generation = generation,
            // Field 9 rides only the filter request — the sole nextword reader
            // (`nextword/src/filter.rs` collapses same-roman predictions under ROMAN_ONLY).
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled, candidateDisplayMode),
        ) ?: return RustEngineBridge.NextWordFilterResult(emptyList(), wasStale = false)
        if (!resp.hasFilter()) {
            RustEngineBridge.recordFailure("nextwordFilter", "missing filter result")
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

    fun boostCandidates(
        words: List<String>,
        predictedFirstChars: Set<String>,
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): List<String> {
        val payload = com.siansiansu.taigikeyboard.engine.proto.BoostCandidates
            .newBuilder()
            .addAllWords(words)
            .addAllPredictedFirstChars(predictedFirstChars)
            .build()
        val resp = dispatch(
            methodSetter = { it.boostCandidates = payload },
            op = "nextwordBoostCandidates",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        ) ?: return words
        if (!resp.hasBoost()) {
            RustEngineBridge.recordFailure("nextwordBoostCandidates", "missing boost result")
            return words
        }
        return resp.boost.wordsList.toList()
    }

    fun queryState(
        mode: InputMode,
        translateSwapped: Boolean,
        associationRecordingEnabled: Boolean,
        generation: Long,
    ): RustEngineBridge.NextWordStateSnapshot {
        val payload = com.siansiansu.taigikeyboard.engine.proto.NextWordQueryState
            .newBuilder()
            .build()
        val resp = dispatch(
            methodSetter = { it.queryState = payload },
            op = "nextwordQueryState",
            generation = generation,
            config = nextwordConfig(mode, translateSwapped, associationRecordingEnabled),
        ) ?: return RustEngineBridge.NextWordStateSnapshot(null, false, 0L)
        if (!resp.hasStateSnapshot()) {
            RustEngineBridge.recordFailure("nextwordQueryState", "missing state snapshot")
            return RustEngineBridge.NextWordStateSnapshot(null, false, 0L)
        }
        val s = resp.stateSnapshot
        return RustEngineBridge.NextWordStateSnapshot(
            lastSelectedWord = if (s.lastSelectedWord.isEmpty()) null else s.lastSelectedWord,
            isShowing = s.isShowing,
            currentGeneration = s.currentGeneration,
        )
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
        associationRecordingEnabled: Boolean,
        candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
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
            .setIsTranslateSwapped(translateSwapped)
            .setIsAssociationRecordingEnabled(associationRecordingEnabled)
            .setPlatformId(Platform.PLATFORM_ANDROID)
            .build()

    private inline fun dispatch(
        methodSetter: (NextWordRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig,
    ): NextWordResponse? {
        val nextwordBuilder = NextWordRequest.newBuilder()
        methodSetter(nextwordBuilder)
        val request = Request
            .newBuilder()
            .setId(RustEngineBridge.nextRequestIdInternal())
            .setGeneration(generation)
            .setConfigSnapshot(config)
            .setNextword(nextwordBuilder.build())
            .build()
        val response = RustEngineBridge.sendRawBytes(request.toByteArray())
        if (response == null) {
            RustEngineBridge.recordFailure(op, "response decode failed")
            return null
        }
        if (response.error != ErrorCode.OK) {
            RustEngineBridge.recordFailure(op, "engine returned ${response.error}", response.error.number)
            return null
        }
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
        val resp = dispatch(methodSetter, op, generation, config)
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
}

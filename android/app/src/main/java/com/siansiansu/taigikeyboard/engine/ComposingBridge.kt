// 中文: Composing + Continuous-input 橋 — 12 composing ops + 4 continuous ops。
// 中文: 對應 iOS RustEngineBridge+Composing.swift。共用 RustEngineBridge.sendRawBytes 做 JNI roundtrip。
// 中文: 巢狀型別(ComposingTransition / ContinuousCandidate / CandidateMode 等)留在 RustEngineBridge,public API 不破。

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.ComposingRequest
import com.siansiansu.taigikeyboard.engine.proto.ComposingResponse
import com.siansiansu.taigikeyboard.engine.proto.CustomDictEntry
import com.siansiansu.taigikeyboard.engine.proto.ErrorCode
import com.siansiansu.taigikeyboard.engine.proto.FrequencyEntry
import com.siansiansu.taigikeyboard.engine.proto.Request
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug

/**
 * Impl object backing `RustEngineBridge` composing / continuous-input facade
 * methods. NOT a public API — callers stay on `RustEngineBridge.*` per F1=A
 * facade contract. Module-internal visibility keeps the surface honest.
 */
internal object ComposingBridge {
    // region Composing slice (12 ops)

    fun composingStart(
        text: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Start
            .newBuilder()
            .setText(text)
            .build()
        return composingDispatch(
            methodSetter = { it.start = payload },
            op = "composingStart",
            generation = generation,
            config = RustEngineBridge.appConfig(mode, toggles),
        )
    }

    fun composingAppend(
        ch: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Append
            .newBuilder()
            .setChar(ch)
            .build()
        return composingDispatch(
            methodSetter = { it.append = payload },
            op = "composingAppend",
            generation = generation,
            config = RustEngineBridge.appConfig(mode, toggles),
        )
    }

    fun composingAppendHyphen(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.AppendHyphen
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.appendHyphen = payload },
            op = "composingAppendHyphen",
            generation = generation,
            config = RustEngineBridge.appConfig(mode, toggles),
        )
    }

    fun composingReplaceLast(
        replacement: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ReplaceLast
            .newBuilder()
            .setReplacement(replacement)
            .build()
        return composingDispatch(
            methodSetter = { it.replaceLast = payload },
            op = "composingReplaceLast",
            generation = generation,
            config = RustEngineBridge.appConfig(mode, toggles),
        )
    }

    fun composingDeleteBackward(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.DeleteBackward
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.deleteBackward = payload },
            op = "composingDeleteBackward",
            generation = generation,
            config = RustEngineBridge.appConfig(mode, toggles),
        )
    }

    fun composingCommitDerived(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.CommitDerived
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.commitDerived = payload },
            op = "composingCommitDerived",
            generation = generation,
            config = RustEngineBridge.appConfig(mode, toggles),
        )
    }

    fun composingCommitRaw(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.CommitRaw
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.commitRaw = payload },
            op = "composingCommitRaw",
            generation = generation,
            config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    fun composingSelectSuggestion(
        text: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.SelectSuggestion
            .newBuilder()
            .setText(text)
            .build()
        return composingDispatch(
            methodSetter = { it.selectSuggestion = payload },
            op = "composingSelectSuggestion",
            generation = generation,
            config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    fun composingCommitPreeditThenInsertExternal(
        text: String,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto
            .CommitPreeditThenInsertExternal
            .newBuilder()
            .setText(text)
            .build()
        return composingDispatch(
            methodSetter = { it.commitPreeditThenInsertExternal = payload },
            op = "composingCommitPreeditThenInsertExternal",
            generation = generation,
            config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    fun composingReset(generation: Long): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.Reset
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.reset = payload },
            op = "composingReset",
            generation = generation,
            config = null,
        )
    }

    fun composingSetSelectedCandidateIndex(
        index: Int,
        generation: Long,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto
            .SetSelectedCandidateIndex
            .newBuilder()
            .setIndex(index)
            .build()
        return composingDispatch(
            methodSetter = { it.setSelectedCandidateIndex = payload },
            op = "composingSetSelectedCandidateIndex",
            generation = generation,
            config = null,
        )
    }

    fun composingQueryState(generation: Long): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.QueryState
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.queryState = payload },
            op = "composingQueryState",
            generation = generation,
            config = null,
        )
    }

    // endregion
    // region Continuous-input (4 ops) — v3.5.8

    fun composingEnterContinuous(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.EnterContinuous
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.enterContinuous = payload },
            op = "composingEnterContinuous",
            generation = generation,
            config = RustEngineBridge.appConfig(mode, toggles),
        )
    }

    fun composingFetchAtPos(
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        frequencyEntries: List<FrequencyEntry>,
        nowMs: Long,
        customEntries: List<CustomDictEntry>,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
    ): RustEngineBridge.ContinuousFetchResult {
        val payload = com.siansiansu.taigikeyboard.engine.proto.FetchAtPos
            .newBuilder()
            .setPosition(0)
            .addAllFrequencyEntries(frequencyEntries)
            .setNowMs(nowMs)
            .addAllCustomEntries(customEntries)
            .build()
        return composingFetchDispatch(
            methodSetter = { it.fetchAtPos = payload },
            op = "composingFetchAtPos",
            generation = generation,
            config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    fun composingCommitContinuous(
        displayText: String,
        canonicalText: String,
        consumedBytes: Int,
        syllableCount: Int,
        mode: NormalizeMode,
        toggles: ToneTogglesCarrier,
        generation: Long,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
    ): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.CommitContinuous
            .newBuilder()
            .setDisplayText(displayText)
            .setCanonicalText(canonicalText)
            .setConsumedBytes(consumedBytes)
            .setSyllableCount(syllableCount)
            .build()
        return composingDispatch(
            methodSetter = { it.commitContinuous = payload },
            op = "composingCommitContinuous",
            generation = generation,
            config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts),
        )
    }

    fun composingResetContinuous(generation: Long): RustEngineBridge.ComposingTransition {
        val payload = com.siansiansu.taigikeyboard.engine.proto.ResetContinuous
            .newBuilder()
            .build()
        return composingDispatch(
            methodSetter = { it.resetContinuous = payload },
            op = "composingResetContinuous",
            generation = generation,
            config = null,
        )
    }

    // endregion
    // region Private dispatch

    /**
     * Encode → FFI roundtrip → decode for the composing slice. Returns the
     * raw `ComposingResponse` proto so callers that need access to the
     * `continuous` carrier (FetchAtPos) can reach it without a second
     * dispatch. Generation is passed through verbatim — composing-slice
     * generation bumping is owned by `ComposingManager.bumpGeneration()`,
     * not this layer.
     *
     * 中文: composing slice 的 FFI roundtrip,回傳原始 proto 供需要 continuous 載體的 caller(FetchAtPos)使用。
     */
    private inline fun composingProtoRoundtrip(
        methodSetter: (ComposingRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig?,
    ): ComposingResponse? {
        val composingBuilder = ComposingRequest.newBuilder()
        methodSetter(composingBuilder)
        val requestBuilder = Request
            .newBuilder()
            .setId(RustEngineBridge.nextRequestIdInternal())
            .setGeneration(generation)
            .setComposing(composingBuilder.build())
        if (config != null) {
            requestBuilder.configSnapshot = config
        }
        val request = requestBuilder.build()
        RustEngineBridge.backend.tdebug("RustEngineBridge") {
            "[FFI->] fn=composingDispatch op=$op id=${request.id} generation=$generation"
        }
        val response = RustEngineBridge.sendRawBytes(request.toByteArray())
        if (response == null) {
            RustEngineBridge.recordFailure(op, "response decode failed")
            return null
        }
        if (response.error != ErrorCode.OK) {
            RustEngineBridge.recordFailure(op, "engine returned ${response.error}", response.error.number)
            return null
        }
        if (!response.hasComposing()) {
            RustEngineBridge.recordFailure(op, "missing composing payload")
            return null
        }
        return response.composing
    }

    private inline fun composingDispatch(
        methodSetter: (ComposingRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig?,
    ): RustEngineBridge.ComposingTransition {
        val payload = composingProtoRoundtrip(methodSetter, op, generation, config)
            ?: return RustEngineBridge.ComposingTransition.NOOP
        val transition = synthComposing(payload)
        RustEngineBridge.backend.tdebug("RustEngineBridge") {
            "[FFI<-] fn=composingDispatch op=$op effects=${transition.effects.size} composing=${transition.isComposing}"
        }
        return transition
    }

    /**
     * Phase 6 FetchAtPos dispatcher. Synthesizes both the standard
     * [RustEngineBridge.ComposingTransition] (for engine snapshot mirroring)
     * and the [RustEngineBridge.ContinuousFetchResult.candidates] tri-state
     * read off `ComposingResponse.continuous`.
     *
     * 中文: Phase 6 FetchAtPos 專用分派 — 同時產生 ComposingTransition 與
     * 中文: ContinuousFetchResult.candidates(從 proto.continuous 三態解碼)。
     */
    private inline fun composingFetchDispatch(
        methodSetter: (ComposingRequest.Builder) -> Unit,
        op: String,
        generation: Long,
        config: AppConfig?,
    ): RustEngineBridge.ContinuousFetchResult {
        val payload = composingProtoRoundtrip(methodSetter, op, generation, config)
            ?: return RustEngineBridge.ContinuousFetchResult.NOOP
        val transition = synthComposing(payload)
        val candidates: List<RustEngineBridge.ContinuousCandidate>? = if (payload.hasContinuous()) {
            payload.continuous.candidatesList.map { msg ->
                // v3.5.8 Phase 9 Item 5 — `hanji` is proto3 `optional`;
                // protobuf-javalite exposes presence via `hasHanji()`.
                // Map absent → `null` (NOT empty string) so the
                // bridge data class's `hanji: String?` carries the
                // wire-absent distinction faithfully (TAILO candidate).
                //
                // Defensive `roman` fallback per
                // `docs/engine/continuous-candidate-display.md` §7 +
                // Codex pre-impl F4 verdict A: if `msg.roman` is
                // empty (old-Rust-new-platform wire skew, or proto
                // regen skipped), fall back to `displayText` so the
                // Item 6 dual-line render does not show a blank title
                // row. Bundled releases never hit this branch.
                // 中文: Item 5 — hanji 為 proto3 optional;wire absent → Kotlin null。
                // 中文: roman 防禦性 fallback — wire skew 時 displayText 兜底,避免空 title。
                val roman = if (msg.roman.isEmpty()) msg.displayText else msg.roman
                RustEngineBridge.ContinuousCandidate(
                    consumedSpanStart = msg.consumedSpanStart,
                    consumedSpanEnd = msg.consumedSpanEnd,
                    syllableCount = msg.syllableCount,
                    displayText = msg.displayText,
                    score = msg.score,
                    form = msg.form,
                    mode = RustEngineBridge.CandidateMode.decode(msg.modeValue),
                    roman = roman,
                    hanji = if (msg.hasHanji()) msg.hanji else null,
                )
            }
        } else {
            null
        }
        RustEngineBridge.backend.tdebug("RustEngineBridge") {
            val count = candidates?.size ?: -1
            "[FFI<-] fn=composingFetchDispatch op=$op effects=${transition.effects.size} candidates=$count"
        }
        return RustEngineBridge.ContinuousFetchResult(
            transition = transition,
            candidates = candidates,
            isBridgeFailure = false,
        )
    }

    private fun synthComposing(proto: ComposingResponse): RustEngineBridge.ComposingTransition {
        // Effect contract is exhaustive — every emitted Effect.kind maps to a
        // Kotlin case. The InputConnection-bound effects route through
        // DefaultComposingDelegate; the 3 NextWord-shaped effects route
        // through SmartbarManager.dispatchComposingNextWordEffect via the
        // NextWordEffectRouter sibling on ComposingManager.
        val effects: List<RustEngineBridge.ComposingTransition.Effect> = proto.effectList.mapNotNull { eff ->
            when {
                eff.hasUpdatePreedit() -> {
                    RustEngineBridge.ComposingTransition.Effect.UpdatePreedit(eff.updatePreedit.display)
                }

                eff.hasClearPreeditWithoutCommit() -> {
                    RustEngineBridge.ComposingTransition.Effect.ClearPreeditWithoutCommit
                }

                eff.hasCommitTextReplacingPreedit() -> {
                    RustEngineBridge.ComposingTransition.Effect.CommitTextReplacingPreedit(
                        eff.commitTextReplacingPreedit.text,
                    )
                }

                eff.hasDeleteBackwardFromDocument() -> {
                    RustEngineBridge.ComposingTransition.Effect.DeleteBackwardFromDocument
                }

                eff.hasResetAutocomplete() -> {
                    RustEngineBridge.ComposingTransition.Effect.ResetAutocomplete
                }

                eff.hasPerformAutocomplete() -> {
                    RustEngineBridge.ComposingTransition.Effect.PerformAutocomplete
                }

                eff.hasResetAutocompleteContext() -> {
                    RustEngineBridge.ComposingTransition.Effect.ResetAutocompleteContext
                }

                eff.hasNextWordUpdateLastSelectedWord() -> {
                    RustEngineBridge.ComposingTransition.Effect.NextWordUpdateLastSelectedWord(
                        text = eff.nextWordUpdateLastSelectedWord.text,
                        roman = eff.nextWordUpdateLastSelectedWord.roman,
                    )
                }

                eff.hasNextWordWordSelected() -> {
                    RustEngineBridge.ComposingTransition.Effect.NextWordWordSelected(
                        text = eff.nextWordWordSelected.text,
                        roman = eff.nextWordWordSelected.roman,
                        triggerPrediction = eff.nextWordWordSelected.triggerPrediction,
                    )
                }

                eff.hasNextWordClearForNewComposing() -> {
                    RustEngineBridge.ComposingTransition.Effect.NextWordClearForNewComposing
                }

                else -> {
                    null
                }
            }
        }
        return RustEngineBridge.ComposingTransition(
            rawInput = proto.preedit.rawInput,
            displayText = proto.preedit.displayText,
            effects = effects,
            selectedCandidateIndex = proto.selectedCandidateIndex,
            isComposing = proto.isComposing,
        )
    }

    // endregion
}

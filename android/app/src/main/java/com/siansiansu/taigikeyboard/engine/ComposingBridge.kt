// Composing + Continuous-input ops — extensions on RustEngineBridge mirroring
// iOS RustEngineBridge+Composing.swift via the shared JNI roundtrip; nested types stay in RustEngineBridge.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.ComposingRequest
import com.siansiansu.taigikeyboard.engine.proto.ComposingResponse
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.settings.EngineSettings

// region Composing slice (10 ops)

fun RustEngineBridge.composingStart(
    text: String,
    settings: EngineSettings,
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
        config = continuousAppConfig(settings),
    )
}

fun RustEngineBridge.composingAppend(
    ch: String,
    settings: EngineSettings,
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
        config = continuousAppConfig(settings),
    )
}

// Separator hyphen distinguishes raw "tai-uan" from "taiuan", which changes the candidate trie key.
fun RustEngineBridge.composingAppendHyphen(
    settings: EngineSettings,
    generation: Long,
): RustEngineBridge.ComposingTransition {
    val payload = com.siansiansu.taigikeyboard.engine.proto.AppendHyphen
        .newBuilder()
        .build()
    return composingDispatch(
        methodSetter = { it.appendHyphen = payload },
        op = "composingAppendHyphen",
        generation = generation,
        config = continuousAppConfig(settings),
    )
}

fun RustEngineBridge.composingReplaceLast(
    replacement: String,
    settings: EngineSettings,
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
        config = continuousAppConfig(settings),
    )
}

// Engine owns the delete-to-empty → Idle transition and the 1-char delete path that must not eat document text.
fun RustEngineBridge.composingDeleteBackward(
    settings: EngineSettings,
    generation: Long,
): RustEngineBridge.ComposingTransition {
    val payload = com.siansiansu.taigikeyboard.engine.proto.DeleteBackward
        .newBuilder()
        .build()
    return composingDispatch(
        methodSetter = { it.deleteBackward = payload },
        op = "composingDeleteBackward",
        generation = generation,
        config = continuousAppConfig(settings),
    )
}

// The engine commits the whole composition
// (`combined_display(nailed, pending, config)`), not the literal keystrokes.
fun RustEngineBridge.composingCommitRaw(
    settings: EngineSettings,
    generation: Long,
): RustEngineBridge.ComposingTransition {
    val payload = com.siansiansu.taigikeyboard.engine.proto.CommitRaw
        .newBuilder()
        .build()
    return composingDispatch(
        methodSetter = { it.commitRaw = payload },
        op = "composingCommitRaw",
        generation = generation,
        config = continuousAppConfig(settings),
    )
}

// Commits the preedit and inserts the external string (space / Enter / punctuation) atomically, to avoid flicker.
// E.g. an emoji tap mid-composition: the engine commits the rendered
// composition first, then inserts `text`.
fun RustEngineBridge.composingCommitPreeditThenInsertExternal(
    text: String,
    settings: EngineSettings,
    generation: Long,
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
        config = continuousAppConfig(settings),
    )
}

fun RustEngineBridge.composingReset(generation: Long): RustEngineBridge.ComposingTransition {
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

// endregion
// region Continuous-input (2 ops) — v3.5.8

/**
 * Read-only candidate query for the current `Phase::Continuous { raw }`.
 * Caller MUST share the active composing-session generation — FetchAtPos
 * is read-only and bumping generation would reset engine state before
 * the fetch (`engine/composing/src/requests.rs::query`).
 *
 * The user's own data is not among the arguments: the engine reads its
 * stores itself and ranks in the same call
 * (`docs/architecture/user-data-engine-roadmap.md` P8b). `nowMs` is the
 * clock its recency ranking reads. Mirrors desktop-core
 * `engine::composing::fetch_at_pos`.
 */
fun RustEngineBridge.composingFetchAtPos(
    // Built once by the caller (`continuousAppConfig`) from
    // one snapshot of the live settings.
    config: AppConfig,
    generation: Long,
    nowMs: Long,
    // The user's dictionary toggles (the ones Tab3 browse resolves); the
    // engine turns them into its source filter.
    dictionaryToggles: RustEngineBridge.DictionaryToggles,
    // §34/S22 — invert of the Show Typed Text First setting. Default `false` = show
    // (proto3-absent sentinel → engine prepends the literal-roman
    // candidate, the pre-toggle always-on behaviour for callers/tests).
    literalRomanCandidateDisabled: Boolean = false,
    // Invert of the Enable Custom Dictionary setting: the engine reads the
    // user's dictionary only with it on. Default `false` = read it.
    customDictionaryDisabled: Boolean = false,
): List<RustEngineBridge.ContinuousCandidate> {
    val payload = com.siansiansu.taigikeyboard.engine.proto.FetchAtPos
        .newBuilder()
        .setNowMs(nowMs)
        .setLiteralRomanCandidateDisabled(literalRomanCandidateDisabled)
        .setCustomDictionaryDisabled(customDictionaryDisabled)
        .setToggles(dictionaryTogglesProto(dictionaryToggles))
        .build()
    return composingFetchDispatch(
        methodSetter = { it.fetchAtPos = payload },
        op = "composingFetchAtPos",
        generation = generation,
        config = config,
    )
}

/**
 * Commit a candidate segment in `Phase::Continuous`. The pick MUST come from
 * a [RustEngineBridge.ContinuousCandidate] returned by an immediately
 * preceding [composingFetchAtPos] call — mismatched values mis-align the
 * committed segment. `consumedBytes >= pending.utf8.size` makes a final
 * commit (exit to Idle); anything less nails the segment, writing nothing
 * (Model B). The engine resolves the document text from the pick's scripts
 * under [settings] and counts the pick itself (R5). Mirrors iOS
 * `RustEngineBridge.composingCommitContinuous`.
 */
fun RustEngineBridge.composingCommitContinuous(
    pick: RustEngineBridge.ContinuousPick,
    settings: EngineSettings,
    generation: Long,
): RustEngineBridge.ContinuousCommitResult {
    val request = pick.toRequest()
    val payload = composingProtoRoundtrip(
        methodSetter = { it.commitContinuous = request },
        op = "composingCommitContinuous",
        generation = generation,
        config = continuousAppConfig(settings),
    ) ?: return RustEngineBridge.ContinuousCommitResult.FAILED
    // Always set on this path; an absent one reads as UNSPECIFIED → ignored.
    return RustEngineBridge.ContinuousCommitResult(
        transition = synthComposing(payload),
        outcome = RustEngineBridge.ContinuousCommitOutcome.from(payload.commit),
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
 */
private inline fun composingProtoRoundtrip(
    methodSetter: (ComposingRequest.Builder) -> Unit,
    op: String,
    generation: Long,
    config: AppConfig?,
): ComposingResponse? {
    val composingBuilder = ComposingRequest.newBuilder()
    methodSetter(composingBuilder)
    val composingRequest = composingBuilder.build()
    val response = RustEngineBridge.dispatch(op) {
        setGeneration(generation)
        setComposing(composingRequest)
        if (config != null) {
            configSnapshot = config
        }
        RustEngineBridge.backend.tdebug("RustEngineBridge") {
            "[FFI->] fn=composingDispatch op=$op id=$id generation=$generation"
        }
    } ?: return null
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
 * Phase 6 FetchAtPos dispatcher: the candidates read off
 * `ComposingResponse.continuous`. Empty when the round-trip failed, when
 * `Phase::Continuous` was not active (the carrier absent — e.g. a generation
 * a `bumpGeneration` has since replaced), or when nothing matched: each is
 * "no candidates this frame".
 */
private inline fun composingFetchDispatch(
    methodSetter: (ComposingRequest.Builder) -> Unit,
    op: String,
    generation: Long,
    config: AppConfig?,
): List<RustEngineBridge.ContinuousCandidate> {
    val payload = composingProtoRoundtrip(methodSetter, op, generation, config)
        ?: return emptyList()
    // FetchAtPos is read-only: the response carries no effects and the
    // platform mirrors nothing from it, so only the candidate carrier is
    // decoded (no `synthComposing` walk per fetch).
    val candidates: List<RustEngineBridge.ContinuousCandidate> = if (payload.hasContinuous()) {
        payload.continuous.candidatesList.map { msg ->
            // v3.5.8 Phase 9 Item 5 — `hanji` is proto3 `optional`;
            // protobuf-javalite exposes presence via `hasHanji()`.
            // Map absent → `null` (NOT empty string) so the
            // bridge data class's `hanji: String?` carries the
            // wire-absent distinction faithfully (hanji-less candidate).
            RustEngineBridge.ContinuousCandidate(
                consumedSpanEnd = msg.consumedSpanEnd,
                syllableCount = msg.syllableCount,
                displayText = msg.displayText,
                roman = msg.roman,
                hanji = if (msg.hasHanji()) msg.hanji else null,
                canonicalTl = msg.canonicalTl,
            )
        }
    } else {
        emptyList()
    }
    RustEngineBridge.backend.tdebug("RustEngineBridge") {
        val count = candidates.size
        "[FFI<-] fn=composingFetchDispatch op=$op candidates=$count"
    }
    return candidates
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

            eff.hasClearCandidates() -> {
                RustEngineBridge.ComposingTransition.Effect.ClearCandidates
            }

            eff.hasRefreshCandidates() -> {
                RustEngineBridge.ComposingTransition.Effect.RefreshCandidates
            }

            eff.hasResetCandidateContext() -> {
                RustEngineBridge.ComposingTransition.Effect.ResetCandidateContext
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
                    preceding = eff.nextWordWordSelected.precedingList,
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
        isComposing = proto.isComposing,
    )
}

// endregion

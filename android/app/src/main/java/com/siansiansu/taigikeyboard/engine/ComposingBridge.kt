// Composing + Continuous-input ops — extensions on RustEngineBridge mirroring
// iOS RustEngineBridge+Composing.swift via the shared JNI roundtrip; its DTOs are top-level types here.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.CommitContinuous
import com.siansiansu.taigikeyboard.engine.proto.CommitOutcome
import com.siansiansu.taigikeyboard.engine.proto.CommitPrediction
import com.siansiansu.taigikeyboard.engine.proto.CommitResolution
import com.siansiansu.taigikeyboard.engine.proto.CommitScript
import com.siansiansu.taigikeyboard.engine.proto.CommittedWord
import com.siansiansu.taigikeyboard.engine.proto.ComposingRequest
import com.siansiansu.taigikeyboard.engine.proto.ComposingResponse
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.settings.EngineSettings

// region Composing DTOs

/**
 * Bridge-synthesized companion to the proto `ComposingResponse`.
 * Consumed by `ComposingManager` and its delegate.
 */
data class ComposingTransition(
    val rawInput: String,
    val displayText: String,
    val effects: List<Effect>,
    val isComposing: Boolean,
) {
    sealed class Effect {
        data class UpdatePreedit(
            val display: String,
        ) : Effect()

        object ClearPreeditWithoutCommit : Effect()

        data class CommitTextReplacingPreedit(
            val text: String,
        ) : Effect()

        object ClearCandidates : Effect()

        object RefreshCandidates : Effect()

        object ResetCandidateContext : Effect()

        /**
         * v3.5.8 Phase 4 — continuous-input mid-commit handshake. Maps to
         * `NextWordRequest::UpdateLastSelectedWord(text, roman, now_ms)`.
         * Platform delegate forwards to `NextWordController.updateLastSelectedWord`
         * which injects `nowMs` + envelope generation.
         */
        data class NextWordUpdateLastSelectedWord(
            val text: String,
            val roman: String,
        ) : Effect()

        /**
         * v3.5.8 Phase 4 — continuous-input final-commit handshake. Maps to
         * `NextWordRequest::WordSelected(text, roman, require_roman_mode=false,
         * trigger_prediction, now_ms, preceding)`. Forward `triggerPrediction`
         * and `preceding` exactly — `preceding` is the nailed segments committed
         * before `text`, learned as one sequence (behavioral-invariants §40).
         */
        data class NextWordWordSelected(
            val text: String,
            val roman: String,
            val triggerPrediction: Boolean,
            val preceding: List<CommittedWord>,
        ) : Effect()

        /**
         * v3.5.8 Phase 4 — continuous-input abort handshake. Maps to
         * `NextWordRequest::ClearForNewComposing(now_ms)`. Platform delegate
         * forwards to `NextWordController.onClearCandidates()` (Android equivalent
         * of iOS `NextWordController.clearDisplay()`); NOT the structurally
         * distinct `ResetAll` intent.
         */
        object NextWordClearForNewComposing : Effect()
    }

    companion object {
        val NOOP = ComposingTransition(
            rawInput = "",
            displayText = "",
            effects = emptyList(),
            isComposing = false,
        )
    }
}

/**
 * Single span-local continuous-input candidate. Wire mirror of
 * `protos::engine::CandidateMessage` (Phase 6).
 *
 * `consumedSpanEnd` is a byte offset into the **original raw user input**
 * stored in `Phase::Continuous { raw }` — TL/POJ users → ASCII bytes, TPS
 * users → Bopomofo bytes. The tap path hands it back as `consumedBytes`
 * so the engine drops that many bytes from `pending` on commit.
 */
data class ContinuousCandidate(
    val consumedSpanEnd: Int,
    val syllableCount: Int,
    val displayText: String,
    /**
     * v3.5.8 Phase 9 Item 5 — display-romanization sidechannel for
     * dual-line cell render. Always non-empty for dictionary-
     * sourced candidates; the engine renders it for the active
     * input mode (TL, or POJ-display in POJ mode). UI reads
     * `displayText` for commit / `user_frequency.db` writes and
     * `roman` only for cell-title display.
     */
    val roman: String,
    /**
     * v3.5.8 Phase 9 Item 5 — hanji display sidechannel. `null`
     * iff the proto3 `optional string hanji` was absent on the
     * wire (TAILO candidate). Present-empty is treated as
     * present (engine never emits `Some("")` today; defensive).
     */
    val hanji: String?,
    /**
     * v3.6.1 R2 — canonical TL identity sidechannel
     * (`CandidateMessage.canonical_tl`). Unlike `roman` (the
     * POJ-rendered display form in POJ mode), this stays the canonical
     * TL the `(hanji, canonical-TL)` word identity is keyed on. The tap
     * path round-trips it into `commitContinuous(associationTl = …)` so
     * the NextWord association learns the same TL a normal candidate
     * commit records. Empty only for TPS-OOV hanji-absent candidates
     * with no dict TL.
     */
    val canonicalTl: String,
)

/**
 * One continuous-candidate pick (R5): which of its scripts the document
 * gets, and the candidate metadata the engine resolves the text from
 * (`engine/composing/src/commit_text.rs`). Every field but [script]
 * round-trips verbatim from the [ContinuousCandidate] the user picked;
 * [consumedBytes] is its `consumedSpanEnd`, an absolute offset into the
 * pending raw buffer. Mirrors iOS `RustEngineBridge.ContinuousPick`.
 */
data class ContinuousPick(
    val script: CommitScript,
    /** The candidate's display romanization ([ContinuousCandidate.roman]). */
    val roman: String,
    /**
     * Identity keys the engine counts the pick under — the canonical
     * `hanji ?: roman` and TL (Core Principle #6), never the rendering.
     */
    val canonicalText: String,
    val associationTl: String,
    /** §50 — the pick's hanji, null for a hanji-less candidate. */
    val hanji: String?,
    val consumedBytes: Int,
    val syllableCount: Int,
) {
    /** The wire request. */
    fun toRequest(): CommitContinuous {
        // A local, not the property: inside `apply` the builder's own
        // `hanji` would shadow it.
        val hanji = hanji
        return CommitContinuous
            .newBuilder()
            .setScript(script)
            .setRoman(roman)
            .setCanonicalText(canonicalText)
            // R2: canonical TL → NextWord next_tl/prev_tl. Empty → engine
            // falls back to the raw committed slice.
            .setAssociationTl(associationTl)
            .apply { if (!hanji.isNullOrEmpty()) setHanji(hanji) }
            .setConsumedBytes(consumedBytes)
            .setSyllableCount(syllableCount)
            .build()
    }
}

/**
 * What a continuous pick did, as the engine answered it
 * (`ComposingResponse.commit`) — never read off the composing mirror: a
 * generation mismatch resets the engine to Idle before the intent runs
 * (`engine/composing/src/handle.rs`), so a mirror read would report a
 * commit that never happened.
 */
sealed interface ContinuousCommitOutcome {
    /** Nothing changed: a stale generation, a rejected pick, or a failed round-trip. */
    data object Ignored : ContinuousCommitOutcome

    /** The segment was nailed and the composition continues (Model B: no document write). */
    data object Nailed : ContinuousCommitOutcome

    /**
     * The whole composition was written and the engine is Idle.
     * [earnsAutoSpace] is the engine's §23 verdict on what the pick wrote;
     * the live Auto-Space setting is the caller's.
     */
    data class Finalized(
        val earnsAutoSpace: Boolean,
    ) : ContinuousCommitOutcome

    companion object {
        // CROSS-PLATFORM INVARIANT — mirrors iOS `RustEngineBridge.ContinuousCommitOutcome.init(_:)`.
        fun from(resolution: CommitResolution): ContinuousCommitOutcome =
            when (resolution.outcome) {
                CommitOutcome.COMMIT_OUTCOME_NAILED -> Nailed
                CommitOutcome.COMMIT_OUTCOME_FINALIZED -> Finalized(resolution.earnsAutoSpace)
                // UNSPECIFIED: an answer without a resolution changed
                // nothing this side can tell.
                else -> Ignored
            }
    }
}

/** Result of [composingCommitContinuous]: the transition to replay and what the pick did. */
data class ContinuousCommitResult(
    val transition: ComposingTransition,
    val outcome: ContinuousCommitOutcome,
) {
    companion object {
        val FAILED = ContinuousCommitResult(ComposingTransition.NOOP, ContinuousCommitOutcome.Ignored)
    }
}

/**
 * One NextWord prediction tap (D1): which of its scripts the document
 * gets, and the prediction's own fields the engine resolves the text from
 * (`engine/composing/src/commit_text.rs`) and counts the usage under.
 * Mirrors iOS `RustEngineBridge.PredictionPick`.
 */
data class PredictionPick(
    val script: CommitScript,
    /**
     * The prediction's display romanization (`EnginePrediction.text` of a
     * prediction with a subtitle); empty for a Hanji-only prediction.
     */
    val roman: String,
    val hanji: String,
    /** The canonical TL the engine counts the usage under. */
    val canonicalTl: String,
) {
    /** The wire request. */
    fun toRequest(): CommitPrediction =
        CommitPrediction
            .newBuilder()
            .setScript(script)
            .setRoman(roman)
            .setHanji(hanji)
            .setCanonicalTl(canonicalTl)
            .build()
}

/**
 * What a prediction tap writes into the document, as the engine resolved
 * it. [earnsAutoSpace] is the engine's §23 verdict; the live Auto-Space
 * setting is the caller's.
 */
data class PredictionCommit(
    val text: String,
    val earnsAutoSpace: Boolean,
)

// endregion

// region Composing slice (10 ops)

fun RustEngineBridge.composingStart(
    text: String,
    settings: EngineSettings,
    generation: Long,
): ComposingTransition {
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
): ComposingTransition {
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
): ComposingTransition {
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
): ComposingTransition {
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
): ComposingTransition {
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
): ComposingTransition {
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
): ComposingTransition {
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

fun RustEngineBridge.composingReset(generation: Long): ComposingTransition {
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
    // one snapshot of the live settings; its `dictionarySources` is the
    // fetch's source filter.
    config: AppConfig,
    generation: Long,
    nowMs: Long,
    // §34/S22 — invert of the Show Typed Text First setting. Default `false` = show
    // (proto3-absent sentinel → engine prepends the literal-roman
    // candidate, the pre-toggle always-on behaviour for callers/tests).
    literalRomanCandidateDisabled: Boolean = false,
): List<ContinuousCandidate> {
    val payload = com.siansiansu.taigikeyboard.engine.proto.FetchAtPos
        .newBuilder()
        .setNowMs(nowMs)
        .setLiteralRomanCandidateDisabled(literalRomanCandidateDisabled)
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
 * a [ContinuousCandidate] returned by an immediately
 * preceding [composingFetchAtPos] call — mismatched values mis-align the
 * committed segment. `consumedBytes >= pending.utf8.size` makes a final
 * commit (exit to Idle); anything less nails the segment, writing nothing
 * (Model B). The engine resolves the document text from the pick's scripts
 * under [settings] and counts the pick itself (R5). Mirrors iOS
 * `RustEngineBridge.composingCommitContinuous`.
 */
fun RustEngineBridge.composingCommitContinuous(
    pick: ContinuousPick,
    settings: EngineSettings,
    generation: Long,
): ContinuousCommitResult {
    val request = pick.toRequest()
    val payload = composingProtoRoundtrip(
        methodSetter = { it.commitContinuous = request },
        op = "composingCommitContinuous",
        generation = generation,
        config = continuousAppConfig(settings),
    ) ?: return ContinuousCommitResult.FAILED
    // Always set on this path; an absent one reads as UNSPECIFIED → ignored.
    return ContinuousCommitResult(
        transition = synthComposing(payload),
        outcome = ContinuousCommitOutcome.from(payload.commit),
    )
}

// endregion
// region NextWord prediction tap (1 op) — D1

/**
 * Resolve a NextWord prediction tap. The engine answers the document text
 * under [settings] and counts the usage itself; the composition is left as
 * it is, so the response's snapshot carries nothing to replay. `null` when
 * the engine wrote nothing (IGNORED, or a failed round-trip). Mirrors iOS
 * `RustEngineBridge.composingCommitPrediction`.
 */
fun RustEngineBridge.composingCommitPrediction(
    pick: PredictionPick,
    settings: EngineSettings,
    generation: Long,
): PredictionCommit? {
    val request = pick.toRequest()
    val payload = composingProtoRoundtrip(
        methodSetter = { it.commitPrediction = request },
        op = "composingCommitPrediction",
        generation = generation,
        config = continuousAppConfig(settings),
    ) ?: return null
    val outcome = ContinuousCommitOutcome.from(payload.commit)
    if (outcome !is ContinuousCommitOutcome.Finalized) return null
    return PredictionCommit(text = payload.commit.documentText, earnsAutoSpace = outcome.earnsAutoSpace)
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
): ComposingTransition {
    val payload = composingProtoRoundtrip(methodSetter, op, generation, config)
        ?: return ComposingTransition.NOOP
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
): List<ContinuousCandidate> {
    val payload = composingProtoRoundtrip(methodSetter, op, generation, config)
        ?: return emptyList()
    // FetchAtPos is read-only: the response carries no effects and the
    // platform mirrors nothing from it, so only the candidate carrier is
    // decoded (no `synthComposing` walk per fetch).
    val candidates: List<ContinuousCandidate> = if (payload.hasContinuous()) {
        payload.continuous.candidatesList.map { msg ->
            // v3.5.8 Phase 9 Item 5 — `hanji` is proto3 `optional`;
            // protobuf-javalite exposes presence via `hasHanji()`.
            // Map absent → `null` (NOT empty string) so the
            // bridge data class's `hanji: String?` carries the
            // wire-absent distinction faithfully (hanji-less candidate).
            ContinuousCandidate(
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

private fun synthComposing(proto: ComposingResponse): ComposingTransition {
    // Effect contract is exhaustive — every emitted Effect.kind maps to a
    // Kotlin case. The InputConnection-bound effects route through
    // DefaultComposingDelegate; the 3 NextWord-shaped effects route
    // through SmartbarManager.dispatchComposingNextWordEffect via the
    // NextWordEffectRouter sibling on ComposingManager.
    val effects: List<ComposingTransition.Effect> = proto.effectList.mapNotNull { eff ->
        when {
            eff.hasUpdatePreedit() -> {
                ComposingTransition.Effect.UpdatePreedit(eff.updatePreedit.display)
            }

            eff.hasClearPreeditWithoutCommit() -> {
                ComposingTransition.Effect.ClearPreeditWithoutCommit
            }

            eff.hasCommitTextReplacingPreedit() -> {
                ComposingTransition.Effect.CommitTextReplacingPreedit(
                    eff.commitTextReplacingPreedit.text,
                )
            }

            eff.hasClearCandidates() -> {
                ComposingTransition.Effect.ClearCandidates
            }

            eff.hasRefreshCandidates() -> {
                ComposingTransition.Effect.RefreshCandidates
            }

            eff.hasResetCandidateContext() -> {
                ComposingTransition.Effect.ResetCandidateContext
            }

            eff.hasNextWordUpdateLastSelectedWord() -> {
                ComposingTransition.Effect.NextWordUpdateLastSelectedWord(
                    text = eff.nextWordUpdateLastSelectedWord.text,
                    roman = eff.nextWordUpdateLastSelectedWord.roman,
                )
            }

            eff.hasNextWordWordSelected() -> {
                ComposingTransition.Effect.NextWordWordSelected(
                    text = eff.nextWordWordSelected.text,
                    roman = eff.nextWordWordSelected.roman,
                    triggerPrediction = eff.nextWordWordSelected.triggerPrediction,
                    preceding = eff.nextWordWordSelected.precedingList,
                )
            }

            eff.hasNextWordClearForNewComposing() -> {
                ComposingTransition.Effect.NextWordClearForNewComposing
            }

            else -> {
                null
            }
        }
    }
    return ComposingTransition(
        rawInput = proto.preedit.rawInput,
        displayText = proto.preedit.displayText,
        effects = effects,
        isComposing = proto.isComposing,
    )
}

// endregion

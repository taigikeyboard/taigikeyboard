// Composing + Continuous-input ops (12 composing, 4 continuous) — extensions on RustEngineBridge mirroring
// iOS RustEngineBridge+Composing.swift via the shared JNI roundtrip; nested types stay in RustEngineBridge.

package com.siansiansu.taigikeyboard.engine

import com.siansiansu.taigikeyboard.engine.proto.AppConfig
import com.siansiansu.taigikeyboard.engine.proto.ComposingRequest
import com.siansiansu.taigikeyboard.engine.proto.ComposingResponse
import com.siansiansu.taigikeyboard.engine.proto.CustomDictEntry
import com.siansiansu.taigikeyboard.engine.proto.FrequencyEntry
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode

// region Composing slice (12 ops)

fun RustEngineBridge.composingStart(
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

fun RustEngineBridge.composingAppend(
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

// Separator hyphen distinguishes raw "tai-uan" from "taiuan", which changes the candidate trie key.
fun RustEngineBridge.composingAppendHyphen(
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

fun RustEngineBridge.composingReplaceLast(
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

// Engine owns the delete-to-empty → Idle transition and the 1-char delete path that must not eat document text.
fun RustEngineBridge.composingDeleteBackward(
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

// Commits the derived display string, e.g. raw "ho2" commits as "hó".
fun RustEngineBridge.composingCommitDerived(
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

// Dispatched by phase: the Composing arm commits literal keystrokes ("ho2"), the Continuous arm commits
// derived_display(pending) ("hó") — which is why the Continuous side needs mode + toggles.
// v3.5.8 §10.2 platform pass: under `Phase::Continuous`, `CommitRaw`
// routes to `commit_raw_continuous` which renders the whole
// composition via `combined_display(nailed, raw, config)` — so the
// continuous spacing flags ride here. Composing-arm `CommitRaw`
// ignores them. Defaults = v3.5.7 roman-first so contract tests stay
// behavior-identical; EVERY production Continuous call site MUST pass
// explicit live values via `ComposingManager.continuousSpacingFlags`
// (the sole production caller does — verified) or hanji-first
// silently regresses.
fun RustEngineBridge.composingCommitRaw(
    mode: NormalizeMode,
    toggles: ToneTogglesCarrier,
    generation: Long,
    effectiveSwapped: Boolean = false,
    outputBothScripts: Boolean = false,
    candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
): RustEngineBridge.ComposingTransition {
    val payload = com.siansiansu.taigikeyboard.engine.proto.CommitRaw
        .newBuilder()
        .build()
    return composingDispatch(
        methodSetter = { it.commitRaw = payload },
        op = "composingCommitRaw",
        generation = generation,
        config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts, candidateDisplayMode),
    )
}

// v3.5.8 §10.2 platform pass: under `Phase::Continuous`,
// `SelectSuggestion` routes to `select_suggestion_under_continuous`
// which prepends `nailed_prefix(nailed, config)` — so the continuous
// spacing flags must ride here (previously `config = null` →
// `AppConfig::default()` → spacing always ON → hanji-first spurious
// spaces). The composing-arm `select_suggestion` ignores `config`
// entirely (commits `text` verbatim), so this is a no-op there.
// Defaults = v3.5.7 roman-first; the sole production caller
// (`ComposingManager.selectSuggestion`) passes explicit live values.
fun RustEngineBridge.composingSelectSuggestion(
    text: String,
    mode: NormalizeMode,
    toggles: ToneTogglesCarrier,
    generation: Long,
    effectiveSwapped: Boolean = false,
    outputBothScripts: Boolean = false,
    candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
): RustEngineBridge.ComposingTransition {
    val payload = com.siansiansu.taigikeyboard.engine.proto.SelectSuggestion
        .newBuilder()
        .setText(text)
        .build()
    return composingDispatch(
        methodSetter = { it.selectSuggestion = payload },
        op = "composingSelectSuggestion",
        generation = generation,
        config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts, candidateDisplayMode),
    )
}

// Commits the preedit and inserts the external string (space / Enter / punctuation) atomically, to avoid flicker.
// v3.5.8 §10.2 platform pass: under `Phase::Continuous` (e.g. emoji
// tap mid-continuous) this routes to
// `commit_preedit_then_insert_external_under_continuous` which
// renders the nailed prefix via `combined_display(nailed, raw,
// config)` — so the continuous spacing flags ride here too. (Not in
// the 2026-05-18 enumerated 4 ops, but the same class of Continuous
// nailed-rendering path: excluding it would re-create the exact
// hanji-first spurious-space regression the narrowed plumb
// minimizes — see continuous-input-ranking.md §10.2.) Defaults =
// v3.5.7 roman-first; production callers pass explicit live values.
fun RustEngineBridge.composingCommitPreeditThenInsertExternal(
    text: String,
    mode: NormalizeMode,
    toggles: ToneTogglesCarrier,
    generation: Long,
    effectiveSwapped: Boolean = false,
    outputBothScripts: Boolean = false,
    candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
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
        config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts, candidateDisplayMode),
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

// Reports the selected index so NextWord / Booster can read the context word. Does not commit.
fun RustEngineBridge.composingSetSelectedCandidateIndex(
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

fun RustEngineBridge.composingQueryState(generation: Long): RustEngineBridge.ComposingTransition {
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

/**
 * `Phase::Composing { raw }` → `Phase::Continuous { raw, committed: [] }`.
 * Phase 6 contract: no payload — buffer is whatever earlier `Start` /
 * `Append` populated. Engine no-ops on Idle / already-Continuous / empty
 * `Composing.raw`. AppConfig is required because the snapshot's preedit
 * display goes through `derived_display(raw, config)`.
 */
fun RustEngineBridge.composingEnterContinuous(
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

/**
 * Read-only candidate query for the current `Phase::Continuous { raw }`.
 * `position` is reserved as `0` in v3.5.8 (Phase 6 dispatch validates).
 * Caller MUST share the active composing-session generation — FetchAtPos
 * is read-only and bumping generation would reset engine state before
 * the fetch (`engine/composing/src/dispatch.rs:103-160`).
 *
 * `frequencyEntries` + `nowMs` are the v3.5.8 Phase 9.3a/9.3c plumb for
 * `user_freq_boost` + `SortKey.recency_rank`. Caller pre-filters entries
 * to candidate-relevant `displayTextKey`s (`hanji ?? roman`) — see
 * `engine/protos/proto/composing.proto:144-148`. Defaults `emptyList()`
 * + `0L` reproduce the PR-9.2 neutral-boost behaviour (`user_freq_boost
 * = 1.0`, `recency_rank = 1` everywhere); the platform plumb is
 * responsible for populating real values via a two-phase fetch
 * (`ComposingManager.fetchContinuousCandidates`). Mirrors iOS
 * `RustEngineBridge.composingFetchAtPos` PR-9.3b.
 *
 * v3.5.8 Phase 9 Item 12 — `customEntries` carries the platform's
 * `custom_dictionary.db` matches (raw stored `(roman, hanji)`
 * columns; DB stays native). Default `emptyList()` = no custom
 * matches / feature off — backward-compatible no-op. The engine
 * synthesizes a full-buffer candidate per entry and dedupes
 * `(roman, hanji)` against the FST hits (custom wins the
 * collision). Mirrors iOS `RustEngineBridge.composingFetchAtPos`.
 *
 * v3.5.8 §10.2 platform pass: the FetchAtPos snapshot renders the
 * combined marked region (`combined_display`) and per-segment recased
 * candidates, so it needs the continuous spacing flags to match the
 * commit-time rendering. Defaults = v3.5.7 roman-first; production
 * callers pass explicit live values via continuousSpacingFlags.
 */
fun RustEngineBridge.composingFetchAtPos(
    mode: NormalizeMode,
    toggles: ToneTogglesCarrier,
    generation: Long,
    frequencyEntries: List<FrequencyEntry> = emptyList(),
    nowMs: Long = 0L,
    customEntries: List<CustomDictEntry> = emptyList(),
    effectiveSwapped: Boolean = false,
    outputBothScripts: Boolean = false,
    // PR-9.6 — dictionary source-toggle bitmask (same one Tab3 browse
    // sends). Default `0u` = proto3-absent sentinel → engine all-on,
    // preserving pre-PR-9.6 behaviour for callers (incl. tests).
    enabledSourcesBitmask: UInt = 0u,
    // §34/S22 — invert of the 顯示當咧拍的字 setting. Default `false` = show
    // (proto3-absent sentinel → engine prepends the literal-roman
    // candidate, the pre-toggle always-on behaviour for callers/tests).
    literalRomanCandidateDisabled: Boolean = false,
    // 候選詞顯示 — ROMAN_ONLY makes the engine collapse same-roman rows.
    candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
): RustEngineBridge.ContinuousFetchResult {
    val payload = com.siansiansu.taigikeyboard.engine.proto.FetchAtPos
        .newBuilder()
        .setPosition(0)
        .addAllFrequencyEntries(frequencyEntries)
        .setNowMs(nowMs)
        .addAllCustomEntries(customEntries)
        .setEnabledSourcesBitmask(enabledSourcesBitmask.toInt())
        .setLiteralRomanCandidateDisabled(literalRomanCandidateDisabled)
        .build()
    return composingFetchDispatch(
        methodSetter = { it.fetchAtPos = payload },
        op = "composingFetchAtPos",
        generation = generation,
        config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts, candidateDisplayMode),
    )
}

/**
 * Commit a candidate segment in `Phase::Continuous`. `displayText` /
 * `consumedBytes` / `syllableCount` MUST come from a
 * [RustEngineBridge.ContinuousCandidate] returned by an immediately
 * preceding [composingFetchAtPos] call — sending mismatched values
 * mis-aligns the committed segment. `consumedBytes >= pending.utf8.size`
 * triggers a final commit (exit to Idle). Programmer-error inputs collapse
 * to noop on the engine side.
 *
 * v3.5.8 §10.2 platform pass: the repro path. Mid-commit renders
 * `combined_display(nailed, pending, config)`; final-commit renders
 * `nailed_prefix(nailed, config)` — both need the spacing flags so
 * segments join with the right (roman: space / hanji-first: none /
 * both-scripts: space) word boundary. Defaults = v3.5.7 roman-first;
 * production callers pass explicit live values.
 */
fun RustEngineBridge.composingCommitContinuous(
    displayText: String,
    canonicalText: String,
    associationTl: String,
    consumedBytes: Int,
    syllableCount: Int,
    mode: NormalizeMode,
    toggles: ToneTogglesCarrier,
    generation: Long,
    effectiveSwapped: Boolean = false,
    outputBothScripts: Boolean = false,
    candidateDisplayMode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
): RustEngineBridge.ComposingTransition {
    val payload = com.siansiansu.taigikeyboard.engine.proto.CommitContinuous
        .newBuilder()
        .setDisplayText(displayText)
        .setCanonicalText(canonicalText)
        // R2: canonical TL → NextWord next_tl/prev_tl. Empty → engine
        // falls back to the raw committed slice.
        .setAssociationTl(associationTl)
        .setConsumedBytes(consumedBytes)
        .setSyllableCount(syllableCount)
        .build()
    return composingDispatch(
        methodSetter = { it.commitContinuous = payload },
        op = "composingCommitContinuous",
        generation = generation,
        config = RustEngineBridge.continuousAppConfig(mode, toggles, effectiveSwapped, outputBothScripts, candidateDisplayMode),
    )
}

/**
 * Abort continuous-input. Drops `Phase::Continuous` committed list +
 * pending raw, exits to Idle, emits the standard abort effect trio
 * (`ClearPreeditWithoutCommit` + `ResetAutocomplete` +
 * `NextWordClearForNewComposing`). Committed segments stay in the
 * document — earlier `CommitTextReplacingPreedit` effects already wrote
 * them.
 */
fun RustEngineBridge.composingResetContinuous(generation: Long): RustEngineBridge.ComposingTransition {
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
 * Phase 6 FetchAtPos dispatcher. Synthesizes both the standard
 * [RustEngineBridge.ComposingTransition] (for engine snapshot mirroring)
 * and the [RustEngineBridge.ContinuousFetchResult.candidates] tri-state
 * read off `ComposingResponse.continuous`.
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
                canonicalTl = msg.canonicalTl,
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

// 中文: Android composing 平台殼 — 將 Rust composing engine(engine/composing crate)
// 中文: 的 Intent → Effect 串到 InputConnection。狀態實質存在 Rust singleton EngineHandle,
// 中文: 此層只 cache 最新 raw/display/isComposing 給既有呼叫者讀取,不重建狀態機。
// 中文: bumpGeneration 在 onStartInputView(restarting=false) 觸發,讓 Rust 偵測 input-context 變動。

package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.InputConnection
import com.siansiansu.taigikeyboard.engine.NormalizeMode
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.ToneTogglesCarrier
import com.siansiansu.taigikeyboard.engine.proto.FrequencyEntry
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import java.util.concurrent.atomic.AtomicLong

/**
 * Android platform wrapper around the Rust shared-core composing engine
 * (`engine/composing` crate, accessed via `RustEngineBridge.composing*`).
 *
 * Engine state (phase + raw input + selectedCandidateIndex) lives inside
 * the Rust singleton EngineHandle; this wrapper:
 * - mirrors the latest response into local fields so existing callers
 *   (TextInputManager, CandidateUpdateCoordinator, SmartbarManager,
 *   CandidateClickHandler) do not need re-shape,
 * - dispatches the bridge-emitted `Effect[]` through [ComposingDelegate]
 *   in proto-list order against the live [InputConnection],
 * - encodes the documented Android divergence for the 1-char delete path
 *   (plan §5b.1): query state first, route through `composingReset` when
 *   the buffer holds exactly one character so `deleteSurroundingText` does
 *   NOT remove a pre-existing document char.
 *
 * Lifecycle: [bumpGeneration] is called from
 * `TextInputManager.onStartInputView(restarting=false)` (commit 11) when a
 * real input-context change occurs. Engine compares incoming generation to
 * its last-seen and silently drops state on mismatch (no effects emitted
 * from the drop itself; the request's own effects then apply against
 * fresh state).
 */
class ComposingManager(
    private val settingsProvider: EngineSettingsProvider,
    private val delegate: ComposingDelegate = DefaultComposingDelegate,
    private val nextWordRouter: NextWordEffectRouter = NoopNextWordEffectRouter,
    private val logger: LoggerBackend = NullLoggerBackend,
    /**
     * User-frequency snapshot source for the Continuous-input two-phase
     * fetch. `null` keeps unit-test + Preview construction compiling
     * unchanged (the orchestrator falls through to the neutral phase-1
     * list, exactly mirroring the cold-start branch). The runtime call
     * site (`TextInputManager` keyboard-mode swap) always passes
     * `CompositionRoot.userFreq`. Mirrors iOS `ComposingManager.swift`
     * `userFrequencyService` default-arg shape.
     */
    private val userFrequencyService: UserFrequencyService? = null,
) {
    @Volatile
    private var cachedRawInput: String = ""

    @Volatile
    private var cachedDisplayText: String = ""

    @Volatile
    private var cachedIsComposing: Boolean = false

    @Volatile
    private var cachedSelectedCandidateIndex: Int = -1

    /**
     * Generation source. Lives on the companion so it survives
     * `ComposingManager` reconstruction across `onStartInputView` calls.
     * Engine-side `last_generation` is also process-singleton; both must
     * be monotonic in the same address space so the generation-mismatch
     * silent-drop semantics actually fire on real input-context changes.
     */
    private val currentGeneration: Long
        get() = sharedGeneration.get()

    /**
     * `true` while the manager is dispatching effects from a self-driven
     * commit. Suppresses redundant generation bumps from `textWillChange`
     * / `onUpdateSelection` firing on candidate taps / self-commits.
     */
    @Volatile
    var selfCommitInProgress: Boolean = false
        internal set

    val selectedCandidateIndex: Int
        get() = cachedSelectedCandidateIndex

    fun isComposing(): Boolean = cachedIsComposing

    fun getRawInput(): String? = if (cachedIsComposing) cachedRawInput else null

    fun getComposingText(): String? = if (cachedIsComposing) cachedDisplayText.ifEmpty { cachedRawInput } else null

    /**
     * Bump on real input-context change. Engine drops state silently on the
     * next request. Wired by [com.siansiansu.taigikeyboard.ime.text.TextInputManager]
     * in commit 11.
     *
     * Process-singleton via companion `AtomicLong` so it survives
     * `ComposingManager` reconstruction.
     */
    fun bumpGeneration() {
        sharedGeneration.incrementAndGet()
    }

    companion object {
        private const val TAG = "ComposingManager"

        // Starts at 1; first bump → 2. Engine-side `last_generation`
        // initializes to 0 so the very first request is already a
        // mismatch (silent reset of fresh engine = no-op).
        private val sharedGeneration: AtomicLong = AtomicLong(1L)
    }

    // region Intent dispatch API

    fun startComposing(
        char: String,
        ic: InputConnection,
    ) {
        logger.tdebug(TAG) { "[COMPOSE] fn=startComposing char='$char'" }
        val settings = settingsProvider.current
        val mode = resolveMode(settings.inputMode)
        // Snapshot generation BEFORE the dispatch so the tail-call promote
        // shares the same value. `applyTransition` may synchronously re-enter
        // via `onUpdateSelection` → `bumpGeneration` on hosts that fire
        // selection callbacks inside `commitText` / `setComposingText`;
        // re-reading `currentGeneration` would let EnterContinuous silently
        // reset newer composing state.
        val generation = currentGeneration
        if (cachedIsComposing) {
            // Mid-composition restart: clear-without-commit before starting fresh.
            applyAsSelfCommit(
                RustEngineBridge.composingReset(generation),
                ic,
            )
        }
        applyTransition(
            RustEngineBridge.composingStart(
                char,
                mode,
                carrier(settings.toneToggles),
                generation,
            ),
            ic,
        )
        promoteToContinuousIfEligible(settings, ic, generation)
    }

    fun appendCharacter(
        char: String,
        ic: InputConnection,
    ) {
        logger.tdebug(TAG) { "[COMPOSE] fn=appendCharacter char='$char'" }
        val settings = settingsProvider.current
        val generation = currentGeneration
        applyTransition(
            RustEngineBridge.composingAppend(
                char,
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                generation,
            ),
            ic,
        )
        promoteToContinuousIfEligible(settings, ic, generation)
    }

    fun appendHyphen(ic: InputConnection) {
        logger.tdebug(TAG) { "[COMPOSE] fn=appendHyphen" }
        val settings = settingsProvider.current
        val generation = currentGeneration
        applyTransition(
            RustEngineBridge.composingAppendHyphen(
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                generation,
            ),
            ic,
        )
        promoteToContinuousIfEligible(settings, ic, generation)
    }

    fun replaceLastCharacter(
        replacement: String,
        ic: InputConnection,
    ) {
        logger.tdebug(TAG) { "[COMPOSE] fn=replaceLastCharacter replacement='$replacement'" }
        val settings = settingsProvider.current
        val generation = currentGeneration
        applyTransition(
            RustEngineBridge.composingReplaceLast(
                replacement,
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                generation,
            ),
            ic,
        )
        promoteToContinuousIfEligible(settings, ic, generation)
    }

    /**
     * Delete one grapheme. Returns `true` if the wrapper consumed the key.
     *
     * Android divergence (plan §5b.1): the 1-char-empty-after-delete path
     * routes through [RustEngineBridge.composingReset] rather than
     * [RustEngineBridge.composingDeleteBackward]. Reason: Android's
     * in-document composing region is removed by `ClearPreeditWithoutCommit`
     * already; an additional `DeleteBackwardFromDocument` would delete a
     * pre-existing document char. iOS's floating marked-text model has the
     * opposite need.
     *
     * Reads the buffer length from the local cache (kept in sync via
     * [applyTransition] on every prior dispatch) — saves one FFI round-trip
     * per backspace vs. issuing `composingQueryState` first.
     */
    fun deleteBackward(ic: InputConnection): Boolean {
        logger.tdebug(TAG) { "[COMPOSE] fn=deleteBackward" }
        if (!cachedIsComposing || cachedRawInput.isEmpty()) return false
        val transition = if (cachedRawInput.length == 1) {
            RustEngineBridge.composingReset(currentGeneration)
        } else {
            val settings = settingsProvider.current
            RustEngineBridge.composingDeleteBackward(
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
            )
        }
        applyTransition(transition, ic)
        return true
    }

    fun commitComposition(ic: InputConnection) {
        logger.tdebug(TAG) { "[COMPOSE] fn=commitComposition" }
        if (!cachedIsComposing) return
        // `Intent::CommitDerived` is a no-op in `Phase::Continuous`
        // (engine/composing/tests/continuous_phase.rs:656), and the auto-
        // promote tail puts every active composition into Continuous. Route
        // through SelectSuggestion which commits across all three phases.
        // Empty preedit → canonical CommitDerived (no-op on Idle).
        val derived = getComposingText().orEmpty()
        if (derived.isEmpty()) {
            val settings = settingsProvider.current
            applyAsSelfCommit(
                RustEngineBridge.composingCommitDerived(
                    resolveMode(settings.inputMode),
                    carrier(settings.toneToggles),
                    currentGeneration,
                ),
                ic,
            )
            return
        }
        applyAsSelfCommit(
            RustEngineBridge.composingSelectSuggestion(derived, currentGeneration),
            ic,
        )
    }

    fun commitRawInput(ic: InputConnection) {
        logger.tdebug(TAG) { "[COMPOSE] fn=commitRawInput" }
        // v3.5.8 Phase 9 Item 3 (2026-05-13): engine handles `Phase::Continuous`
        // CommitRaw natively now — commits `derived_display(pending, config)`
        // and fires NextWordWordSelected (matches commit_continuous final-
        // commit shape). The Phase 7B SelectSuggestion bypass is gone; the
        // engine owns per-phase routing. See
        // engine/composing/tests/continuous_phase.rs::commit_raw_under_continuous_*.
        // 中文: Phase 9 Item 3 — engine 在 Continuous 下走 derived_display + NextWord;
        // 中文: 平台不再 SelectSuggestion 繞路,直接送 CommitRaw 由引擎依 phase 決定行為。
        val settings = settingsProvider.current
        applyAsSelfCommit(
            RustEngineBridge.composingCommitRaw(
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
            ),
            ic,
        )
    }

    fun selectSuggestion(
        suggestion: String,
        ic: InputConnection,
    ) {
        logger.tdebug(TAG) { "[COMPOSE] fn=selectSuggestion len=${suggestion.length}" }
        applyAsSelfCommit(
            RustEngineBridge.composingSelectSuggestion(suggestion, currentGeneration),
            ic,
        )
    }

    fun commitPreeditThenInsertExternal(
        text: String,
        ic: InputConnection,
    ) {
        logger.tdebug(TAG) { "[COMPOSE] fn=commitPreeditThenInsertExternal len=${text.length}" }
        val settings = settingsProvider.current
        applyAsSelfCommit(
            RustEngineBridge.composingCommitPreeditThenInsertExternal(
                text,
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
            ),
            ic,
        )
    }

    fun reset(ic: InputConnection) {
        logger.tdebug(TAG) { "[COMPOSE] fn=reset" }
        applyAsSelfCommit(
            RustEngineBridge.composingReset(currentGeneration),
            ic,
        )
    }

    // region Continuous-input platform integration (v3.5.8)

    /**
     * Synchronous tail-call promote into `Phase::Continuous` after every raw-
     * input mutation. Caller (Start / Append / AppendHyphen / ReplaceLast)
     * passes the same `generation` snapshot it captured before its own
     * dispatch — re-reading [currentGeneration] here is unsafe because
     * synchronous `onUpdateSelection` callbacks during the preceding
     * `applyTransition` can bump it.
     *
     * Engine no-ops on empty buffer / already-Continuous; the local
     * `cachedRawInput.isEmpty()` short-circuit saves the FFI roundtrip in
     * the empty-buffer case (cache is set by the immediately preceding
     * same-thread `applyTransition` so it is fresh).
     */
    private fun promoteToContinuousIfEligible(
        settings: com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings,
        ic: InputConnection,
        generation: Long,
    ) {
        if (cachedRawInput.isEmpty()) return
        val transition = RustEngineBridge.composingEnterContinuous(
            resolveMode(settings.inputMode),
            carrier(settings.toneToggles),
            generation,
        )
        // EnterContinuous emits zero effects; applyTransition still runs to
        // refresh the local mirror with the engine snapshot.
        applyTransition(transition, ic)
    }

    /**
     * Span-local candidate query for the current `Phase::Continuous { raw }`.
     * Read-only; engine returns the candidate set in score-desc order.
     * Returns `emptyList()` when not in Continuous phase, when no syllable
     * inventory is installed, or when the FST returns no hits — the caller
     * cannot distinguish these cases. Graceful degrade is OK because the
     * lexicon path handles the same input via its own search.
     *
     * Two-phase fetch closes Gap B (`docs/engine/
     * continuous-input-ranking.md` §3.2) by feeding the engine's
     * `user_freq_boost` + `SortKey.recency_rank` axes:
     * 1. Neutral fetch (empty `frequency_entries`, `now_ms = 0`) discovers
     *    candidate `displayText` keys — Android cannot know them up-front.
     * 2. Batch query `user_frequency.db WHERE word IN (...)` for those keys.
     * 3. Populated fetch on the same `currentGeneration` snapshot re-ranks
     *    the candidate set with `user_freq_boost(count)` saturated at
     *    `MAX_BOOST = 5.0` per `engine/ranking/src/score.rs`.
     *
     * `currentGeneration` is captured once so a `bumpGeneration()` between
     * the two FFI calls cannot corrupt the populated fetch — engine resets
     * to Idle on generation mismatch (`engine/composing/src/handle.rs:61-66`)
     * and we surface that as the documented "no candidates this frame"
     * degrade rather than an inconsistent boost. The phase-2 `transition`
     * already reflects the Idle reset; returning the phase-1 list would
     * render stale candidates against the new context, so we return `[]`
     * instead. Mirrors iOS PR #265 Codex Q5 / R2.
     *
     * Bridge-failure handling distinguishes "engine returned Idle" (legit
     * reset; apply Idle transition + return `[]`) from "FFI roundtrip
     * failed" (transient encode/decode/non-OK; engine state unchanged —
     * apply phase-1 transition + return phase-1 candidates). Without the
     * `isBridgeFailure` flag both scenarios collapse to a `NOOP` transition
     * + `null` candidates, and applying `NOOP` clobbers the mirror with
     * false Idle state. Phase-1 FFI failure short-circuits the whole
     * frame; phase-2 FFI failure degrades to neutral-ranked phase-1
     * results. Mirrors iOS PR #265 r3216857164.
     *
     * Cold-start: when `user_frequency.db` has not yet been opened (the
     * race window between `TaigiKeyboardApplication.onCreate`'s best-effort
     * `ensureInitialized` launch and that Task completing), skip phase 2
     * and return the neutral list — engine produced neutral-boost ranking
     * on the phase-1 response. Also covers
     * `userFrequencyService == null` (tests / Preview construction).
     *
     * CROSS-PLATFORM INVARIANT — mirrors
     * `ios/Sources/TaigiKeyboard/Input/Composing/ComposingManager.swift:232`.
     * Drift causes silent divergence in the ranking the user sees after
     * their first selection of a phrase.
     *
     * Android divergence (intentional, per `rules/cross-platform-alignment.md`
     * §3): iOS is sync because Swift `frequencyDataBatch` is sync; Android
     * is `suspend` because Kotlin `frequencyDataBatch` owns `Dispatchers.IO`
     * internally (`UserFrequencyService.kt:225`). Same observable behaviour,
     * different threading model.
     *
     * Caller invariant: enters on the IME main thread (the autocomplete
     * service wraps the call in `withContext(Dispatchers.Main)`); the
     * SQLite hop happens inside `UserFrequencyService.frequencyDataBatch`'s
     * own `withContext(Dispatchers.IO)`, after which the suspension resumes
     * back on Main for the second `applyTransition` — `InputConnection`
     * writes are Main-only.
     */
    // 中文: 連續輸入候選查詢 — suspend two-phase fetch:
    // 中文: 中性查 → SQLite 查 user-freq → 帶 freq 重查 + 重排。
    // 中文: generation 一次取樣,中途 bump 會讓 phase 2 回空,等同無 candidate 這 frame。
    // 中文: isBridgeFailure 分辨「引擎回 Idle」與「FFI 失敗」— 後者不可套 transition。
    suspend fun fetchContinuousCandidates(ic: InputConnection): List<RustEngineBridge.ContinuousCandidate> {
        val settings = settingsProvider.current
        val mode = resolveMode(settings.inputMode)
        val toggles = carrier(settings.toneToggles)
        val generation = currentGeneration

        // Phase 1: neutral fetch to learn candidate displayText keys.
        val neutral = RustEngineBridge.composingFetchAtPos(
            mode = mode,
            toggles = toggles,
            generation = generation,
        )
        // Phase-1 FFI failure: do NOT apply the synthesized `NOOP` — that
        // would clobber the mirror with false Idle state. Surface as "no
        // candidates this frame"; the mirror keeps reflecting the most
        // recent successful transition (typically the keystroke's
        // append/promote that brought us into Continuous), so the next
        // keystroke's fetch finds the right engine state. Mirrors iOS
        // PR #265 r3216857164 pre-impl S5 + post-impl T2.
        if (neutral.isBridgeFailure) {
            return emptyList()
        }
        val neutralCandidates = neutral.candidates
        if (neutralCandidates.isNullOrEmpty()) {
            applyTransition(neutral.transition, ic)
            return emptyList()
        }

        // Cold-start (or no service injected): user_frequency.db not yet
        // open. Skip phase 2 — engine already produced neutral-boost
        // ranking on the phase-1 response.
        val userFreq = userFrequencyService
        if (userFreq == null || !userFreq.isConnected()) {
            applyTransition(neutral.transition, ic)
            return neutralCandidates
        }

        // Phase 2: populated fetch with the user-frequency snapshot.
        // `buildFrequencyEntries` runs the SQL inside the service's own
        // `Dispatchers.IO` block, then resumes back on the caller's Main
        // context before the second FFI call.
        val entries = buildFrequencyEntries(neutralCandidates, userFreq)
        val nowMs = System.currentTimeMillis()
        val boosted = RustEngineBridge.composingFetchAtPos(
            mode = mode,
            toggles = toggles,
            generation = generation,
            frequencyEntries = entries,
            nowMs = nowMs,
        )
        // Phase-2 FFI failure: engine state did NOT change since phase-1
        // (the request never reached the engine). Apply phase-1's transition
        // (the real engine snapshot from the moment phase-1 succeeded) and
        // return phase-1 candidates — degrade to neutral-ranked instead of
        // dropping the frame. Mirrors iOS PR #265 r3216857164.
        if (boosted.isBridgeFailure) {
            applyTransition(neutral.transition, ic)
            return neutralCandidates
        }
        applyTransition(boosted.transition, ic)
        // Engine determinism: same `Phase::Continuous { raw }` returns the
        // same candidate set. A `null` phase-2 carrier with `isBridgeFailure
        // == false` means a `bumpGeneration` raced in between and engine
        // reset to Idle BEFORE this fetch — the applied transition already
        // mirrors that Idle state, so returning phase-1 candidates would
        // render stale suggestions against the new context. Surface as
        // "no candidates this frame" instead. Mirrors iOS PR #265 Codex
        // pre/post-impl Q5/R2.
        return boosted.candidates ?: emptyList()
    }

    /**
     * Marshal the per-candidate `user_frequency.db` snapshot into the proto
     * `FrequencyEntry` list required by `FetchAtPos`. Dedupes by
     * `displayText` (engine's `display_text_key` = `hanji ?? roman`) so a
     * candidate list with the same hanji twice (different roman) issues
     * only one SQL placeholder; the engine's `build_frequency_map` is
     * last-write-wins on duplicates either way (`engine/ranking/src/
     * score.rs::build_frequency_map`). Only entries present in the DB are
     * marshalled — missing rows mean "no user usage yet" and the engine
     * applies `user_freq_boost(0) = 1.0` neutral. Mirrors iOS
     * `ComposingManager.swift:308 buildFrequencyEntries`. Proto marshaling
     * delegates to `RustEngineBridge.frequencyDataToProtoEntries` so the
     * legacy `processCandidates` site and this Continuous-fetch site share
     * a single `count` clamp + field-naming source of truth.
     */
    // 中文: 候選詞 user_frequency.db 快照 → proto FrequencyEntry。
    // 中文: 以 displayText distinct 壓 SQL placeholder;DB 沒有的 row 不送 → 引擎自動 neutral。
    private suspend fun buildFrequencyEntries(
        candidates: List<RustEngineBridge.ContinuousCandidate>,
        userFreq: UserFrequencyService,
    ): List<FrequencyEntry> {
        val uniqueKeys = candidates.map { it.displayText }.distinct()
        val snapshot = userFreq.frequencyDataBatch(uniqueKeys)
        return RustEngineBridge.frequencyDataToProtoEntries(snapshot)
    }

    /**
     * Commit one Continuous candidate. `displayText` / `consumedBytes` /
     * `syllableCount` MUST come verbatim from a [RustEngineBridge.ContinuousCandidate]
     * returned by an immediately preceding [fetchContinuousCandidates] call.
     *
     * Returns an effect-backed [RustEngineBridge.CommitContinuousResult] so
     * callers can gate side-effects (frequency recording, auto-space) on
     * actual commit success rather than coarse `cachedIsComposing` mirror
     * state. Generation mismatch silently resets the engine to Idle in
     * `engine/composing/src/handle.rs:61-65` BEFORE the intent runs, in
     * which case `Intent::CommitContinuous` becomes a phase-mismatch noop —
     * the post-call mirror flips to `cachedIsComposing=false` (engine is
     * Idle) but no `CommitTextReplacingPreedit` Effect is emitted. Without
     * the effect-backed gate, callers would record frequency for uncommitted
     * text and append a stray space.
     *
     * Mid-commit emits `[CommitTextReplacingPreedit, UpdatePreedit,
     * NextWordUpdateLastSelectedWord, PerformAutocomplete]`; final-commit
     * (`consumedBytes >= pending.utf8.size`) emits `[CommitTextReplacingPreedit,
     * ResetAutocomplete, ResetAutocompleteContext, NextWordWordSelected]`
     * and exits Continuous.
     */
    fun commitContinuous(
        displayText: String,
        consumedBytes: Int,
        syllableCount: Int,
        ic: InputConnection,
    ): RustEngineBridge.CommitContinuousResult {
        logger.tdebug(TAG) {
            "[COMPOSE] fn=commitContinuous displayLen=${displayText.length} consumedBytes=$consumedBytes syllCount=$syllableCount"
        }
        val settings = settingsProvider.current
        val transition = RustEngineBridge.composingCommitContinuous(
            displayText = displayText,
            consumedBytes = consumedBytes,
            syllableCount = syllableCount,
            mode = resolveMode(settings.inputMode),
            toggles = carrier(settings.toneToggles),
            generation = currentGeneration,
        )
        // Inspect transition BEFORE dispatching effects so we return an
        // effect-backed signal. `applyAsSelfCommit` body inlined (3 lines)
        // for the same reason — semantics identical to the helper.
        val didCommit = transition.effects.any { effect ->
            effect is RustEngineBridge.ComposingTransition.Effect.CommitTextReplacingPreedit
        }
        val didFinalCommit = didCommit && !transition.isComposing
        selfCommitInProgress = true
        try {
            applyTransition(transition, ic)
        } finally {
            selfCommitInProgress = false
        }
        return RustEngineBridge.CommitContinuousResult(
            didCommit = didCommit,
            didFinalCommit = didFinalCommit,
        )
    }

    /**
     * Abort Continuous-input. Drops `Phase::Continuous`'s pending + committed
     * list, exits to Idle, emits the standard abort effect trio
     * (`ClearPreeditWithoutCommit` + `ResetAutocomplete` +
     * `NextWordClearForNewComposing`). Committed segments stay in the
     * document — earlier `CommitTextReplacingPreedit` effects already wrote
     * them. Used by `TextInputManager.onInputModeChanged` so stale Continuous
     * state can't leak across TL ↔ POJ ↔ TPS swaps.
     */
    fun resetContinuous(ic: InputConnection) {
        logger.tdebug(TAG) { "[COMPOSE] fn=resetContinuous" }
        applyAsSelfCommit(
            RustEngineBridge.composingResetContinuous(currentGeneration),
            ic,
        )
    }

    // endregion

    /**
     * Sync internal cache after the host editor reports no composing region
     * (cursor move via tap, selection change). Bumps the generation so the
     * next intent dispatch causes the engine to silently drop its state.
     * Local cache is cleared immediately so `getComposingText()` returns
     * null right away.
     *
     * Self-commit suppression: if the manager is mid-self-commit, skip —
     * the region clear is the IME's own write, not an external user action.
     */
    fun onExternalComposingRegionCleared() {
        if (selfCommitInProgress) return
        if (!cachedIsComposing) return
        cachedRawInput = ""
        cachedDisplayText = ""
        cachedIsComposing = false
        cachedSelectedCandidateIndex = -1
        bumpGeneration()
    }

    // endregion

    private fun applyAsSelfCommit(
        transition: RustEngineBridge.ComposingTransition,
        ic: InputConnection,
    ) {
        selfCommitInProgress = true
        try {
            applyTransition(transition, ic)
        } finally {
            selfCommitInProgress = false
        }
    }

    private fun applyTransition(
        transition: RustEngineBridge.ComposingTransition,
        ic: InputConnection,
    ) {
        cachedRawInput = transition.rawInput
        cachedDisplayText = transition.displayText
        cachedIsComposing = transition.isComposing
        cachedSelectedCandidateIndex = transition.selectedCandidateIndex
        for (effect in transition.effects) {
            logger.tdebug("ComposingDelegate") {
                "[COMMIT] fn=applyTransition effect=${effect.describeKind()}"
            }
            when (effect) {
                is RustEngineBridge.ComposingTransition.Effect.NextWordUpdateLastSelectedWord,
                is RustEngineBridge.ComposingTransition.Effect.NextWordWordSelected,
                RustEngineBridge.ComposingTransition.Effect.NextWordClearForNewComposing,
                -> {
                    // NextWord-shaped composing effects flow through a sibling
                    // router, not the InputConnection-bound delegate. Engine
                    // emits these only on Continuous mid/final commits + resets;
                    // the platform NextWord callback path on SelectSuggestion
                    // (CandidateClickHandler.onNextWordPrediction) and this
                    // engine effect path do not double-fire.
                    nextWordRouter.route(effect)
                }

                else -> {
                    delegate.execute(effect, ic)
                }
            }
        }
    }

    private fun resolveMode(raw: String): NormalizeMode =
        when (raw) {
            "poj" -> NormalizeMode.POJ
            "english" -> NormalizeMode.ENGLISH
            else -> NormalizeMode.TL
        }

    private fun carrier(toggles: com.siansiansu.taigikeyboard.ime.core.settings.ToneToggles): ToneTogglesCarrier =
        ToneTogglesCarrier(
            isDoubleTapOoEnabled = toggles.isDoubleTapOOEnabled,
            isDoubleTapNnEnabled = toggles.isDoubleTapNNEnabled,
        )
}

/**
 * One-line debug name for a [RustEngineBridge.ComposingTransition.Effect].
 * Centralises the per-variant pretty-printer used by [ComposingManager.applyTransition]
 * and [DefaultComposingDelegate]; keeping it here means new variants only need
 * one update site for log readability.
 */
private fun RustEngineBridge.ComposingTransition.Effect.describeKind(): String =
    when (this) {
        is RustEngineBridge.ComposingTransition.Effect.UpdatePreedit -> "UpdatePreedit len=${display.length}"
        RustEngineBridge.ComposingTransition.Effect.ClearPreeditWithoutCommit -> "ClearPreeditWithoutCommit"
        is RustEngineBridge.ComposingTransition.Effect.CommitTextReplacingPreedit -> "CommitTextReplacingPreedit len=${text.length}"
        RustEngineBridge.ComposingTransition.Effect.DeleteBackwardFromDocument -> "DeleteBackwardFromDocument"
        RustEngineBridge.ComposingTransition.Effect.ResetAutocomplete -> "ResetAutocomplete"
        RustEngineBridge.ComposingTransition.Effect.PerformAutocomplete -> "PerformAutocomplete"
        RustEngineBridge.ComposingTransition.Effect.ResetAutocompleteContext -> "ResetAutocompleteContext"
        is RustEngineBridge.ComposingTransition.Effect.NextWordUpdateLastSelectedWord -> "NextWordUpdateLastSelectedWord"
        is RustEngineBridge.ComposingTransition.Effect.NextWordWordSelected -> "NextWordWordSelected trigger=$triggerPrediction"
        RustEngineBridge.ComposingTransition.Effect.NextWordClearForNewComposing -> "NextWordClearForNewComposing"
    }

/**
 * Policy helper: does an `onUpdateSelection` payload indicate the host
 * editor no longer reports a composing region? Both coordinates are `-1`
 * when no region exists.
 */
internal fun hostReportsNoComposingRegion(
    candidatesStart: Int,
    candidatesEnd: Int,
): Boolean = candidatesStart == -1 && candidatesEnd == -1

/**
 * Clear the host editor's composing region at the [InputConnection] layer
 * without committing whatever text it contains. Issues
 * `setComposingText("", 1)` then `finishComposingText()` — pre-zero is
 * mandatory because `finishComposingText()` on its own silently commits.
 *
 * Used by bare-IC sites that do NOT route through [ComposingManager.reset]
 * (e.g. `TextInputManager.resetComposingText` when composingManager is
 * null or the keyboard mode bypasses composing).
 */
internal fun clearHostComposingRegion(ic: InputConnection?) {
    ic?.setComposingText("", 1)
    ic?.finishComposingText()
}

/**
 * Helper used during `dispatch` and `commitComposition` paths to surface a
 * caller-supplied raw snapshot's display form via the Rust engine. Callers
 * that already issued `composingQueryState` get the display text from the
 * response; this helper exists for off-path consumers (e.g. async refresh).
 */
internal fun deriveDisplay(
    raw: String,
    settingsProvider: EngineSettingsProvider,
): String {
    if (raw.isEmpty()) return ""
    if (RustEngineBridge.containsTps(raw)) return raw
    val settings = settingsProvider.current
    val mode = when (settings.inputMode) {
        "poj" -> NormalizeMode.POJ
        "english" -> NormalizeMode.ENGLISH
        else -> NormalizeMode.TL
    }
    val carrier = ToneTogglesCarrier(
        isDoubleTapOoEnabled = settings.toneToggles.isDoubleTapOOEnabled,
        isDoubleTapNnEnabled = settings.toneToggles.isDoubleTapNNEnabled,
    )
    return RustEngineBridge.normalizeTone(raw, mode, carrier)
}

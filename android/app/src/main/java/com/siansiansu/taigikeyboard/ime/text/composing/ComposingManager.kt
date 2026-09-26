// Android platform shell for the Rust composing engine (engine/composing crate): pipes Intent → Effect into
// InputConnection. State lives in the Rust singleton EngineHandle; this layer only mirrors the latest
// raw/display/isComposing into 3 StateFlows for sync callers (.value) and Compose
// observers, without rebuilding the state machine. bumpGeneration fires from
// onStartInputView on a new editor so Rust can detect an input-context change; a same-editor restart
// keeps the manager and reconciles it against the host (reconcileWithHost).

package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.ExtractedTextRequest
import android.view.inputmethod.InputConnection
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.composingAppend
import com.siansiansu.taigikeyboard.engine.composingAppendHyphen
import com.siansiansu.taigikeyboard.engine.composingCommitContinuous
import com.siansiansu.taigikeyboard.engine.composingCommitDerived
import com.siansiansu.taigikeyboard.engine.composingCommitPreeditThenInsertExternal
import com.siansiansu.taigikeyboard.engine.composingCommitRaw
import com.siansiansu.taigikeyboard.engine.composingDeleteBackward
import com.siansiansu.taigikeyboard.engine.composingEnterContinuous
import com.siansiansu.taigikeyboard.engine.composingFetchAtPos
import com.siansiansu.taigikeyboard.engine.composingReplaceLast
import com.siansiansu.taigikeyboard.engine.composingReset
import com.siansiansu.taigikeyboard.engine.composingResetContinuous
import com.siansiansu.taigikeyboard.engine.composingSelectSuggestion
import com.siansiansu.taigikeyboard.engine.composingStart
import com.siansiansu.taigikeyboard.engine.dictionaryFilters
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.util.concurrent.atomic.AtomicLong

/**
 * Android platform wrapper around the Rust shared-core composing engine
 * (`engine/composing` crate, accessed via `RustEngineBridge.composing*`).
 *
 * Engine state (phase + raw input) lives inside
 * the Rust singleton EngineHandle; this wrapper:
 * - mirrors the latest response into per-field [StateFlow]s
 *   ([rawInput] / [displayText] / [isComposing])
 *   so existing synchronous callers (TextInputManager,
 *   CandidateUpdateCoordinator, SmartbarManager, CandidateClickHandler)
 *   keep their `.value`-equivalent read shape while future Compose
 *   observers can `collectAsStateWithLifecycle()` on the same flows,
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
) {
    // Engine-mirror state: 3 MutableStateFlow, public read-only StateFlow surface; sync getters read `.value`.
    // CROSS-PLATFORM PAIR — mirrors iOS `ComposingManager.swift` @Observable mirror.
    private val _rawInput = MutableStateFlow("")
    val rawInput: StateFlow<String> = _rawInput.asStateFlow()

    private val _displayText = MutableStateFlow("")
    val displayText: StateFlow<String> = _displayText.asStateFlow()

    private val _isComposing = MutableStateFlow(false)
    val isComposing: StateFlow<Boolean> = _isComposing.asStateFlow()

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
     * Value snapshot of the composing state a worker-side candidate fetch is
     * computed against: the raw buffer (`null` when not composing) and the
     * input-context generation. Two tokens compare equal iff a result fetched
     * under one is still valid to publish under the other.
     */
    data class StateToken(
        val rawInput: String?,
        val generation: Long,
    )

    fun stateToken(): StateToken = StateToken(getRawInput(), currentGeneration)

    /**
     * `true` while the manager is dispatching effects from a self-driven
     * commit. Suppresses redundant generation bumps from `textWillChange`
     * / `onUpdateSelection` firing on candidate taps / self-commits.
     * Platform suppression flag — not view state — so kept as `@Volatile`
     * rather than wrapped in StateFlow.
     */
    @Volatile
    var selfCommitInProgress: Boolean = false
        internal set

    fun isComposing(): Boolean = _isComposing.value

    fun getRawInput(): String? = if (_isComposing.value) _rawInput.value else null

    fun getComposingText(): String? = if (_isComposing.value) _displayText.value.ifEmpty { _rawInput.value } else null

    /**
     * Bump on real input-context change. Engine drops state silently on the
     * next request, so the mirror is zeroed here too — a mirror still
     * claiming "composing" after a bump would let [reconcileWithHost] keep a
     * composition the engine has already discarded (Codex post-impl,
     * 2026-09-12). Wired by
     * [com.siansiansu.taigikeyboard.ime.text.TextInputManager] in commit 11.
     *
     * Process-singleton via companion `AtomicLong` so it survives
     * `ComposingManager` reconstruction.
     */
    fun bumpGeneration() {
        _rawInput.value = ""
        _displayText.value = ""
        _isComposing.value = false
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
        // Snapshot generation BEFORE the dispatch so the tail-call promote
        // shares the same value. `applyTransition` may synchronously re-enter
        // via `onUpdateSelection` → `bumpGeneration` on hosts that fire
        // selection callbacks inside `commitText` / `setComposingText`;
        // re-reading `currentGeneration` would let EnterContinuous silently
        // reset newer composing state.
        val generation = currentGeneration
        if (_isComposing.value) {
            // Mid-composition restart: clear-without-commit before starting fresh.
            applyAsSelfCommit(
                RustEngineBridge.composingReset(generation),
                ic,
            )
        }
        applyTransition(
            RustEngineBridge.composingStart(
                char,
                settings,
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
                settings,
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
                settings,
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
                settings,
                generation,
            ),
            ic,
        )
        promoteToContinuousIfEligible(settings, ic, generation)
    }

    /**
     * Delete one grapheme. Returns `true` if the wrapper consumed the key.
     *
     * **Model B** (v3.5.8 Phase 9, Codex pre-impl point 5): route ALL
     * composing backspace to the engine. The old Android divergence —
     * routing the 1-char / empty-raw case through
     * [RustEngineBridge.composingReset] to stop a stray
     * `DeleteBackwardFromDocument` deleting a pre-existing document char —
     * no longer applies: under Model B `delete_backward_continuous` never
     * emits `DeleteBackwardFromDocument` (nailed segments are not in the
     * document), and `Continuous { raw: "", nailed: [...] }` is a valid
     * live state where the next backspace must **unnail** the last segment.
     * The old `_rawInput.value.isEmpty()` early-return wrongly let that key
     * fall through to the host (deleting a real document char); the
     * `length == 1` reset shortcut would wrongly abort the whole
     * composition when a nailed prefix exists. Bare `Phase::Composing`
     * reaching here would violate the Phase 7B invariant; we deliberately
     * do not branch on phase (no safe phase signal in the mirror).
     *
     * The engine owns every sub-case (pending shrink / unnail / exit).
     */
    fun deleteBackward(ic: InputConnection): Boolean {
        logger.tdebug(TAG) { "[COMPOSE] fn=deleteBackward" }
        if (!_isComposing.value) return false
        val settings = settingsProvider.current
        applyTransition(
            RustEngineBridge.composingDeleteBackward(
                settings,
                currentGeneration,
            ),
            ic,
        )
        return true
    }

    fun commitComposition(ic: InputConnection) {
        logger.tdebug(TAG) { "[COMPOSE] fn=commitComposition" }
        if (!_isComposing.value) return
        // Model B (§10.3 + v3.5.8 Phase 9 Finding 2): finalize the WHOLE
        // current composition via CommitRaw — under Continuous the engine
        // commits `Σ nailed.display_text + derived(pending)` (the whole
        // composition) and fires the terminal NextWord, building the string
        // from engine state so there is no prefix duplication. The old
        // `SelectSuggestion(getComposingText())` reroute double-counted the
        // nailed prefix once the composing buffer became the whole
        // composition (`select_suggestion_under_continuous` prepends
        // `nailed_prefix`). `selectSuggestion(candidate)` still uses
        // SelectSuggestion (bare candidate → engine prepends correctly).
        // Empty preedit → CommitDerived (a no-op on Idle). Bare
        // `Phase::Composing` reaching here would violate the Phase 7B
        // invariant; we deliberately do not branch on phase (Codex pre-impl
        // point 3).
        val settings = settingsProvider.current
        if (getComposingText().orEmpty().isEmpty()) {
            applyAsSelfCommit(
                RustEngineBridge.composingCommitDerived(
                    settings,
                    currentGeneration,
                ),
                ic,
            )
            return
        }
        applyAsSelfCommit(
            RustEngineBridge.composingCommitRaw(
                settings,
                currentGeneration,
            ),
            ic,
        )
    }

    fun commitRawInput(ic: InputConnection) {
        logger.tdebug(TAG) { "[COMPOSE] fn=commitRawInput" }
        // v3.5.8 Phase 9 Item 3 + Model B (§10.3): engine handles
        // `Phase::Continuous` CommitRaw natively — under Model B it commits
        // the WHOLE composition (`Σ nailed.display_text + derived(pending)`)
        // and fires the terminal NextWordWordSelected (matches
        // commit_continuous final-commit shape). The Phase 7B
        // SelectSuggestion bypass is gone; the engine owns per-phase
        // routing. See
        // engine/composing/tests/continuous_phase.rs::commit_raw_under_continuous_*.
        val settings = settingsProvider.current
        applyAsSelfCommit(
            RustEngineBridge.composingCommitRaw(
                settings,
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
        val settings = settingsProvider.current
        applyAsSelfCommit(
            RustEngineBridge.composingSelectSuggestion(
                suggestion,
                settings,
                currentGeneration,
            ),
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
                settings,
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
     * `_rawInput.value.isEmpty()` short-circuit saves the FFI roundtrip in
     * the empty-buffer case (StateFlow is set by the immediately preceding
     * same-thread `applyTransition` so it is fresh).
     */
    private fun promoteToContinuousIfEligible(
        settings: com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings,
        ic: InputConnection,
        generation: Long,
    ) {
        if (_rawInput.value.isEmpty()) return
        val transition = RustEngineBridge.composingEnterContinuous(
            settings,
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
     * One fetch: the engine reads the user's own data itself — the counts,
     * the custom dictionary (unless the setting turns it off) and the learned
     * phrases — and ranks in the same call
     * (`docs/architecture/user-data-engine-roadmap.md` P8b). `nowMs` is the
     * clock its recency ranking reads. An FFI failure, or an answer for a
     * generation a `bumpGeneration` has since replaced (an Idle snapshot, no
     * carrier), is "no candidates this frame".
     *
     * CROSS-PLATFORM INVARIANT — mirrors macOS
     * `macos/Sources/TaigiInputMethodCore/Composing/ComposingManager.swift`
     * `fetchCandidates`.
     *
     * Android divergence (intentional, per `.claude/rules/cross-platform-alignment.md`
     * §3): the Apple platforms run the fetch on the main thread and mirror
     * its snapshot; Android runs this function on `Dispatchers.Default`
     * (`CandidateUpdateCoordinator`) so the dictionary scan never blocks a
     * keystroke, and mirrors nothing — `FetchAtPos` is read-only in the
     * engine (no effects, no state change), so there is no transition to
     * apply; the coordinator validates [stateToken] before publishing. Same
     * observable behaviour, different threading model.
     *
     * Thread-agnostic: touches only `StateFlow` / atomic state, the volatile
     * prefs cache behind [settingsProvider], and the JNI bridge (the engine
     * serialises internally). Never touches `InputConnection`.
     */
    suspend fun fetchContinuousCandidates(): List<RustEngineBridge.ContinuousCandidate> {
        val settings = settingsProvider.current
        return RustEngineBridge.composingFetchAtPos(
            config = RustEngineBridge.continuousAppConfig(settings),
            generation = currentGeneration,
            nowMs = System.currentTimeMillis(),
            // PR-9.6 — the dictionary source-toggle bitmask the Tab3 browse path
            // sends too (12 source toggles + kautian subcollections).
            enabledSourcesBitmask = RustEngineBridge
                .dictionaryFilters(RustEngineBridge.DictionaryToggles.from(settings))
                .dictionaryFilterBitmask,
            // §34/S22 — invert of the Show Typed Text First setting.
            literalRomanCandidateDisabled = !settings.isLiteralRomanCandidateEnabled,
            customDictionaryDisabled = !settings.isCustomDictEnabled,
        )
    }

    // v3.5.8 Phase 9 Bug 1 (Option A): `displayText` is the swap/TPS/both-
    // scripts-formatted DOCUMENT string (caller mirrors the legacy lexicon
    // formatter); `canonicalText` is the canonical key (`hanji ?? roman`)
    // routed to NextWord so association learning stays mode-independent.

    /**
     * Commit one Continuous candidate. `displayText` / `consumedBytes` /
     * `syllableCount` MUST come verbatim from a [RustEngineBridge.ContinuousCandidate]
     * returned by an immediately preceding [fetchContinuousCandidates] call.
     *
     * Returns an effect-backed [RustEngineBridge.CommitContinuousResult] so
     * callers can gate side-effects (frequency recording, auto-space) on
     * actual commit success rather than coarse `_isComposing.value` mirror
     * state. Generation mismatch silently resets the engine to Idle in
     * `engine/composing/src/handle.rs:61-65` BEFORE the intent runs, in
     * which case `Intent::CommitContinuous` becomes a phase-mismatch noop —
     * the post-call mirror flips to `_isComposing.value=false` (engine is
     * Idle) but no `CommitTextReplacingPreedit` Effect is emitted. Without
     * the effect-backed gate, callers would record frequency for uncommitted
     * text and append a stray space.
     *
     * **Model B** (§10): mid-commit (nail) emits `[UpdatePreedit(whole
     * composition), NextWordUpdateLastSelectedWord, PerformAutocomplete]`
     * and stays Continuous — NO `CommitTextReplacingPreedit`; final-commit
     * (`consumedBytes >= pending.utf8.size`) emits `[CommitTextReplacingPreedit
     * (whole composition), ResetAutocomplete, ResetAutocompleteContext,
     * NextWordWordSelected]` and exits Continuous. `didCommit` therefore
     * keys on `CommitTextReplacingPreedit` (final) OR
     * `NextWordUpdateLastSelectedWord` (the per-segment nail signal); a
     * noop emits neither.
     */
    fun commitContinuous(
        displayText: String,
        canonicalText: String,
        associationTl: String,
        consumedBytes: Int,
        syllableCount: Int,
        ic: InputConnection,
        // §50 — the pick's hanji, null for a hanji-less candidate.
        hanji: String? = null,
    ): RustEngineBridge.CommitContinuousResult {
        logger.tdebug(TAG) {
            "[COMPOSE] fn=commitContinuous displayLen=${displayText.length} canonicalLen=${canonicalText.length} consumedBytes=$consumedBytes syllCount=$syllableCount"
        }
        val settings = settingsProvider.current
        val transition = RustEngineBridge.composingCommitContinuous(
            displayText = displayText,
            canonicalText = canonicalText,
            associationTl = associationTl,
            hanji = hanji,
            consumedBytes = consumedBytes,
            syllableCount = syllableCount,
            settings = settings,
            generation = currentGeneration,
        )
        // Inspect transition BEFORE dispatching effects so we return an
        // effect-backed signal. `applyAsSelfCommit` body inlined (3 lines)
        // for the same reason — semantics identical to the helper.
        val hasCommitText = transition.effects.any { effect ->
            effect is RustEngineBridge.ComposingTransition.Effect.CommitTextReplacingPreedit
        }
        val hasNail = transition.effects.any { effect ->
            effect is RustEngineBridge.ComposingTransition.Effect.NextWordUpdateLastSelectedWord
        }
        // Model B: nail (mid-commit) emits no commit-text; the learning
        // effect is its success signal. Final-commit emits commit-text +
        // exits. noop emits neither → both flags false (closes the
        // generation-mismatch race, unchanged guarantee).
        val didCommit = hasCommitText || hasNail
        val didFinalCommit = hasCommitText && !transition.isComposing
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
     * Abort Continuous-input. Drops `Phase::Continuous`'s pending + nailed
     * list, exits to Idle, emits the standard abort effect trio
     * (`ClearPreeditWithoutCommit` + `ResetAutocomplete` +
     * `NextWordClearForNewComposing`). **Model B** (§10.6): nailed segments
     * were never literal document text — `ClearPreeditWithoutCommit` clears
     * the WHOLE marked composition; abort discards it entirely (no document
     * write, no `DeleteBackwardFromDocument`). Used by
     * `TextInputManager.onInputModeChanged` so stale Continuous state can't
     * leak across TL ↔ POJ ↔ TPS swaps.
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
        if (!_isComposing.value) return
        bumpGeneration()
    }

    /**
     * Host `onUpdateSelection` entry point. `-1/-1` means the host reports no
     * composing region — but that report may predate the IME's own last
     * `setComposingText` (an app focus / draft-restore selection move sent
     * before it processed our write). Treating every such report as current
     * lost the first key after an IME / app switch (USER report 2026-09-12),
     * so the decision is delegated to [reconcileWithHost].
     */
    fun onHostSelectionUpdate(
        candidatesStart: Int,
        candidatesEnd: Int,
        ic: InputConnection?,
    ) {
        if (hostReportsNoComposingRegion(candidatesStart, candidatesEnd)) reconcileWithHost(ic)
    }

    /**
     * Re-reads the host to decide whether a "no composing region" signal is
     * current. One `getExtractedText` snapshot supplies both the text and
     * the cursor — the report's own selection may predate our write, so it
     * is never used as a coordinate. The snapshot is proxied to the editor
     * after our earlier `setComposingText`, so on hosts that honour the
     * read-after-edit contract a stale report sees the applied preedit; a
     * host that answers from a cache (Chromium UI-thread path, older Compose
     * batches) can only be stale as a whole, which lands on the clear
     * branch ([displayText] is what the last `UpdatePreedit` wrote;
     * `_displayText` is assigned before the effects run, so a host callback
     * re-entering from inside `setComposingText` already compares against
     * the new text). Preedit found right before a collapsed cursor → keep
     * state (no generation bump; an in-flight fetch still publishes) and
     * re-assert the composing span over it — the FlorisBoard
     * `setComposingRegion` shape — so a host that really dropped the span
     * composes on again instead of inserting the whole preedit on the next
     * key. Anything else — mismatch, selection, `null` (dead connection or
     * an editor without extraction) — is the pre-existing
     * [onExternalComposingRegionCleared].
     * Remaining heuristic: a cursor at the end of identical text elsewhere
     * is kept too (`behavioral-invariants.md` §13).
     */
    fun reconcileWithHost(ic: InputConnection?) {
        if (selfCommitInProgress || !_isComposing.value) return
        val expected = _displayText.value
        val region = if (expected.isEmpty()) null else hostPreeditRegion(ic, expected)
        if (region == null) {
            logger.tdebug(TAG) { "[COMPOSE] fn=reconcileWithHost cleared expectedLen=${expected.length}" }
            onExternalComposingRegionCleared()
            return
        }
        logger.tdebug(TAG) { "[COMPOSE] fn=reconcileWithHost kept region=$region" }
        ic?.setComposingRegion(region.first, region.second)
    }

    /**
     * Absolute `[start, end)` of [expected] when it sits right before the
     * host's collapsed cursor, from one [InputConnection.getExtractedText]
     * snapshot; `null` when the read fails, the selection is not collapsed,
     * or the text before the cursor differs.
     */
    private fun hostPreeditRegion(
        ic: InputConnection?,
        expected: String,
    ): Pair<Int, Int>? {
        val extracted = ic?.getExtractedText(ExtractedTextRequest(), 0) ?: return null
        val text = extracted.text ?: return null
        if (extracted.selectionStart != extracted.selectionEnd) return null
        val localEnd = extracted.selectionEnd
        val localStart = localEnd - expected.length
        if (localStart < 0 || localEnd > text.length) return null
        if (text.subSequence(localStart, localEnd).toString() != expected) return null
        return (extracted.startOffset + localStart) to (extracted.startOffset + localEnd)
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

    // `internal` so JVM tests can feed a transition without dispatching the
    // engine (JNI); production callers are all inside this class.
    internal fun applyTransition(
        transition: RustEngineBridge.ComposingTransition,
        ic: InputConnection,
    ) {
        // Write order: raw → display → isComposing
        // (mirrors iOS @Observable apply() — see CROSS-PLATFORM PAIR note above).
        // Per-field StateFlows emit ONLY when the assigned value differs from
        // the current `.value` (MutableStateFlow compares + skips), so any
        // observer wired up later sees at most one emission per changed field
        // per transition — never a redundant Idle→Idle flash.
        // `selfCommitInProgress` stays a plain @Volatile (synchronous
        // suppression flag, never observed by UI).
        _rawInput.value = transition.rawInput
        _displayText.value = transition.displayText
        _isComposing.value = transition.isComposing
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
private fun hostReportsNoComposingRegion(
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

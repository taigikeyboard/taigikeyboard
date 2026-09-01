// 中文: Android composing 平台殼 — 將 Rust composing engine(engine/composing crate)
// 中文: 的 Intent → Effect 串到 InputConnection。狀態實質存在 Rust singleton EngineHandle,
// 中文: 此層只把最新 raw/display/isComposing/selectedCandidateIndex 鏡射到 4 個 StateFlow
// 中文: 給既有同步呼叫者(.value)與未來 Compose 觀察者(.collectAsStateWithLifecycle)讀取,
// 中文: 不重建狀態機。bumpGeneration 在 onStartInputView(restarting=false) 觸發,讓 Rust 偵測
// 中文: input-context 變動。

package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.InputConnection
import com.siansiansu.taigikeyboard.engine.LexiconBridge
import com.siansiansu.taigikeyboard.engine.NormalizeMode
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.ToneTogglesCarrier
import com.siansiansu.taigikeyboard.engine.proto.CustomDictEntry
import com.siansiansu.taigikeyboard.engine.proto.FrequencyEntry
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryDerivation
import com.siansiansu.taigikeyboard.ime.dictionary.CustomDictionaryService
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Android platform wrapper around the Rust shared-core composing engine
 * (`engine/composing` crate, accessed via `RustEngineBridge.composing*`).
 *
 * Engine state (phase + raw input + selectedCandidateIndex) lives inside
 * the Rust singleton EngineHandle; this wrapper:
 * - mirrors the latest response into per-field [StateFlow]s
 *   ([rawInput] / [displayText] / [isComposing] / [selectedCandidateIndex])
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
    /**
     * v3.5.8 Phase 9 Item 12 — `custom_dictionary.db` access for the
     * Continuous fetch. Same service the legacy lexicon path uses
     * (`LexiconService.lookupCustomDictionary`). `null` keeps unit-test
     * + Preview construction compiling unchanged (custom merge simply
     * yields no entries, identical to the feature-disabled branch).
     * The runtime call site passes the shared service. DB stays native
     * (`feedback_user_data_sqlite_stays_native`). Mirrors iOS
     * `ComposingManager.swift` `customDictionaryRepository`.
     */
    // 中文: Item 12 — Continuous 路徑查 custom_dictionary.db,與 legacy lexicon path 共用同一 service;
    // 中文: null 保持測試/Preview 可構造 (等同 feature 關閉,無 custom)。DB 留 native。
    private val customDictionaryService: CustomDictionaryService? = null,
) {
    // 中文: 引擎鏡射狀態 — 4 個 MutableStateFlow,公開 read-only StateFlow 表面,
    // 中文: 既有同步 getters 改讀 .value(語義/null 規則完全不變)。
    // CROSS-PLATFORM PAIR — mirrors iOS `ComposingManager.swift` @Observable mirror.
    private val _rawInput = MutableStateFlow("")
    val rawInput: StateFlow<String> = _rawInput.asStateFlow()

    private val _displayText = MutableStateFlow("")
    val displayText: StateFlow<String> = _displayText.asStateFlow()

    private val _isComposing = MutableStateFlow(false)
    val isComposing: StateFlow<Boolean> = _isComposing.asStateFlow()

    private val _selectedCandidateIndex = MutableStateFlow(-1)
    val selectedCandidateIndex: StateFlow<Int> = _selectedCandidateIndex.asStateFlow()

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
     * Platform suppression flag — not view state — so kept as `@Volatile`
     * rather than wrapped in StateFlow.
     */
    @Volatile
    var selfCommitInProgress: Boolean = false
        internal set

    fun isComposing(): Boolean = _isComposing.value

    fun getRawInput(): String? = if (_isComposing.value) _rawInput.value else null

    fun getComposingText(): String? =
        if (_isComposing.value) _displayText.value.ifEmpty { _rawInput.value } else null

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
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
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
                    resolveMode(settings.inputMode),
                    carrier(settings.toneToggles),
                    currentGeneration,
                ),
                ic,
            )
            return
        }
        val spacing = continuousSpacingFlags(settings)
        applyAsSelfCommit(
            RustEngineBridge.composingCommitRaw(
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
                effectiveSwapped = spacing.effectiveSwapped,
                outputBothScripts = spacing.outputBothScripts,
                candidateDisplayMode = settings.candidateDisplayMode,
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
        // 中文: Phase 9 Item 3 + Model B — engine 在 Continuous 下提交整段組字 + 終端 NextWord;
        // 中文: 平台不再 SelectSuggestion 繞路,直接送 CommitRaw 由引擎依 phase 決定行為。
        val settings = settingsProvider.current
        val spacing = continuousSpacingFlags(settings)
        applyAsSelfCommit(
            RustEngineBridge.composingCommitRaw(
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
                effectiveSwapped = spacing.effectiveSwapped,
                outputBothScripts = spacing.outputBothScripts,
                candidateDisplayMode = settings.candidateDisplayMode,
            ),
            ic,
        )
    }

    fun selectSuggestion(
        suggestion: String,
        ic: InputConnection,
    ) {
        logger.tdebug(TAG) { "[COMPOSE] fn=selectSuggestion len=${suggestion.length}" }
        // §10.2 platform pass: under Continuous this routes to
        // `select_suggestion_under_continuous` (prepends `nailed_prefix`),
        // so pass the live spacing flags instead of the old null config.
        val settings = settingsProvider.current
        val spacing = continuousSpacingFlags(settings)
        applyAsSelfCommit(
            RustEngineBridge.composingSelectSuggestion(
                suggestion,
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
                effectiveSwapped = spacing.effectiveSwapped,
                outputBothScripts = spacing.outputBothScripts,
                candidateDisplayMode = settings.candidateDisplayMode,
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
        val spacing = continuousSpacingFlags(settings)
        applyAsSelfCommit(
            RustEngineBridge.composingCommitPreeditThenInsertExternal(
                text,
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
                effectiveSwapped = spacing.effectiveSwapped,
                outputBothScripts = spacing.outputBothScripts,
                candidateDisplayMode = settings.candidateDisplayMode,
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
     * Android divergence (intentional, per `.claude/rules/cross-platform-alignment.md`
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

        // v3.5.8 Phase 9 Item 12 — query `custom_dictionary.db` once
        // for the current raw buffer; the same `customEntries` feeds
        // both fetch phases (the result depends only on the raw
        // buffer, stable across the two FFI calls). `buildCustomEntries`
        // suspends on `Dispatchers.IO`; it self-guards against a
        // keystroke racing during that await by re-checking
        // `_rawInput.value` after `service.search` resumes (the
        // generation guard CANNOT cover this — `generation` only bumps
        // on a new input context, never per keystroke; Codex post-impl
        // 2026-05-15 P2).
        // 中文: Item 12 — 查 custom_dictionary.db 一次,兩 phase 共用;buildCustomEntries 內部
        // 中文: await 後 re-check _rawInput.value 自防 keystroke race(generation 不因 keystroke bump,guard 蓋不到)。
        val customEntries = buildCustomEntries(_rawInput.value, settings)
        val generation = currentGeneration
        val spacing = continuousSpacingFlags(settings)

        // PR-9.6 — compute the dictionary source-toggle bitmask from the
        // SAME settings snapshot + SAME `dictionaryFilters` bridge the Tab3
        // browse path uses (`DictionarySearchViewModel`), so keyboard
        // candidates honour the same 12 source toggles + kautian
        // subcollection (腔調/姓名) toggles. Computed once and shared by
        // both fetch phases (depends only on `settings`, stable across the
        // two FFI calls — mirrors `customEntries`).
        val enabledSourcesBitmask = LexiconBridge.dictionaryFilters(
            LexiconBridge.DictionaryToggles.from(settings),
        ).dictionaryFilterBitmask

        // §34/S22 — invert the 顯示羅馬字 setting into the engine's
        // `disabled` wire flag. Computed once from the same snapshot and
        // shared by both fetch phases so a mid-fetch settings change cannot
        // make the two phases disagree (mirrors `enabledSourcesBitmask`).
        val literalRomanCandidateDisabled = !settings.isLiteralRomanCandidateEnabled
        // Same single-snapshot rule for the display mode (engine collapses
        // same-roman rows under ROMAN_ONLY in both fetch phases).
        val candidateDisplayMode = settings.candidateDisplayMode

        // Phase 1: neutral fetch to learn candidate displayText keys.
        val neutral = RustEngineBridge.composingFetchAtPos(
            mode = mode,
            toggles = toggles,
            generation = generation,
            customEntries = customEntries,
            effectiveSwapped = spacing.effectiveSwapped,
            outputBothScripts = spacing.outputBothScripts,
            enabledSourcesBitmask = enabledSourcesBitmask,
            literalRomanCandidateDisabled = literalRomanCandidateDisabled,
            candidateDisplayMode = candidateDisplayMode,
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
            customEntries = customEntries,
            effectiveSwapped = spacing.effectiveSwapped,
            outputBothScripts = spacing.outputBothScripts,
            enabledSourcesBitmask = enabledSourcesBitmask,
            literalRomanCandidateDisabled = literalRomanCandidateDisabled,
            candidateDisplayMode = candidateDisplayMode,
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
        // R5 pair-key (#7): query by display text (dedup'd), get back one
        // ROW per `(word, tl)` reading + the legacy `tl == ""` bucket, and
        // marshal each as a `FrequencyEntry` carrying `canonicalTl` so the
        // engine can build a `(display_text, canonical_tl)`-keyed map.
        val uniqueKeys = candidates.map { it.displayText }.distinct()
        val rows = userFreq.frequencyDataBatch(uniqueKeys)
        return RustEngineBridge.frequencyRowsToProtoEntries(rows)
    }

    /**
     * v3.5.8 Phase 9 Item 12 — query `custom_dictionary.db` for the
     * current raw buffer and marshal matches into the proto
     * `CustomDictEntry` list carried by `FetchAtPos`. The engine owns
     * the merge + `(roman, hanji)` dedupe + ranking (spec G3 —
     * platform never re-ranks); this only fetches + marshals.
     *
     * v3.6.1 R3 — derives the single family-native query key via
     * `CustomDictionaryDerivation.deriveCustomQueryKey(rawInput, mode)`
     * and runs the cross-mode `custom_search_key` JOIN query
     * (`CustomDictionaryService.search`). **Marshals the RAW stored
     * `(roman, hanzi)` columns** — NOT a display-massaged form — so the
     * engine's `(roman, hanji)` dedupe key collides correctly against
     * `dict.bin`'s `DictionaryRecord.tl` / `.hanzi` (Codex pre-impl
     * 2026-05-15). The stored roman may be TL or POJ display form
     * (whichever the user typed) — v3.5.9 B-4 leaves it raw on the
     * lattice axis here and only folds it to canonical TL inside the
     * engine when synthesizing the `user_frequency.db` commit key.
     * Empty stored hanzi → proto-absent `hanji` (romanization-only
     * entry → engine derives `CandidateMode.Tailo`).
     *
     * CROSS-PLATFORM INVARIANT — the query-key derivation + side-table
     * query mirror iOS `CustomDictionaryDerivation.deriveCustomQueryKey`
     * + `CustomDictionaryRepository` query. Drift causes silent
     * custom-match divergence between platforms
     * (.claude/rules/cross-platform-alignment.md §3a).
     *
     * `null` service (tests / Preview), disabled feature, or `null`
     * query key (residue-only input) → empty list, identical to the
     * no-custom engine path. Runs its SQLite hop inside
     * `CustomDictionaryService.search`'s own `Dispatchers.IO`; resumes
     * on the caller context before the FFI.
     */
    // 中文: Item 12 — 查 custom_dictionary.db 並 marshal 成 proto CustomDictEntry;
    // 中文: R3 — 用 deriveCustomQueryKey 產家族鍵走 custom_search_key JOIN 查詢,送原始 (roman,hanzi);
    // 中文: 空 hanzi → proto-absent hanji (純羅馬字 → 引擎判 TAILO);查詢鍵衍生 + 側表查詢與 iOS 為跨平台不變式。
    private suspend fun buildCustomEntries(
        rawInput: String,
        settings: com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings,
    ): List<CustomDictEntry> {
        val service = customDictionaryService ?: return emptyList()
        if (!settings.isCustomDictEnabled || rawInput.isEmpty()) return emptyList()
        // v3.6.1 R3 — derive the single family-native query key from the raw
        // buffer + settings input mode. `settings.inputMode` is the platform
        // string (incl. "tps"); `InputMode.fromPrefString` collapses "tps" → TL
        // and the engine upgrades to the TPS family via `contains_tps` on the
        // raw input. `null` key (residue-only input) → no custom matches.
        // 中文: R3 — 由 raw buffer + 設定 input mode 產出單一家族查詢鍵;"tps" 折成 TL,引擎以 contains_tps(raw) 升家族。
        val queryKey =
            CustomDictionaryDerivation.deriveCustomQueryKey(
                rawInput,
                com.siansiansu.taigikeyboard.ime.core.settings.InputMode.fromPrefString(settings.inputMode),
            ) ?: return emptyList()
        return try {
            val rows = service.search(family = queryKey.family, form = queryKey.form, key = queryKey.key, limit = 20)
            // v3.5.8 Phase 9 Item 12 — await-race guard (Codex post-impl
            // 2026-05-15 P2). `service.search` suspends on `Dispatchers
            // .IO`; a keystroke landing during that await mutates
            // `_rawInput.value` WITHOUT bumping `generation` (generation
            // only bumps on a new input context, not per keystroke), so
            // the generation guard cannot catch this. If the buffer
            // moved under us these rows belong to a stale prefix —
            // inject nothing rather than wrong candidates; the racing
            // keystroke's own fetch produces the correct custom set.
            // 中文: Item 12 — await race guard:search suspend 期間若 keystroke 改了 _rawInput.value,
            // 中文:   generation 不會因 keystroke bump,guard 抓不到 → 這批 rows 是 stale prefix,丟空不注入錯候選。
            if (_rawInput.value != rawInput) {
                return emptyList()
            }
            rows.map { entry ->
                val builder = CustomDictEntry.newBuilder().setRoman(entry.roman)
                if (entry.hanzi.isNotEmpty()) {
                    builder.setHanji(entry.hanzi)
                }
                builder.build()
            }
        } catch (e: Exception) {
            logger.w(TAG, "[CONTINUOUS] custom dict query failed: ${e.message}", e)
            emptyList()
        }
    }

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
    // v3.5.8 Phase 9 Bug 1 (Option A): `displayText` is the swap/TPS/both-
    // scripts-formatted DOCUMENT string (caller mirrors the legacy lexicon
    // formatter); `canonicalText` is the canonical key (`hanji ?? roman`)
    // routed to NextWord so association learning stays mode-independent.
    fun commitContinuous(
        displayText: String,
        canonicalText: String,
        associationTl: String,
        consumedBytes: Int,
        syllableCount: Int,
        ic: InputConnection,
    ): RustEngineBridge.CommitContinuousResult {
        logger.tdebug(TAG) {
            "[COMPOSE] fn=commitContinuous displayLen=${displayText.length} canonicalLen=${canonicalText.length} consumedBytes=$consumedBytes syllCount=$syllableCount"
        }
        val settings = settingsProvider.current
        val spacing = continuousSpacingFlags(settings)
        val transition = RustEngineBridge.composingCommitContinuous(
            displayText = displayText,
            canonicalText = canonicalText,
            associationTl = associationTl,
            consumedBytes = consumedBytes,
            syllableCount = syllableCount,
            mode = resolveMode(settings.inputMode),
            toggles = carrier(settings.toneToggles),
            generation = currentGeneration,
            effectiveSwapped = spacing.effectiveSwapped,
            outputBothScripts = spacing.outputBothScripts,
            candidateDisplayMode = settings.candidateDisplayMode,
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
        _rawInput.value = ""
        _displayText.value = ""
        _isComposing.value = false
        _selectedCandidateIndex.value = -1
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
        // Write order: raw → display → isComposing → selectedCandidateIndex
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
        _selectedCandidateIndex.value = transition.selectedCandidateIndex
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

    /**
     * The v3.5.8 §10.2 word-boundary-spacing flags the engine's
     * `continuous_word_space` predicate needs, derived from live
     * settings. Single source of the platform-side `effectiveSwapped`
     * combine so all Continuous entry points agree (mis-set → silent
     * hanji-first spurious spaces). `effectiveSwapped` folds TPS into the
     * swap signal because the engine receives TPS as `"tl"`/`"poj"`
     * `input_mode` (its own `input_mode == "tps"` branch never fires
     * from the platform). [EngineSettings.inputMode] is the raw string
     * (`"tps"` representable) per the documented Android divergence.
     */
    // 中文: §10.2 字界空格旗標的唯一來源 — effectiveSwapped = 翻譯反轉 OR TPS。
    // 中文: TPS 在引擎端是 "tl"/"poj" input_mode,故 TPS 必須在平台端折進 swap 訊號。
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Input/Composing/ComposingManager.swift continuousSpacingFlags.
    // Drift causes silent divergence (hanji-first spurious word-boundary spaces).
    private fun continuousSpacingFlags(
        settings: com.siansiansu.taigikeyboard.ime.core.settings.EngineSettings,
    ): ContinuousSpacingFlags =
        ContinuousSpacingFlags(
            effectiveSwapped = settings.isTranslateSwapped || settings.inputMode == "tps",
            outputBothScripts = settings.isOutputBothScripts,
        )

    private data class ContinuousSpacingFlags(
        val effectiveSwapped: Boolean,
        val outputBothScripts: Boolean,
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

package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.InputConnection
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import com.siansiansu.taigikeyboard.ime.dictionary.TPSConverter
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverter
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels

/**
 * Platform wrapper around the pure [ComposingState] engine.
 *
 * Responsibilities kept here (platform-specific):
 * - Reads `EngineSettingsProvider.current.inputMode` + `.toneToggles` per
 *   dispatch so settings stay live-read (see `ios-exemplar.md` §3 warning
 *   on snapshot-initializers; Android DataStore analogue).
 * - Executes [ComposingTransition.Effect] values against the live
 *   [InputConnection] via [ComposingDelegate] (default binding per
 *   `composing-state-boundary.md` §11.2).
 * - Preserves current API signatures (methods take `ic: InputConnection`
 *   per call) so call-sites in `TextInputManager` / `CandidateClickHandler`
 *   / `CandidateUpdateCoordinator` / etc. do not need to be rewritten.
 *
 * The engine boundary lives in [ComposingState] + [ComposingTransition] —
 * this wrapper is intentionally mechanical.
 *
 * Mirrors iOS `Input/Composing/ComposingManager.swift` post-G4-impl shape.
 */
class ComposingManager(
    private val settingsProvider: EngineSettingsProvider,
    private val delegate: ComposingDelegate = DefaultComposingDelegate,
) {
    @Volatile
    private var state: ComposingState = ComposingState()

    /**
     * Off-main-thread display re-derivation writes through this mirror so
     * `updateCandidates` / UI can read the derived form without blocking
     * the dispatch path. The raw preedit is emitted first by
     * [ComposingState.apply], then [applyDerivedDisplay] replaces it
     * asynchronously. See `composing-state-boundary.md` §11 Android
     * addendum for the rationale — iOS derives synchronously; Android
     * keeps the async path as a refactor-freeze preservation.
     */
    @Volatile
    private var cachedDerivedDisplay: String = ""

    val selectedCandidateIndex: Int
        get() = state.selectedCandidateIndex

    fun isComposing(): Boolean = state.isComposing

    fun getRawInput(): String? = if (state.isComposing) state.rawInput else null

    fun getComposingText(): String? =
        if (state.isComposing) {
            cachedDerivedDisplay.ifEmpty { state.rawInput }
        } else {
            null
        }

    // region Intent dispatch API — signatures preserved from pre-A4 shape

    fun startComposing(
        char: String,
        ic: InputConnection,
    ) {
        // Mid-composition restart: zero-then-finish the old preedit before
        // starting the new one. Android's composing region is in the
        // document (vs iOS floating marked text), so `setComposingText`
        // alone would replace the region without committing it, but the
        // explicit pre-zero pins
        // `INVARIANT_composing_clear_preedit_does_not_commit` and matches
        // the contract landed by the parity correction in PR #151 (see
        // `composing-state-boundary.md` §11.6 + §11.10).
        if (state.isComposing) {
            dispatch(ComposingState.Intent.Reset, ic)
        }
        dispatch(ComposingState.Intent.Start(char), ic)
    }

    fun appendCharacter(
        char: String,
        ic: InputConnection,
    ) {
        dispatch(ComposingState.Intent.Append(char), ic)
    }

    fun appendHyphen(ic: InputConnection) {
        dispatch(ComposingState.Intent.AppendHyphen, ic)
    }

    /**
     * Replace the last raw-input character (TPS auto-correct). Intentionally
     * preserves [selectedCandidateIndex] — unlike [appendCharacter] which
     * snaps back to `0`.
     */
    fun replaceLastCharacter(
        replacement: String,
        ic: InputConnection,
    ) {
        dispatch(ComposingState.Intent.ReplaceLast(replacement), ic)
    }

    /**
     * Delete one grapheme. Returns `true` if the wrapper consumed the key
     * (was composing with non-empty raw); callers (TextInputManager) fall
     * through to send a KeyEvent.KEYCODE_DEL on `false` to let the host
     * editor process the backspace.
     *
     * Android divergence from iOS pure-state emission: the 1-char
     * empty-after-delete path routes through [ComposingState.Intent.Reset]
     * rather than [ComposingState.Intent.DeleteBackward] so Android does
     * NOT issue `deleteSurroundingText(1, 0)` after clearing the preedit.
     * iOS's floating marked-text model makes `deleteBackwardFromDocument`
     * the "normal backspace" companion to the clear; Android's in-document
     * composing region is already removed by `ClearPreeditWithoutCommit`,
     * so the extra document delete would remove a pre-existing char. See
     * `composing-state-boundary.md` §11.10 for the divergence note.
     */
    fun deleteBackward(ic: InputConnection): Boolean {
        if (!state.isComposing || state.rawInput.isEmpty()) return false
        if (state.rawInput.length == 1) {
            dispatch(ComposingState.Intent.Reset, ic)
        } else {
            dispatch(ComposingState.Intent.DeleteBackward, ic)
        }
        return true
    }

    /**
     * Commit the tone-marked derived form.
     *
     * Fast / slow path split preserves pre-A4 `displayDirty` semantics so a
     * mid-composition cursor move (editor clears the composing region
     * externally) does NOT duplicate text at the new cursor:
     *
     * - **Fast path** (`cachedDerivedDisplay` non-empty, i.e. async
     *   derivation already replaced the raw preedit with the derived form):
     *   issue `finishComposingText()` only. If the editor cleared the
     *   region externally, this is a no-op — matches pre-A4
     *   `displayDirty == false` behavior. If the region is still live,
     *   `finishComposingText` commits whatever text the region shows
     *   (which is the derived form).
     * - **Slow path** (`cachedDerivedDisplay` empty, i.e. async derivation
     *   hasn't caught up to the latest keystroke): route through
     *   [ComposingState.Intent.CommitDerived] which synchronously derives
     *   + atomically commits via `commitText`. Matches pre-A4
     *   `displayDirty == true` behavior. In the externally-cleared-region
     *   case this retains the pre-A4 duplicate-insertion behavior — a
     *   narrow regression vs the fast-path no-op, but rare (<50 ms
     *   between keystroke and commit).
     */
    fun commitComposition(ic: InputConnection) {
        if (!state.isComposing) return
        if (cachedDerivedDisplay.isNotEmpty()) {
            ic.finishComposingText()
            state = ComposingState()
            cachedDerivedDisplay = ""
        } else {
            dispatch(ComposingState.Intent.CommitDerived, ic)
        }
    }

    /**
     * Commit the literal raw keystrokes (bypass tone conversion). Mirror of
     * iOS `ComposingManager.commitRawInput()`. No Android call-site invokes
     * this today; kept for API parity with iOS and to exercise the
     * `.CommitRaw` pure-state intent in tests.
     */
    fun commitRawInput(ic: InputConnection) {
        dispatch(ComposingState.Intent.CommitRaw, ic)
    }

    fun selectSuggestion(
        suggestion: String,
        ic: InputConnection,
    ) {
        dispatch(ComposingState.Intent.SelectSuggestion(suggestion), ic)
    }

    /**
     * Commit the current preedit (if any) and insert externally-supplied
     * [text] in one atomic `InputConnection.commitText` call. Used by
     * non-Taigi input surfaces — emoji palette, clipboard paste — so an
     * active Taigi preedit never leaks a silent commit through direct
     * `finishComposingText` bypass paths. Mirrors iOS
     * `ComposingManager.commitPreeditThenInsertExternal(_:)`.
     *
     * Replaces the pre-A5 `MediaInputManager.sendEmojiKeyPress` direct
     * `finishComposingText + commitText` sequence — see
     * `composing-state-boundary.md` §11.6 deferred parity follow-up.
     */
    fun commitPreeditThenInsertExternal(
        text: String,
        ic: InputConnection,
    ) {
        dispatch(ComposingState.Intent.CommitPreeditThenInsertExternal(text), ic)
    }

    /**
     * Reset all composing state.
     *
     * Mirrors iOS `ComposingState.apply(.reset)` which emits
     * `clearPreeditWithoutCommit`. See `composing-state-boundary.md` §11.6
     * and `behavioral-invariants.md` §13 for the binding pin
     * (`INVARIANT_composing_clear_preedit_does_not_commit`).
     */
    fun reset(ic: InputConnection) {
        dispatch(ComposingState.Intent.Reset, ic)
    }

    /**
     * Sync internal composing state after the host editor reports no
     * composing region (Android framework signals this via
     * `InputMethodService.onUpdateSelection` with
     * `candidatesStart == -1 && candidatesEnd == -1`, e.g. cursor move
     * via tap, selection change). Zeroes [state] + [cachedDerivedDisplay]
     * without touching [InputConnection] — the region is already gone on
     * the host side, so any IC call here would either no-op or mutate
     * text at the new cursor position.
     *
     * Closes the root cause of the [commitComposition] fast/slow split
     * documented in `composing-state-boundary.md` §11.10 divergence #3:
     * without this hook, a stale `cachedDerivedDisplay` would survive
     * an external clear and drive the fast-path `finishComposingText()`,
     * OR a stale `state.isComposing` would drive the slow-path
     * `CommitDerived` into a duplicate `commitText` at the new cursor.
     *
     * Pinned by `INVARIANT_composing_external_region_clear_discards_state`
     * (`behavioral-invariants.md` §13). Android-only binding contract;
     * iOS's floating marked text model has no in-document region for the
     * host editor to clear externally.
     */
    fun onExternalComposingRegionCleared() {
        if (!state.isComposing) return
        state = ComposingState()
        cachedDerivedDisplay = ""
    }

    // endregion

    /**
     * Apply a pre-computed derived display to the live preedit. Called by
     * [com.siansiansu.taigikeyboard.ime.text.CandidateUpdateCoordinator]
     * after background tone conversion completes — replaces the raw-keystroke
     * placeholder emitted by live-typing intents (see
     * `composing-state-boundary.md` §11 Android addendum).
     */
    internal fun applyDerivedDisplay(
        derivedText: String,
        ic: InputConnection,
    ) {
        if (!state.isComposing) return
        cachedDerivedDisplay = derivedText
        ic.setComposingText(derivedText, 1)
    }

    /**
     * Derive the display form for a caller-supplied raw-input snapshot.
     * Used by the async display derivation path in
     * [com.siansiansu.taigikeyboard.ime.text.CandidateUpdateCoordinator],
     * which captures [raw] at job-launch time so the stale-job guard
     * (`manager.getRawInput() == raw`) remains valid even if the
     * composing buffer mutated mid-derivation (e.g. `a → ab → a` races
     * where a survived job could read mutable state).
     *
     * Settings stay live-read per [EngineSettingsProvider.current]
     * contract. Returns an empty string when [raw] is empty.
     */
    internal fun deriveDisplay(raw: String): String {
        if (raw.isEmpty()) return ""
        if (TPSConverter.containsTPS(raw)) return raw
        val settings = settingsProvider.current
        val mode = resolveInputMode(settings.inputMode)
        return ToneConverter.convertToToneMarks(raw, mode, settings.toneToggles)
    }

    private fun dispatch(
        intent: ComposingState.Intent,
        ic: InputConnection,
    ) {
        val settings = settingsProvider.current
        val mode = resolveInputMode(settings.inputMode)
        val (newState, transition) = state.apply(intent, mode, settings.toneToggles)
        state = newState
        // Reset the derived-display mirror on every dispatch so
        // `getComposingText()` returns the raw placeholder until the async
        // derivation loop (CandidateUpdateCoordinator) calls
        // `applyDerivedDisplay`. Mirrors pre-A4 observable behavior where
        // the placeholder flash is visible to `handleEnter` / `handleSpace`
        // callers that capture `committedText` before committing — see
        // `composing-state-boundary.md` §11.10.
        cachedDerivedDisplay = ""
        for (effect in transition.effects) {
            delegate.execute(effect, ic)
        }
    }

    private fun resolveInputMode(raw: String): ToneConverterModels.InputMode =
        when (raw) {
            "poj" -> ToneConverterModels.InputMode.POJ
            "tl", "tps" -> ToneConverterModels.InputMode.TL
            else -> ToneConverterModels.InputMode.POJ
        }
}

/**
 * Policy helper: does an `onUpdateSelection` payload indicate the host
 * editor no longer reports a composing region? Both coordinates are `-1`
 * when no region exists (per
 * `InputMethodService.onUpdateSelection(int,int,int,int,int,int)`).
 *
 * Pulled out as a top-level function so the policy is unit-testable
 * without instantiating the Android framework classes `TextInputManager`
 * depends on. Consumer: [TextInputManager.onUpdateSelection].
 */
internal fun hostReportsNoComposingRegion(
    candidatesStart: Int,
    candidatesEnd: Int,
): Boolean = candidatesStart == -1 && candidatesEnd == -1

/**
 * Clear the host editor's composing region at the [InputConnection] layer
 * without committing whatever text it contains. Issues
 * `setComposingText("", 1)` then `finishComposingText()` — the pre-zero
 * is mandatory because `finishComposingText()` on its own silently
 * commits the active composing region (see
 * `composing-state-boundary.md` §11.2 rule 1).
 *
 * Pulled out as a top-level function so bare-`InputConnection` sites
 * that do NOT route through [ComposingManager.reset] (e.g.
 * `TextInputManager.resetComposingText` — called when `composingManager`
 * is null or the current keyboard mode bypasses composing) still honor
 * `INVARIANT_composing_clear_preedit_does_not_commit`
 * (`behavioral-invariants.md` §13). [ComposingManager.reset] remains the
 * canonical owner for composing-aware sites; this helper is the
 * IC-layer-only equivalent.
 */
internal fun clearHostComposingRegion(ic: InputConnection?) {
    ic?.setComposingText("", 1)
    ic?.finishComposingText()
}

package com.siansiansu.taigikeyboard.ime.text.composing

import android.view.inputmethod.InputConnection
import com.siansiansu.taigikeyboard.engine.NormalizeMode
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.ToneTogglesCarrier
import com.siansiansu.taigikeyboard.ime.core.settings.EngineSettingsProvider
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
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

    fun getComposingText(): String? =
        if (cachedIsComposing) cachedDisplayText.ifEmpty { cachedRawInput } else null

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
        // Starts at 1; first bump → 2. Engine-side `last_generation`
        // initializes to 0 so the very first request is already a
        // mismatch (silent reset of fresh engine = no-op).
        private val sharedGeneration: AtomicLong = AtomicLong(1L)
    }

    // region Intent dispatch API

    fun startComposing(char: String, ic: InputConnection) {
        val settings = settingsProvider.current
        val mode = resolveMode(settings.inputMode)
        if (cachedIsComposing) {
            // Mid-composition restart: clear-without-commit before starting fresh.
            applyAsSelfCommit(
                RustEngineBridge.composingReset(currentGeneration),
                ic,
            )
        }
        applyTransition(
            RustEngineBridge.composingStart(
                char,
                mode,
                carrier(settings.toneToggles),
                currentGeneration,
            ),
            ic,
        )
    }

    fun appendCharacter(char: String, ic: InputConnection) {
        val settings = settingsProvider.current
        applyTransition(
            RustEngineBridge.composingAppend(
                char,
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
            ),
            ic,
        )
    }

    fun appendHyphen(ic: InputConnection) {
        val settings = settingsProvider.current
        applyTransition(
            RustEngineBridge.composingAppendHyphen(
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
            ),
            ic,
        )
    }

    fun replaceLastCharacter(replacement: String, ic: InputConnection) {
        val settings = settingsProvider.current
        applyTransition(
            RustEngineBridge.composingReplaceLast(
                replacement,
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
            ),
            ic,
        )
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
        if (!cachedIsComposing) return
        val settings = settingsProvider.current
        applyAsSelfCommit(
            RustEngineBridge.composingCommitDerived(
                resolveMode(settings.inputMode),
                carrier(settings.toneToggles),
                currentGeneration,
            ),
            ic,
        )
    }

    fun commitRawInput(ic: InputConnection) {
        applyAsSelfCommit(
            RustEngineBridge.composingCommitRaw(currentGeneration),
            ic,
        )
    }

    fun selectSuggestion(suggestion: String, ic: InputConnection) {
        applyAsSelfCommit(
            RustEngineBridge.composingSelectSuggestion(suggestion, currentGeneration),
            ic,
        )
    }

    fun commitPreeditThenInsertExternal(text: String, ic: InputConnection) {
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
        applyAsSelfCommit(
            RustEngineBridge.composingReset(currentGeneration),
            ic,
        )
    }

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
            delegate.execute(effect, ic)
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

package com.siansiansu.taigikeyboard.ime.text

import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.inputmethod.CursorAnchorInfo
import android.view.inputmethod.EditorInfo
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.popup.KeyPopupManager
import com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
import com.siansiansu.taigikeyboard.ime.text.composing.hostReportsNoComposingRegion
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.ImeKeyEventDispatcher
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyTouchCoordinator
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardThemeSurfaceController
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardUiCoordinator
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardUiState
import com.siansiansu.taigikeyboard.ime.text.keyboard.TextInputKeyHandler
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutManager
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.StateFlow

/**
 * Manages all keyboard mode / layout / popup /
 * composing / smartbar / candidate-fetch coordination.
 *
 * Implements [TaigiKeyboard.EventListener]; consumes key events and
 * routes them through Rust engine bridges + UI state managers. Holds
 * the lifetime of the [LayoutManager], [SmartbarManager], [ComposingManager],
 * [CandidateUpdateCoordinator], and [CapsStateManager] subsystems.
 */
class TextInputManager(
    private val taigikeyboard: TaigiKeyboard,
    private val prefs: com.siansiansu.taigikeyboard.ime.core.PrefHelper,
) : CoroutineScope by MainScope(),
    TaigiKeyboard.EventListener {
    private val osHandler = Handler(Looper.getMainLooper())
    var textViewGroup: android.view.ViewGroup? = null
        private set

    var keyVariation: KeyVariation = KeyVariation.NORMAL

    // Assigned by TaigiKeyboard.onCreate immediately after the SmartbarManager
    // is constructed — A7 reverses the pre-existing lazy-lookup cycle.
    lateinit var smartbarManager: SmartbarManager

    // Composing manager (synchronized access via composingLock)
    private val composingLock = Any()
    private var composingManager: ComposingManager? = null

    // Composing related properties
    private var isComposingEnabled: Boolean = false

    // --- Compose-side state surface -------------------------------------------------

    /** Single popup window stack reused for the IME-service lifetime. */
    private val popupHost: KeyPopupManager = KeyPopupManager(taigikeyboard)

    /** Cached reference to the keyboard ComposeView host inside the input view.
     *  Resolved once after [KeyboardUiCoordinator.mountKeyboardComposeView] so
     *  the popup anchor resolution avoids a `findViewById` walk. */
    private var composeHost: View? = null

    /** Applies the View-layer side of the active theme (background gradient on the
     *  common `text_input_content` parent + smartbar chrome transparency). Bound in
     *  [onRegisterInputView]; the Compose key body resolves its own transparency. */
    private var themeSurface: KeyboardThemeSurfaceController? = null

    // --- Delegated handlers ---

    private val capsStateManager =
        CapsStateManager(
            taigikeyboard = taigikeyboard,
            onInvalidateAllKeys = { pushAppearance() },
            onInvalidateCharacterKeys = { pushAppearance() },
        )

    private val appearanceResolver = KeyboardAppearanceResolver(
        prefs = prefs,
        taigikeyboard = taigikeyboard,
        capsStateManager = capsStateManager,
        isComposingProvider = { synchronized(composingLock) { composingManager?.isComposing() == true } },
        translateSwappedProvider = {
            if (this::smartbarManager.isInitialized) {
                smartbarManager.getCachedIsTranslateSwapped()
            } else {
                prefs.isTranslateSwapped
            }
        },
    )

    /** Owns the keyboard-body UI state + layout-reload cancellation chain.
     *  TIM keeps the appearance snapshot owner and the smartbar/measure side
     *  effects via the [onLayoutChanged] / [onActiveModeChanged] callbacks. */
    private val uiCoordinator = KeyboardUiCoordinator(
        scope = this,
        layoutManagerFactory = { LayoutManager(taigikeyboard, prefs) },
        activeSubtypeProvider = { taigikeyboard.activeSubtype },
        translateSwappedProvider = {
            if (this::smartbarManager.isInitialized) {
                smartbarManager.getCachedIsTranslateSwapped()
            } else {
                prefs.isTranslateSwapped
            }
        },
        onLayoutChanged = { pushAppearance() },
        onActiveModeChanged = {
            smartbarManager.activeContainer = smartbarManager.preferredContainer
            textViewGroup?.post { measureAndUpdateKeyboardHeight() }
        },
    )

    /** Forwarder of [KeyboardUiCoordinator.keyboardUi] preserved on TIM for
     *  source-compatible public access. */
    val keyboardUi: StateFlow<KeyboardUiState> = uiCoordinator.keyboardUi

    /** Touch state machine that drives [popupHost] + key-press dispatch. */
    private val coordinator: KeyTouchCoordinator =
        KeyTouchCoordinator(
            popupHost = popupHost,
            dispatcher = ImeKeyEventDispatcher(
                taigikeyboard = taigikeyboard,
                prefs = prefs,
                capsStateManager = capsStateManager,
                composeHostProvider = { composeHost },
                onDispatchKeyPress = { data -> sendKeyPress(data) },
            ),
        )

    private lateinit var candidateCoordinator: CandidateUpdateCoordinator

    /** Per-keystroke handler — owns DELETE / ENTER / SPACE / Taigi /
     *  numeric / TLD dispatch. Bare `composingManager` reads stay
     *  un-synchronized (write-side synchronization is on TIM). */
    private val keyHandler = TextInputKeyHandler(
        taigikeyboard = taigikeyboard,
        prefs = prefs,
        capsStateManager = capsStateManager,
        uiCoordinator = uiCoordinator,
        osHandler = osHandler,
        composingManagerProvider = { composingManager },
        candidateCoordinatorProvider = { candidateCoordinator },
        smartbarManagerProvider = { smartbarManager },
    )

    // --- Public delegation API (preserves original interface) ---

    val caps: Boolean get() = capsStateManager.caps
    val capsLock: Boolean get() = capsStateManager.capsLock

    fun getCapsState(): Pair<Boolean, Boolean> = capsStateManager.getCapsState()

    fun getComposingManager(): ComposingManager? = synchronized(composingLock) { composingManager }

    /**
     * Trigger the standard Taigi candidate recompute pipeline.
     * Used by [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
     * after a Continuous mid-commit, where the engine emits
     * `PerformAutocomplete` but `DefaultComposingDelegate` treats it as a
     * no-op — the candidate flow has historically been driven from the
     * keystroke pipeline, not from effect dispatch.
     */
    fun requestTaigiCandidateRefresh() {
        candidateCoordinator.updateTaigiCandidates()
    }

    companion object {
        private const val TAG = "TextInputManager"
    }

    private val logger get() = taigikeyboard.compositionRoot.logger

    override fun onCreate() {
        logger.i(TAG, "onCreate()")

        candidateCoordinator =
            CandidateUpdateCoordinator(
                scope = this,
                taigikeyboard = taigikeyboard,
                // Synchronized getter: the coordinator resolves the manager from
                // a worker thread while `composingManager` is swapped on Main.
                getComposingManager = ::getComposingManager,
                smartbarManager = smartbarManager,
            )
    }

    override fun onRegisterInputView(inputView: InputView) {
        logger.i(TAG, "onRegisterInputView(inputView)")

        // All Main-thread setup runs synchronously so `TaigiKeyboard.onWindowShown`
        // (which fires after this returns) sees `textViewGroup` already populated.
        // Without this, `setActiveInput(R.id.text_input)` would race the
        // background coroutine, find `textViewGroup == null`, and
        // `mainViewFlipper.indexOfChild(null) == -1` would wrap to the last
        // child (`media_input`) — visible as the emoji keyboard appearing on
        // first install. Pins
        // `INVARIANT_keyboard_register_input_view_main_thread_setup`.
        popupHost.attachHostView(inputView)
        popupHost.installPopupViewTreeOwnersIfNeeded()

        textViewGroup = inputView.findViewById(R.id.text_input)

        // Publish layout + appearance + active mode into KeyboardUiState BEFORE
        // mounting the ComposeView. `mountKeyboardComposeView` runs `setContent`
        // on the already-attached InputView, which makes Compose create the
        // keyboard body's FIRST composition synchronously — it must read a
        // populated state so `KeyboardImeRoot` renders a non-empty body at the
        // IME window's first measure. An empty body would let `wrap_content` pin
        // the input view to smartbar-only height on cold open (the prior async
        // publish landed too late). `onStartInputView` re-publishes the
        // editor-specific mode synchronously for the register-before-start order.
        val activeKeyboardMode = uiCoordinator.activeKeyboardMode
        uiCoordinator.ensureLayoutLoadedNow(activeKeyboardMode)
        pushAppearance()
        uiCoordinator.publishActiveMode(activeKeyboardMode)

        composeHost = uiCoordinator.mountKeyboardComposeView(
            inputView = inputView,
            coordinator = coordinator,
            popupHost = popupHost,
            onHeightFactorChanged = { factor ->
                smartbarManager.smartbarView?.setHeightFactor(factor)
            },
        )

        themeSurface = KeyboardThemeSurfaceController(inputView)

        val overlayView =
            inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateOverlayView>(
                R.id.candidate_overlay,
            )
        smartbarManager.registerCandidateOverlayView(overlayView)

        val layoutOverlay =
            inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.LayoutSelectionOverlayView>(
                R.id.layout_selection_overlay,
            )
        smartbarManager.registerLayoutSelectionOverlayView(layoutOverlay)

        val symbolOverlay =
            inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.SymbolSelectionOverlayView>(
                R.id.symbol_selection_overlay,
            )
        smartbarManager.registerSymbolSelectionOverlayView(symbolOverlay)

        val settingsOverlay =
            inputView.findViewById<com.siansiansu.taigikeyboard.ime.text.smartbar.SettingsSelectionOverlayView>(
                R.id.settings_selection_overlay,
            )
        smartbarManager.registerSettingsSelectionOverlayView(settingsOverlay)

        textViewGroup?.post {
            measureAndUpdateKeyboardHeight()
        }
    }

    override fun onDestroy() {
        logger.i(TAG, "onDestroy()")

        candidateCoordinator.destroy()
        coordinator.reset()
        popupHost.dismissAllPopups()

        cancel()
        osHandler.removeCallbacksAndMessages(null)
        smartbarManager.onDestroy()
    }

    override fun onStartInputView(
        info: EditorInfo?,
        restarting: Boolean,
    ) {
        // Null EditorInfo keeps `keyVariation` as-is per legacy behaviour — a
        // prior PASSWORD session stays PASSWORD/non-composing when the host
        // editor drops its EditorInfo on a restart. Only `isComposingEnabled`
        // is recomputed from the (unchanged) variation.
        val keyboardMode: KeyboardMode
        if (info != null) {
            val classification = EditorInfoClassifier.classify(info)
            keyboardMode = classification.mode
            keyVariation = classification.keyVariation
            isComposingEnabled = classification.isComposingEnabled
        } else {
            keyboardMode = KeyboardMode.CHARACTERS
            isComposingEnabled = keyVariation != KeyVariation.PASSWORD
        }

        synchronized(composingLock) {
            composingManager =
                if (isComposingEnabled && keyboardMode == KeyboardMode.CHARACTERS) {
                    ComposingManager(
                        settingsProvider = taigikeyboard.prefs,
                        // Route NextWord-shaped composing effects (Continuous
                        // mid/final commits, abort) into the SmartbarManager-
                        // owned NextWordHandler.
                        nextWordRouter = { effect -> smartbarManager.dispatchComposingNextWordEffect(effect) },
                        logger = taigikeyboard.compositionRoot.logger,
                        // User-frequency snapshot source for the
                        // Continuous-input two-phase fetch. Mirrors iOS
                        // `ComposingManager(userFrequencyService:
                        // CompositionRoot.userFrequencyService)`.
                        userFrequencyService = taigikeyboard.compositionRoot.userFreq,
                        // v3.5.8 Phase 9 Item 12 — `custom_dictionary.db`
                        // source for the Continuous fetch. Same shared
                        // instance the legacy lexicon path uses. Mirrors
                        // iOS `ComposingManager(customDictionaryRepository:
                        // CompositionRoot.customDictionaryRepository)`.
                        customDictionaryService = taigikeyboard.compositionRoot.customDict,
                    )
                } else {
                    null
                }
        }

        capsStateManager.updateCapsState()
        keyHandler.resetComposingText()
        uiCoordinator.publishKeyVariation(keyVariation)
        // Publish the editor's target mode layout SYNCHRONOUSLY so a fresh
        // ComposeView (register-before-start lifecycle order) measures the
        // correct non-empty body instead of async-swapping NUMERIC/PHONE in
        // after the first measure. Makes the following `setActiveKeyboardMode` a
        // cache hit → synchronous `publishActiveMode`. Warm shows → no I/O.
        uiCoordinator.ensureLayoutLoadedNow(keyboardMode)
        uiCoordinator.setActiveKeyboardMode(keyboardMode)
        // imeOptions / confirm-key label / composing flag may all flip on a
        // new editor — refresh appearance so KeyContent re-derives ENTER /
        // SPACE label visuals against the new EditorInfo.
        pushAppearance()
        smartbarManager.onStartInputView(keyboardMode, isComposingEnabled)

        // v3.5.4 lifecycle (plan §4.2): bump on every onStartInputView,
        // including `restarting=true`. The block above unconditionally
        // reassigns `composingManager` and the preceding `resetComposingText()`
        // wiped the host editor's composing region — both signals say
        // "platform-side state is fresh", so the process-singleton Rust
        // engine MUST drop its preedit too. Without this bump, a restart
        // (orientation flip, soft-keyboard re-show) would leave the engine
        // holding stale buffer text that the next `deleteBackward` query
        // would read as authoritative (Codex post-impl PR #197 r3163335192).
        composingManager?.bumpGeneration()
    }

    override fun onFinishInputView(finishingInput: Boolean) {
        candidateCoordinator.cancelAll()
        smartbarManager.onFinishInputView()
    }

    override fun onWindowShown() {
        refreshTheme()
    }

    /**
     * Re-pushes the key appearance AND re-applies the View-layer theme (background
     * gradient on the common parent + smartbar chrome transparency). Called when the
     * keyboard becomes visible so a theme change made while it was hidden takes
     * effect. The smartbar child views are reliably registered by this point
     * (onWindowShown fires after the full attach dispatch).
     */
    fun refreshTheme() {
        pushAppearance()
        themeSurface?.apply(appearanceResolver.resolvedColors())
    }

    fun invalidateAllKeys() {
        pushAppearance()
    }

    /** Targeted invalidation became a no-op when the keyboard moved to
     *  Compose: per-key recomposition is driven by the appearance data class,
     *  so a full appearance push covers any subset of keys at no extra cost.
     *  [keyCodes] is retained for source compatibility with callers (e.g.,
     *  `SmartbarManager.toggleTranslateSwapped`) but is ignored. */
    @Suppress("UNUSED_PARAMETER")
    fun invalidateKeysByCode(vararg keyCodes: Int) {
        pushAppearance()
    }

    private fun measureAndUpdateKeyboardHeight() {
        val viewGroup = textViewGroup ?: return
        val contentView = viewGroup.findViewById<android.view.View>(R.id.text_input_content)
        if (contentView?.measuredHeight ?: 0 > 0) {
            val height = contentView.measuredHeight
            smartbarManager.setKeyboardHeight(height)
        }
    }

    override fun onSubtypeChanged(newSubtype: Subtype) {
        uiCoordinator.reloadForSubtype(newSubtype)
    }

    override fun onInputModeChanged(newInputMode: String) {
        if (logger.isDebugEnabled) logger.i(TAG, "onInputModeChanged($newInputMode)")

        // ComposingManager reads inputMode + toneToggles per dispatch
        // via EngineSettingsProvider.current (live read) — no direct
        // field mutation needed.

        // Drop any in-flight Continuous state BEFORE the layout reload kicks
        // off the autocomplete-service rebuild. TL/POJ/TPS each use distinct
        // `Phase::Continuous { raw }` byte conventions; leftover pending bytes
        // would mis-align consumed-span offsets when the next FetchAtPos lands
        // in the new mode. `bumpGeneration` then forces the engine to silently
        // drop residual state at the FFI boundary on the next request.
        val ic = taigikeyboard.currentInputConnection
        if (ic != null) {
            getComposingManager()?.resetContinuous(ic)
        }
        getComposingManager()?.bumpGeneration()

        uiCoordinator.reloadForInputMode(newInputMode)
    }

    /**
     * Re-fetches the Taigi candidates for the composition as it stands. The
     * 候選詞顯示 picker is not a pure cell-rendering switch: the engine
     * collapses same-roman rows under 羅馬字 (§44), so the list itself
     * changes; repainting the cached suggestions would keep the duplicates
     * on screen until the next keystroke. No-op when nothing is composing —
     * the coordinator would otherwise clear a prediction strip.
     */
    fun refetchCandidatesForDisplayModeChange() {
        if (composingManager?.isComposing() == true) {
            candidateCoordinator.updateTaigiCandidates()
        }
    }

    override fun onKeyboardLayoutTypeChanged(newLayoutType: String) {
        if (logger.isDebugEnabled) logger.i(TAG, "onKeyboardLayoutTypeChanged($newLayoutType)")

        uiCoordinator.reloadForLayoutType()
    }

    fun reloadCurrentLayout() {
        logger.i(TAG, "reloadCurrentLayout()")
        uiCoordinator.reloadCurrentLayout()
    }

    fun reloadAllLayoutsInBackground() {
        logger.i(TAG, "reloadAllLayoutsInBackground()")
        uiCoordinator.reloadAllLayoutsInBackground()
    }

    override fun onUpdateCursorAnchorInfo(cursorAnchorInfo: CursorAnchorInfo?) {
        cursorAnchorInfo ?: return
        capsStateManager.updateCapsState()
    }

    override fun onUpdateSelection(
        oldSelStart: Int,
        oldSelEnd: Int,
        newSelStart: Int,
        newSelEnd: Int,
        candidatesStart: Int,
        candidatesEnd: Int,
    ) {
        // When the host editor reports no composing region (both
        // candidate offsets == -1, e.g. user taps to move the cursor or
        // changes the selection), sync internal ComposingManager state so
        // a later commit/reset does not re-insert stale preedit at the
        // new cursor. Closes the root cause of the commitComposition
        // fast/slow split in `composing-state-boundary.md` §11.10
        // divergence #3; pinned by
        // `INVARIANT_composing_external_region_clear_discards_state`.
        if (hostReportsNoComposingRegion(candidatesStart, candidatesEnd)) {
            composingManager?.onExternalComposingRegionCleared()
        }
    }

    /** Forwarder kept for external callers — [SmartbarManager],
     *  [com.siansiansu.taigikeyboard.ime.media.MediaInputManager], and the
     *  [ImeKeyEventDispatcher] `onDispatchKeyPress` lambda. */
    fun sendKeyPress(keyData: KeyData) = keyHandler.sendKeyPress(keyData)

    /** Republishes the latest appearance snapshot. Equality on the
     *  [com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardAppearance]
     *  data class collapses no-op refreshes (e.g., back-to-back
     *  `onWindowShown` + `invalidateAllKeys`) into a single
     *  [KeyboardUiState] value via [KeyboardUiCoordinator.publishAppearance],
     *  so downstream Compose recomposition only fires when visuals actually
     *  change. */
    private fun pushAppearance() {
        uiCoordinator.publishAppearance(appearanceResolver.snapshot())
    }
}

// 中文: IME 輸入主編排器 — 管理 keyboard mode、layout、popup、composing、smartbar、
// 中文: 候選 debounce、Caps 狀態整合等。對應 iOS 端 KeyboardController 的角色,
// 中文: 同時也是 TaigiKeyboard.EventListener,負責把按鍵事件轉成 Rust 引擎呼叫與 UI 更新。

package com.siansiansu.taigikeyboard.ime.text

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.KeyEvent
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.CursorAnchorInfo
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.engine.CaseTransformBridge
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.popup.KeyAnchor
import com.siansiansu.taigikeyboard.ime.popup.KeyPopupManager
import com.siansiansu.taigikeyboard.ime.popup.buildPopupCells
import com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
import com.siansiansu.taigikeyboard.ime.text.composing.clearHostComposingRegion
import com.siansiansu.taigikeyboard.ime.text.composing.hostReportsNoComposingRegion
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyBounds
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyEventDispatcher
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyTouchCoordinator
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardAppearance
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardHeightFactor
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardImeRoot
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayoutData
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardUiState
import com.siansiansu.taigikeyboard.ime.text.keyboard.computeKeyLetter
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutManager
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
import com.siansiansu.taigikeyboard.localization.SettingsTexts
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import java.util.*

/**
 * IME 輸入主編排器 — manages all keyboard mode / layout / popup /
 * composing / smartbar / candidate-debounce coordination.
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
    private var activeKeyboardMode: KeyboardMode = KeyboardMode.CHARACTERS
    private val osHandler = Handler(Looper.getMainLooper())
    var textViewGroup: android.view.ViewGroup? = null
        private set

    var keyVariation: KeyVariation = KeyVariation.NORMAL
    private val layoutManager: LayoutManager by lazy { LayoutManager(taigikeyboard, prefs) }

    // Assigned by TaigiKeyboard.onCreate immediately after the SmartbarManager
    // is constructed — A7 reverses the pre-existing lazy-lookup cycle.
    lateinit var smartbarManager: SmartbarManager

    // Cancels previous layout reload to prevent race conditions on rapid mode switches
    private var layoutReloadJob: Job? = null

    // Composing manager (synchronized access via composingLock)
    private val composingLock = Any()
    private var composingManager: ComposingManager? = null

    // Composing related properties
    private var isComposingEnabled: Boolean = false

    // --- Compose-side state surface -------------------------------------------------

    /** Single source of truth for the keyboard body Composable. Mutations
     *  always copy a fresh layouts map so snapshot equality drives
     *  recomposition. */
    private val _keyboardUi = MutableStateFlow(KeyboardUiState.EMPTY)
    val keyboardUi: StateFlow<KeyboardUiState> = _keyboardUi.asStateFlow()

    /** Single popup window stack reused for the IME-service lifetime. */
    private val popupHost: KeyPopupManager = KeyPopupManager(taigikeyboard)

    /** Touch state machine that drives [popupHost] + key-press dispatch. */
    private val coordinator: KeyTouchCoordinator =
        KeyTouchCoordinator(popupHost = popupHost, dispatcher = ImeKeyEventDispatcher())

    /** Lazy view used both as the popup `showAtLocation` parent and as the
     *  reference for vibration haptics. Bound in [onRegisterInputView]. */
    private var hostView: View? = null

    /** Cached reference to the keyboard ComposeView host inside [hostView].
     *  Resolved once after [mountKeyboardComposeView] so each [resolveAnchor]
     *  call avoids a `findViewById` walk. */
    private var composeHost: View? = null

    /** Scratch buffer reused by `getLocationInWindow` inside
     *  [ImeKeyEventDispatcher.resolveAnchor] — main-thread only. */
    private val locationScratch = IntArray(2)

    /** Color-settings parse cache — re-parses only when the JSON string
     *  changes. Mirrors the legacy `KeyboardView.getColorSettings` cache. */
    private var cachedColorSettingsJson: String = ""
    private var cachedColorSettings: KeyboardColorSettings = KeyboardColorSettings()
    private var cachedFontType: String = ""
    private var cachedTypeface: android.graphics.Typeface = android.graphics.Typeface.DEFAULT

    // --- Delegated handlers ---

    private val capsStateManager =
        CapsStateManager(
            taigikeyboard = taigikeyboard,
            onInvalidateAllKeys = { pushAppearance() },
            onInvalidateCharacterKeys = { pushAppearance() },
        )

    private lateinit var candidateCoordinator: CandidateUpdateCoordinator

    // --- Public delegation API (preserves original interface) ---

    val caps: Boolean get() = capsStateManager.caps
    val capsLock: Boolean get() = capsStateManager.capsLock

    fun getCapsState(): Pair<Boolean, Boolean> = capsStateManager.getCapsState()

    fun getComposingManager(): ComposingManager? = synchronized(composingLock) { composingManager }

    /**
     * Trigger the standard debounced Taigi candidate recompute pipeline.
     * Used by [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
     * after a Continuous mid-commit, where the engine emits
     * `PerformAutocomplete` but `DefaultComposingDelegate` treats it as a
     * no-op — the candidate flow has historically been driven from the
     * keystroke pipeline, not from effect dispatch.
     */
    fun requestTaigiCandidateRefresh() {
        candidateCoordinator.updateTaigiCandidatesDebounced()
    }

    companion object {
        private const val TAG = "TextInputManager"
        private val DOUBLE_SPACE_PERIOD_REGEX = """[.!?‽\s][\s]""".toRegex()
    }

    override fun onCreate() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onCreate()")

        candidateCoordinator =
            CandidateUpdateCoordinator(
                scope = this,
                taigikeyboard = taigikeyboard,
                getComposingManager = { composingManager },
                smartbarManager = smartbarManager,
            )
    }

    /**
     * Loads [KeyboardLayoutData] for [mode] off the main thread, then
     * publishes it into [_keyboardUi] so the Composable can render. Replaces
     * the legacy `addKeyboardView(mode)` path that constructed a
     * [com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView] per
     * mode and added it to the [android.widget.ViewFlipper].
     */
    private suspend fun ensureLayoutLoaded(mode: KeyboardMode) {
        if (mode == KeyboardMode.CLIPBOARD) return
        if (_keyboardUi.value.layouts.containsKey(mode)) return
        val computed = withContext(Dispatchers.IO) {
            layoutManager.fetchComputedLayout(mode, taigikeyboard.activeSubtype)
        }
        val data = KeyboardLayoutData.from(computed)
        publishLayout(mode, data)
    }

    private fun publishLayout(
        mode: KeyboardMode,
        data: KeyboardLayoutData,
    ) {
        _keyboardUi.update { current ->
            if (current.layouts[mode] == data) {
                current
            } else {
                current.copy(layouts = current.layouts + (mode to data))
            }
        }
    }

    private fun setActiveMode(mode: KeyboardMode) {
        _keyboardUi.update { current ->
            if (current.activeMode == mode) current else current.copy(activeMode = mode)
        }
    }

    override fun onRegisterInputView(inputView: InputView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onRegisterInputView(inputView)")

        // All Main-thread setup runs synchronously so `TaigiKeyboard.onWindowShown`
        // (which fires after this returns) sees `textViewGroup` already populated.
        // Without this, `setActiveInput(R.id.text_input)` would race the
        // background coroutine, find `textViewGroup == null`, and
        // `mainViewFlipper.indexOfChild(null) == -1` would wrap to the last
        // child (`media_input`) — visible as the emoji keyboard appearing on
        // first install. Pins
        // `INVARIANT_keyboard_register_input_view_main_thread_setup`.
        hostView = inputView
        popupHost.attachHostView(inputView)
        popupHost.installPopupViewTreeOwnersIfNeeded()

        textViewGroup = inputView.findViewById(R.id.text_input)
        mountKeyboardComposeView(inputView)

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

        // Layout fetch is the only piece that needs IO — keep it async.
        launch(Dispatchers.Default) {
            val activeKeyboardMode = getActiveKeyboardMode()
            ensureLayoutLoaded(activeKeyboardMode)
            withContext(Dispatchers.Main) {
                pushAppearance()
                setActiveMode(activeKeyboardMode)
            }
        }
    }

    /**
     * Adds a single ComposeView under the smartbar inside `text_input_content`.
     * Replaces the legacy [android.widget.ViewFlipper] of per-mode
     * KeyboardView children; mode switching now flips a state field on
     * [_keyboardUi] instead of swapping View children.
     */
    private fun mountKeyboardComposeView(inputView: InputView) {
        val container = inputView.findViewById<ViewGroup>(R.id.text_input_content) ?: return
        val placeholder = inputView.findViewById<View>(R.id.keyboard_compose_host)
        val composeView = ComposeView(inputView.context).apply {
            id = R.id.keyboard_compose_host
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            )
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            setContent {
                KeyboardImeRoot(
                    uiStateFlow = _keyboardUi,
                    coordinator = coordinator,
                    popupHost = popupHost,
                    onHeightFactorChanged = { factor ->
                        smartbarManager.smartbarView?.setHeightFactor(factor)
                    },
                )
            }
        }
        if (placeholder != null) {
            val index = container.indexOfChild(placeholder)
            container.removeView(placeholder)
            container.addView(composeView, index)
        } else {
            container.addView(composeView)
        }
        composeHost = composeView
    }

    override fun onDestroy() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onDestroy()")

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
        val keyboardMode =
            when (info) {
                null -> {
                    KeyboardMode.CHARACTERS
                }

                else -> {
                    when (info.inputType and InputType.TYPE_MASK_CLASS) {
                        InputType.TYPE_CLASS_NUMBER -> {
                            keyVariation = KeyVariation.NORMAL
                            KeyboardMode.NUMERIC
                        }

                        InputType.TYPE_CLASS_PHONE -> {
                            keyVariation = KeyVariation.NORMAL
                            KeyboardMode.PHONE
                        }

                        InputType.TYPE_CLASS_TEXT -> {
                            keyVariation =
                                when (info.inputType and InputType.TYPE_MASK_VARIATION) {
                                    InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
                                    InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS,
                                    -> {
                                        KeyVariation.EMAIL_ADDRESS
                                    }

                                    InputType.TYPE_TEXT_VARIATION_PASSWORD,
                                    InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
                                    InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
                                    -> {
                                        KeyVariation.PASSWORD
                                    }

                                    InputType.TYPE_TEXT_VARIATION_URI -> {
                                        KeyVariation.URI
                                    }

                                    else -> {
                                        KeyVariation.NORMAL
                                    }
                                }
                            KeyboardMode.CHARACTERS
                        }

                        else -> {
                            keyVariation = KeyVariation.NORMAL
                            KeyboardMode.CHARACTERS
                        }
                    }
                }
            }
        isComposingEnabled =
            when (keyboardMode) {
                KeyboardMode.NUMERIC,
                KeyboardMode.PHONE,
                KeyboardMode.PHONE2,
                -> false

                else -> keyVariation != KeyVariation.PASSWORD
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
                    )
                } else {
                    null
                }
        }

        capsStateManager.updateCapsState()
        resetComposingText()
        _keyboardUi.update { current ->
            if (current.keyVariation == keyVariation) {
                current
            } else {
                current.copy(keyVariation = keyVariation)
            }
        }
        setActiveKeyboardMode(keyboardMode)
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
        pushAppearance()
    }

    fun getActiveKeyboardMode(): KeyboardMode = activeKeyboardMode

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

    private fun setActiveKeyboardMode(mode: KeyboardMode) {
        val actualMode =
            if (mode == KeyboardMode.CLIPBOARD) {
                KeyboardMode.CHARACTERS
            } else {
                mode
            }

        activeKeyboardMode = actualMode
        if (_keyboardUi.value.layouts.containsKey(actualMode)) {
            setActiveMode(actualMode)
            smartbarManager.activeContainerId = smartbarManager.getPreferredContainerId()
            textViewGroup?.post { measureAndUpdateKeyboardHeight() }
        } else {
            launch(Dispatchers.Default) {
                ensureLayoutLoaded(actualMode)
                withContext(Dispatchers.Main) {
                    setActiveMode(actualMode)
                    smartbarManager.activeContainerId = smartbarManager.getPreferredContainerId()
                    textViewGroup?.post { measureAndUpdateKeyboardHeight() }
                }
            }
        }
    }

    override fun onSubtypeChanged(newSubtype: Subtype) {
        layoutReloadJob?.cancel()
        layoutReloadJob =
            launch {
                val computed = withContext(Dispatchers.IO) {
                    layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, newSubtype)
                }
                publishLayout(KeyboardMode.CHARACTERS, KeyboardLayoutData.from(computed))
                pushAppearance()
            }
    }

    override fun onInputModeChanged(newInputMode: String) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onInputModeChanged($newInputMode)")

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

        layoutReloadJob?.cancel()
        layoutReloadJob =
            launch {
                val computed = withContext(Dispatchers.IO) {
                    layoutManager.fetchComputedLayout(
                        KeyboardMode.CHARACTERS,
                        taigikeyboard.activeSubtype,
                        overrideInputMode = newInputMode,
                    )
                }
                publishLayout(KeyboardMode.CHARACTERS, KeyboardLayoutData.from(computed))
                pushAppearance()
            }
    }

    override fun onKeyboardLayoutTypeChanged(newLayoutType: String) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onKeyboardLayoutTypeChanged($newLayoutType)")

        layoutReloadJob?.cancel()
        layoutReloadJob =
            launch {
                val computed = withContext(Dispatchers.IO) {
                    layoutManager.fetchComputedLayout(KeyboardMode.CHARACTERS, taigikeyboard.activeSubtype)
                }
                publishLayout(KeyboardMode.CHARACTERS, KeyboardLayoutData.from(computed))
                pushAppearance()
            }
    }

    fun reloadCurrentLayout() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "reloadCurrentLayout()")

        val currentMode = activeKeyboardMode
        layoutReloadJob?.cancel()
        layoutReloadJob =
            launch {
                val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()
                val computed = withContext(Dispatchers.IO) {
                    layoutManager.fetchComputedLayout(currentMode, taigikeyboard.activeSubtype, isTranslateSwapped)
                }
                publishLayout(currentMode, KeyboardLayoutData.from(computed))
                pushAppearance()
            }
    }

    fun reloadAllLayoutsInBackground() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "reloadAllLayoutsInBackground()")

        launch {
            val isTranslateSwapped = smartbarManager.getCachedIsTranslateSwapped()
            val modes = _keyboardUi.value.layouts.keys
                .toList()
            for (mode in modes) {
                if (mode != activeKeyboardMode) {
                    val computed = withContext(Dispatchers.IO) {
                        layoutManager.fetchComputedLayout(mode, taigikeyboard.activeSubtype, isTranslateSwapped)
                    }
                    withContext(Dispatchers.Main) {
                        publishLayout(mode, KeyboardLayoutData.from(computed))
                    }
                }
            }
        }
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

    // Clears any residual host composing region without committing its
    // content. Called at session-start + non-composing fallback paths
    // (DELETE / ENTER / NUMERIC-PHONE key) where a stale region could
    // otherwise be silently committed by a bare `finishComposingText()`.
    // Pinned by `INVARIANT_composing_clear_preedit_does_not_commit`
    // (`behavioral-invariants.md` §13).
    private fun resetComposingText(notifyInputConnection: Boolean = true) {
        if (notifyInputConnection) {
            clearHostComposingRegion(taigikeyboard.currentInputConnection)
        }
    }

    /**
     * Handles a [KeyCode.DELETE] event.
     */
    private fun handleDelete() {
        val ic = taigikeyboard.currentInputConnection ?: return

        if (composingManager?.deleteBackward(ic) == true) {
            if (BuildConfig.DEBUG) {
                val rawInput = composingManager?.getRawInput()
                val composingText = composingManager?.getComposingText()
                Log.d(TAG, "[DELETE] deleteBackward=true, rawInput='$rawInput', composingText='$composingText'")
            }
            candidateCoordinator.scheduleDisplayDerivation()
            candidateCoordinator.updateTaigiCandidatesDebounced()
            return
        }
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[DELETE] deleteBackward=false or composingManager=null")
        }

        ic.beginBatchEdit()
        resetComposingText()
        ic.sendKeyEvent(
            KeyEvent(
                KeyEvent.ACTION_DOWN,
                KeyEvent.KEYCODE_DEL,
            ),
        )
        ic.endBatchEdit()

        // English mode: update candidates after backspace
        if (taigikeyboard.prefs.inputMode == "english") {
            candidateCoordinator.updateEnglishCandidatesDebounced()
            return
        }

        // NextWord: re-predict from remaining text
        val textBeforeCursor = ic.getTextBeforeCursor(100, 0)?.toString() ?: ""
        smartbarManager.handleBackspaceForNextWord(textBeforeCursor)
    }

    /**
     * Handles a [KeyCode.ENTER] event.
     */
    private fun handleEnter() {
        val ic = taigikeyboard.currentInputConnection ?: return

        if (composingManager?.isComposing() == true) {
            val committedText = composingManager?.getComposingText() ?: ""
            val capturedRawInput = composingManager?.getRawInput() ?: ""
            composingManager?.commitComposition(ic)
            smartbarManager.clearCandidates()

            val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
            val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

            if (BuildConfig.DEBUG) {
                Log.d(TAG, "[ENTER] composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction")
            }

            if (imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION == 0 &&
                maskedAction in
                listOf(
                    EditorInfo.IME_ACTION_DONE,
                    EditorInfo.IME_ACTION_GO,
                    EditorInfo.IME_ACTION_NEXT,
                    EditorInfo.IME_ACTION_PREVIOUS,
                    EditorInfo.IME_ACTION_SEARCH,
                    EditorInfo.IME_ACTION_SEND,
                )
            ) {
                if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[ENTER] performing action: $maskedAction")
                }
                ic.performEditorAction(maskedAction)
                return
            }

            if (taigikeyboard.prefs.isAutoSpaceEnabled && !taigikeyboard.prefs.isTranslateSwapped) {
                if (!committedText.endsWith("-")) {
                    ic.commitText(" ", 1)
                }
            }

            if (!taigikeyboard.prefs.isTranslateSwapped && committedText.isNotEmpty()) {
                smartbarManager.handleNextWordPrediction(
                    displayText = committedText,
                    committedText = committedText,
                    roman = committedText,
                    rawInput = capturedRawInput,
                )
            }
            return
        }

        resetComposingText()
        val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
        val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[ENTER] non-composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction")
        }

        if (imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION > 0) {
            ic.commitText("\n", 1)
        } else {
            when (maskedAction) {
                EditorInfo.IME_ACTION_DONE,
                EditorInfo.IME_ACTION_GO,
                EditorInfo.IME_ACTION_NEXT,
                EditorInfo.IME_ACTION_PREVIOUS,
                EditorInfo.IME_ACTION_SEARCH,
                EditorInfo.IME_ACTION_SEND,
                -> {
                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[ENTER] performing action: $maskedAction")
                    }
                    ic.performEditorAction(maskedAction)
                }

                else -> {
                    ic.commitText("\n", 1)
                }
            }
        }
    }

    /**
     * Handles a [KeyCode.SHIFT] event.
     */
    private fun handleShift() {
        capsStateManager.handleShift()
    }

    /**
     * Handles a [KeyCode.SPACE] event.
     */
    private fun handleSpace() {
        val ic = taigikeyboard.currentInputConnection ?: return

        // English mode: commit space, clear candidates
        if (taigikeyboard.prefs.inputMode == "english") {
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()
            return
        }

        // TPS mode: space as tone 1/4 syllable boundary marker.
        // If the current syllable has no explicit tone mark, space adds a syllable
        // boundary and stays in composing mode (like Microsoft Zhuyin's space for tone 1).
        // If the syllable already has a tone mark or ends with space, fall through to commit.
        if (taigikeyboard.prefs.keyboardLayoutType == "tps" && composingManager?.isComposing() == true) {
            val rawInput = composingManager?.getRawInput() ?: ""
            val lastChar = rawInput.lastOrNull()
            if (lastChar != null && !RustEngineBridge.isTpsToneMark(lastChar) && lastChar != ' ') {
                composingManager?.appendCharacter(" ", ic)
                candidateCoordinator.scheduleDisplayDerivation()
                candidateCoordinator.updateTaigiCandidatesDebounced()
                return
            }
        }

        if (composingManager?.isComposing() == true) {
            val committedText = composingManager?.getComposingText() ?: ""
            val capturedRawInput = composingManager?.getRawInput() ?: ""
            composingManager?.commitComposition(ic)
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()

            if (committedText.isNotEmpty()) {
                smartbarManager.handleNextWordPrediction(
                    displayText = committedText,
                    committedText = committedText,
                    roman = committedText,
                    rawInput = capturedRawInput,
                )
            }
            return
        }

        if (taigikeyboard.prefs.doubleSpacePeriod) {
            if (capsStateManager.hasSpaceRecentlyPressed) {
                osHandler.removeCallbacksAndMessages(null)
                val text = ic.getTextBeforeCursor(2, 0) ?: ""
                if (text.length == 2 && !text.matches(DOUBLE_SPACE_PERIOD_REGEX)) {
                    ic.deleteSurroundingText(1, 0)
                    ic.commitText(".", 1)
                }
                capsStateManager.hasSpaceRecentlyPressed = false
            } else {
                capsStateManager.hasSpaceRecentlyPressed = true
                osHandler.postDelayed({
                    capsStateManager.hasSpaceRecentlyPressed = false
                }, 300)
            }
        }
        ic.commitText(KeyCode.SPACE.toChar().toString(), 1)
    }

    /**
     * Main logic point for sending a key press.
     */
    fun sendKeyPress(keyData: KeyData) {
        taigikeyboard.compositionRoot.logger.tdebug(TAG) {
            "[SEND] fn=sendKeyPress code=${keyData.code} label='${keyData.label}' type=${keyData.type}"
        }
        val ic = taigikeyboard.currentInputConnection

        when (keyData.code) {
            KeyCode.DELETE -> {
                handleDelete()
            }

            KeyCode.ENTER -> {
                handleEnter()
            }

            KeyCode.LANGUAGE_SWITCH -> {
                taigikeyboard.switchToNextInputMethod()
            }

            KeyCode.SETTINGS -> {
                taigikeyboard.launchSettings()
            }

            KeyCode.SHIFT -> {
                handleShift()
            }

            KeyCode.SHOW_INPUT_METHOD_PICKER -> {
                val im =
                    taigikeyboard.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
                im.showInputMethodPicker()
            }

            KeyCode.SWITCH_TO_MEDIA_CONTEXT -> {
                taigikeyboard.setActiveInput(R.id.media_input)
            }

            KeyCode.SWITCH_TO_TEXT_CONTEXT -> {
                taigikeyboard.setActiveInput(R.id.text_input)
            }

            KeyCode.VIEW_CHARACTERS -> {
                setActiveKeyboardMode(KeyboardMode.CHARACTERS)
            }

            KeyCode.VIEW_NUMERIC -> {
                setActiveKeyboardMode(KeyboardMode.NUMERIC)
            }

            KeyCode.VIEW_NUMERIC_ADVANCED -> {
                if (taigikeyboard.prefs.isTranslateSwapped && activeKeyboardMode == KeyboardMode.SYMBOLS) {
                    ic?.beginBatchEdit()
                    ic?.commitText("、", 1)
                    ic?.endBatchEdit()
                } else {
                    setActiveKeyboardMode(KeyboardMode.NUMERIC_ADVANCED)
                }
            }

            KeyCode.VIEW_PHONE -> {
                setActiveKeyboardMode(KeyboardMode.PHONE)
            }

            KeyCode.VIEW_PHONE2 -> {
                setActiveKeyboardMode(KeyboardMode.PHONE2)
            }

            KeyCode.VIEW_SYMBOLS -> {
                setActiveKeyboardMode(KeyboardMode.SYMBOLS)
            }

            KeyCode.VIEW_SYMBOLS2 -> {
                setActiveKeyboardMode(KeyboardMode.SYMBOLS2)
            }

            KeyCode.VIEW_CLIPBOARD -> {
                setActiveKeyboardMode(KeyboardMode.CLIPBOARD)
            }

            KeyCode.TRANSLATE -> {
                smartbarManager.toggleTranslateSwapped()
            }

            else -> {
                ic?.beginBatchEdit()
                when (activeKeyboardMode) {
                    KeyboardMode.NUMERIC,
                    KeyboardMode.NUMERIC_ADVANCED,
                    KeyboardMode.PHONE,
                    KeyboardMode.PHONE2,
                    -> {
                        resetComposingText()
                        when (keyData.type) {
                            KeyType.CHARACTER,
                            KeyType.NUMERIC,
                            -> {
                                val text = keyData.code.toChar().toString()
                                ic?.commitText(text, 1)
                            }

                            else -> {
                                when (keyData.code) {
                                    KeyCode.PHONE_PAUSE,
                                    KeyCode.PHONE_WAIT,
                                    -> {
                                        val text = keyData.code.toChar().toString()
                                        ic?.commitText(text, 1)
                                    }
                                }
                            }
                        }
                    }

                    else -> {
                        when (keyData.type) {
                            KeyType.CHARACTER -> {
                                when (keyData.code) {
                                    KeyCode.SPACE -> {
                                        handleSpace()
                                    }

                                    KeyCode.URI_COMPONENT_TLD -> {
                                        if (composingManager?.isComposing() == true) {
                                            composingManager?.commitComposition(ic)
                                            smartbarManager.clearCandidates()
                                        }
                                        val tld =
                                            when (caps) {
                                                true -> keyData.label.uppercase(Locale.getDefault())
                                                false -> keyData.label.lowercase(Locale.getDefault())
                                            }
                                        ic?.commitText(tld, 1)
                                    }

                                    else -> {
                                        handleTaigiInput(keyData)
                                        ic?.endBatchEdit()
                                        return
                                    }
                                }
                            }

                            else -> {
                                if (BuildConfig.DEBUG) {
                                    Log.e(
                                        this::class.simpleName,
                                        "sendKeyPress(keyData): Received unknown key: $keyData",
                                    )
                                }
                            }
                        }
                    }
                }
                ic?.endBatchEdit()
            }
        }
    }

    /**
     * Handle Taigi character input.
     */
    private fun handleTaigiInput(keyData: KeyData) {
        val inputStart = System.currentTimeMillis()
        val ic = taigikeyboard.currentInputConnection ?: return
        taigikeyboard.compositionRoot.logger.tdebug(TAG) {
            "[TAIGI] fn=handleTaigiInput code=${keyData.code} label='${keyData.label}'"
        }

        val baseText =
            if (keyData.label.isNotEmpty() &&
                keyData.label != keyData.code.toChar().toString()
            ) {
                keyData.label
            } else {
                keyData.code.toChar().toString()
            }

        val inputMode = InputMode.fromPrefString(taigikeyboard.prefs.inputMode)
        // Per-keystroke (not per-frame) — no cache needed; direct bridge call.
        var char = CaseTransformBridge.transformInputCase(
            text = baseText,
            letterCase = CaseTransformBridge.LetterCase.from(caps = caps, capsLock = capsLock),
            mode = inputMode,
        )

        // TPS layout: context-aware character adjustments via CharacterInputPipeline
        // (mirrors iOS CharacterInputPipeline.adjust collapsed entry point).
        if (taigikeyboard.prefs.keyboardLayoutType == "tps") {
            val rawInput = composingManager?.getRawInput() ?: ""
            val adjustment = CharacterInputPipeline.adjust(char, rawInput)
            char = adjustment.char
            adjustment.replaceLast?.let { composingManager?.replaceLastCharacter(it, ic) }
        }

        // English mode: commit directly
        if (taigikeyboard.prefs.inputMode == "english") {
            ic.commitText(char, 1)
            if (caps && !capsLock) {
                capsStateManager.resetSingleShift()
            }
            candidateCoordinator.updateEnglishCandidatesDebounced()
            return
        }

        val manager = composingManager ?: return

        // Standalone digit: commit directly without entering composing mode.
        // Digits only enter composing as tone markers appended to existing romanization.
        if (!manager.isComposing() && char.length == 1 && char[0].isDigit()) {
            ic.commitText(char, 1)
            if (smartbarManager.isShowingNextWordCandidates()) {
                smartbarManager.clearCandidates()
            }
            return
        }

        // 組字字元（字母、TPS 符號、連字符號、˙）→ 進入組字
        if (isComposingCharacter(char)) {
            if (manager.isComposing()) {
                if (char == "-") {
                    manager.appendHyphen(ic)
                } else {
                    manager.appendCharacter(char, ic)
                }
                if (taigikeyboard.prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
                candidateCoordinator.scheduleDisplayDerivation()
                if (BuildConfig.DEBUG) {
                    Log.d(
                        "PERF",
                        "[1] handleTaigiInput composing: ${System.currentTimeMillis() - inputStart}ms",
                    )
                }
                candidateCoordinator.updateTaigiCandidatesDebounced()
            } else {
                if (char == "-" && smartbarManager.isShowingNextWordCandidates()) {
                    ic.commitText("-", 1)
                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[INPUT] '-' committed in NextWord mode, keeping suggestions")
                    }
                } else {
                    manager.startComposing(char, ic)
                    if (taigikeyboard.prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
                    candidateCoordinator.scheduleDisplayDerivation()
                    if (BuildConfig.DEBUG) {
                        Log.d(
                            "PERF",
                            "[1] handleTaigiInput newComposing: ${System.currentTimeMillis() - inputStart}ms",
                        )
                    }
                    candidateCoordinator.updateTaigiCandidatesDebounced()
                }
            }
        } else if (manager.isComposing() && char.length == 1 && char[0].isDigit()) {
            // 組字中輸入數字 → 作為聲調標記追加
            manager.appendCharacter(char, ic)
            if (taigikeyboard.prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
            candidateCoordinator.scheduleDisplayDerivation()
            if (BuildConfig.DEBUG) {
                Log.d(
                    "PERF",
                    "[1] handleTaigiInput composing digit: ${System.currentTimeMillis() - inputStart}ms",
                )
            }
            candidateCoordinator.updateTaigiCandidatesDebounced()
        } else {
            // 非組字字元（標點、符號、箭頭等）→ 確認組字後直接輸出
            if (manager.isComposing()) {
                manager.commitComposition(ic)
            }
            ic.commitText(char, 1)
            smartbarManager.clearCandidates()
            return
        }
    }

    /**
     * Check if the character should enter composition mode (allowlist).
     *
     * Only romanization letters, TPS bopomofo, TPS tone marks,
     * hyphen (syllable boundary), and ˙ (U+02D9, TPS tone 8) enter composing.
     * Everything else (punctuation, symbols, arrows, emoji, etc.) commits directly.
     */
    private fun isComposingCharacter(char: String): Boolean {
        val first = char.firstOrNull() ?: return false
        // isLetter() covers: a-z, A-Z (Lu/Ll), TPS bopomofo ㄅ-ㆷ (Lo),
        // TPS tone marks ˋ ˊ ˇ ˆ (Lm).
        // Three TPS tone marks are Sk (Symbol, modifier), not caught by isLetter:
        //   ˪ (U+02EA, tone 3), ˫ (U+02EB, tone 7), ˙ (U+02D9, tone 8)
        return first.isLetter() ||
            first == '-' ||
            first == '˪' ||
            first == '˫' ||
            first == '˙'
    }

    // --- Appearance + key-event dispatch helpers --------------------------------------

    private fun resolveColorSettings(): KeyboardColorSettings {
        val json = prefs.colorSettings
        if (json != cachedColorSettingsJson) {
            cachedColorSettingsJson = json
            cachedColorSettings = KeyboardColorSettings.fromJson(json)
        }
        return cachedColorSettings
    }

    private fun resolveTypeface(): android.graphics.Typeface {
        val fontType = prefs.fontType
        if (fontType != cachedFontType) {
            cachedFontType = fontType
            cachedTypeface = TypefaceLoader.getTypefaceByType(fontType, taigikeyboard)
        }
        return cachedTypeface
    }

    private fun currentAppearance(): KeyboardAppearance {
        val isComposing = synchronized(composingLock) { composingManager?.isComposing() == true }
        return KeyboardAppearance(
            keyboardLayoutType = prefs.keyboardLayoutType,
            inputMode = prefs.inputMode,
            caps = capsStateManager.caps,
            capsLock = capsStateManager.capsLock,
            isComposing = isComposing,
            isTranslateSwapped = if (::smartbarManager.isInitialized) {
                smartbarManager.getCachedIsTranslateSwapped()
            } else {
                prefs.isTranslateSwapped
            },
            imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0,
            confirmKeyLabel = SettingsTexts.confirmKeyLabel(prefs.inputMode, prefs.isTranslateSwapped),
            colorSettings = resolveColorSettings(),
            typeface = resolveTypeface(),
            keyFontSizeScale = prefs.keyFontSizeScale,
            keyCornerRadius = prefs.keyCornerRadius,
            keyBorderWidth = prefs.keyBorderWidth,
            heightFactor = KeyboardHeightFactor.fromPreferenceString(prefs.heightFactor),
            keyHeightScale = prefs.keyHeightScale,
        )
    }

    /** Republishes the latest appearance snapshot. Equality on the
     *  [KeyboardAppearance] data class collapses no-op refreshes (e.g.,
     *  back-to-back `onWindowShown` + `invalidateAllKeys`) into a single
     *  StateFlow value, so downstream Compose recomposition only fires when
     *  visuals actually change. */
    private fun pushAppearance() {
        val next = currentAppearance()
        _keyboardUi.update { current ->
            if (current.appearance == next) current else current.copy(appearance = next)
        }
    }

    /** Adapter the keyboard body uses to call back into the IME service.
     *  Built once per [TextInputManager] instance. */
    private inner class ImeKeyEventDispatcher : KeyEventDispatcher {
        override fun dispatchKeyPress(data: KeyData) = sendKeyPress(data)

        override fun showInputMethodPicker() {
            val im = taigikeyboard.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
            im.showInputMethodPicker()
        }

        override fun keyPressVibrate() {
            if (!prefs.isVibrationFeedbackEnabled) return
            hostView?.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
        }

        override fun keyPressSound(data: KeyData) = taigikeyboard.keyPressSound(data)

        override val longPressDelayMs: Long
            get() = prefs.longPressDelay.toLong()

        override fun resolveAnchor(
            bounds: KeyBounds,
            keyboardWidth: Int,
            desiredKeyWidth: Int,
            desiredKeyHeight: Int,
        ): KeyAnchor {
            // [composeHost] is the keyboard ComposeView; its window-coords +
            // key-relative offset give the absolute window position needed by
            // `PopupWindow.showAtLocation`.
            composeHost?.getLocationInWindow(locationScratch)
            val anchorTopXInWindow = locationScratch[0] + bounds.visible.left
            val anchorTopYInWindow = locationScratch[1] + bounds.visible.top

            val isLandscape = taigikeyboard.resources.configuration.orientation ==
                android.content.res.Configuration.ORIENTATION_LANDSCAPE
            val computedLabel = computeKeyLetter(
                bounds.data,
                prefs.inputMode,
                capsStateManager.caps,
                capsStateManager.capsLock,
            )
            // Skip popup-cell resolution when this key has no popup variants
            // — avoids the ~80% of presses that never trigger a long-press
            // extend. The empty list is safe because [KeyTouchCoordinator]
            // gates the long-press path on `data.popup.isNotEmpty()` already.
            val popupCells = if (bounds.data.popup.isEmpty()) {
                emptyList()
            } else {
                buildPopupCells(
                    bounds.data,
                    prefs.inputMode,
                    capsStateManager.caps,
                    capsStateManager.capsLock,
                    taigikeyboard.resources,
                )
            }

            return KeyAnchor(
                data = bounds.data,
                measuredWidth = bounds.visible.width,
                measuredHeight = bounds.visible.height,
                xInKeyboard = bounds.visible.left,
                keyboardWidth = keyboardWidth,
                xInWindow = anchorTopXInWindow,
                yInWindow = anchorTopYInWindow,
                computedLabel = computedLabel,
                popupCells = popupCells,
                isLandscape = isLandscape,
                desiredKeyWidth = desiredKeyWidth,
                desiredKeyHeight = desiredKeyHeight,
            )
        }
    }
}

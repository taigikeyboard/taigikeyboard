// 中文: IME 輸入主編排器 — 管理 keyboard mode、layout、popup、composing、smartbar、
// 中文: 候選 debounce、Caps 狀態整合等。對應 iOS 端 KeyboardController 的角色,
// 中文: 同時也是 TaigiKeyboard.EventListener,負責把按鍵事件轉成 Rust 引擎呼叫與 UI 更新。

package com.siansiansu.taigikeyboard.ime.text

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.CursorAnchorInfo
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.engine.CaseTransformBridge
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.Subtype
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.popup.KeyPopupManager
import com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager
import com.siansiansu.taigikeyboard.ime.text.composing.clearHostComposingRegion
import com.siansiansu.taigikeyboard.ime.text.composing.hostReportsNoComposingRegion
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.ImeKeyEventDispatcher
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyTouchCoordinator
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardUiCoordinator
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardUiState
import com.siansiansu.taigikeyboard.ime.text.layout.LayoutManager
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.Locale

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

    /** Lazy view used both as the popup `showAtLocation` parent and as the
     *  reference for vibration haptics. Bound in [onRegisterInputView]. */
    private var hostView: View? = null

    /** Cached reference to the keyboard ComposeView host inside [hostView].
     *  Resolved once after [KeyboardUiCoordinator.mountKeyboardComposeView] so
     *  the popup anchor resolution avoids a `findViewById` walk. */
    private var composeHost: View? = null

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
                hostViewProvider = { hostView },
                composeHostProvider = { composeHost },
                onDispatchKeyPress = { data -> sendKeyPress(data) },
            ),
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

    private val logger get() = taigikeyboard.compositionRoot.logger

    override fun onCreate() {
        logger.i(TAG, "onCreate()")

        candidateCoordinator =
            CandidateUpdateCoordinator(
                scope = this,
                taigikeyboard = taigikeyboard,
                getComposingManager = { composingManager },
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
        hostView = inputView
        popupHost.attachHostView(inputView)
        popupHost.installPopupViewTreeOwnersIfNeeded()

        textViewGroup = inputView.findViewById(R.id.text_input)
        composeHost = uiCoordinator.mountKeyboardComposeView(
            inputView = inputView,
            coordinator = coordinator,
            popupHost = popupHost,
            onHeightFactorChanged = { factor ->
                smartbarManager.smartbarView?.setHeightFactor(factor)
            },
        )

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
            val activeKeyboardMode = uiCoordinator.activeKeyboardMode
            uiCoordinator.ensureLayoutLoaded(activeKeyboardMode)
            withContext(Dispatchers.Main) {
                pushAppearance()
                uiCoordinator.publishActiveMode(activeKeyboardMode)
            }
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
        resetComposingText()
        uiCoordinator.publishKeyVariation(keyVariation)
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
        pushAppearance()
    }

    fun getActiveKeyboardMode(): KeyboardMode = uiCoordinator.activeKeyboardMode

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
            logger.debug(TAG) {
                val rawInput = composingManager?.getRawInput()
                val composingText = composingManager?.getComposingText()
                "[DELETE] deleteBackward=true, rawInput='$rawInput', composingText='$composingText'"
            }
            candidateCoordinator.scheduleDisplayDerivation()
            candidateCoordinator.updateTaigiCandidatesDebounced()
            return
        }
        logger.debug(TAG) { "[DELETE] deleteBackward=false or composingManager=null" }

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
            // Model B §10.3: clear the candidate strip + NextWord state
            // BEFORE the commit so the engine's terminal NextWordWordSelected
            // (fired by commitComposition→CommitRaw) SURVIVES. clearCandidates()
            // bumps the NextWord generation; running it AFTER the commit (the
            // old order) stale-drops the engine's in-flight prediction query
            // (= Enter regression). The engine effect is the SOLE
            // association/prediction source — the old manual
            // handleNextWordPrediction double-recorded the association and
            // wasted the engine's prediction. Enter KEEPS the engine
            // prediction (nothing clears between this commit and the async
            // render). clearCandidates() only touches the smartbar strip +
            // NextWord state, never the IC composing region (owned by
            // ComposingManager), so running it pre-commit is safe.
            // 中文: Model B — clearCandidates 移到 commit 之前,讓引擎終端 NextWord
            // 中文: 預測存活(Enter 保留預測);引擎 effect 為關聯唯一來源。
            smartbarManager.clearCandidates()
            composingManager?.commitComposition(ic)

            val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
            val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

            logger.debug(TAG) { "[ENTER] composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction" }

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
                logger.debug(TAG) { "[ENTER] performing action: $maskedAction" }
                ic.performEditorAction(maskedAction)
                return
            }

            if (taigikeyboard.prefs.isAutoSpaceEnabled && !taigikeyboard.prefs.isTranslateSwapped) {
                if (!committedText.endsWith("-")) {
                    ic.commitText(" ", 1)
                }
            }
            // Model B §10.3: NO manual handleNextWordPrediction. The engine's
            // terminal NextWordWordSelected (from commitComposition→CommitRaw
            // above, dispatched via dispatchComposingNextWordEffect) is the
            // SOLE association/prediction source; clearCandidates() was moved
            // before the commit so that engine prediction survives → Enter
            // keeps showing next-word predictions.
            return
        }

        resetComposingText()
        val imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0
        val maskedAction = imeOptions and EditorInfo.IME_MASK_ACTION

        logger.debug(TAG) { "[ENTER] non-composing mode, imeOptions=$imeOptions, maskedAction=$maskedAction" }

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
                    logger.debug(TAG) { "[ENTER] performing action: $maskedAction" }
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
            // Model B §10.3: the engine commit (commitComposition→CommitRaw)
            // fires the terminal NextWordWordSelected → records the
            // association (the SOLE source; the old manual
            // handleNextWordPrediction double-recorded it AND re-showed the
            // prediction — the Model-B Space regression). Space SUPPRESSES the
            // next-word *display*: clearCandidates() runs AFTER the commit
            // (kept order) → bumps the NextWord generation so the engine's
            // in-flight prediction query is dropped stale.
            // 中文: Model B — 引擎 commit 已記關聯(唯一來源);Space 維持 commit 後
            // 中文: clearCandidates 抑制下詞顯示(舊手動呼叫會雙記並重新顯示)。
            composingManager?.commitComposition(ic)
            ic.commitText(" ", 1)
            smartbarManager.clearCandidates()
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
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.CHARACTERS)
            }

            KeyCode.VIEW_NUMERIC -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.NUMERIC)
            }

            KeyCode.VIEW_NUMERIC_ADVANCED -> {
                if (taigikeyboard.prefs.isTranslateSwapped && uiCoordinator.activeKeyboardMode == KeyboardMode.SYMBOLS) {
                    ic?.beginBatchEdit()
                    ic?.commitText("、", 1)
                    ic?.endBatchEdit()
                } else {
                    uiCoordinator.setActiveKeyboardMode(KeyboardMode.NUMERIC_ADVANCED)
                }
            }

            KeyCode.VIEW_PHONE -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.PHONE)
            }

            KeyCode.VIEW_PHONE2 -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.PHONE2)
            }

            KeyCode.VIEW_SYMBOLS -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.SYMBOLS)
            }

            KeyCode.VIEW_SYMBOLS2 -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.SYMBOLS2)
            }

            KeyCode.VIEW_CLIPBOARD -> {
                uiCoordinator.setActiveKeyboardMode(KeyboardMode.CLIPBOARD)
            }

            KeyCode.TRANSLATE -> {
                smartbarManager.toggleTranslateSwapped()
            }

            else -> {
                ic?.beginBatchEdit()
                when (uiCoordinator.activeKeyboardMode) {
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
                                logger.e(TAG, "sendKeyPress(keyData): Received unknown key: $keyData")
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
                logger.debug("PERF") {
                    "[1] handleTaigiInput composing: ${System.currentTimeMillis() - inputStart}ms"
                }
                candidateCoordinator.updateTaigiCandidatesDebounced()
            } else {
                if (char == "-" && smartbarManager.isShowingNextWordCandidates()) {
                    ic.commitText("-", 1)
                    logger.debug(TAG) { "[INPUT] '-' committed in NextWord mode, keeping suggestions" }
                } else {
                    manager.startComposing(char, ic)
                    if (taigikeyboard.prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
                    candidateCoordinator.scheduleDisplayDerivation()
                    logger.debug("PERF") {
                        "[1] handleTaigiInput newComposing: ${System.currentTimeMillis() - inputStart}ms"
                    }
                    candidateCoordinator.updateTaigiCandidatesDebounced()
                }
            }
        } else if (manager.isComposing() && char.length == 1 && char[0].isDigit()) {
            // 組字中輸入數字 → 作為聲調標記追加
            manager.appendCharacter(char, ic)
            if (taigikeyboard.prefs.isToolbarAutoCollapse) smartbarManager.collapseToolbarIfOpen()
            candidateCoordinator.scheduleDisplayDerivation()
            logger.debug("PERF") {
                "[1] handleTaigiInput composing digit: ${System.currentTimeMillis() - inputStart}ms"
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

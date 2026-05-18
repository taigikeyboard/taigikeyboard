// 中文: Smartbar(候選列)管理器 — 候選詞 StateFlow 來源、英文建議、numeric row、Toolbar 容器、
// 中文: NextWord 顯示協調等狀態的 owner;TextInputManager 只往這裡推狀態,不直接寫 UI。

package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.util.Log
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import androidx.core.view.children
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.TraceContext
import com.siansiansu.taigikeyboard.ime.core.logging.TraceId
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.dictionary.SuggestionCaseTransformer
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.text.TextInputManager
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.util.concurrent.atomic.AtomicReference

/**
 * Smartbar 管理器
 *
 * 負責管理 Smartbar 的狀態與候選詞顯示
 * 支援動態生成候選詞按鈕，最多顯示 200 個候選詞
 * 候選詞數量由 LexiconService 控制（預設 limit = 200）
 */
class SmartbarManager(
    private val taigikeyboard: TaigiKeyboard,
    private val prefs: PrefHelper,
    private val compositionRoot: CompositionRoot,
    private val textInputManager: TextInputManager,
) : TaigiKeyboard.EventListener {
    private var isComposingEnabled: Boolean = false
    var smartbarView: SmartbarView? = null
        private set
    var candidateOverlayView: CandidateOverlayView? = null
        private set
    var layoutSelectionOverlayView: LayoutSelectionOverlayView? = null
        private set
    var symbolSelectionOverlayView: SymbolSelectionOverlayView? = null
        private set
    var settingsSelectionOverlayView: SettingsSelectionOverlayView? = null
        private set

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    // Current suggestions list (for frequency recording, AtomicReference for thread safety)
    private val currentSuggestionsRef = AtomicReference<List<TaigiWord>>(emptyList())
    private inline var currentSuggestions: List<TaigiWord>
        get() = currentSuggestionsRef.get()
        set(value) {
            currentSuggestionsRef.set(value)
        }

    // Expand/collapse state
    private var isExpanded: Boolean = false
    private var hasCandidates: Boolean = false

    // Keyboard total height (smartbar + keyboard)
    private var keyboardHeight: Int = 0

    // isTranslateSwapped local cache (avoid DataStore async write timing issues)
    private var cachedIsTranslateSwapped: Boolean = false
    private var cachedOutputBothScripts: Boolean = false

    // Compose-side render state. Subsystems that read `currentSuggestions` /
    // `hasCandidates` continue to do so directly; this flow drives only the
    // candidate strip + English 3-col rendering.
    private var candidateUpdateSeq: Long = 0L
    private var cachedColorSettingsJson: String? = null
    private var cachedColorSettings: KeyboardColorSettings? = null
    private val _candidateStripState =
        MutableStateFlow(
            CandidateStripState(
                mode = CandidateMode.Empty,
                display = INITIAL_DISPLAY_PARAMS,
                updateSeq = 0L,
            ),
        )
    val candidateStripState: StateFlow<CandidateStripState> = _candidateStripState.asStateFlow()

    // --- Delegated handlers ---

    private val nextWordHandler =
        NextWordHandler(
            // Use the IME-lifecycle scope so the context-timeout `delay` job
            // is cancelled in `TaigiKeyboard.onDestroy`. A7 also cancels
            // SmartbarManager's own scope in `onDestroy`, but the 30 s
            // NextWord timeout stays on the service scope so cancellation
            // semantics match iOS (`nextword-engine-boundary.md` §13.5).
            scope = taigikeyboard.serviceScope,
            settingsProvider = taigikeyboard.prefs,
            nextWord = compositionRoot.nextWord,
            logger = compositionRoot.logger,
            onUpdateCandidates = { updateCandidates(it) },
            onClearCandidates = { clearCandidates() },
        )

    private val toolbarManager =
        ToolbarManager(
            prefs = prefs,
            smartbarViewProvider = { smartbarView },
            candidateOverlayViewProvider = { candidateOverlayView },
            layoutSelectionOverlayViewProvider = { layoutSelectionOverlayView },
            symbolSelectionOverlayViewProvider = { symbolSelectionOverlayView },
            settingsSelectionOverlayViewProvider = { settingsSelectionOverlayView },
            onInputModeChanged = { mode ->
                when (mode) {
                    "emoji" -> taigikeyboard.setActiveInput(R.id.media_input)
                    "hide_self" -> taigikeyboard.requestHideSelf(0)
                }
            },
            onLayoutSelected = { newLayoutType ->
                textInputManager.onKeyboardLayoutTypeChanged(newLayoutType)
            },
            onActiveContainerChanged = { /* handled by toolbarManager.activeContainerId setter */ },
            getKeyboardHeight = { keyboardHeight },
        )

    private val candidateClickHandler =
        CandidateClickHandler(
            scope = scope,
            prefs = prefs,
            taigikeyboard = taigikeyboard,
            userFreq = compositionRoot.userFreq,
            getCurrentSuggestions = { currentSuggestions },
            getIsTranslateSwapped = { cachedIsTranslateSwapped },
            getOutputBothScripts = { cachedOutputBothScripts },
            getComposingManager = { taigikeyboard.textInputManager.getComposingManager() },
            onClearCandidates = { clearCandidates() },
            onNextWordPrediction = { displayText, committedText, roman, hanzi, rawInput ->
                handleNextWordPrediction(displayText, committedText, roman, hanzi, rawInput)
            },
            onRequestCandidateRefresh = { taigikeyboard.textInputManager.requestTaigiCandidateRefresh() },
        )

    // --- Public delegation API (preserves original interface) ---

    var activeContainerId: Int
        get() = toolbarManager.activeContainerId
        set(value) {
            toolbarManager.activeContainerId = value
        }

    fun isShowingNextWordCandidates(): Boolean = nextWordHandler.isShowingNextWordCandidates()

    fun handleNextWordPrediction(
        displayText: String,
        committedText: String,
        roman: String,
        hanzi: String? = null,
        rawInput: String = "",
    ) = nextWordHandler.handleNextWordPrediction(displayText, committedText, roman, hanzi, rawInput)

    fun updateLastSelectedWord(word: String) = nextWordHandler.updateLastSelectedWord(word)

    fun handleBackspaceForNextWord(textBeforeCursor: String) = nextWordHandler.handleBackspaceForNextWord(textBeforeCursor)

    /**
     * Dispatch a NextWord-shaped composing-engine Effect
     * (`NextWordUpdateLastSelectedWord` / `NextWordWordSelected` /
     * `NextWordClearForNewComposing`) to the underlying [NextWordHandler].
     * Wired into [com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager]
     * via the [com.siansiansu.taigikeyboard.ime.text.composing.NextWordEffectRouter]
     * constructor parameter.
     *
     * Unrelated to [handleNextWordPrediction] which is the platform candidate-
     * tap path for `Phase::Composing` (engine does NOT emit
     * `NextWordWordSelected` there, so the two paths do not double-fire).
     */
    fun dispatchComposingNextWordEffect(effect: com.siansiansu.taigikeyboard.engine.RustEngineBridge.ComposingTransition.Effect) {
        when (effect) {
            is com.siansiansu.taigikeyboard.engine.RustEngineBridge.ComposingTransition.Effect.NextWordUpdateLastSelectedWord -> {
                nextWordHandler.updateLastSelectedWord(
                    word = effect.text,
                    roman = effect.roman.ifEmpty { effect.text },
                )
            }

            is com.siansiansu.taigikeyboard.engine.RustEngineBridge.ComposingTransition.Effect.NextWordWordSelected -> {
                // Final-commit dispatch. Forward `triggerPrediction` exactly —
                // engine's `transition.rs:644-653` emits `true` today but the
                // bridge contract is "verbatim" so a future false must not be
                // silently overridden.
                nextWordHandler.handleEngineWordSelected(
                    text = effect.text,
                    roman = effect.roman.ifEmpty { effect.text },
                    triggerPrediction = effect.triggerPrediction,
                )
            }

            com.siansiansu.taigikeyboard.engine.RustEngineBridge.ComposingTransition.Effect.NextWordClearForNewComposing -> {
                // The composing crate has no nextword dep so it cannot
                // dispatch ClearForNewComposing to the NextWord engine
                // directly — this effect is its request to the platform.
                // Engine-side cleanup is therefore UNCONDITIONAL.
                //
                // The visible-strip clear is conditional: Continuous
                // backspace-pop emits this alongside UpdatePreedit /
                // PerformAutocomplete (engine/composing/tests/continuous_phase.rs)
                // while Taigi composing candidates are still active; an
                // unconditional candidate wipe would cause visible flicker.
                if (nextWordHandler.isShowingNextWordCandidates()) {
                    clearCandidates()
                } else {
                    nextWordHandler.clearNextWordState()
                }
            }

            else -> {
                // Router contract: ComposingManager.applyTransition only
                // forwards the three NextWord-shaped Effects. Reaching this
                // branch means a routing-layer bug. Debug: crash loudly to
                // surface it. Release: the stray effect is dropped so the
                // user's keyboard never crashes; the Log.e below is a
                // best-effort diagnostic that R8 strips under the
                // zero-logs-in-release privacy policy (rules/security-rules.md),
                // so in production this is intentionally swallowed, not surfaced.
                val msg = "NextWordEffectRouter received non-NextWord effect: $effect"
                if (com.siansiansu.taigikeyboard.BuildConfig.DEBUG) {
                    throw IllegalStateException(msg)
                } else {
                    android.util.Log.e("SmartbarManager", msg)
                }
            }
        }
    }

    fun getLastSelectedWord(): String? = nextWordHandler.getLastSelectedWord()

    /**
     * Envelope generation owned by [NextWordHandler]. Accessor retained
     * for NextWord bridge-state sharing; its prior consumer (the platform
     * autocomplete context-boost path) was retired in v3.5.8 Item 13.
     */
    fun getNextwordEnvelopeGeneration(): Long = nextWordHandler.nextwordEnvelopeGeneration()

    fun collapseToolbarIfOpen() = toolbarManager.collapseToolbarIfOpen()

    fun getPreferredContainerId(): Int = toolbarManager.getPreferredContainerId()

    // --- Number row ---

    private val numberRowButtonOnClickListener =
        View.OnClickListener { v ->
            val keyData =
                when (v.id) {
                    R.id.number_row_0 -> KeyData(48, "0")
                    R.id.number_row_1 -> KeyData(49, "1")
                    R.id.number_row_2 -> KeyData(50, "2")
                    R.id.number_row_3 -> KeyData(51, "3")
                    R.id.number_row_4 -> KeyData(52, "4")
                    R.id.number_row_5 -> KeyData(53, "5")
                    R.id.number_row_6 -> KeyData(54, "6")
                    R.id.number_row_7 -> KeyData(55, "7")
                    R.id.number_row_8 -> KeyData(56, "8")
                    R.id.number_row_9 -> KeyData(57, "9")
                    else -> KeyData(0)
                }
            TraceContext.withTrace(TraceId.next()) {
                taigikeyboard.textInputManager.sendKeyPress(keyData)
            }
        }

    companion object {
        private const val TAG = "SmartbarManager"

        private val INITIAL_DISPLAY_PARAMS =
            CandidateDisplayParams(
                isTranslateSwapped = false,
                fontType = "",
                layoutType = "",
                orMapsToER = false,
                textSizeScale = 1.0f,
                candidateTextColor = null,
                candidateBackgroundColor = null,
                themeTitleColor = 0,
                themeSubtitleColor = 0,
                themeKeyBgColor = 0,
                themePressedHighlightColor = 0,
                smartbarHeightPx = 0,
            )
    }

    fun registerSmartbarView(smartbarView: SmartbarView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "registerSmartbarView(smartbarView)")

        this.smartbarView = smartbarView

        val numberRow = smartbarView.findViewById<LinearLayout>(R.id.number_row)
        for (numberRowButton in numberRow.children) {
            if (numberRowButton is Button) {
                numberRowButton.setOnClickListener(numberRowButtonOnClickListener)
            }
        }

        // Expand/collapse button click
        smartbarView.expandToggleButton?.setOnClickListener {
            toggleExpandState()
        }

        // Toolbar toggle and actions (delegated)
        toolbarManager.setupToolbar(smartbarView)

        // Re-publish state with fresh display params so the new Compose strip
        // picks up the latest theme colors / height / font scale immediately
        // after onConfigurationChanged recreates the input view; otherwise
        // stale CandidateDisplayParams persist until the next updateCandidates
        // / updateEnglishCandidates / clearCandidates call.
        pushCandidateState(_candidateStripState.value.mode)
    }

    fun onTaigiCandidateClicked(
        word: TaigiWord,
        index: Int,
    ) {
        candidateClickHandler.handleCandidateClick(word, index)
    }

    fun onEnglishCandidateClicked(index: Int) {
        candidateClickHandler.handleEnglishCandidateClick(index)
    }

    fun registerLayoutSelectionOverlayView(overlayView: LayoutSelectionOverlayView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "registerLayoutSelectionOverlayView(overlayView)")

        this.layoutSelectionOverlayView = overlayView

        overlayView.onLayoutSelected = { newLayoutType ->
            textInputManager.onKeyboardLayoutTypeChanged(newLayoutType)
            layoutSelectionOverlayView?.hide()
            if (prefs.isToolbarAutoCollapse) collapseToolbarIfOpen()
        }
    }

    fun registerSymbolSelectionOverlayView(overlayView: SymbolSelectionOverlayView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "registerSymbolSelectionOverlayView(overlayView)")

        this.symbolSelectionOverlayView = overlayView

        overlayView.onSymbolSelected = { symbol ->
            taigikeyboard.currentInputConnection?.commitText(symbol, 1)
        }
    }

    fun registerSettingsSelectionOverlayView(overlayView: SettingsSelectionOverlayView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "registerSettingsSelectionOverlayView(overlayView)")

        this.settingsSelectionOverlayView = overlayView

        overlayView.onHide = {
            cachedOutputBothScripts = prefs.outputBothScripts
            // Double-tap toggles are live-read by ComposingManager via
            // EngineSettingsProvider.current — no manual refresh needed.
        }

        overlayView.onOpenApp = {
            val context = overlayView.context
            val intent =
                android.content.Intent(
                    context,
                    com.siansiansu.taigikeyboard.settings.SettingsMainActivity::class.java,
                )
            intent.flags = android.content.Intent.FLAG_ACTIVITY_NEW_TASK or
                android.content.Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED or
                android.content.Intent.FLAG_ACTIVITY_CLEAR_TOP
            context.startActivity(intent)
            taigikeyboard.requestHideSelf(0)
            settingsSelectionOverlayView?.hide()
        }
    }

    fun registerCandidateOverlayView(overlayView: CandidateOverlayView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "registerCandidateOverlayView(overlayView)")

        this.candidateOverlayView = overlayView

        overlayView.onCollapse = {
            collapseCandidateView()
        }

        overlayView.onSuggestionSelected = { word, index ->
            candidateClickHandler.handleOverlaySuggestionSelected(word, index)
        }

        overlayView.onTranslateToggle = {
            toggleTranslateSwapped()
        }
    }

    override fun onDestroy() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onDestroy()")

        // A7: cancel the manager-owned scope so CandidateClickHandler jobs do
        // not outlive the IME service instance. Pre-A7 this scope leaked
        // because `SmartbarManager` was a resurrectable companion singleton.
        scope.cancel()

        smartbarView = null
        layoutSelectionOverlayView = null
        symbolSelectionOverlayView = null
        settingsSelectionOverlayView = null
    }

    fun onStartInputView(
        keyboardMode: KeyboardMode,
        isComposingEnabled: Boolean,
    ) {
        this.isComposingEnabled = isComposingEnabled

        // Reset NextWord context (switching input fields)
        nextWordHandler.resetContext()

        // Initialize cache
        cachedIsTranslateSwapped = prefs.isTranslateSwapped
        cachedOutputBothScripts = prefs.outputBothScripts

        when {
            keyboardMode == KeyboardMode.NUMERIC ||
                keyboardMode == KeyboardMode.PHONE ||
                keyboardMode == KeyboardMode.PHONE2 -> {
                smartbarView?.visibility = View.GONE
            }

            else -> {
                smartbarView?.visibility = View.VISIBLE
                layoutSelectionOverlayView?.hide()
                symbolSelectionOverlayView?.hide()
                settingsSelectionOverlayView?.hide()
                activeContainerId = R.id.candidates_container
                toolbarManager.updateInputModeSwitcherState()
            }
        }
    }

    fun onFinishInputView() {
        clearCandidates()
    }

    fun updateCandidates(suggestions: List<TaigiWord>) {
        val view = smartbarView ?: return

        if (suggestions.isEmpty()) {
            clearCandidates()
            return
        }

        val isNextWord = suggestions.firstOrNull()?.id?.let { it < 0 } ?: false
        if (BuildConfig.DEBUG) {
            Log.d(
                TAG,
                "[DEBUG] updateCandidates: count=${suggestions.size}, isNextWord=$isNextWord, first='${suggestions.firstOrNull()?.displayText}'",
            )
        }

        val (caps, capsLock) = textInputManager.getCapsState()
        val composingText = textInputManager.getComposingManager()?.getComposingText() ?: ""
        val inputMode =
            when (prefs.inputMode) {
                "poj" -> InputMode.POJ
                "tl", "tps" -> InputMode.TL
                else -> InputMode.POJ
            }

        val transformedSuggestions =
            SuggestionCaseTransformer.transform(
                suggestions = suggestions,
                composingText = composingText,
                caps = caps,
                capsLock = capsLock,
                inputMode = inputMode,
            )

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[CASE] caps=$caps, capsLock=$capsLock, composingText='$composingText'")
        }

        currentSuggestions = transformedSuggestions
        hasCandidates = true
        nextWordHandler.setShowingNextWord(isNextWord)

        // Switch to candidates view (but don't force-switch from toolbar)
        if (activeContainerId != R.id.candidates_container &&
            activeContainerId != R.id.toolbar_container
        ) {
            activeContainerId = R.id.candidates_container
        }

        view.applyCustomBackgroundColor(colorSettings().candidateBackgroundColor)

        if (BuildConfig.DEBUG) {
            Log.d(
                TAG,
                "[DEBUG] updateCandidates completed: count=${transformedSuggestions.size}, containerVisible=${view.candidatesContainer?.visibility == View.VISIBLE}",
            )
        }

        updateExpandButtonVisibility()

        pushCandidateState(CandidateMode.Taigi(transformedSuggestions))
    }

    fun getCachedIsTranslateSwapped(): Boolean = cachedIsTranslateSwapped

    fun toggleTranslateSwapped() {
        cachedIsTranslateSwapped = !cachedIsTranslateSwapped
        prefs.isTranslateSwapped = cachedIsTranslateSwapped
        cachedOutputBothScripts = prefs.outputBothScripts

        if (currentSuggestions.isNotEmpty()) {
            updateCandidates(currentSuggestions)

            if (isExpanded) {
                candidateOverlayView?.updateSuggestions(currentSuggestions)
            }
        }

        textInputManager.reloadCurrentLayout()
        textInputManager.reloadAllLayoutsInBackground()
        textInputManager.invalidateKeysByCode(
            com.siansiansu.taigikeyboard.ime.text.key.KeyCode.TRANSLATE,
            com.siansiansu.taigikeyboard.ime.text.key.KeyCode.VIEW_NUMERIC_ADVANCED,
        )

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[TRANSLATE] isTranslateSwapped 切換為: $cachedIsTranslateSwapped")
        }
    }

    fun clearCandidates() {
        if (BuildConfig.DEBUG) {
            val stackTrace = Thread.currentThread().stackTrace
            val caller = stackTrace.getOrNull(3)?.methodName ?: "unknown"
            Log.d(TAG, "[DEBUG] clearCandidates() called from: $caller, hadCandidates=$hasCandidates")
        }

        currentSuggestions = emptyList()
        hasCandidates = false
        nextWordHandler.clearNextWordState()

        if (activeContainerId == R.id.candidates_container ||
            activeContainerId == R.id.english_candidates_container
        ) {
            activeContainerId = R.id.candidates_container
        }

        updateExpandButtonVisibility()

        if (isExpanded) {
            collapseCandidateView()
        }

        pushCandidateState(CandidateMode.Empty)
    }

    private fun toggleExpandState() {
        isExpanded = !isExpanded
        smartbarView?.setExpandButtonState(isExpanded)

        if (isExpanded) {
            expandCandidateView()
        } else {
            collapseCandidateView()
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[EXPAND] State toggled: isExpanded=$isExpanded")
        }
    }

    private fun updateExpandButtonVisibility() {
        smartbarView?.setExpandButtonVisible(hasCandidates)
    }

    fun setKeyboardHeight(height: Int) {
        keyboardHeight = height
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[HEIGHT] Keyboard height set to: $height")
        }
    }

    private fun expandCandidateView() {
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[EXPAND] Expanding candidate view")
        }

        val overlay = candidateOverlayView
        if (overlay == null) {
            if (BuildConfig.DEBUG) {
                Log.w(TAG, "[EXPAND] Overlay view not registered")
            }
            return
        }

        layoutSelectionOverlayView?.hide()
        symbolSelectionOverlayView?.hide()
        settingsSelectionOverlayView?.hide()
        overlay.show(currentSuggestions, keyboardHeight)
    }

    private fun collapseCandidateView() {
        isExpanded = false
        smartbarView?.setExpandButtonState(false)
        candidateOverlayView?.hide()

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[EXPAND] Collapsing candidate view")
        }
    }

    fun updateEnglishCandidates(suggestions: List<TaigiWord>) {
        smartbarView ?: return

        if (suggestions.isEmpty()) {
            clearCandidates()
            return
        }

        currentSuggestions = suggestions.take(3)
        hasCandidates = true
        nextWordHandler.clearNextWordState()

        if (activeContainerId != R.id.english_candidates_container) {
            activeContainerId = R.id.english_candidates_container
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[ENGLISH] Updated 3-column candidates: ${currentSuggestions.map { it.roman }}")
        }

        pushCandidateState(CandidateMode.English(currentSuggestions))
    }

    private fun pushCandidateState(mode: CandidateMode) {
        candidateUpdateSeq += 1
        _candidateStripState.value =
            CandidateStripState(
                mode = mode,
                display = currentDisplay(),
                updateSeq = candidateUpdateSeq,
            )
    }

    private fun colorSettings(): KeyboardColorSettings {
        val json = prefs.colorSettings
        val cached = cachedColorSettings
        if (cached != null && cachedColorSettingsJson == json) return cached
        return KeyboardColorSettings.fromJson(json).also {
            cachedColorSettings = it
            cachedColorSettingsJson = json
        }
    }

    private fun currentDisplay(): CandidateDisplayParams {
        val context = taigikeyboard.context
        val colorSettings = colorSettings()
        val height =
            smartbarView?.height?.takeIf { it > 0 }
                ?: context.resources.getDimension(R.dimen.smartbar_height).toInt()
        return CandidateDisplayParams(
            isTranslateSwapped = cachedIsTranslateSwapped,
            fontType = prefs.fontType,
            layoutType = prefs.keyboardLayoutType,
            orMapsToER = prefs.tpsOrMapsToER,
            textSizeScale = prefs.candidateTextSizeScale,
            candidateTextColor = colorSettings.candidateTextColor,
            candidateBackgroundColor = colorSettings.candidateBackgroundColor,
            themeTitleColor = getColorFromAttr(context, R.attr.smartbar_candidate_fgColor),
            themeSubtitleColor = getColorFromAttr(context, R.attr.smartbar_candidate_subtitle_fgColor),
            themeKeyBgColor = getColorFromAttr(context, R.attr.key_bgColor),
            themePressedHighlightColor = getColorFromAttr(context, R.attr.semiTransparentColor),
            smartbarHeightPx = height,
        )
    }
}

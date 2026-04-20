package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.util.Log
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import androidx.core.view.children
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.dictionary.SuggestionCaseTransformer
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels
import com.siansiansu.taigikeyboard.ime.text.TextInputManager
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import java.util.concurrent.atomic.AtomicReference

/**
 * Smartbar 管理器
 *
 * 負責管理 Smartbar 的狀態與候選詞顯示
 * 支援動態生成候選詞按鈕，最多顯示 200 個候選詞
 * 候選詞數量由 LexiconService 控制（預設 limit = 200）
 */
class SmartbarManager private constructor() : TaigiKeyboard.EventListener {
    private val taigikeyboard: TaigiKeyboard = TaigiKeyboard.getInstance()
    private var isComposingEnabled: Boolean = false
    private val textInputManager: TextInputManager = TextInputManager.getInstance()
    private val prefs: PrefHelper by lazy { PrefHelper(taigikeyboard.context) }
    private val compositionRoot: CompositionRoot = CompositionRoot.shared(taigikeyboard)
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

    // RecyclerView Adapter
    private var candidateAdapter: CandidateAdapter? = null

    // --- Delegated handlers ---

    private val nextWordHandler =
        NextWordHandler(
            // Use the IME-lifecycle scope so the context-timeout `delay` job
            // is cancelled in `TaigiKeyboard.onDestroy`. SmartbarManager's
            // own scope is not cancelled in its `onDestroy`, so a pending
            // 30 s timeout would otherwise leak across input sessions
            // (see `nextword-engine-boundary.md` §13.5).
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

    fun getLastSelectedWord(): String? = nextWordHandler.getLastSelectedWord()

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
            taigikeyboard.textInputManager.sendKeyPress(keyData)
        }

    companion object {
        private const val TAG = "SmartbarManager"
        private var instance: SmartbarManager? = null

        @Synchronized
        fun getInstance(): SmartbarManager {
            if (instance == null) {
                instance = SmartbarManager()
            }
            return instance!!
        }
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

        // Initialize RecyclerView and Adapter
        setupCandidateRecyclerView(smartbarView)

        // Expand/collapse button click
        smartbarView.expandToggleButton?.setOnClickListener {
            toggleExpandState()
        }

        // Toolbar toggle and actions (delegated)
        toolbarManager.setupToolbar(smartbarView)

        // English 3-column candidate click events
        setupEnglishCandidates(smartbarView)
    }

    private fun setupCandidateRecyclerView(smartbarView: SmartbarView) {
        val recyclerView = smartbarView.candidatesRecyclerView ?: return

        val layoutManager =
            LinearLayoutManager(
                taigikeyboard.context,
                LinearLayoutManager.HORIZONTAL,
                false,
            )
        recyclerView.layoutManager = layoutManager

        candidateAdapter =
            CandidateAdapter(
                context = taigikeyboard.context,
                isTranslateSwapped = { cachedIsTranslateSwapped },
                fontType = { prefs.fontType },
                layoutType = { prefs.keyboardLayoutType },
                orMapsToER = { prefs.tpsOrMapsToER },
                onCandidateClick = { word, index ->
                    candidateClickHandler.handleCandidateClick(word, index)
                },
            )

        recyclerView.adapter = candidateAdapter
        recyclerView.itemAnimator = null

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[RECYCLER] RecyclerView and Adapter initialized")
        }
    }

    private fun setupEnglishCandidates(smartbarView: SmartbarView) {
        smartbarView.englishCandidate1?.setOnClickListener {
            candidateClickHandler.handleEnglishCandidateClick(0)
        }
        smartbarView.englishCandidate2?.setOnClickListener {
            candidateClickHandler.handleEnglishCandidateClick(1)
        }
        smartbarView.englishCandidate3?.setOnClickListener {
            candidateClickHandler.handleEnglishCandidateClick(2)
        }
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
            TaigiKeyboard.getInstance().currentInputConnection?.commitText(symbol, 1)
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

        smartbarView = null
        layoutSelectionOverlayView = null
        symbolSelectionOverlayView = null
        settingsSelectionOverlayView = null
        instance = null
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
        val adapter = candidateAdapter ?: return

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
                "poj" -> ToneConverterModels.InputMode.POJ
                "tl", "tps" -> ToneConverterModels.InputMode.TL
                else -> ToneConverterModels.InputMode.POJ
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

        val res = taigikeyboard.context.resources
        val smartbarHeight =
            view.height.takeIf { it > 0 }
                ?: res.getDimension(R.dimen.smartbar_height).toInt()
        adapter.setTextSizeScale(prefs.candidateTextSizeScale)
        val colorSettings =
            com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
                .fromJson(prefs.colorSettings)
        adapter.setCustomTextColor(colorSettings.candidateTextColor)
        view.applyCustomBackgroundColor(colorSettings.candidateBackgroundColor)
        adapter.setTextSize(smartbarHeight)

        adapter.submitList(transformedSuggestions) {
            view.resetCandidateScrollPosition()
        }

        if (BuildConfig.DEBUG) {
            Log.d(
                TAG,
                "[DEBUG] updateCandidates completed: itemCount=${adapter.itemCount}, containerVisible=${view.candidatesContainer?.visibility == View.VISIBLE}",
            )
        }

        updateExpandButtonVisibility()
    }

    fun getCachedIsTranslateSwapped(): Boolean = cachedIsTranslateSwapped

    fun toggleTranslateSwapped() {
        cachedIsTranslateSwapped = !cachedIsTranslateSwapped
        prefs.isTranslateSwapped = cachedIsTranslateSwapped
        cachedOutputBothScripts = prefs.outputBothScripts

        if (currentSuggestions.isNotEmpty()) {
            updateCandidates(currentSuggestions)
            candidateAdapter?.notifyDataSetChanged()

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

        smartbarView?.resetCandidateScrollPosition()
        candidateAdapter?.submitList(emptyList())

        if (activeContainerId == R.id.candidates_container ||
            activeContainerId == R.id.english_candidates_container
        ) {
            activeContainerId = R.id.candidates_container
        }

        updateExpandButtonVisibility()

        if (isExpanded) {
            collapseCandidateView()
        }
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
        val view = smartbarView ?: return

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

        val res = taigikeyboard.context.resources
        val smartbarHeight =
            view.height.takeIf { it > 0 }
                ?: res.getDimension(R.dimen.smartbar_height).toInt()
        val englishTextSizePx = smartbarHeight * 0.36f
        val scaledDensity = res.displayMetrics.density * res.configuration.fontScale
        val englishTextSizeSp = englishTextSizePx / scaledDensity

        view.englishCandidate1?.apply {
            textSize = englishTextSizeSp
            text = currentSuggestions.getOrNull(0)?.roman ?: ""
            visibility = if (currentSuggestions.isNotEmpty()) View.VISIBLE else View.INVISIBLE
        }
        view.englishCandidate2?.apply {
            textSize = englishTextSizeSp
            text = currentSuggestions.getOrNull(1)?.roman ?: ""
            visibility = if (currentSuggestions.size > 1) View.VISIBLE else View.INVISIBLE
        }
        view.englishCandidate3?.apply {
            textSize = englishTextSizeSp
            text = currentSuggestions.getOrNull(2)?.roman ?: ""
            visibility = if (currentSuggestions.size > 2) View.VISIBLE else View.INVISIBLE
        }

        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[ENGLISH] Updated 3-column candidates: ${currentSuggestions.map { it.roman }}")
        }
    }
}

// IME service 主類 — 繼承 LifecycleInputMethodService,作為 InputMethodService 的入口點。
// 持有 TextInputManager / SmartbarManager / MediaInputManager / SubtypeManager 等 IME-scoped 元件;
// 透過 EventListener 把按鍵事件發到 TextInputManager,自身負責生命週期與 Compose Recomposer 注入。

package com.siansiansu.taigikeyboard.ime.core

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.media.AudioManager
import android.os.*
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.CursorAnchorInfo
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import androidx.compose.runtime.Recomposer
import androidx.compose.ui.platform.AndroidUiDispatcher
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.compose.ui.viewinterop.AndroidView
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.TaigiKeyboardApplication
import com.siansiansu.taigikeyboard.i18n.DisplayLanguage
import com.siansiansu.taigikeyboard.i18n.StringResolver
import com.siansiansu.taigikeyboard.i18n.buildStringResolver
import com.siansiansu.taigikeyboard.ime.core.logging.debug
import com.siansiansu.taigikeyboard.ime.lifecycle.LifecycleInputMethodService
import com.siansiansu.taigikeyboard.ime.media.MediaInputManager
import com.siansiansu.taigikeyboard.ime.text.TextInputManager
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager
import com.siansiansu.taigikeyboard.settings.SettingsMainActivity
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

class TaigiKeyboard : LifecycleInputMethodService() {
    lateinit var prefs: PrefHelper
        private set

    lateinit var compositionRoot: CompositionRoot
        private set

    val context: Context
        get() = inputView?.context ?: this
    private var inputView: InputView? = null

    private var audioManager: AudioManager? = null
    private var keyPressVibrator: KeyPressVibrator? = null
    private val osHandler = Handler(Looper.getMainLooper())

    /**
     * IME-lifecycle coroutine scope cancelled in [onDestroy]. Exposed so
     * engine wrappers (e.g. [com.siansiansu.taigikeyboard.ime.text.smartbar.NextWordHandler])
     * can launch work that must NOT outlive the input-method service.
     * Marked `internal` to keep the visibility narrow — do not leak the
     * scope outside the app module.
     */
    internal val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    /**
     * Service-scoped Compose [Recomposer] that drives popup `ComposeView`
     * compositions hosted inside [android.widget.PopupWindow]. Set as the
     * `parentCompositionContext` on every popup ComposeView via
     * [com.siansiansu.taigikeyboard.ime.popup.KeyPopupManager], bypassing
     * Compose's default `WindowRecomposerFactory.LifecycleAware` lookup —
     * which fails on PopupWindow because `PopupDecorView` has no
     * `ViewTreeLifecycleOwner` tag set on it.
     *
     * Uses [AndroidUiDispatcher.CurrentThread] so the recomposer ships with
     * a [androidx.compose.runtime.MonotonicFrameClock] for animation /
     * frame-aware Compose APIs. Cancelled in [onDestroy] before the
     * service scope itself.
     */
    internal lateinit var popupRecomposer: Recomposer
        private set
    private var popupRecomposerJob: Job? = null

    /**
     * Set when this IME has just written an auto space, so the space now in
     * front of the caret is known to be OURS — the question the punctuation
     * swap has to answer before it deletes anything
     * ([com.siansiansu.taigikeyboard.ime.text.keyboard.TextInputKeyHandler]).
     *
     * Provenance, not a mode: neither a display mode changed since the commit
     * nor "the output mode would space a word like that" may authorize eating
     * a space the USER typed. Service-scoped because the two collaborators
     * that arm and read it — `CandidateClickHandler` and `TextInputKeyHandler`
     * — are built separately. Mirrors the desktop's `armedAutoSpaceCaret`,
     * minus the caret verification Android has no equivalent query for.
     */
    private var isAutoSpaceArmed = false

    /**
     * The arm as it stood when the current event began. Every event consumes
     * the arm before dispatching ([beginInputEvent]), so a keystroke that
     * writes anything else to the document leaves nothing for the next
     * punctuation key to swap with.
     */
    private var wasAutoSpaceArmedAtEventStart = false

    /** Consumes the auto-space arm for one user event — a key, or a candidate tap. */
    internal fun beginInputEvent() {
        wasAutoSpaceArmedAtEventStart = isAutoSpaceArmed
        isAutoSpaceArmed = false
    }

    /**
     * Re-arms after this IME has written a space the next attaching
     * punctuation may swap with — the auto space itself, and the swap's own
     * re-inserted space so `?!` chains keep swapping.
     */
    internal fun armAutoSpaceSwap() {
        isAutoSpaceArmed = true
    }

    /**
     * Whether the space in front of the caret is one this IME wrote and
     * 自動空白 is still on. The setting is read live so switching the feature
     * off stops the swap; the provenance is the consumed arm.
     */
    internal val isAutoSpaceSwapArmed: Boolean
        get() = wasAutoSpaceArmedAtEventStart && prefs.isAutoSpaceEnabled

    /** Forgets any armed space — a new editor's document is not ours to rewrite. */
    internal fun clearAutoSpaceArm() {
        isAutoSpaceArmed = false
    }

    lateinit var subtypeManager: SubtypeManager
    lateinit var activeSubtype: Subtype

    lateinit var textInputManager: TextInputManager
        private set
    lateinit var smartbarManager: SmartbarManager
        private set
    lateinit var mediaInputManager: MediaInputManager
        private set

    private val navbarManager = NavigationBarManager()

    companion object {
        private const val TAG = "TaigiKeyboard"
        private const val IME_ID: String = "com.siansiansu.taigikeyboard/.ime.core.TaigiKeyboard"

        fun checkIfImeIsEnabled(context: Context): Boolean {
            val logger = CompositionRoot.shared(context).logger
            // Use InputMethodManager API instead of Settings.Secure for Android 14+ compatibility
            val inputMethodManager = context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
            if (inputMethodManager == null) {
                logger.e(TAG, "InputMethodManager is null")
                return false
            }

            val enabledInputMethods = inputMethodManager.enabledInputMethodList
            val isEnabled = enabledInputMethods.any { it.id == IME_ID }

            if (logger.isDebugEnabled) {
                val imeIds = enabledInputMethods.joinToString(":") { it.id }
                logger.i(TAG, "List of enabled IMEs: $imeIds")
                logger.i(TAG, "Is $IME_ID enabled: $isEnabled")
            }

            return isEnabled
        }
    }

    override fun onCreate() {
        if (BuildConfig.DEBUG) {
            StrictMode.setThreadPolicy(
                StrictMode.ThreadPolicy
                    .Builder()
                    .detectDiskReads()
                    .detectDiskWrites()
                    .detectNetwork() // or .detectAll() for all detectable problems
                    .penaltyLog()
                    .build(),
            )
            StrictMode.setVmPolicy(
                StrictMode.VmPolicy
                    .Builder()
                    .detectLeakedSqlLiteObjects()
                    .detectLeakedClosableObjects()
                    .penaltyLog()
                    .penaltyDeath()
                    .build(),
            )
        }

        audioManager = getSystemService(Context.AUDIO_SERVICE) as AudioManager

        // Read prefs + service graph from the Application. Warmup + DataStore
        // migration + custom-dict seed have already been launched by
        // `TaigiKeyboardApplication.onCreate` (per audit §A7 warmup relocation).
        // Prefs migration stays fire-and-forget async — same race profile as
        // the prior `serviceScope.launch { migrateFromSharedPreferences }`,
        // so no observable-behavior change on first-launch reads below.
        val app = application as TaigiKeyboardApplication
        prefs = app.prefs
        compositionRoot = app.compositionRoot
        keyPressVibrator = KeyPressVibrator(this, prefs)

        compositionRoot.logger.i(TAG, "onCreate()")

        subtypeManager = SubtypeManager(prefs)
        activeSubtype = subtypeManager.getActiveSubtype() ?: Subtype.DEFAULT

        // Construct the IME manager graph directly (A7: no more `getInstance()`
        // cycle). Order matters — SmartbarManager ctor needs TextInputManager;
        // TextInputManager reaches SmartbarManager lazily via a `lateinit` set
        // right after SmartbarManager is built.
        textInputManager = TextInputManager(this, prefs)
        smartbarManager = SmartbarManager(this, prefs, compositionRoot, textInputManager)
        textInputManager.smartbarManager = smartbarManager
        mediaInputManager = MediaInputManager(this)

        // Observe inputMode changes and reload keyboard layout
        serviceScope.launch {
            prefs.observeInputMode().collect { newInputMode ->
                compositionRoot.logger.debug(TAG) { "InputMode changed to: $newInputMode" }
                onInputModeChanged(newInputMode)
            }
        }

        // Observe keyboardLayoutType changes and reload keyboard layout
        serviceScope.launch {
            prefs.observeKeyboardLayoutType().collect { newLayoutType ->
                compositionRoot.logger.debug(TAG) { "KeyboardLayoutType changed to: $newLayoutType" }
                onKeyboardLayoutTypeChanged(newLayoutType)
            }
        }

        // Observe display-language changes and refresh the legacy View smartbar /
        // media-input a11y labels. Compose overlays follow the picker via
        // ProvideDisplayLanguage; the non-Compose buttons need this imperative
        // push because the input view is reused across show() (a one-time
        // inflation set would go stale on live-switch). Build the resolver from
        // the flow tag so it reflects the just-selected language.
        serviceScope.launch {
            prefs.observeDisplayLanguage().collect { tag ->
                inputView?.applyAccessibilityStrings(displayLanguageResolver(tag))
            }
        }

        setTheme(R.style.KeyboardTheme)

        AppVersionTracker.updateVersionOnInstallAndLastUse(this, prefs)

        super.onCreate()

        // Bootstrap the popup Recomposer before any popup ComposeView is
        // created. KeyPopupManager is constructed eagerly inside
        // [TextInputManager], so this must be ready before the manager
        // installs popup view-tree owners on first show().
        val popupRecomposerContext = AndroidUiDispatcher.CurrentThread
        popupRecomposer = Recomposer(popupRecomposerContext)
        popupRecomposerJob = serviceScope.launch(
            popupRecomposerContext,
            start = CoroutineStart.UNDISPATCHED,
        ) {
            popupRecomposer.runRecomposeAndApplyChanges()
        }

        textInputManager.onCreate()
        mediaInputManager.onCreate()
    }

    @SuppressLint("InflateParams")
    override fun onCreateInputView(): View? {
        compositionRoot.logger.i(TAG, "onCreateInputView()")

        baseContext.setTheme(R.style.KeyboardTheme)

        // Assign before manager.onCreateInputView() — managers read `inputView` directly.
        val view = layoutInflater.inflate(R.layout.taigikeyboard, null) as InputView
        inputView = view

        // 設定 ViewTree owners 讓 ComposeView 能找到 LifecycleOwner.
        // Bottom inset padding for the keyboard body now lives declaratively
        // inside `KeyboardImeRoot` via `WindowInsets.navigationBars` (Phase D
        // §1b parity-correction); media_input still owns its own padding via
        // `InputView.onApplyWindowInsets`.
        installViewTreeOwners()

        textInputManager.onCreateInputView()
        mediaInputManager.onCreateInputView()

        // 更新導覽列顏色以配合鍵盤主題
        // InputMethodService 需要使用 getWindow().getWindow() 來取得真正的 Window 物件
        getWindow().getWindow()?.let { navbarManager.updateNavigationBar(it, this) }

        // Compose host shell — inner subtrees migrate to native Compose
        // incrementally while the legacy InputView remains the keyboard body.
        return ComposeView(this).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            )
            // `onConfigurationChanged` calls `setInputView` again, detaching this
            // host. Dispose on detach (not on lifecycle destroy) so an old
            // composition does not linger until the IME service ends.
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            setContent {
                AndroidView(factory = { view })
            }
        }
    }

    fun registerInputView(inputView: InputView) {
        compositionRoot.logger.i(TAG, "registerInputView(inputView)")

        this.inputView = inputView

        // Apply a11y labels for the current display language on attach. The
        // observeDisplayLanguage collector emits the current tag on collection
        // start, but that first emission can land while inputView is still null,
        // and the Flow is not a replayed StateFlow — a freshly-inflated smartbar
        // (first show / config change) is never re-served, so it needs this seed.
        inputView.applyAccessibilityStrings(displayLanguageResolver())

        textInputManager.onRegisterInputView(inputView)
        mediaInputManager.onRegisterInputView(inputView)
    }

    /** Builds a [StringResolver] for the display-language [tag] (defaults to the persisted selection). */
    private fun displayLanguageResolver(tag: String = prefs.displayLanguageTag): StringResolver =
        buildStringResolver(this, DisplayLanguage.fromTag(tag))

    override fun onDestroy() {
        compositionRoot.logger.i(TAG, "onDestroy()")

        if (::popupRecomposer.isInitialized) {
            popupRecomposer.cancel()
        }
        popupRecomposerJob?.cancel()
        serviceScope.cancel()
        osHandler.removeCallbacksAndMessages(null)

        super.onDestroy()
        textInputManager.onDestroy()
        mediaInputManager.onDestroy()
    }

    override fun onStartInputView(
        info: EditorInfo?,
        restarting: Boolean,
    ) {
        currentInputConnection?.requestCursorUpdates(InputConnection.CURSOR_UPDATE_MONITOR)

        super.onStartInputView(info, restarting)
        clearAutoSpaceArm()
        textInputManager.onStartInputView(info, restarting)
        mediaInputManager.onStartInputView(info, restarting)
    }

    override fun onFinishInputView(finishingInput: Boolean) {
        currentInputConnection?.requestCursorUpdates(0)

        super.onFinishInputView(finishingInput)
        textInputManager.onFinishInputView(finishingInput)
        mediaInputManager.onFinishInputView(finishingInput)
    }

    override fun onWindowShown() {
        compositionRoot.logger.i(TAG, "onWindowShown()")

        activeSubtype = subtypeManager.getActiveSubtype() ?: Subtype.DEFAULT
        onSubtypeChanged(activeSubtype)
        setActiveInput(R.id.text_input)

        super.onWindowShown()
        textInputManager.onWindowShown()
        mediaInputManager.onWindowShown()
    }

    override fun onWindowHidden() {
        compositionRoot.logger.i(TAG, "onWindowHidden()")

        super.onWindowHidden()
        textInputManager.onWindowHidden()
        mediaInputManager.onWindowHidden()
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        // Handle theme change when system dark mode changes
        val uiModeChanged = (newConfig.diff(resources.configuration) and Configuration.UI_MODE_NIGHT_MASK) != 0
        if (uiModeChanged) {
            // 先更新導覽列顏色，再重建 input view
            // InputMethodService 需要使用 getWindow().getWindow() 來取得真正的 Window 物件
            getWindow().getWindow()?.let { navbarManager.updateNavigationBar(it, this) }
            onCreateInputView()?.let { setInputView(it) }
        }

        super.onConfigurationChanged(newConfig)
        textInputManager.onConfigurationChanged(newConfig)
        mediaInputManager.onConfigurationChanged(newConfig)
    }

    override fun onUpdateCursorAnchorInfo(cursorAnchorInfo: CursorAnchorInfo?) {
        super.onUpdateCursorAnchorInfo(cursorAnchorInfo)
        textInputManager.onUpdateCursorAnchorInfo(cursorAnchorInfo)
        mediaInputManager.onUpdateCursorAnchorInfo(cursorAnchorInfo)
    }

    override fun onUpdateSelection(
        oldSelStart: Int,
        oldSelEnd: Int,
        newSelStart: Int,
        newSelEnd: Int,
        candidatesStart: Int,
        candidatesEnd: Int,
    ) {
        super.onUpdateSelection(
            oldSelStart,
            oldSelEnd,
            newSelStart,
            newSelEnd,
            candidatesStart,
            candidatesEnd,
        )
        textInputManager.onUpdateSelection(
            oldSelStart,
            oldSelEnd,
            newSelStart,
            newSelEnd,
            candidatesStart,
            candidatesEnd,
        )
        mediaInputManager.onUpdateSelection(
            oldSelStart,
            oldSelEnd,
            newSelStart,
            newSelEnd,
            candidatesStart,
            candidatesEnd,
        )
    }

    /**
     * Makes a key press vibration via [KeyPressVibrator] (direct Vibrator, so
     * the app toggle — not the OS touch-haptic setting — decides). Shared by
     * the text-input keyboard body (through
     * [com.siansiansu.taigikeyboard.ime.text.keyboard.ImeKeyEventDispatcher])
     * and the view-based media input (`MediaInputManager` bottom buttons).
     */
    fun keyPressVibrate() {
        keyPressVibrator?.vibrate()
    }

    /**
     * Makes a key press sound.
     * Uses playSoundEffect which automatically respects system settings.
     */
    fun keyPressSound(keyData: KeyData? = null) {
        if (!prefs.isSoundFeedbackEnabled) return
        val effect =
            when (keyData?.code) {
                KeyCode.SPACE -> AudioManager.FX_KEYPRESS_SPACEBAR
                KeyCode.DELETE -> AudioManager.FX_KEYPRESS_DELETE
                KeyCode.ENTER -> AudioManager.FX_KEYPRESS_RETURN
                else -> AudioManager.FX_KEYPRESS_STANDARD
            }
        audioManager?.playSoundEffect(effect)
    }

    /**
     * Hides the IME and launches [SettingsMainActivity].
     */
    fun launchSettings() {
        requestHideSelf(0)
        val intent = Intent(this, SettingsMainActivity::class.java)
        intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK or
            Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED or
            Intent.FLAG_ACTIVITY_CLEAR_TOP
        startActivity(intent)
    }

    /**
     * 切換到系統的下一個輸入法
     */
    fun switchToNextInputMethod() {
        try {
            // switchToNextInputMethod(false) 切換到下一個輸入法
            // 參數 false 表示不只切換到此應用的輸入法
            switchToNextInputMethod(false)
        } catch (e: Exception) {
            compositionRoot.logger.e(TAG, "Failed to switch to next input method", e)
        }
    }

    private fun onSubtypeChanged(newSubtype: Subtype) {
        textInputManager.onSubtypeChanged(newSubtype)
        mediaInputManager.onSubtypeChanged(newSubtype)
    }

    private fun onInputModeChanged(newInputMode: String) {
        textInputManager.onInputModeChanged(newInputMode)
        mediaInputManager.onInputModeChanged(newInputMode)
    }

    private fun onKeyboardLayoutTypeChanged(newLayoutType: String) {
        textInputManager.onKeyboardLayoutTypeChanged(newLayoutType)
        mediaInputManager.onKeyboardLayoutTypeChanged(newLayoutType)
    }

    fun setActiveInput(type: Int) {
        val flipper = inputView?.mainViewFlipper ?: return
        val target = when (type) {
            R.id.text_input -> textInputManager.textViewGroup
            R.id.media_input -> mediaInputManager.mediaViewGroup
            else -> return
        }
        // `indexOfChild` returns -1 when the lookup view is null or not in
        // the flipper. Without this clamp, `ViewAnimator.setDisplayedChild(-1)`
        // wraps to `childCount - 1` and silently flips to the wrong tab —
        // the historical "emoji keyboard on first install" symptom.
        val index = (target?.let { flipper.indexOfChild(it) } ?: -1).coerceAtLeast(0)
        flipper.displayedChild = index
    }

    interface EventListener {
        fun onCreate() {}

        fun onCreateInputView() {}

        fun onRegisterInputView(inputView: InputView) {}

        fun onDestroy() {}

        fun onStartInputView(
            info: EditorInfo?,
            restarting: Boolean,
        ) {}

        fun onFinishInputView(finishingInput: Boolean) {}

        fun onWindowShown() {}

        fun onWindowHidden() {}

        fun onConfigurationChanged(newConfig: Configuration) {}

        fun onUpdateCursorAnchorInfo(cursorAnchorInfo: CursorAnchorInfo?) {}

        fun onUpdateSelection(
            oldSelStart: Int,
            oldSelEnd: Int,
            newSelStart: Int,
            newSelEnd: Int,
            candidatesStart: Int,
            candidatesEnd: Int,
        ) {}

        fun onSubtypeChanged(newSubtype: Subtype) {}

        fun onInputModeChanged(newInputMode: String) {}

        fun onKeyboardLayoutTypeChanged(newLayoutType: String) {}
    }
}

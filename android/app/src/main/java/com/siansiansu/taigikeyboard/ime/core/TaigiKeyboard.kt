
package com.siansiansu.taigikeyboard.ime.core

import android.annotation.SuppressLint
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.media.AudioManager
import android.os.*
import android.provider.Settings
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.view.WindowInsetsController
import android.view.WindowManager
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import android.view.inputmethod.CursorAnchorInfo
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.LinearLayout
import com.squareup.moshi.Json
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.lifecycle.LifecycleInputMethodService
import com.siansiansu.taigikeyboard.ime.media.MediaInputManager
import com.siansiansu.taigikeyboard.ime.text.TextInputManager
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.dictionary.LexiconService
import com.siansiansu.taigikeyboard.ime.text.composing.UserFrequencyService
import com.siansiansu.taigikeyboard.settings.SettingsMainActivity
import com.siansiansu.taigikeyboard.util.*
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

private var taigikeyboardInstance: TaigiKeyboard? = null

class TaigiKeyboard : LifecycleInputMethodService() {
    lateinit var prefs: PrefHelper
        private set

    val context: Context
        get() = inputView?.context ?: this
    private var inputView: InputView? = null

    private var audioManager: AudioManager? = null
    private val osHandler = Handler(Looper.getMainLooper())
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

    lateinit var subtypeManager: SubtypeManager
    lateinit var activeSubtype: Subtype

    val textInputManager: TextInputManager
    val mediaInputManager: MediaInputManager

    private val navbarManager = NavigationBarManager()

    init {
        taigikeyboardInstance = this

        textInputManager = TextInputManager.getInstance()
        mediaInputManager = MediaInputManager.getInstance()
    }

    companion object {
        private const val IME_ID: String = "com.siansiansu.taigikeyboard/.ime.core.TaigiKeyboard"

        fun checkIfImeIsEnabled(context: Context): Boolean {
            // Use InputMethodManager API instead of Settings.Secure for Android 14+ compatibility
            val inputMethodManager = context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
            if (inputMethodManager == null) {
                if (BuildConfig.DEBUG) Log.e(TaigiKeyboard::class.simpleName, "InputMethodManager is null")
                return false
            }

            val enabledInputMethods = inputMethodManager.enabledInputMethodList
            val isEnabled = enabledInputMethods.any { it.id == IME_ID }

            if (BuildConfig.DEBUG) {
                val imeIds = enabledInputMethods.joinToString(":") { it.id }
                Log.i(TaigiKeyboard::class.simpleName, "List of enabled IMEs: $imeIds")
                Log.i(TaigiKeyboard::class.simpleName, "Is $IME_ID enabled: $isEnabled")
            }

            return isEnabled
        }

        fun checkIfImeIsSelected(context: Context): Boolean {
            return try {
                val selectedImeId = Settings.Secure.getString(
                    context.contentResolver,
                    Settings.Secure.DEFAULT_INPUT_METHOD
                )
                if (BuildConfig.DEBUG) Log.i(TaigiKeyboard::class.simpleName, "Selected IME: $selectedImeId")
                selectedImeId == IME_ID
            } catch (e: SecurityException) {
                // DEFAULT_INPUT_METHOD might be restricted in future Android versions
                if (BuildConfig.DEBUG) {
                    Log.e(TaigiKeyboard::class.simpleName, "SecurityException when checking selected IME", e)
                }
                false
            }
        }

        @Synchronized
        fun getInstance(): TaigiKeyboard {
            return taigikeyboardInstance!!
        }
    }

    override fun onCreate() {
        if (BuildConfig.DEBUG) {
            StrictMode.setThreadPolicy(
                StrictMode.ThreadPolicy.Builder()
                    .detectDiskReads()
                    .detectDiskWrites()
                    .detectNetwork() // or .detectAll() for all detectable problems
                    .penaltyLog()
                    .build()
            )
            StrictMode.setVmPolicy(
                StrictMode.VmPolicy.Builder()
                    .detectLeakedSqlLiteObjects()
                    .detectLeakedClosableObjects()
                    .penaltyLog()
                    .penaltyDeath()
                    .build()
            )
        }
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onCreate()")

        audioManager = getSystemService(Context.AUDIO_SERVICE) as AudioManager
        prefs = PrefHelper(this)

        // Migrate from SharedPreferences to DataStore on first launch
        serviceScope.launch {
            prefs.migrateFromSharedPreferences()
        }

        prefs.initDefaultPreferences()
        subtypeManager = SubtypeManager(this, prefs)
        activeSubtype = subtypeManager.getActiveSubtype() ?: Subtype.DEFAULT

        // Observe inputMode changes and reload keyboard layout
        serviceScope.launch {
            prefs.observeInputMode().collect { newInputMode ->
                if (BuildConfig.DEBUG) {
                    Log.d(this@TaigiKeyboard::class.simpleName, "InputMode changed to: $newInputMode")
                }
                onInputModeChanged(newInputMode)
            }
        }

        // Observe phahTaigiLayoutEnabled changes and reload keyboard layout
        serviceScope.launch {
            prefs.observePhahTaigiLayoutEnabled().collect { enabled ->
                Log.d(this@TaigiKeyboard::class.simpleName, "PhahTaigiLayoutEnabled Flow collected: $enabled")
                onPhahTaigiLayoutChanged(enabled)
            }
        }

        setTheme(R.style.KeyboardTheme)

        AppVersionUtils.updateVersionOnInstallAndLastUse(this, prefs)

        // Initialize user frequency service
        UserFrequencyService.init(this)

        super.onCreate()
        textInputManager.onCreate()
        mediaInputManager.onCreate()
    }

    @SuppressLint("InflateParams")
    override fun onCreateInputView(): View? {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onCreateInputView()")

        baseContext.setTheme(R.style.KeyboardTheme)

        inputView = layoutInflater.inflate(R.layout.taigikeyboard, null) as InputView

        // 設定 ViewTree owners 讓 ComposeView 能找到 LifecycleOwner
        installViewTreeOwners()

        // Apply navigation bar insets as margin to inner container
        // Use robust detection following fcitx5-android's approach
        if (BuildConfig.DEBUG) {
            Log.d("TaigiKeyboard", "Setting up WindowInsets listener on inputView")
        }

        ViewCompat.setOnApplyWindowInsetsListener(inputView!!) { v, insets ->
            if (BuildConfig.DEBUG) {
                Log.d("TaigiKeyboard", "=== WindowInsets listener called ===")
            }

            val innerContainer = v.findViewById<LinearLayout>(R.id.inner_input_view_container)

            if (innerContainer != null) {
                // Try multiple sources for navigation bar height
                val navBars = insets.getInsets(WindowInsetsCompat.Type.navigationBars())
                val mandatory = insets.getInsets(WindowInsetsCompat.Type.mandatorySystemGestures())
                val systemGestures = insets.getInsets(WindowInsetsCompat.Type.systemGestures())

                if (BuildConfig.DEBUG) {
                    Log.d("TaigiKeyboard", "Navigation bar detection:")
                    Log.d("TaigiKeyboard", "  navigationBars: ${navBars.bottom}")
                    Log.d("TaigiKeyboard", "  mandatorySystemGestures: ${mandatory.bottom}")
                    Log.d("TaigiKeyboard", "  systemGestures: ${systemGestures.bottom}")
                }

                // Use the maximum value from different sources
                val navBarHeight = maxOf(navBars.bottom, mandatory.bottom, systemGestures.bottom)

                if (navBarHeight > 0) {
                    // 使用 padding 而不是 margin，讓背景可以延伸到導覽列區域
                    innerContainer.setPadding(
                        innerContainer.paddingLeft,
                        innerContainer.paddingTop,
                        innerContainer.paddingRight,
                        navBarHeight
                    )
                    innerContainer.requestLayout()

                    if (BuildConfig.DEBUG) {
                        Log.d("TaigiKeyboard", "  Final height used: $navBarHeight")
                        Log.d("TaigiKeyboard", "  Applied as bottom padding: ${innerContainer.paddingBottom}")
                    }
                } else {
                    if (BuildConfig.DEBUG) {
                        Log.w("TaigiKeyboard", "  WARNING: No navigation bar height detected! All insets are 0")
                    }
                }
            } else {
                if (BuildConfig.DEBUG) {
                    Log.e("TaigiKeyboard", "  ERROR: innerContainer not found!")
                }
            }

            // Don't consume insets - let them propagate
            insets
        }

        textInputManager.onCreateInputView()
        mediaInputManager.onCreateInputView()

        // 更新導覽列顏色以配合鍵盤主題
        // InputMethodService 需要使用 getWindow().getWindow() 來取得真正的 Window 物件
        getWindow().getWindow()?.let { navbarManager.updateNavigationBar(it, this) }

        return inputView
    }

    fun registerInputView(inputView: InputView) {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "registerInputView(inputView)")

        this.inputView = inputView

        textInputManager.onRegisterInputView(inputView)
        mediaInputManager.onRegisterInputView(inputView)
    }

    override fun onDestroy() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onDestroy()")

        osHandler.removeCallbacksAndMessages(null)
        LexiconService.close()
        taigikeyboardInstance = null

        super.onDestroy()
        textInputManager.onDestroy()
        mediaInputManager.onDestroy()
    }

    override fun onStartInputView(info: EditorInfo?, restarting: Boolean) {
        currentInputConnection?.requestCursorUpdates(InputConnection.CURSOR_UPDATE_MONITOR)

        super.onStartInputView(info, restarting)
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
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onWindowShown()")

        activeSubtype = subtypeManager.getActiveSubtype() ?: Subtype.DEFAULT
        onSubtypeChanged(activeSubtype)
        setActiveInput(R.id.text_input)

        super.onWindowShown()
        textInputManager.onWindowShown()
        mediaInputManager.onWindowShown()
    }

    override fun onWindowHidden() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onWindowHidden()")

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
        candidatesEnd: Int
    ) {
        super.onUpdateSelection(
            oldSelStart,
            oldSelEnd,
            newSelStart,
            newSelEnd,
            candidatesStart,
            candidatesEnd
        )
        textInputManager.onUpdateSelection(
            oldSelStart,
            oldSelEnd,
            newSelStart,
            newSelEnd,
            candidatesStart,
            candidatesEnd
        )
        mediaInputManager.onUpdateSelection(
            oldSelStart,
            oldSelEnd,
            newSelStart,
            newSelEnd,
            candidatesStart,
            candidatesEnd
        )
    }

    override fun onComputeInsets(outInsets: Insets?) {
        super.onComputeInsets(outInsets)
        val inputView = this.inputView ?: return

        if (!isInputViewShown) {
            return
        }

        val innerInputViewContainer =
            inputView.findViewById<LinearLayout>(R.id.inner_input_view_container) ?: return

        // Get the actual Y position of the keyboard content in the window
        val location = IntArray(2)
        innerInputViewContainer.getLocationInWindow(location)
        val topInset = location[1]

        // Set insets - use TOUCHABLE_INSETS_VISIBLE to allow app input area to remain touchable
        outInsets?.contentTopInsets = topInset
        outInsets?.visibleTopInsets = topInset
        outInsets?.touchableInsets = Insets.TOUCHABLE_INSETS_VISIBLE

        // if (BuildConfig.DEBUG) {
        //     val screenHeight = resources.displayMetrics.heightPixels
        //     val keyboardBottom = topInset + innerInputViewContainer.height

        //     Log.d("KeyboardInsets", "=== onComputeInsets ===")
        //     Log.d("KeyboardInsets", "Screen height: $screenHeight")
        //     Log.d("KeyboardInsets", "inputView height: ${inputView.height}")
        //     Log.d("KeyboardInsets", "innerContainer location Y: $topInset")
        //     Log.d("KeyboardInsets", "innerContainer height: ${innerInputViewContainer.height}")
        //     Log.d("KeyboardInsets", "innerContainer measuredHeight: ${innerInputViewContainer.measuredHeight}")
        //     Log.d("KeyboardInsets", "innerContainer paddingBottom: ${innerInputViewContainer.paddingBottom}")
        //     Log.d("KeyboardInsets", "Keyboard bottom position: $keyboardBottom")
        //     Log.d("KeyboardInsets", "Extends beyond screen: ${keyboardBottom > screenHeight}")
        //     Log.d("KeyboardInsets", "contentTopInsets: ${outInsets?.contentTopInsets}")
        //     Log.d("KeyboardInsets", "visibleTopInsets: ${outInsets?.visibleTopInsets}")
        // }
    }

    override fun onConfigureWindow(win: Window, isFullscreen: Boolean, isCandidatesOnly: Boolean) {
        super.onConfigureWindow(win, isFullscreen, isCandidatesOnly)

        // Follow fcitx5-android approach: set window to MATCH_PARENT
        win.setLayout(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT
        )

        if (BuildConfig.DEBUG) {
            Log.d("TaigiKeyboard", "onConfigureWindow: MATCH_PARENT x MATCH_PARENT")
        }
    }

    override fun setInputView(view: View) {
        super.setInputView(view)

        // Follow fcitx5-android approach: set inputArea and inputView to MATCH_PARENT
        window?.window?.findViewById<View>(android.R.id.inputArea)?.let { inputArea ->
            inputArea.layoutParams = inputArea.layoutParams.apply {
                height = ViewGroup.LayoutParams.MATCH_PARENT
            }
        }

        view.layoutParams = view.layoutParams?.apply {
            height = ViewGroup.LayoutParams.MATCH_PARENT
        } ?: ViewGroup.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT
        )

        if (BuildConfig.DEBUG) {
            Log.d("TaigiKeyboard", "setInputView: inputArea and view set to MATCH_PARENT")
        }
    }

    override fun updateFullscreenMode() {
        super.updateFullscreenMode()
    }

    /**
     * Makes a key press vibration.
     * Uses performHapticFeedback which automatically respects system settings.
     */
    fun keyPressVibrate(view: View) {
        view.performHapticFeedback(android.view.HapticFeedbackConstants.KEYBOARD_TAP)
    }

    /**
     * Makes a key press sound.
     * Uses playSoundEffect which automatically respects system settings.
     */
    fun keyPressSound(keyData: KeyData? = null) {
        val effect = when (keyData?.code) {
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
        val i = Intent(this, SettingsMainActivity::class.java)
        i.flags = Intent.FLAG_ACTIVITY_NEW_TASK or
                  Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED or
                  Intent.FLAG_ACTIVITY_CLEAR_TOP
        startActivity(i)
    }

    /**
     * @return If the language switch should be shown.
     */
    fun shouldShowLanguageSwitch(): Boolean {
        return subtypeManager.subtypes.size > 1
    }

    fun switchToNextSubtype() {
        activeSubtype = subtypeManager.switchToNextSubtype() ?: Subtype.DEFAULT
        onSubtypeChanged(activeSubtype)
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
            if (BuildConfig.DEBUG) {
                Log.e(this::class.simpleName, "Failed to switch to next input method", e)
            }
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

    private fun onPhahTaigiLayoutChanged(enabled: Boolean) {
        textInputManager.onPhahTaigiLayoutChanged(enabled)
        mediaInputManager.onPhahTaigiLayoutChanged(enabled)
    }

    fun setActiveInput(type: Int) {
        when (type) {
            R.id.text_input -> {
                inputView?.mainViewFlipper?.displayedChild =
                    inputView?.mainViewFlipper?.indexOfChild(textInputManager.textViewGroup) ?: 0
            }
            R.id.media_input -> {
                inputView?.mainViewFlipper?.displayedChild =
                    inputView?.mainViewFlipper?.indexOfChild(mediaInputManager.mediaViewGroup) ?: 0
            }
        }
    }

    interface EventListener {
        fun onCreate() {}
        fun onCreateInputView() {}
        fun onRegisterInputView(inputView: InputView) {}
        fun onDestroy() {}

        fun onStartInputView(info: EditorInfo?, restarting: Boolean) {}
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
            candidatesEnd: Int
        ) {}

        fun onSubtypeChanged(newSubtype: Subtype) {}
        fun onInputModeChanged(newInputMode: String) {}
        fun onPhahTaigiLayoutChanged(enabled: Boolean) {}
    }

    /**
     * Data class which holds the base information for this IME. Matches the structure of
     * ime/config.json so it can be parsed. Used by [SubtypeManager] and by the prefs.
     * NOTE: this class and its corresponding json file is subject to change in future versions.
     * @property packageName The package name of this IME.
     * @property characterLayouts A map of valid layout names to use from. Each value defined
     *  should have a <layout_name>.json file in ime/text/characters/ to avoid empty layouts.
     *  The key is the layout name, the value is the layout label (string shown in UI).
     * @property defaultSubtypes A list of predefined default subtypes. This subtypes are used to
     *  define which locales are supported and which layout is preferred for that locale.
     */
    data class ImeConfig(
        @Json(name = "package")
        val packageName: String,
        val characterLayouts: Map<String, String> = mapOf(),
        val defaultSubtypes: List<DefaultSubtype> = listOf()
    )
}

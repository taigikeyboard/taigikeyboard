package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.util.AttributeSet
import android.view.View
import android.view.WindowInsets
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ViewFlipper
import androidx.core.view.WindowInsetsCompat
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.i18n.StringResolver
import com.siansiansu.taigikeyboard.i18n.generated.StringKey

/**
 * Root keyboard view inflated by `TaigiKeyboard.onCreateInputView`. Hosts the
 * `main_view_flipper` that switches between text input (Compose-rendered
 * keyboard body) and media input (legacy view-based emoji palette).
 *
 * Owns navigation-bar inset padding for the **media input only** — the text
 * input keyboard body resolves bottom insets declaratively inside
 * `KeyboardImeRoot` via Compose `WindowInsets`. Phase D §1b parity-correction
 * retired the imperative `setOnApplyWindowInsetsListener` block that applied
 * padding to the shared inner container; this override keeps media-input
 * navbar handling intact while the text-input path moves to Compose.
 */
class InputView : FrameLayout {
    companion object {
        private const val TAG = "InputView"
    }

    // A7: `InputView` is inflated only inside `TaigiKeyboard.onCreateInputView`,
    // so the constructor `Context` is the IME service itself.
    private val taigikeyboard: TaigiKeyboard
        get() = context as TaigiKeyboard

    var mainViewFlipper: ViewFlipper? = null
        private set

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    override fun onAttachedToWindow() {
        CompositionRoot.shared(context).logger.i(TAG, "onAttachedToWindow()")

        super.onAttachedToWindow()

        mainViewFlipper = findViewById(R.id.main_view_flipper)

        taigikeyboard.registerInputView(this)

        // Force request insets so [onApplyWindowInsets] fires on first
        // attach — required on API 35+ (Android 15) where window-inset
        // dispatch otherwise skips the IME root.
        requestApplyInsets()
    }

    override fun onApplyWindowInsets(insets: WindowInsets): WindowInsets {
        val compat = WindowInsetsCompat.toWindowInsetsCompat(insets, this)
        val navBars = compat.getInsets(WindowInsetsCompat.Type.navigationBars())
        val mandatory = compat.getInsets(WindowInsetsCompat.Type.mandatorySystemGestures())
        val gestures = compat.getInsets(WindowInsetsCompat.Type.systemGestures())
        val navBarHeight = maxOf(navBars.bottom, mandatory.bottom, gestures.bottom)
        // 0.9× factor preserved verbatim from the legacy listener — pins
        // `INVARIANT_keyboard_navbar_inset_padding_factor`. Applied only to
        // `media_input`; the text-input keyboard body computes its own
        // padding via Compose `WindowInsets`.
        val adjusted = (navBarHeight * 0.9f).toInt()
        findViewById<LinearLayout>(R.id.media_input)?.let { mediaRoot ->
            if (mediaRoot.paddingBottom != adjusted) {
                mediaRoot.setPadding(
                    mediaRoot.paddingLeft,
                    mediaRoot.paddingTop,
                    mediaRoot.paddingRight,
                    adjusted,
                )
            }
        }
        return super.onApplyWindowInsets(insets)
    }

    /**
     * Sets the a11y `contentDescription` on the legacy View-based smartbar +
     * media-input buttons from the in-app display-language [resolver], so the
     * labels follow the display-language picker (Hanji/en/ja/TL/POJ) rather than
     * the OS locale. The XML carries no `android:contentDescription` for these
     * buttons; `TaigiKeyboard` applies it here on attach (via
     * `registerInputView`) and re-applies on every display-language change.
     *
     * The Compose candidate/symbol/layout overlays follow the picker through
     * `ProvideDisplayLanguage`; this is the imperative counterpart for the
     * non-Compose smartbar/media buttons.
     */
    fun applyAccessibilityStrings(resolver: StringResolver) {
        fun setDescription(
            viewId: Int,
            key: StringKey,
        ) {
            findViewById<View>(viewId)?.contentDescription = resolver.resolve(key)
        }
        setDescription(R.id.toolbar_toggle_button, StringKey.KEYBOARD_TOGGLE_TOOLBAR)
        setDescription(R.id.toolbar_symbol_button, StringKey.KEYBOARD_SYMBOL_PANEL)
        setDescription(R.id.toolbar_layout_button, StringKey.KEYBOARD_SWITCH_LAYOUT)
        setDescription(R.id.toolbar_globe_button, StringKey.KEYBOARD_SWITCH_INPUT_METHOD)
        setDescription(R.id.toolbar_dismiss_button, StringKey.KEYBOARD_DISMISS_KEYBOARD)
        setDescription(R.id.toolbar_settings_button, StringKey.KEYBOARD_SETTINGS)
        setDescription(R.id.expand_toggle_button, StringKey.KEYBOARD_EXPAND_CANDIDATES)
        setDescription(R.id.media_input_backspace_button, StringKey.KEYBOARD_DELETE_ICON)
    }
}

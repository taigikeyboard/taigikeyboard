// 中文: IME 根 View — 由 TaigiKeyboard.onCreateInputView() inflate;
// 中文: 內含 ViewFlipper 切換文字輸入(Compose 鍵盤)與媒體輸入(舊式 emoji 面板)。
// 中文: navbar inset 只負責「媒體輸入」;文字輸入 inset 走 KeyboardImeRoot 的 Compose WindowInsets。

package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.util.AttributeSet
import android.util.Log
import android.view.WindowInsets
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ViewFlipper
import androidx.core.view.WindowInsetsCompat
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R

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
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onAttachedToWindow()")

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
}
